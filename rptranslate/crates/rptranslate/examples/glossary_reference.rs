//! Real-model terminology checks with independently declared expected phrases.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::{
    glossary::{Domain, Glossary},
    offline::OfflineTranslator,
    protection::{Kind, Plan},
};
use std::{fs, path::Path, time::Instant};

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
    let bio = Glossary::builtin(Domain::Biology).map_err(|e| format!("{e:?}"))?;
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 256,
        mode: GenerationMode::Cached,
    };
    let cases = [
        (
            Domain::Geotechnical,
            "We measured maximum strain.",
            "最大应变",
        ),
        (
            Domain::Geotechnical,
            "We calculated site response.",
            "场地响应",
        ),
        (
            Domain::Geotechnical,
            "We estimated seismic vulnerability.",
            "地震易损性",
        ),
        (
            Domain::Geotechnical,
            "We measured spectral amplitude.",
            "谱幅值",
        ),
        (
            Domain::Geotechnical,
            "We used a Ricker wavelet.",
            "Ricker 子波",
        ),
        (
            Domain::Geotechnical,
            "We measured incidence angle.",
            "入射角",
        ),
        (
            Domain::Geotechnical,
            "We examined input motions.",
            "输入地震动",
        ),
        (
            Domain::Biology,
            "We isolated a bacterial strain.",
            "细菌菌株",
        ),
    ];
    for (i, (domain, source, target)) in cases.iter().enumerate() {
        let glossary = if *domain == Domain::Biology {
            &bio
        } else {
            &geo
        };
        let baseline = session
            .translate(source, options, Kernel::Auto)
            .map_err(|e| format!("baseline {i}: {e:?}"))?;
        let output = session
            .translate_glossary(source, glossary, options, Kernel::Auto)
            .map_err(|e| format!("glossary {i}: {e:?}"))?;
        if output.term_spans != 1 || output.model_calls == 0 || !output.text.contains(target) {
            return Err(format!("fixture {i}: expected term absent"));
        }
        println!(
            "TERM_FIXTURE {precision} {i} source={source:?} baseline={:?} protected={:?}",
            baseline.text, output.text
        );
    }
    println!(
        "TERM_REFERENCE PASS precision={precision} cases={} geo_aliases={} bio_aliases={}",
        cases.len(),
        geo.len(),
        bio.len()
    );
    if let Some(paper) = paper {
        let input = fs::read_to_string(paper).map_err(|e| e.to_string())?;
        let mut total = 0;
        let mut success = 0;
        let mut terms = 0;
        let mut retained = 0;
        let mut before = 0;
        let mut seconds = 0.0;
        for line in input.lines() {
            let (id, source) = line.split_once('\t').ok_or("invalid segment")?;
            total += 1;
            let plan = match Plan::prepare_indexed(source, &geo) {
                Ok(plan) => plan,
                Err(error) => {
                    println!("TERM_REJECTED {id} plan={error:?}");
                    continue;
                }
            };
            let expected: Vec<_> = plan
                .spans()
                .iter()
                .filter(|s| s.kind == Kind::Term)
                .map(|s| s.replacement.as_str())
                .collect();
            let baseline = session.translate(source, options, Kernel::Auto);
            let start = Instant::now();
            let output = session.translate_glossary(source, &geo, options, Kernel::Auto);
            seconds += start.elapsed().as_secs_f64();
            match (baseline, output) {
                (Ok(b), Ok(o)) => {
                    success += 1;
                    if o.term_spans != expected.len() {
                        return Err(format!("segment {id}: inconsistent term count"));
                    }
                    terms += expected.len();
                    let mut rest = o.text.as_str();
                    for target in &expected {
                        if let Some(at) = rest.find(target) {
                            retained += 1;
                            rest = &rest[at + target.len()..];
                        }
                    }
                    // Baseline target presence, not ordered or semantic accuracy.
                    before += expected.iter().filter(|t| b.text.contains(**t)).count();
                    println!(
                        "TERM_PREVIEW {id} source={source:?} baseline={:?} protected={:?}",
                        b.text, o.text
                    );
                }
                (b, o) => {
                    println!(
                        "TERM_REJECTED {id} baseline={:?} protected={:?}",
                        b.err(),
                        o.err()
                    );
                }
            }
        }
        if total == 0 || success == 0 || terms == 0 || retained != terms {
            return Err("paper sample term restoration gate failed".into());
        }
        println!(
            "TERM_PAPER observed={total} success={success} rejected={} expected={terms} baseline_target_presence={before} ordered_restored={retained} seconds={seconds:.6} quality=observational",
            total - success
        );
    }
    Ok(())
}
