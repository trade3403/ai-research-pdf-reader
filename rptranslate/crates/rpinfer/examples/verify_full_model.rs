//! Lifecycle and token-level public entry-point verification with real weights.
use rpinfer::{
    marian_model::{
        GenerationMode, GenerationOptions, MarianModel, ModelError, MODEL_PAYLOAD_BYTES,
    },
    matmul::Kernel,
    tensor::Error,
};
use std::path::Path;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let root = Path::new(&path);
    if MarianModel::load(root, MODEL_PAYLOAD_BYTES - 1).err() != Some(ModelError::Budget) {
        return Err("model budget not enforced".into());
    }
    let model = MarianModel::load(root, MODEL_PAYLOAD_BYTES).map_err(|e| format!("{e:?}"))?;
    if model.payload_bytes() != MODEL_PAYLOAD_BYTES
        || model.default_options()
            != (GenerationOptions {
                beam_width: 4,
                max_new: 511,
                mode: GenerationMode::Cached,
            })
    {
        return Err("wrong model inventory or default generation options".into());
    }
    for (source, mask, expected) in [
        (&[][..], None, Error::Shape),
        (&[65001][..], None, Error::Index),
        (&[65000][..], None, Error::Shape),
        (&[1, 0][..], Some(&[true][..]), Error::Shape),
    ] {
        if model.encode(source, mask, Kernel::Auto).err() != Some(expected) {
            return Err("invalid source accepted".into());
        }
    }
    if model.encode(&[0; 513], None, Kernel::Auto).err() != Some(Error::Limit) {
        return Err("source length limit missing".into());
    }
    let source = [2047, 2048, 0, 65000];
    let mut mask = vec![true, true, true, false];
    let a = model
        .encode(&source, Some(&mask), Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    mask.fill(false);
    let b = model
        .encode(&source, None, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    if a.source_keep() != [true, true, true, false]
        || a.source_keep() != b.source_keep()
        || a.memory() != b.memory()
    {
        return Err("request-local mask ownership or PAD behavior mismatch".into());
    }
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 4,
        mode: GenerationMode::Cached,
    };
    for invalid in [
        GenerationOptions {
            max_new: 0,
            ..options
        },
        GenerationOptions {
            max_new: 512,
            ..options
        },
        GenerationOptions {
            beam_width: 17,
            ..options
        },
    ] {
        if a.generate(invalid).err() != Some(Error::Limit) {
            return Err("invalid generation options accepted".into());
        }
    }
    let prefix = [65000, 4];
    let full = a
        .logits(&prefix, None, false)
        .map_err(|e| format!("{e:?}"))?;
    let cached = a
        .cached_logits(&prefix, None)
        .map_err(|e| format!("{e:?}"))?;
    for (&x, &y) in full.data().iter().zip(cached.data()) {
        if (x - y).abs() > 1e-4 + 1e-4 * x.abs() {
            return Err("public logits mismatch".into());
        }
    }
    let first = a.generate(options).map_err(|e| format!("{e:?}"))?;
    let second = b.generate(options).map_err(|e| format!("{e:?}"))?;
    let direct = model
        .generate_tokens(&source, None, options, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    if first != second || first != direct || first.is_empty() || !first[0].ended {
        return Err("session or direct generation mismatch".into());
    }
    drop(a);
    drop(b);
    drop(model);
    let reloaded = MarianModel::load(root, MODEL_PAYLOAD_BYTES).map_err(|e| format!("{e:?}"))?;
    let after_reload = reloaded
        .generate_tokens(&source, None, options, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    if after_reload != first {
        return Err("reload generation mismatch".into());
    }
    println!(
        "FULL_MODEL PASS tensors=256 payload_bytes={} sessions=2 generation_calls=4 invalid_cases=9 reload=true logits_elements={}",
        reloaded.payload_bytes(),
        full.data().len() + cached.data().len()
    );
    Ok(())
}
