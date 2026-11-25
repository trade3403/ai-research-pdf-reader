//! Compare independent actual checkpoint linear weights, no model activation claims.
use rpinfer::{quantized_linear::{Int8Kernel, QuantizedLinear}, quantized_table::QuantizedTable, tensor::Tensor};
use rpmodel::package;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let (_, mut reader) = package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    let mut out = BufWriter::new(io::stdout().lock());
    for (name, width, outputs) in [
        ("model.encoder.layers.0.self_attn.q_proj", 512, 512),
        ("model.encoder.layers.0.fc1", 512, 2048),
        ("model.decoder.layers.5.fc2", 2048, 512),
    ] {
        let table = QuantizedTable::from_fp32(
            &mut reader,
            &format!("{name}.weight"),
            64,
            outputs * width + outputs * width / 64 * 4,
        )
        .map_err(|e| format!("{e:?}"))?;
        let bias = Tensor::new(
            &[outputs],
            reader
                .read_f32(&format!("{name}.bias"), 0, outputs)
                .map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
        let input = Tensor::new(
            &[3, width],
            (0..3 * width)
                .map(|n| ((n % 31) as f32 - 15.) / 16.)
                .collect(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let linear = QuantizedLinear {
            weight: &table,
            bias: Some(&bias),
        };
        let full = linear
            .forward(&input, false)
            .map_err(|e| format!("{e:?}"))?;
        let last = linear.forward(&input, true).map_err(|e| format!("{e:?}"))?;
        let scalar = linear
            .forward_with(&input, false, Int8Kernel::Scalar)
            .map_err(|e| format!("{e:?}"))?;
        if full.data().iter().zip(scalar.data()).any(|(a, b)| a.to_bits() != b.to_bits()) {
            return Err("scalar and automatic kernel differ".into());
        }
        if last.data() != &full.data()[2 * outputs..] {
            return Err("last-only mismatch".into());
        }
        for tensor in [&full, &last] {
            for &v in tensor.data() {
                out.write_all(&v.to_le_bytes()).map_err(|e| e.to_string())?;
            }
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}
