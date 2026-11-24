use rpmodel::{normalization::CharsMap, spm::Model};
use std::io::Read;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: normalize_spm FILE TEXT")?;
    let input = args.next().ok_or("missing input")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let model = Model::parse(&bytes).map_err(|e| format!("{e:?}"))?;
    if model.normalizer != "nmt_nfkc" {
        return Err("expected nmt_nfkc".into());
    }
    let map = CharsMap::parse(model.charsmap).map_err(|e| format!("{e:?}"))?;
    print!("{}", map.normalize(&input).map_err(|e| format!("{e:?}"))?);
    Ok(())
}
