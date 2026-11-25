//! Offline Rust conversion and complete owning model load probe.
use rpinfer::mixed_model::{convert, MixedModel, PAYLOAD_BYTES};
use std::path::Path;

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, source, dest] if mode == "convert" => {
            convert(Path::new(source), Path::new(dest), PAYLOAD_BYTES)?;
        }
        [mode, root] if mode == "verify" => {
            let model = MixedModel::load(Path::new(root), PAYLOAD_BYTES)?;
            let shared = model.matrix("model.shared.weight").ok_or("missing shared")?;
            for name in [
                "model.encoder.embed_tokens.weight",
                "model.decoder.embed_tokens.weight",
            ] {
                if !std::ptr::eq(shared, model.matrix(name).ok_or("missing alias")?) {
                    return Err("duplicate shared table".into());
                }
            }
            if model.config().vocab != 65001 || model.counts() != (97, 159) {
                return Err("wrong loaded profile".into());
            }
            println!(
                "MIXED_MODEL_LOADED q_tensors=97 f_tensors=159 payload_bytes={} aliases=2",
                model.payload_bytes()
            );
        }
        _ => return Err("usage: mixed_model_reference convert SOURCE DEST | verify DEST".into()),
    }
    Ok(())
}
