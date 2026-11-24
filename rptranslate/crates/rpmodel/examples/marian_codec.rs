use rpmodel::{marian::Vocabulary, spm::Model};
use std::io::{self, BufRead, Read, Write};

fn read(path: &str) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    Ok(data)
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let spm = args.next().ok_or("usage: marian_codec SPM VOCAB")?;
    let vocab = args.next().ok_or("missing vocab")?;
    let bytes = read(&spm)?;
    let json = read(&vocab)?;
    let v = Vocabulary::parse(std::str::from_utf8(&json).map_err(|e| e.to_string())?)
        .map_err(|e| format!("vocab: {e:?}"))?;
    let model = Model::parse(&bytes).map_err(|e| format!("model: {e:?}"))?;
    let tok = model.nmt_unigram().map_err(|e| format!("profile: {e:?}"))?;
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let ids = v
            .encode(&tok, &line)
            .map_err(|e| format!("encode: {e:?}"))?;
        let decoded = v
            .decode_skipping_special(&ids)
            .map_err(|e| format!("decode: {e:?}"))?;
        let ids = ids.iter().map(u32::to_string).collect::<Vec<_>>();
        let encoded = decoded
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        writeln!(output, "{}\t{}", ids.join(","), encoded).map_err(|e| e.to_string())?;
    }
    Ok(())
}
