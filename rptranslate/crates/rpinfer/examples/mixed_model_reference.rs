//! Offline Rust conversion and complete owning model load probe.
use rpinfer::mixed_model::{convert, MixedModel, PAYLOAD_BYTES};
use rpmodel::quantized::Reader;
use std::{fs::File, path::Path, time::Instant};

fn batch_reference(root: &Path) -> Result<(), String> {
    let file = File::open(root.join("weights.rpq")).map_err(|e| format!("{e:?}"))?;
    let mut reader = Reader::open(file, 512 * 1024 * 1024).map_err(|e| format!("{e:?}"))?;
    let tensors = reader.tensors().clone();
    let mut scalar_ms = 0.;
    let mut batch_ms = 0.;
    let mut elements = 0;
    let mut blocks = 0;
    for (name, info) in &tensors {
        let mut old_codes = Vec::with_capacity(info.elements as usize);
        let mut old_scales = Vec::with_capacity(info.blocks as usize);
        let start = Instant::now();
        for index in 0..info.blocks {
            let block = reader
                .read_block(name, index, info.block_size as usize + 4)
                .map_err(|e| format!("{e:?}"))?;
            old_codes.extend_from_slice(&block.codes);
            old_scales.push(block.scale);
        }
        scalar_ms += start.elapsed().as_secs_f64() * 1000.;
        let start = Instant::now();
        let mut codes = vec![0; info.elements as usize];
        let mut scales = vec![0.; info.blocks as usize];
        reader
            .read_tensor_into(name, &mut codes, &mut scales)
            .map_err(|e| format!("{e:?}"))?;
        batch_ms += start.elapsed().as_secs_f64() * 1000.;
        if codes != old_codes
            || scales
                .iter()
                .zip(&old_scales)
                .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err(format!("batch mismatch: {name}"));
        }
        elements += info.elements;
        blocks += info.blocks;
    }
    if (tensors.len(), elements, blocks) != (97, 77_320_704, 1_208_136) {
        return Err("batch reference inventory".into());
    }
    println!(
        "BATCH_READ_REFERENCE PASS tensors=97 elements={elements} blocks={blocks} bitwise=true scalar_ms={scalar_ms:.3} batch_ms={batch_ms:.3}"
    );
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, source, dest] if mode == "convert" => {
            convert(Path::new(source), Path::new(dest), PAYLOAD_BYTES)?;
        }
        [mode, root] if mode == "verify" => {
            let model = MixedModel::load(Path::new(root), PAYLOAD_BYTES)?;
            let shared = model
                .matrix("model.shared.weight")
                .ok_or("missing shared")?;
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
        [mode, root] if mode == "batch-reference" => batch_reference(Path::new(root))?,
        _ => return Err("usage: mixed_model_reference convert SOURCE DEST | verify DEST | batch-reference DEST".into()),
    }
    Ok(())
}
