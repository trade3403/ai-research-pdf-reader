//! Real offline release baseline, one model and tokenizer session.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::{avx2_available, Kernel},
    mixed_model::PAYLOAD_BYTES,
};
use rpmodel::{marian::Vocabulary, package, spm::Model};
use rptranslate::offline::{Error, OfflineTranslator, LANGUAGE};
use std::{path::Path, time::Instant};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let mode = std::env::args().nth(2).unwrap_or_else(|| "fp32".into());
    let start = Instant::now();
    let translator = match mode.as_str() {
        "fp32" => OfflineTranslator::load(Path::new(&path), MODEL_PAYLOAD_BYTES),
        "int8" => OfflineTranslator::load_mixed(Path::new(&path), PAYLOAD_BYTES),
        _ => return Err("unknown benchmark precision".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let session = std::hint::black_box(translator.session().map_err(|e| format!("{e:?}"))?);
    let session_ms = start.elapsed().as_secs_f64() * 1000.;
    println!(
        "BENCH_LOAD\t{load_ms:.3}\t{session_ms:.9}\t{}\t{}",
        translator.payload_bytes(),
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
    // Rebuild the previous tokenizer path only as a timing/reference probe.
    let source_spm = package::tokenizer_asset(Path::new(&path), "source.spm")
        .map_err(|e| format!("{e:?}"))?;
    let vocab = package::tokenizer_asset(Path::new(&path), "vocab.json")
        .map_err(|e| format!("{e:?}"))?;
    let vocabulary = Vocabulary::parse(std::str::from_utf8(&vocab).map_err(|e| format!("{e:?}"))?)
        .map_err(|e| format!("{e:?}"))?;
    let mut rebuild_ms = 0.;
    let mut reuse_ms = 0.;
    for text in cases {
        let start = Instant::now();
        let tokenizer = Model::parse(&source_spm)
            .map_err(|e| format!("{e:?}"))?
            .nmt_unigram()
            .map_err(|e| format!("{e:?}"))?;
        let expected = vocabulary
            .encode(&tokenizer, &format!("{LANGUAGE} {text}"))
            .map_err(|e| format!("{e:?}"))?;
        rebuild_ms += start.elapsed().as_secs_f64() * 1000.;
        let start = Instant::now();
        let fresh = translator.session().map_err(|e| format!("{e:?}"))?;
        let actual = fresh.encode(text).map_err(|e| format!("{e:?}"))?;
        reuse_ms += start.elapsed().as_secs_f64() * 1000.;
        if actual != expected || fresh.encode(" ") != Err(Error::Empty) {
            return Err("resident tokenizer mismatch".into());
        }
    }
    drop(vocabulary);
    drop(vocab);
    drop(source_spm);
    let first = session
        .translate(cases[0], greedy, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    let fresh = translator.session().map_err(|e| format!("{e:?}"))?;
    fresh
        .translate(cases[1], greedy, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    let repeated = fresh
        .translate(cases[0], greedy, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    if first != repeated || first.reached_limit {
        return Err("resident session state leak".into());
    }
    println!("BENCH_REUSE\t6\t{rebuild_ms:.6}\t{reuse_ms:.6}\ttrue");

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
