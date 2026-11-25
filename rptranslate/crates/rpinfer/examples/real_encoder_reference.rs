//! Test probe: all six real encoder layers, per-layer post-norm states.
use rpinfer::{
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let (config, mut reader) =
        package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    if MarianEncoder::load(&mut reader, &config, ENCODER_PAYLOAD_BYTES - 1).err()
        != Some(LoadError::Tensor(Error::Limit))
    {
        return Err("encoder budget not enforced".into());
    }
    let encoder = MarianEncoder::load(&mut reader, &config, ENCODER_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let embeddings =
        MarianEmbeddings::load(&mut reader, &config, 135_479_204).map_err(|e| format!("{e:?}"))?;
    if encoder.layers().len() != 6
        || encoder.forward(&embeddings, &[], None, Kernel::Auto).err() != Some(Error::Shape)
        || encoder
            .forward(&embeddings, &[0], Some(&[false]), Kernel::Auto)
            .err()
            != Some(Error::Shape)
        || encoder
            .forward(&embeddings, &[0], Some(&[true, true]), Kernel::Auto)
            .err()
            != Some(Error::Shape)
    {
        return Err("invalid encoder input accepted".into());
    }
    let cases: [(&[u32], Option<&[bool]>); 3] = [
        (&[1, 2048, 0], None),
        (
            &[2047, 2048, 4095, 4096, 64999, 0, 65000, 65000],
            Some(&[true, true, true, true, true, true, false, false]),
        ),
        (
            &[
                4, 17, 28, 109, 2047, 2048, 4096, 12991, 32000, 50000, 64999, 0,
            ],
            None,
        ),
    ];
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    for (ids, mask) in cases {
        let mut state = embeddings
            .input(Side::Encoder, ids, 0)
            .map_err(|e| format!("{e:?}"))?;
        write_tensor(&mut out, &state)?;
        for layer in encoder.layers() {
            let attention = layer
                .attention_stage(&state, mask, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &attention)?;
            let ff = layer
                .ffn
                .forward(&attention, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            state = layer
                .final_norm
                .residual(&attention, &ff)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &state)?;
        }
        let forward = encoder
            .forward(&embeddings, ids, mask, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        if forward != state {
            return Err("trace differs from public encoder forward".into());
        }
        write_tensor(&mut out, &forward)?;
    }
    out.flush()?;
    eprintln!(
        "REAL_ENCODER_LOADED layers=6 tensors=96 payload_bytes={} budget_and_bounds=PASS",
        encoder.payload_bytes()
    );
    Ok(())
}
