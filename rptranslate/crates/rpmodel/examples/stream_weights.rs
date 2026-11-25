//! Test-only binary round-trip of all weights through the autonomous reader.
use rpmodel::safetensors::{Dtype, SafeTensorReader};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing weights path")?;
    let file = File::open(path)?;
    if file.metadata()?.len() > 400 * 1024 * 1024 {
        return Err("file exceeds model budget".into());
    }
    let mut reader = SafeTensorReader::open(file).map_err(|e| format!("{e:?}"))?;
    if reader.metadata().get("rpmodel_format").map(String::as_str) != Some("1") {
        return Err("unexpected model format".into());
    }
    let entries: Vec<_> = reader
        .tensors()
        .iter()
        .map(|(name, info)| (name.clone(), info.clone()))
        .collect();
    if entries.len() != 256 {
        return Err("unexpected tensor count".into());
    }
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    for (name, info) in entries {
        if info.dtype != Dtype::F32 {
            return Err("unexpected dtype".into());
        }
        let mut start = 0;
        while start < info.elements {
            let count = (info.elements - start).min(1024 * 1024) as usize;
            let values = reader
                .read_f32(&name, start, count)
                .map_err(|e| format!("{e:?}"))?;
            for value in values {
                if !value.is_finite() {
                    return Err("nonfinite model weight".into());
                }
                out.write_all(&value.to_le_bytes())?;
            }
            start += count as u64;
        }
    }
    out.flush()?;
    Ok(())
}
