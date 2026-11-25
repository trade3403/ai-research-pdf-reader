use rpinfer::{marian_model::MODEL_PAYLOAD_BYTES, matmul::Kernel, mixed_model::PAYLOAD_BYTES};
use rptranslate::{
    context::ContextEngine,
    glossary::{Domain, Glossary},
    offline::OfflineTranslator,
};
use std::{path::Path, process::ExitCode};

fn translation_command(command: &str) -> bool {
    matches!(
        command,
        "translate" | "translate-int8" | "translate-scientific" | "translate-scientific-int8"
    )
}

fn run_glossary(
    command: &str,
    root: &str,
    domain: &str,
    text: &str,
    rest: &[String],
) -> Result<(), String> {
    let explicit = if domain == "auto" {
        None
    } else {
        Some(Domain::parse(domain).ok_or("unknown glossary domain")?)
    };
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
    if command.starts_with("translate-paragraph") {
        let engine = ContextEngine::builtin().map_err(|e| format!("{e:?}"))?;
        let result = session
            .translate_paragraph(text, &engine, explicit, options, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        println!("{}", result.text);
        for (index, segment) in result.segments.iter().enumerate() {
            eprintln!(
                "Segment {index}: {:?} {:?} {:?}; rejected={:?}",
                segment.source, segment.domain, segment.strategy, segment.rejected
            );
        }
        return Ok(());
    }
    if command.starts_with("translate-constrained") {
        let engine = ContextEngine::builtin().map_err(|e| format!("{e:?}"))?;
        let domain = if let Some(domain) = explicit {
            domain
        } else {
            let decision = engine.decide(text, None).map_err(|e| format!("{e:?}"))?;
            eprintln!("Context decision: {decision:?}");
            decision.domain
        };
        let glossary = Glossary::builtin(domain).map_err(|e| format!("{e:?}"))?;
        let result = session
            .translate_constrained(text, &glossary, options, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        println!("{}", result.translation.text);
        eprintln!(
            "Term spans: {}; protected spans: {}; whole source: true",
            result.term_spans, result.protected_spans
        );
        return Ok(());
    }
    let result = if let Some(domain) = explicit {
        let glossary = Glossary::builtin(domain).map_err(|e| format!("{e:?}"))?;
        session
            .translate_glossary(text, &glossary, options, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?
    } else {
        let engine = ContextEngine::builtin().map_err(|e| format!("{e:?}"))?;
        let (decision, result) = session
            .translate_contextual(text, &engine, None, options, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        eprintln!("Context decision: {decision:?}");
        result
    };
    println!("{}", result.text);
    eprintln!(
        "Term spans: {}; protected spans: {}; fragmented context: {}",
        result.term_spans, result.protected_spans, result.fragmented_context
    );
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [command, precision, root] if command == "model-check" => {
            let translator = match precision.as_str() {
                "fp32" => OfflineTranslator::load(Path::new(root), MODEL_PAYLOAD_BYTES),
                "int8" => OfflineTranslator::load_mixed(Path::new(root), PAYLOAD_BYTES),
                _ => return Err("unknown precision".into()),
            }
            .map_err(|e| format!("{e:?}"))?;
            let session = translator.session().map_err(|e| format!("{e:?}"))?;
            let tokens = session
                .encode("We measured maximum strain.")
                .map_err(|e| format!("{e:?}"))?;
            if tokens.is_empty() {
                return Err("empty tokenizer probe".into());
            }
            println!(
                "MODEL_CHECK PASS precision={precision} payload_bytes={} tokens={} avx2={}",
                translator.payload_bytes(),
                tokens.len(),
                rpinfer::matmul::avx2_available()
            );
        }
        [command] if command == "self-check" => {
            let bytes = rptranslate::self_check()?;
            println!("D1.0 PASS: synthetic corpus, scalar dot, model budget = {bytes} bytes");
            println!("Self-check only; no model loaded; no network requests");
        }
        [command, root, domain, text, rest @ ..]
            if matches!(
                command.as_str(),
                "translate-glossary"
                    | "translate-glossary-int8"
                    | "translate-constrained"
                    | "translate-constrained-int8"
                    | "translate-paragraph"
                    | "translate-paragraph-int8"
            ) && rest.len() <= 2 =>
        {
            run_glossary(command, root, domain, text, rest)?;
        }
        [command, root, text, rest @ ..] if translation_command(command) && rest.len() <= 2 => {
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
                "Usage: rptranslate self-check | model-check fp32|int8 MODEL_DIR | translate[-scientific][-int8] MODEL_DIR TEXT [BEAM_WIDTH] [MAX_NEW] | translate-glossary[-int8] MODEL_DIR DOMAIN TEXT [BEAM_WIDTH] [MAX_NEW] | translate-constrained[-int8] MODEL_DIR DOMAIN TEXT [BEAM_WIDTH] [MAX_NEW] | translate-paragraph[-int8] MODEL_DIR DOMAIN TEXT [BEAM_WIDTH] [MAX_NEW]"
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
