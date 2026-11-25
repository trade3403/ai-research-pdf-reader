//! Real-model paragraph candidate audit and explicit coverage reporting.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::{
    context::{Basis, ContextEngine},
    glossary::Domain,
    offline::OfflineTranslator,
    paragraph::{self, Strategy},
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
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 128,
        mode: GenerationMode::Cached,
    };
    let source = "We measured maximum strain. We calculated site response.";
    let result = session
        .translate_paragraph(source, &engine, Some(Domain::Geotechnical), options, Kernel::Auto)
        .map_err(|e| format!("fixed paragraph: {e:?}"))?;
    if result.segments.len() != 2
        || !result.segments[0].text.contains("最大应变")
        || !result.segments[1].text.contains("场地响应")
        || result.text.contains("现场反应")
        || result.text.contains("最大应变强度")
        || result.text.contains("最大应变量")
        || result.text.contains("最大的压力")
        || result.text.contains("最大菌株")
    {
        return Err(format!("fixed paragraph failed: {result:?}"));
    }
    println!("PARAGRAPH_FIXTURE {precision} explicit {result:?}");
    let source = "Soil shear strain increased. We measured maximum strain.";
    let result = session
        .translate_paragraph(source, &engine, None, options, Kernel::Auto)
        .map_err(|e| format!("auto paragraph: {e:?}"))?;
    if result.segments.len() != 2
        || result.segments[0].basis != Some(Basis::Local)
        || result.segments[1].basis != Some(Basis::Document)
        || !result.segments[0].text.contains("剪应变")
        || !result.segments[1].text.contains("最大应变")
    {
        return Err(format!("auto paragraph failed: {result:?}"));
    }
    println!("PARAGRAPH_FIXTURE {precision} auto {result:?}");
    let numeric = "The peak acceleration is 0.1 g.";
    match session.translate_paragraph(
        numeric,
        &engine,
        Some(Domain::Geotechnical),
        options,
        Kernel::Auto,
    ) {
        Ok(result) => {
            if result.text.matches("0.1").count() != 1
                || !result.text.contains("0.1 g")
                || result.text.contains("0.1克")
                || !result.text.contains("峰值加速度")
            {
                return Err(format!("numeric duplication accepted: {result:?}"));
            }
            println!("PARAGRAPH_NUMERIC {precision} accepted {result:?}");
        }
        Err(paragraph::Error::Rejected { index: 0, attempts }) if attempts.len() == 3 => {
            println!("PARAGRAPH_NUMERIC {precision} rejected {attempts:?}");
        }
        Err(error) => return Err(format!("unexpected numeric error {error:?}")),
    }
    let tiny = GenerationOptions {
        max_new: 1,
        ..options
    };
    if session
        .translate_paragraph(source, &engine, None, tiny, Kernel::Auto)
        .is_ok()
    {
        return Err("impossible generation budget accepted".into());
    }
    println!("PARAGRAPH_REFERENCE PASS precision={precision} paragraphs=2 segments=4 numeric_audit=1 negative=1");
    if let Some(paper) = paper {
        let input = fs::read_to_string(paper).map_err(|e| e.to_string())?;
        let mut observed = 0;
        let mut success = 0;
        let mut strategies = [0; 3];
        for line in input.lines() {
            let (id, source) = line.split_once('\t').ok_or("invalid row")?;
            observed += 1;
            match session.translate_paragraph(
                source,
                &engine,
                Some(Domain::Geotechnical),
                options,
                Kernel::Auto,
            ) {
                Ok(result) => {
                    success += 1;
                    for segment in &result.segments {
                        strategies[match segment.strategy {
                            Strategy::Whole => 0,
                            Strategy::Constrained => 1,
                            Strategy::ProtectedGaps => 2,
                        }] += 1;
                    }
                    println!("PARAGRAPH_PREVIEW {id} {result:?}");
                }
                Err(error) => println!("PARAGRAPH_REJECTED {id} {error:?}"),
            }
        }
        if observed == 0 || success == 0 {
            return Err("paper paragraph coverage gate failed".into());
        }
        println!(
            "PARAGRAPH_PAPER observed={observed} success={success} rejected={} whole={} constrained={} gaps={} quality=lexical_only",
            observed - success,
            strategies[0],
            strategies[1],
            strategies[2]
        );
    }
    Ok(())
}
