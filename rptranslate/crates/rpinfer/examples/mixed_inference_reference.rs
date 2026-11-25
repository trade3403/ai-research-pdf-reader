//! Full/cached mixed inference, both integer kernels, and actual text probe.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MarianModel, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_inference::MixedKernels,
    mixed_model::{MixedModel, PAYLOAD_BYTES},
    quantized_linear::Int8Kernel,
    tensor::{Error, Tensor},
};
use rpmodel::{marian::Vocabulary, package, spm::Model};
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn emit(out: &mut impl Write, t: &Tensor) -> io::Result<()> {
    for v in t.data() {
        out.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

fn ids_text(ids: &[u32]) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [packed, fp32] = args.as_slice() else {
        return Err("usage: mixed_inference_reference MIXED FP32".into());
    };
    let root = Path::new(packed);
    let model = MixedModel::load(root, PAYLOAD_BYTES)?;
    let vocab = package::tokenizer_asset(root, "vocab.json").map_err(|e| format!("{e:?}"))?;
    let vocab = Vocabulary::parse(std::str::from_utf8(&vocab).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{e:?}"))?;
    let spm = package::tokenizer_asset(root, "source.spm").map_err(|e| format!("{e:?}"))?;
    let tokenizer = Model::parse(&spm)
        .and_then(|m| m.nmt_unigram())
        .map_err(|e| format!("{e:?}"))?;
    let real_ids = vocab
        .encode(&tokenizer, ">>cmn_Hans<< The result is stable.")
        .map_err(|e| format!("{e:?}"))?;
    let mut out = BufWriter::new(io::stdout().lock());
    let cases = [
        (
            vec![2047, 2048, 4096, 0, 65000],
            vec![65000, 4, 65000, 4096],
            Some(vec![true, true, false, true]),
        ),
        (real_ids.clone(), vec![65000, 4, 4096], None),
    ];
    for (source, prefix, mask) in cases {
        writeln!(out, "SRC|{}", ids_text(&source)).map_err(|e| e.to_string())?;
        let auto = model
            .encode_mixed(&source, None, MixedKernels::default())
            .map_err(|e| format!("{e:?}"))?;
        let scalar = model
            .encode_mixed(
                &source,
                None,
                MixedKernels {
                    integer: Int8Kernel::Scalar,
                    float: Kernel::Scalar,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        let full = auto
            .logits(&prefix, mask.as_deref(), false)
            .map_err(|e| format!("{e:?}"))?;
        let cached = auto
            .cached_logits(&prefix, mask.as_deref())
            .map_err(|e| format!("{e:?}"))?;
        let scalar_logits = scalar
            .logits(&prefix, mask.as_deref(), false)
            .map_err(|e| format!("{e:?}"))?;
        if full.data() != cached.data()
            || full.data() != scalar_logits.data()
            || auto.memory() != scalar.memory()
        {
            return Err("mixed cache or scalar mismatch".into());
        }
        for t in [auto.memory(), &full, &cached, &scalar_logits] {
            emit(&mut out, t).map_err(|e| e.to_string())?;
        }
        if auto.cached_logits(&[], None).err() != Some(Error::Shape)
            || auto.logits(&[65000], Some(&[false]), true).err() != Some(Error::Shape)
            || auto
                .generate(GenerationOptions {
                    beam_width: 16,
                    max_new: 511,
                    mode: GenerationMode::Cached,
                })
                .err()
                != Some(Error::Limit)
        {
            return Err("mixed negative probe failed".into());
        }
        for beam in [1, 4] {
            let opts = GenerationOptions {
                beam_width: beam,
                max_new: 4,
                mode: GenerationMode::Full,
            };
            let full = auto.generate(opts).map_err(|e| format!("{e:?}"))?;
            let cached = auto
                .generate(GenerationOptions {
                    mode: GenerationMode::Cached,
                    ..opts
                })
                .map_err(|e| format!("{e:?}"))?;
            if full != cached {
                return Err("mixed cached search mismatch".into());
            }
            for (rank, h) in cached.iter().enumerate() {
                writeln!(
                    out,
                    "GEN|{beam}|{rank}|{}|{:.17}|{:.17}|{}",
                    ids_text(&h.tokens),
                    h.log_probability,
                    h.score,
                    u8::from(h.ended)
                )
                .map_err(|e| e.to_string())?;
            }
        }
        writeln!(out, "END").map_err(|e| e.to_string())?;
    }
    let opts = GenerationOptions {
        beam_width: 1,
        max_new: 32,
        mode: GenerationMode::Cached,
    };
    let mixed = model
        .encode_mixed(&real_ids, None, MixedKernels::default())
        .and_then(|s| s.generate(opts))
        .map_err(|e| format!("{e:?}"))?;
    drop(model);
    let baseline =
        MarianModel::load(Path::new(fp32), MODEL_PAYLOAD_BYTES).map_err(|e| format!("{e:?}"))?;
    let fp = baseline
        .generate_tokens(&real_ids, None, opts, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    for (name, candidates) in [("mixed", mixed), ("fp32", fp)] {
        let best = candidates.first().ok_or("empty candidates")?;
        let decoded = vocab
            .decode_skipping_special(&best.tokens)
            .map_err(|e| format!("{e:?}"))?;
        let hex = decoded
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        writeln!(
            out,
            "TEXT|{name}|{}|{}|{hex}",
            ids_text(&best.tokens),
            u8::from(best.ended)
        )
        .map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}
