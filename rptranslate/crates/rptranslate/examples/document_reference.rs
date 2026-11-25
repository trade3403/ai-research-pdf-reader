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
    paragraph,
};
use std::{fs::File, io::Read, path::Path, time::Instant};

fn hex(text: &str) -> String {
    text.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn diagnostics(id: u32, phase: &str, error: &paragraph::Error) {
    if let paragraph::Error::Rejected { attempts, .. } = error {
        for attempt in attempts {
            println!(
                "ATTEMPT\t{id}\t{phase}\t{:?}\t{}",
                attempt.strategy,
                attempt.failure.category()
            );
        }
    }
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (root, precision, path, retry) = match args.as_slice() {
        [root, precision, path] => (root, precision, path, false),
        [root, precision, path, flag] if flag == "retry" => (root, precision, path, true),
        _ => return Err("usage: document_reference MODEL PRECISION INPUT.tsv [retry]".into()),
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
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 128,
        mode: GenerationMode::Cached,
    };
    let recovery = if retry {
        document::translate_with_numeric_retry(
            &session,
            &blocks,
            &engine,
            Some(Domain::Geotechnical),
            options,
            Kernel::Auto,
        )
        .map_err(|e| format!("{e:?}"))?
    } else {
        let report = document::translate(
            &session,
            &blocks,
            &engine,
            Some(Domain::Geotechnical),
            options,
            Kernel::Auto,
        )
        .map_err(|e| format!("{e:?}"))?;
        document::NumericRecoveryReport {
            previous: document::RecoveryReport {
                document: report,
                retries: Vec::new(),
            },
            numeric: Vec::new(),
        }
    };
    let seconds = start.elapsed().as_secs_f64();
    let summary = document::assess(&blocks, &recovery, &engine)
        .map_err(|e| format!("quality assessment: {e:?}"))?;
    let report = recovery.previous.document;
    let retries = recovery.previous.retries;
    let numeric = recovery.numeric;
    println!("DOCUMENT\t{precision}\t{load_ms:.6}\t{seconds:.9}");
    for block in &report.blocks {
        let id = block.id;
        match &block.outcome {
            Ok(result) => {
                println!(
                    "ACCEPT\t{id}\t{}\t{}",
                    result.segments.len(),
                    hex(&result.text)
                );
                for segment in &result.segments {
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
    for block in &report.blocks {
        if let Err(error) = &block.outcome {
            diagnostics(block.id, "final", error);
        }
    }
    for retry in retries {
        println!("RETRY\t{}\t{}", retry.id, retry.recovered);
        diagnostics(retry.id, "initial", &retry.initial);
    }
    for record in numeric {
        println!(
            "NUMERIC_RETRY\t{}\t{}",
            record.id,
            record.previous.is_some()
        );
        if let Some(error) = record.previous {
            diagnostics(record.id, "pre_numeric", &error);
        }
        if let Some(error) = record.failure {
            diagnostics(record.id, "numeric", &error);
        }
    }
    println!(
        "SUMMARY\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        summary.blocks,
        summary.accepted,
        summary.rejected,
        summary.source_words,
        summary.accepted_words,
        summary.source_numbers,
        summary.accepted_numbers,
        summary.segments,
        summary.first_pass_accepted,
        summary.beam_recovered,
        summary.numeric_recovered,
        summary.has_rejections(),
        summary.needs_semantic_review()
    );
    Ok(())
}
