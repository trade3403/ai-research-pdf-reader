use rpinfer::{marian_model::MODEL_PAYLOAD_BYTES, matmul::Kernel, mixed_model::PAYLOAD_BYTES};
use rptranslate::offline::OfflineTranslator;
use std::{path::Path, process::ExitCode};

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [command] if command == "self-check" => {
            let bytes = rptranslate::self_check()?;
            println!("D1.0 PASS: synthetic corpus, scalar dot, model budget = {bytes} bytes");
            println!("Self-check only; no model loaded; no network requests");
        }
        [command, root, text, rest @ ..] if matches!(command.as_str(), "translate" | "translate-int8" | "translate-scientific" | "translate-scientific-int8") && rest.len() <= 2 => {
            let translator = if command.ends_with("-int8") {
                OfflineTranslator::load_mixed(Path::new(root), PAYLOAD_BYTES)
            } else {
                OfflineTranslator::load(Path::new(root), MODEL_PAYLOAD_BYTES)
            }
            .map_err(|e| format!("{e:?}"))?;
            let mut options = translator.default_options();
            if let Some(width) = rest.first() {
                options.beam_width = width.parse().map_err(|_| "invalid beam width")?;
            }
            if let Some(limit) = rest.get(1) {
                options.max_new = limit.parse().map_err(|_| "invalid token limit")?;
            }
            let session = translator.session().map_err(|e| format!("{e:?}"))?;
            if command.starts_with("translate-scientific") {
                let result = session
                    .translate_scientific(text, &[], options, Kernel::Auto)
                    .map_err(|e| format!("{e:?}"))?;
                println!("{}", result.text);
                eprintln!(
                    "Protected spans: {}; model calls: {}; fragmented context: {}",
                    result.protected_spans, result.model_calls, result.fragmented_context
                );
                return Ok(());
            }
            let result = session
                .translate(text, options, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            println!("{}", result.text);
            if result.reached_limit {
                eprintln!(
                    "Warning: output reached the generation limit; translation may be incomplete"
                );
            }
        }
        _ => {
            return Err(
                "Usage: rptranslate self-check | translate[-scientific][-int8] MODEL_DIR TEXT [BEAM_WIDTH] [MAX_NEW]"
                    .into(),
            )
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
