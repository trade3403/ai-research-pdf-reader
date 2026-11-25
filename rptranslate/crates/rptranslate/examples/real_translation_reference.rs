//! Reusable offline text session; emit actual token IDs and decoded UTF-8.
use rpinfer::{marian_model::MODEL_PAYLOAD_BYTES, matmul::Kernel};
use rptranslate::offline::{Error, OfflineTranslator};
use std::{
    io::{self, BufRead},
    path::Path,
};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let translator = OfflineTranslator::load(Path::new(&path), MODEL_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let session = translator.session().map_err(|e| format!("{e:?}"))?;
    if session.encode(" ").err() != Some(Error::Empty)
        || session.encode(">>cmn_Hant<< hello").err() != Some(Error::LanguagePrefix)
        || session.encode(&"hello ".repeat(600)).err() != Some(Error::SourceTooLong)
    {
        return Err("invalid input accepted".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        for beam in [1, 4] {
            let mut options = translator.default_options();
            options.beam_width = beam;
            options.max_new = 48;
            let result = session
                .translate(&line, options, Kernel::Auto)
                .map_err(|e| format!("{e:?}"))?;
            let src = result
                .source_tokens
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let tgt = result
                .target_tokens
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let hex = result
                .text
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            println!(
                "{beam}\t{src}\t{tgt}\t{hex}\t{}\t{:.17}",
                u8::from(result.reached_limit),
                result.score
            );
        }
    }
    Ok(())
}
