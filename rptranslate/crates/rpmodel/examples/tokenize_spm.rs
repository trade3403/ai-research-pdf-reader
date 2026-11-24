//! Development CLI for independent reference checks, not a translation API.
use rpmodel::spm::Model;
use std::io::{self, BufRead, Read, Write};

fn hex(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len() * 2);
    for b in text.bytes() {
        write!(&mut out, "{b:02x}").expect("String writes cannot fail");
    }
    out
}

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("usage: tokenize_spm FILE")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let model = Model::parse(&bytes).map_err(|e| format!("model: {e:?}"))?;
    let tok = model.nmt_unigram().map_err(|e| format!("profile: {e:?}"))?;
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let ids = tok.encode(&line).map_err(|e| format!("encode: {e:?}"))?;
        let decoded = tok.decode(&ids).map_err(|e| format!("decode: {e:?}"))?;
        let ids = ids.iter().map(usize::to_string).collect::<Vec<_>>();
        writeln!(output, "{}\t{}", ids.join(","), hex(&decoded)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
