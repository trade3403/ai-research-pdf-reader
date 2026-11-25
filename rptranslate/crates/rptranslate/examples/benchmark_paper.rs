//! Paper throughput measurement; engine and model loading are unchanged.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::{Kernel, avx2_available},
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::offline::OfflineTranslator;
use std::{fs, path::Path, time::Instant};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let root = args.next().ok_or("missing model directory")?;
    let precision = args.next().ok_or("missing precision")?;
    let input = args.next().ok_or("missing segment file")?;
    let start = Instant::now();
    let translator = match precision.as_str() {
        "fp32" => OfflineTranslator::load(Path::new(&root), MODEL_PAYLOAD_BYTES),
        "int8" => OfflineTranslator::load_mixed(Path::new(&root), PAYLOAD_BYTES),
        _ => return Err("unknown precision".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let session = translator.session().map_err(|e| format!("{e:?}"))?;
    let session_ms = start.elapsed().as_secs_f64() * 1000.;
    println!("LOAD\t{load_ms:.3}\t{session_ms:.3}\t{}", avx2_available());
    let input = fs::read_to_string(input).map_err(|e| e.to_string())?;
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 256,
        mode: GenerationMode::Cached,
    };
    for line in input.lines() {
        let (id, source) = line.split_once('\t').ok_or("invalid segment")?;
        let start = Instant::now();
        let result = session
            .translate(source, options, Kernel::Auto)
            .map_err(|e| format!("segment {id}: {e:?}"))?;
        let seconds = start.elapsed().as_secs_f64();
        let hex = result
            .text
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        println!(
            "ROW\t{id}\t{}\t{}\t{seconds:.9}\t{}\t{hex}",
            result.source_tokens.len(),
            result.target_tokens.len() - 1,
            u8::from(result.reached_limit)
        );
    }
    Ok(())
}
