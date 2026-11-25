//! Actual FP32/INT8 context selection, term restoration and abstention.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::{
    context::{Basis, ContextEngine, DocumentContext},
    glossary::Domain,
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
    let engine = ContextEngine::builtin().map_err(|e| format!("{e:?}"))?;
    let geo = DocumentContext::from_text("Seismic waves in a soil basin.")
        .map_err(|e| format!("{e:?}"))?;
    let bio =
        DocumentContext::from_text("Bacterial cells and genes.").map_err(|e| format!("{e:?}"))?;
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 256,
        mode: GenerationMode::Cached,
    };
    let cases = [
        (
            "Soil shear strain increased.",
            None,
            Domain::Geotechnical,
            Basis::Local,
            Some("应变"),
        ),
        (
            "Bacterial cells contain this strain.",
            Some(&geo),
            Domain::Biology,
            Basis::Local,
            Some("菌株"),
        ),
        (
            "The strain increased.",
            Some(&geo),
            Domain::Geotechnical,
            Basis::Document,
            Some("应变"),
        ),
        (
            "The strain increased.",
            Some(&bio),
            Domain::Biology,
            Basis::Document,
            Some("菌株"),
        ),
        (
            "The strain increased.",
            None,
            Domain::General,
            Basis::None,
            None,
        ),
        (
            "Soil and bacterial cells contain this strain.",
            Some(&geo),
            Domain::General,
            Basis::Mixed,
            None,
        ),
        (
            "Protein strain increased.",
            Some(&geo),
            Domain::General,
            Basis::Insufficient,
            None,
        ),
    ];
    for (i, (source, prior, domain, basis, expected)) in cases.iter().enumerate() {
        let (decision, output) = session
            .translate_contextual(source, &engine, *prior, options, Kernel::Auto)
            .map_err(|e| format!("case {i}: {e:?}"))?;
        if decision.domain != *domain || decision.basis != *basis {
            return Err(format!("case {i}: wrong decision {decision:?}"));
        }
        if let Some(target) = expected {
            if output.term_spans == 0 || !output.text.contains(target) {
                return Err(format!("case {i}: missing selected term"));
            }
        } else if output.term_spans != 0 {
            return Err(format!("case {i}: domain term forced on abstention"));
        }
        println!(
            "CONTEXT_FIXTURE {precision} {i} decision={decision:?} output={:?}",
            output.text
        );
    }
    println!(
        "CONTEXT_REFERENCE PASS precision={precision} cases={} selected=4 abstained=3",
        cases.len()
    );
    if let Some(paper) = paper {
        let input = fs::read_to_string(paper).map_err(|e| e.to_string())?;
        let rows: Vec<_> = input
            .lines()
            .map(|line| line.split_once('\t').ok_or("invalid row"))
            .collect::<Result<_, _>>()?;
        let (_, title) = rows.first().ok_or("empty paper sample")?;
        let context = DocumentContext::from_text(title).map_err(|e| format!("{e:?}"))?;
        let mut geo_count = 0;
        let mut success = 0;
        let mut terms = 0;
        for (id, source) in &rows {
            let d = engine
                .decide(source, Some(&context))
                .map_err(|e| format!("paper decision {id}: {e:?}"))?;
            if d.domain == Domain::Geotechnical {
                geo_count += 1;
            }
            match session.translate_contextual(
                source,
                &engine,
                Some(&context),
                options,
                Kernel::Auto,
            ) {
                Ok((decision, output)) => {
                    success += 1;
                    terms += output.term_spans;
                    println!(
                        "CONTEXT_PAPER_PREVIEW {id} decision={decision:?} text={:?}",
                        output.text
                    );
                }
                Err(error) => println!("CONTEXT_PAPER_REJECTED {id} {error:?}"),
            }
        }
        if rows.is_empty() || geo_count == 0 || success == 0 || terms == 0 {
            return Err("paper context gate failed".into());
        }
        println!(
            "CONTEXT_PAPER observed={} geo={geo_count} success={success} rejected={} terms={terms} quality=observational",
            rows.len(),
            rows.len() - success
        );
    }
    Ok(())
}
