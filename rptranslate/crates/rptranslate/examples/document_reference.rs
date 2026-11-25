//! Full extracted-text document audit, with explicit rejected blocks.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::{
    context::ContextEngine,
    document::{self, Block, MAX_BLOCKS, MAX_INPUT_BYTES},
    glossary::Domain,
    offline::OfflineTranslator,
};
use std::{fs::File, io::Read, path::Path, time::Instant};

fn hex(text: &str) -> String {
    text.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [root, precision, path] = args.as_slice() else {
        return Err("usage: document_reference MODEL PRECISION INPUT.tsv".into());
    };
    let cap = MAX_INPUT_BYTES + MAX_BLOCKS * 16;
    let mut input = String::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take((cap + 1) as u64)
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    if input.len() > cap {
        return Err("input file limit".into());
    }
    let mut blocks = Vec::new();
    for line in input.lines() {
        let (id, source) = line.split_once('\t').ok_or("invalid row")?;
        if blocks.len() >= MAX_BLOCKS {
            return Err("block limit".into());
        }
        blocks.push(Block {
            id: id.parse().map_err(|_| "invalid id")?,
            source,
        });
    }
    let start = Instant::now();
    let model = match precision.as_str() {
        "fp32" => OfflineTranslator::load(Path::new(root), MODEL_PAYLOAD_BYTES),
        "int8" => OfflineTranslator::load_mixed(Path::new(root), PAYLOAD_BYTES),
        _ => return Err("unknown precision".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let session = model.session().map_err(|e| format!("{e:?}"))?;
    let engine = ContextEngine::builtin().map_err(|e| format!("{e:?}"))?;
    let start = Instant::now();
    let report = document::translate(
        &session,
        &blocks,
        &engine,
        Some(Domain::Geotechnical),
        GenerationOptions {
            beam_width: 1,
            max_new: 128,
            mode: GenerationMode::Cached,
        },
        Kernel::Auto,
    )
    .map_err(|e| format!("{e:?}"))?;
    let seconds = start.elapsed().as_secs_f64();
    println!("DOCUMENT\t{precision}\t{load_ms:.6}\t{seconds:.9}");
    for block in report.blocks {
        let id = block.id;
        match block.outcome {
            Ok(result) => {
                println!(
                    "ACCEPT\t{id}\t{}\t{}",
                    result.segments.len(),
                    hex(&result.text)
                );
                for segment in result.segments {
                    println!(
                        "SEGMENT\t{id}\t{}\t{}\t{:?}\t{}",
                        segment.source.start,
                        segment.source.end,
                        segment.strategy,
                        hex(&segment.text)
                    );
                }
            }
            Err(error) => println!("REJECT\t{id}\t{}", hex(&format!("{error:?}"))),
        }
    }
    Ok(())
}
