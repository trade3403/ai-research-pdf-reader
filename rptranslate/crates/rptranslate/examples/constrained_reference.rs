//! Real model whole-source constraint observation and fixed lexical gates.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::{
    glossary::{Domain, Glossary},
    offline::OfflineTranslator,
};
use std::{fs, path::Path};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let root = args.next().ok_or("missing model directory")?;
    let precision = args.next().ok_or("missing precision")?;
    let paper = args.next();
    let model = match precision.as_str() {
        "fp32" => OfflineTranslator::load(Path::new(&root), MODEL_PAYLOAD_BYTES),
        "int8" => OfflineTranslator::load_mixed(Path::new(&root), PAYLOAD_BYTES),
        _ => return Err("unknown precision".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let session = model.session().map_err(|e| format!("{e:?}"))?;
    let geo = Glossary::builtin(Domain::Geotechnical).map_err(|e| format!("{e:?}"))?;
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 128,
        mode: GenerationMode::Cached,
    };
    let cases = [
        ("We measured maximum strain.", "最大应变"),
        ("We calculated site response.", "场地响应"),
        ("We measured incidence angle.", "入射角"),
        ("The peak acceleration is 0.1 g.", "0.1 g"),
    ];
    for (i, (source, target)) in cases.iter().enumerate() {
        let baseline = session
            .translate(source, options, Kernel::Auto)
            .map_err(|e| format!("baseline {i}: {e:?}"))?;
        let output = session
            .translate_constrained(source, &geo, options, Kernel::Auto)
            .map_err(|e| format!("constraint {i}: {e:?}"))?;
        if output.translation.source_tokens != baseline.source_tokens
            || output.protected_spans == 0
            || !output.translation.text.contains(target)
        {
            return Err(format!("case {i}: source or target constraint mismatch"));
        }
        println!(
            "CONSTRAINED_FIXTURE {precision} {i} baseline={:?} constrained={:?}",
            baseline.text, output.translation.text
        );
    }
    let tiny = GenerationOptions {
        max_new: 1,
        ..options
    };
    if session
        .translate_constrained("We measured maximum strain.", &geo, tiny, Kernel::Auto)
        .is_ok()
    {
        return Err("impossible token budget accepted".into());
    }
    println!("CONSTRAINED_REFERENCE PASS precision={precision} cases=4 negatives=1");
    if let Some(paper) = paper {
        let input = fs::read_to_string(paper).map_err(|e| e.to_string())?;
        let mut total = 0;
        let mut success = 0;
        let mut terms = 0;
        for line in input.lines() {
            let (id, source) = line.split_once('\t').ok_or("invalid row")?;
            total += 1;
            match session.translate_constrained(source, &geo, options, Kernel::Auto) {
                Ok(result) => {
                    success += 1;
                    terms += result.term_spans;
                    println!(
                        "CONSTRAINED_PREVIEW {id} source={source:?} text={:?}",
                        result.translation.text
                    );
                }
                Err(error) => println!("CONSTRAINED_REJECTED {id} {error:?}"),
            }
        }
        if total == 0 || success == 0 || terms == 0 {
            return Err("paper constrained coverage gate failed".into());
        }
        println!(
            "CONSTRAINED_PAPER observed={total} success={success} rejected={} terms={terms} quality=observational",
            total - success
        );
    }
    Ok(())
}
