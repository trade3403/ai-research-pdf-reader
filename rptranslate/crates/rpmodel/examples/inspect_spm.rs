use rpmodel::spm::Model;
use std::io::Read;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("usage: inspect_spm FILE")?;
    let mut data = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    let m = Model::parse(&data).map_err(|e| format!("{e:?}"))?;
    println!(
        "pieces={} type={} normalizer={:?} charsmap_bytes={} unk={} bos={} eos={} pad={} byte_fallback={}",
        m.pieces.len(),
        m.model_type,
        m.normalizer,
        m.charsmap.len(),
        m.unk_id,
        m.bos_id,
        m.eos_id,
        m.pad_id,
        m.byte_fallback
    );
    Ok(())
}
