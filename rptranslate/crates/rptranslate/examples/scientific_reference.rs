//! D1.10b real-model scientific protection and paper observations.
use rpinfer::{
    marian_model::{GenerationMode, GenerationOptions, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    mixed_model::PAYLOAD_BYTES,
};
use rptranslate::offline::OfflineTranslator;
use std::{fs, path::Path, time::Instant};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let root = args.next().ok_or("missing model directory")?;
    let precision = args.next().ok_or("missing precision")?;
    let input = args.next();
    let model = match precision.as_str() {
        "fp32" => OfflineTranslator::load(Path::new(&root), MODEL_PAYLOAD_BYTES),
        "int8" => OfflineTranslator::load_mixed(Path::new(&root), PAYLOAD_BYTES),
        _ => return Err("unknown precision".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let session = model.session().map_err(|e| format!("{e:?}"))?;
    let options = GenerationOptions {
        beam_width: 1,
        max_new: 256,
        mode: GenerationMode::Cached,
    };
    let fixtures: [(&str, &[&str]); 6] = [
        ("The peak acceleration is 0.1 g.", &["0.1 g"]),
        ("The incidence angle equals 30◦in both cases.", &["30◦"]),
        ("The pressure is 20 kPa.", &["20 kPa"]),
        ("The time step is 1.5 × 10⁻⁴ s.", &["1.5 × 10⁻⁴ s"]),
        ("We use $E=mc^2$ and P–SV waves.", &["$E=mc^2$", "P–SV"]),
        (
            "The Mw 7.8 event was recorded below 2 Hz.",
            &["Mw 7.8", "2 Hz"],
        ),
    ];
    let mut total = 0;
    for (index, (source, expected)) in fixtures.iter().enumerate() {
        let output = session
            .translate_scientific(source, &[], options, Kernel::Auto)
            .map_err(|e| format!("fixture {index}: {e:?}"))?;
        if output.protected_spans != expected.len() || output.model_calls == 0 {
            return Err(format!("fixture {index}: span or model call count"));
        }
        let mut rest = output.text.as_str();
        for literal in *expected {
            let at = rest
                .find(literal)
                .ok_or_else(|| format!("fixture {index}: missing {literal}"))?;
            rest = &rest[at + literal.len()..];
        }
        total += expected.len();
        println!("SCIENCE_FIXTURE {precision} {index}: {}", output.text);
    }
    if session
        .translate_scientific("broken $x", &[], options, Kernel::Auto)
        .is_ok()
    {
        return Err("unclosed formula accepted".into());
    }
    println!("SCIENCE_REFERENCE PASS precision={precision} cases=6 literals={total} negatives=1");
    if let Some(input) = input {
        let input = fs::read_to_string(input).map_err(|e| e.to_string())?;
        for line in input.lines() {
            let (id, source) = line.split_once('\t').ok_or("invalid segment")?;
            let start = Instant::now();
            match session.translate_scientific(source, &[], options, Kernel::Auto) {
                Ok(output) => {
                    let seconds = start.elapsed().as_secs_f64();
                    let hex = output
                        .text
                        .as_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    println!(
                        "SCIENCE_ROW\t{id}\t{}\t{}\t{seconds:.9}\t{hex}",
                        output.protected_spans, output.model_calls
                    );
                }
                Err(error) => println!("SCIENCE_ERROR\t{id}\t{error:?}"),
            }
        }
    }
    Ok(())
}
