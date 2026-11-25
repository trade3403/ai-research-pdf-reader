//! Convert and reload the real shared table; emit actual gather and projection.
use rpinfer::{quantized_table::QuantizedTable, tensor::Tensor};
use rpmodel::{package, quantized};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Write},
    path::Path,
};

const NAME: &str = "model.shared.weight";
const BYTES: usize = 35_360_544;

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [root, dest] = args.as_slice() else {
        return Err("usage: real_quantized_table MODEL_DIR OUTPUT".into());
    };
    let (config, mut reader) =
        package::open_profile(Path::new(root)).map_err(|e| format!("{e:?}"))?;
    let table =
        QuantizedTable::from_fp32(&mut reader, NAME, 64, BYTES).map_err(|e| format!("{e:?}"))?;
    if table.payload_bytes() != BYTES || table.shape() != [65001, 512] {
        return Err("wrong packed inventory".into());
    }
    let bias = Tensor::new(
        &[65001],
        reader
            .read_f32("final_logits_bias", 0, 65001)
            .map_err(|e| format!("{e:?}"))?,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut out = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(dest)
        .map_err(|e| e.to_string())?;
    let bytes = quantized::write_container(
        &mut out,
        &[table.as_input(NAME, &[65001, 512])],
        (BYTES + 1024) as u64,
    )
    .map_err(|e| format!("{e:?}"))?;
    drop(out);
    drop(reader);
    drop(table);
    let file = File::open(dest).map_err(|e| e.to_string())?;
    let mut reader =
        quantized::Reader::open(BufReader::new(file), bytes).map_err(|e| format!("{e:?}"))?;
    let table = QuantizedTable::load(&mut reader, NAME, BYTES).map_err(|e| format!("{e:?}"))?;
    let ids = [0, 1, 2047, 2048, 4095, 4096, 64999, 65000];
    let embedding = table
        .embedding(&ids, config.embedding_scale)
        .map_err(|e| format!("{e:?}"))?;
    let hidden = Tensor::new(
        &[2, 512],
        (0..1024).map(|n| ((n % 31) as f32 - 15.) / 16.).collect(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let logits = table
        .project(&hidden, Some(&bias), false)
        .map_err(|e| format!("{e:?}"))?;
    let last = table
        .project(&hidden, Some(&bias), true)
        .map_err(|e| format!("{e:?}"))?;
    if last.data() != &logits.data()[65001..] {
        return Err("last-only projection mismatch".into());
    }
    let mut stdout = BufWriter::new(io::stdout().lock());
    for tensor in [&embedding, &logits, &last] {
        for &v in tensor.data() {
            stdout
                .write_all(&v.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
    }
    stdout.flush().map_err(|e| e.to_string())?;
    eprintln!(
        "REAL_QTABLE_LOADED rows=65001 width=512 blocks={} payload_bytes={} file_bytes={bytes}",
        table.block_count(),
        table.payload_bytes()
    );
    Ok(())
}
