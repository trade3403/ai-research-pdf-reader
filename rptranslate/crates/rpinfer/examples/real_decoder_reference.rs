//! Test probe for real six-layer encoder/decoder and full vocabulary logits.
use rpinfer::{
    decoder::DecoderMask,
    marian_decoder::{MarianDecoder, DECODER_PAYLOAD_BYTES},
    marian_encoder::{MarianEncoder, ENCODER_PAYLOAD_BYTES},
    marian_weights::{MarianEmbeddings, Side},
    matmul::Kernel,
    tensor::{Error, Tensor},
    vocabulary::LoadError,
};
use rpmodel::package;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn write_tensor(out: &mut impl Write, tensor: &Tensor) -> io::Result<()> {
    for &value in tensor.data() {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

struct Case<'a> {
    source: &'a [u32],
    source_keep: Option<&'a [bool]>,
    target: &'a [u32],
    target_keep: Option<&'a [bool]>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let (config, mut reader) =
        package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    if MarianDecoder::load(&mut reader, &config, DECODER_PAYLOAD_BYTES - 1).err()
        != Some(LoadError::Tensor(Error::Limit))
    {
        return Err("decoder budget not enforced".into());
    }
    let decoder = MarianDecoder::load(&mut reader, &config, DECODER_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let encoder = MarianEncoder::load(&mut reader, &config, ENCODER_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let embeddings =
        MarianEmbeddings::load(&mut reader, &config, 135_479_204).map_err(|e| format!("{e:?}"))?;
    let cases = [
        Case {
            source: &[1, 2048, 0],
            source_keep: None,
            target: &[65000],
            target_keep: None,
        },
        Case {
            source: &[2047, 2048, 4095, 4096, 0, 65000],
            source_keep: Some(&[true, true, true, true, true, false]),
            target: &[65000, 4, 65000, 4096],
            target_keep: Some(&[true, true, false, true]),
        },
        Case {
            source: &[
                4, 17, 28, 109, 2047, 2048, 4096, 12991, 32000, 50000, 64999, 0,
            ],
            source_keep: None,
            target: &[65000, 32000, 4096, 2048, 1],
            target_keep: None,
        },
    ];
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    for case in cases {
        let memory = encoder
            .forward(&embeddings, case.source, case.source_keep, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        let masks = DecoderMask {
            source_keep: case.source_keep,
            target_keep: case.target_keep,
        };
        if decoder
            .forward(&embeddings, &[], &memory, masks, Kernel::Auto)
            .err()
            != Some(Error::Shape)
            || decoder
                .forward(
                    &embeddings,
                    &[65000],
                    &memory,
                    DecoderMask {
                        target_keep: Some(&[false]),
                        source_keep: case.source_keep,
                    },
                    Kernel::Auto,
                )
                .err()
                != Some(Error::Shape)
        {
            return Err("invalid decoder input accepted".into());
        }
        write_tensor(&mut out, &memory)?;
        let mut state = embeddings
            .input(Side::Decoder, case.target, 0)
            .map_err(|e| format!("{e:?}"))?;
        write_tensor(&mut out, &state)?;
        for layer in decoder.layers() {
            let self_state = layer
                .self_stage(&state, masks.target_keep, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &self_state)?;
            let cross = layer
                .cross_stage(&self_state, &memory, masks.source_keep, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &cross)?;
            let branch = layer
                .ffn
                .forward(&cross, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            state = layer
                .final_norm
                .residual(&cross, &branch)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &state)?;
        }
        let forward = decoder
            .forward(&embeddings, case.target, &memory, masks, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        if forward != state {
            return Err("decoder trace differs from public forward".into());
        }
        write_tensor(&mut out, &forward)?;
        let all = embeddings
            .project(&forward, false)
            .map_err(|e| format!("{e:?}"))?;
        let last = decoder
            .logits(&embeddings, case.target, &memory, masks, Kernel::Auto, true)
            .map_err(|e| format!("{e:?}"))?;
        if last.data() != &all.data()[(case.target.len() - 1) * config.vocab..] {
            return Err("last-only projection differs".into());
        }
        write_tensor(&mut out, &all)?;
        write_tensor(&mut out, &last)?;
        // Changing future positions must not affect the first causal row.
        let first = decoder
            .forward(
                &embeddings,
                &case.target[..1],
                &memory,
                DecoderMask {
                    target_keep: None,
                    source_keep: case.source_keep,
                },
                Kernel::Auto,
            )
            .map_err(|e| format!("{e:?}"))?;
        for (&a, &b) in first.data().iter().zip(&forward.data()[..config.hidden]) {
            if (a - b).abs() > 1e-5 + 1e-5 * b.abs() {
                return Err("causal prefix mismatch".into());
            }
        }
    }
    out.flush()?;
    eprintln!(
        "REAL_DECODER_LOADED layers=6 tensors=156 payload_bytes={} budget_and_bounds=PASS",
        decoder.payload_bytes()
    );
    Ok(())
}
