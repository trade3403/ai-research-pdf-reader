//! Development-only line protocol: block size, then hexadecimal FP32 bits.
use rpinfer::quantization::BlockInt8;
use std::io::{self, BufRead};

fn main() -> Result<(), String> {
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let mut words = line.split_ascii_whitespace();
        let block = words
            .next()
            .ok_or("missing block")?
            .parse::<usize>()
            .map_err(|_| "bad block")?;
        let values = words
            .map(|s| u32::from_str_radix(s, 16).map(f32::from_bits))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "invalid bits")?;
        let q =
            BlockInt8::quantize(&values, block, 64 * 1024 * 1024).map_err(|e| format!("{e:?}"))?;
        let out = q
            .dequantize_range(0, values.len(), 64 * 1024 * 1024)
            .map_err(|e| format!("{e:?}"))?;
        let codes = q
            .codes()
            .iter()
            .map(i8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let scales = q
            .scales()
            .iter()
            .map(|v| format!("{:08x}", v.to_bits()))
            .collect::<Vec<_>>()
            .join(",");
        let restored = out
            .iter()
            .map(|v| format!("{:08x}", v.to_bits()))
            .collect::<Vec<_>>()
            .join(",");
        println!("{codes}|{scales}|{restored}|{}", q.payload_bytes());
    }
    Ok(())
}
