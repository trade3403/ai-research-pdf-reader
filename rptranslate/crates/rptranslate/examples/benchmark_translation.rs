//! Real offline release baseline, one model and tokenizer session.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::{avx2_available, Kernel},
};
use rptranslate::offline::OfflineTranslator;
use std::{path::Path, time::Instant};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let start = Instant::now();
    let translator = OfflineTranslator::load(Path::new(&path), MODEL_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let session = translator.session().map_err(|e| format!("{e:?}"))?;
    let session_ms = start.elapsed().as_secs_f64() * 1000.;
    println!(
        "BENCH_LOAD\t{load_ms:.3}\t{session_ms:.3}\t{}\t{}",
        MODEL_PAYLOAD_BYTES,
        avx2_available()
    );
    let cases = [
        "The result is stable.",
        "The temperature is 25 degrees Celsius.",
        "The sample contained 10 mg of sodium chloride.",
        "The experiment was repeated 3 times.",
        "The model uses a neural network for classification.",
        "The accuracy increased from 80% to 90%.",
    ];
    let greedy = GenerationOptions {
        beam_width: 1,
        max_new: 64,
        mode: GenerationMode::Cached,
    };
    for mode in [GenerationMode::Full, GenerationMode::Cached] {
        session
            .translate(cases[0], GenerationOptions { mode, ..greedy }, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
    }
    for (case, text) in cases.iter().take(2).enumerate() {
        let mut expected = None;
        for repeat in 0..3 {
            let modes = if repeat % 2 == 0 {
                [GenerationMode::Full, GenerationMode::Cached]
            } else {
                [GenerationMode::Cached, GenerationMode::Full]
            };
            for mode in modes {
                let start = Instant::now();
                let result = session
                    .translate(text, GenerationOptions { mode, ..greedy }, Kernel::Auto)
                    .map_err(|e| format!("{e:?}"))?;
                let seconds = start.elapsed().as_secs_f64();
                if result.reached_limit || seconds <= 0. {
                    return Err("invalid benchmark generation".into());
                }
                if expected
                    .as_ref()
                    .is_some_and(|ids| ids != &result.target_tokens)
                {
                    return Err("benchmark token sequences differ".into());
                }
                let generated = result.target_tokens.len() - 1;
                println!(
                    "BENCH_RUN\t{case}\t{repeat}\t{mode:?}\t{}\t{generated}\t{seconds:.9}",
                    result.source_tokens.len()
                );
                expected = Some(result.target_tokens);
            }
        }
    }
    for (case, text) in cases.iter().enumerate() {
        let result = session
            .translate(
                text,
                GenerationOptions {
                    beam_width: 4,
                    ..greedy
                },
                Kernel::Auto,
            )
            .map_err(|e| format!("{e:?}"))?;
        let hex = result
            .text
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        println!(
            "BENCH_QUALITY\t{case}\t{}\t{}\t{hex}",
            result.target_tokens.len() - 1,
            u8::from(result.reached_limit)
        );
    }
    Ok(())
}
