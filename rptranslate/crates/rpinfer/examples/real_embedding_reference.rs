//! Binary test-only probe: real shared table, stored positions and tied logits.
use rpmodel::package;
use rpinfer::{
    marian_weights::{MarianEmbeddings, Side},
    tensor::{Error, Tensor},
    vocabulary::{LoadError, RowTable},
};
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn write_values(out: &mut impl Write, values: &[f32]) -> io::Result<()> {
    for &value in values {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let (config, mut reader) =
        package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    const PAYLOAD: usize = 135_479_204;
    if MarianEmbeddings::load(&mut reader, &config, PAYLOAD - 1).err()
        != Some(LoadError::Tensor(Error::Limit))
    {
        return Err("payload budget must reject before loading".into());
    }
    let model =
        MarianEmbeddings::load(&mut reader, &config, PAYLOAD).map_err(|e| format!("{e:?}"))?;
    if model.shared().block_count() != 32 || model.payload_bytes() != PAYLOAD {
        return Err("unexpected block count or payload".into());
    }
    let ids = [0, 1, 2047, 2048, 4095, 4096, 64999, 65000];
    if model.tokens(&[65001]).err() != Some(Error::Index)
        || model.input(Side::Decoder, &[0, 1], 511).err() != Some(Error::Index)
        || model.input(Side::Encoder, &[0], usize::MAX).err() != Some(Error::Limit)
    {
        return Err("invalid input accepted".into());
    }
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    // Check every value in the loaded ChunkedTable, not just the source reader.
    for row in 0..config.vocab {
        write_values(
            &mut out,
            model.shared().row(row).map_err(|e| format!("{e:?}"))?,
        )?;
    }
    for side in [Side::Encoder, Side::Decoder] {
        write_values(&mut out, model.positions(side).data())?;
    }
    write_values(&mut out, model.bias().data())?;
    let tokens = model.tokens(&ids).map_err(|e| format!("{e:?}"))?;
    write_values(&mut out, tokens.data())?;
    for side in [Side::Encoder, Side::Decoder] {
        for offset in [0, 37, 504] {
            let value = model
                .input(side, &ids, offset)
                .map_err(|e| format!("{e:?}"))?;
            write_values(&mut out, value.data())?;
        }
    }
    // Fixed exactly representable rows, independent of embeddings/decoder.
    let hidden = Tensor::new(
        &[3, config.hidden],
        (0..3 * config.hidden)
            .map(|n| ((n % 31) as f32 - 15.) / 16.)
            .collect(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let all = model
        .project(&hidden, false)
        .map_err(|e| format!("{e:?}"))?;
    let last = model.project(&hidden, true).map_err(|e| format!("{e:?}"))?;
    if last.data() != &all.data()[2 * config.vocab..] {
        return Err("last-only projection differs".into());
    }
    write_values(&mut out, all.data())?;
    write_values(&mut out, last.data())?;
    out.flush()?;
    eprintln!("REAL_EMBEDDINGS_LOADED blocks=32 payload_bytes={PAYLOAD} budget_and_bounds=PASS");
    Ok(())
}
