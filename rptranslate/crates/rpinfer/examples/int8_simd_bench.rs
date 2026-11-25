//! Measure full quantized Linear (including activation quantization), not just dot.
use rpinfer::{
    matmul::avx2_available,
    quantized_linear::{Int8Kernel, QuantizedLinear},
    quantized_table::QuantizedTable,
    tensor::Tensor,
};
use rpmodel::package;
use std::{hint::black_box, path::Path, time::Instant};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("missing model directory")?;
    let mode = args.next().ok_or("missing native/portable mode")?;
    let avx2 = avx2_available();
    if !matches!(mode.as_str(), "native" | "portable") || avx2 != (mode == "native") {
        return Err("unexpected AVX2 dispatch capability".into());
    }
    let (_, mut reader) = package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    let mut compared = 0;
    for (name, width, outputs) in [
        ("model.encoder.layers.0.self_attn.q_proj", 512, 512),
        ("model.encoder.layers.0.fc1", 512, 2048),
        ("model.decoder.layers.5.fc2", 2048, 512),
    ] {
        let weight = QuantizedTable::from_fp32(
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
            &[8, width],
            (0..8 * width)
                .map(|n| ((n % 251) as f32 - 125.) / 128.)
                .collect(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let linear = QuantizedLinear {
            weight: &weight,
            bias: Some(&bias),
        };
        let scalar = linear
            .forward_with(&input, false, Int8Kernel::Scalar)
            .map_err(|e| format!("{e:?}"))?;
        let auto = linear
            .forward_with(&input, false, Int8Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        let same = |result: &Tensor| {
            result.shape() == scalar.shape()
                && result
                    .data()
                    .iter()
                    .zip(scalar.data())
                    .all(|(a, b)| a.to_bits() == b.to_bits())
        };
        if !same(&auto) {
            return Err("warmup bit mismatch".into());
        }
        compared += auto.data().len();
        let mut samples = [[0_f64; 2]; 5];
        for (rep, times) in samples.iter_mut().enumerate() {
            for slot in [rep % 2, 1 - rep % 2] {
                let kernel = [Int8Kernel::Scalar, Int8Kernel::Auto][slot];
                let start = Instant::now();
                let result = black_box(
                    linear
                        .forward_with(black_box(&input), false, kernel)
                        .map_err(|e| format!("{e:?}"))?,
                );
                times[slot] = start.elapsed().as_secs_f64() * 1000.;
                if !same(&result) {
                    return Err("timed bit mismatch".into());
                }
            }
        }
        let mut times = [samples.map(|row| row[0]), samples.map(|row| row[1])];
        for t in &mut times {
            t.sort_by(f64::total_cmp);
        }
        let scalar_ms = times[0][2];
        let auto_ms = times[1][2];
        if scalar_ms <= 0. || auto_ms <= 0. {
            return Err("invalid timer".into());
        }
        println!(
            "INT8_SIMD_BENCH mode={mode} matrix={name} rows=8 in={width} out={outputs} scalar_ms={scalar_ms:.6} auto_ms={auto_ms:.6} ratio={:.6} bitwise=true",
            scalar_ms / auto_ms
        );
    }
    println!("INT8_SIMD_REFERENCE PASS mode={mode} avx2={avx2} matrices=3 elements={compared} repeats=5 bitwise=true");
    Ok(())
}
