//! Real-weight kernel regression and bounded long-position probe.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MarianModel, MODEL_PAYLOAD_BYTES},
    matmul::{avx2_available, Kernel},
    tensor::{Error, Tensor},
};
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn write_tensor(out: &mut impl Write, tensor: &Tensor) -> io::Result<()> {
    for value in tensor.data() {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn ids(n: usize, start: u32, stride: u32) -> Vec<u32> {
    (0..n)
        .map(|i| 2 + (start + i as u32 * stride) % 64997)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let mode = std::env::args().nth(2).ok_or("missing probe mode")?;
    let portable = match mode.as_str() {
        "native" => false,
        "portable" => true,
        _ => return Err("unknown probe mode".into()),
    };
    if avx2_available() == portable {
        return Err("expected native AVX2 or forced portable fallback".into());
    }
    let model =
        MarianModel::load(Path::new(&path), MODEL_PAYLOAD_BYTES).map_err(|e| format!("{e:?}"))?;
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    let kernels = [
        Kernel::Scalar,
        Kernel::Blocked,
        Kernel::Auto,
        Kernel::Parallel(4),
    ];
    for case in 0..2 {
        let (source, target, target_keep) = if case == 0 {
            (vec![1, 2048, 0], vec![65000, 4, 4096], None)
        } else {
            (
                vec![2047, 2048, 4095, 4096, 0, 65000],
                vec![65000, 4, 65000, 4096],
                Some(vec![true, true, false, true]),
            )
        };
        for kernel in kernels {
            let session = model
                .encode(&source, None, kernel)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, session.memory())?;
            let full = session
                .logits(&target, target_keep.as_deref(), false)
                .map_err(|e| format!("{e:?}"))?;
            let cached = session
                .cached_logits(&target, target_keep.as_deref())
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, &full)?;
            write_tensor(&mut out, &cached)?;
            for (width, mode) in [(1, GenerationMode::Full), (4, GenerationMode::Cached)] {
                let hypotheses = session
                    .generate(GenerationOptions {
                        beam_width: width,
                        max_new: 4,
                        mode,
                    })
                    .map_err(|e| format!("{e:?}"))?;
                for (rank, h) in hypotheses.iter().enumerate() {
                    let tokens = h
                        .tokens
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",");
                    writeln!(
                        out,
                        "{width}|{rank}|{tokens}|{:.17}|{:.17}|{}",
                        h.log_probability,
                        h.score,
                        u8::from(h.ended)
                    )?;
                }
            }
            writeln!(out, "END")?;
        }
    }
    if !portable {
        // Exercise a moderate prefix with nontrivial row partitioning.
        let mut source = ids(64, 17, 7919);
        source[62] = 0;
        source[63] = 65000;
        let mut target = ids(16, 23, 3571);
        target[0] = 65000;
        for kernel in [Kernel::Auto, Kernel::Parallel(4)] {
            let session = model
                .encode(&source, None, kernel)
                .map_err(|e| format!("{e:?}"))?;
            write_tensor(&mut out, session.memory())?;
            write_tensor(
                &mut out,
                &session
                    .logits(&target, None, false)
                    .map_err(|e| format!("{e:?}"))?,
            )?;
            write_tensor(
                &mut out,
                &session
                    .cached_logits(&target, None)
                    .map_err(|e| format!("{e:?}"))?,
            )?;
        }
        // The 512-position boundary is a full decoder hidden forward but only
        // a last-row vocabulary projection: full 512*vocab exceeds tensor cap.
        let mut source = ids(512, 17, 7919);
        source[507] = 0;
        source[508..].fill(65000);
        let mut target = ids(512, 23, 3571);
        target[0] = 65000;
        let session = model
            .encode(&source, None, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        write_tensor(&mut out, session.memory())?;
        write_tensor(
            &mut out,
            &session
                .logits(&target, None, true)
                .map_err(|e| format!("{e:?}"))?,
        )?;
        if model.encode(&[0; 513], None, Kernel::Auto).err() != Some(Error::Limit)
            || session.logits(&[0; 513], None, true).err() != Some(Error::Limit)
            || session.cached_logits(&target, None).err() != Some(Error::Limit)
            || session
                .generate(GenerationOptions {
                    beam_width: 16,
                    max_new: 511,
                    mode: GenerationMode::Cached,
                })
                .err()
                != Some(Error::Limit)
        {
            return Err("expected source/target/tensor/KV limit rejection".into());
        }
        eprintln!("REAL_POSITION_BOUNDARY PASS source=512 target=512 last_only=true rejected=4");
    }
    out.flush()?;
    eprintln!(
        "REAL_KERNEL_PROBE PASS mode={mode} avx2={}",
        avx2_available()
    );
    Ok(())
}
