//! Offline Rust conversion and complete owning model load probe.
use rpinfer::mixed_model::{convert, MixedModel, PAYLOAD_BYTES};
use rpmodel::{
    quantized::Reader,
    sha256::{digest_reader, digest_reader_reference},
};
use std::{fs::File, path::Path, time::Instant};

fn sha_reference(path: &Path) -> Result<(), String> {
    let limit = 512 * 1024 * 1024;
    let file = File::open(path).map_err(|e| format!("{e:?}"))?;
    let start = Instant::now();
    let expected = digest_reader_reference(file, limit).map_err(|e| format!("{e:?}"))?;
    let reference_ms = start.elapsed().as_secs_f64() * 1000.;
    let file = File::open(path).map_err(|e| format!("{e:?}"))?;
    let start = Instant::now();
    let actual = digest_reader(file, limit).map_err(|e| format!("{e:?}"))?;
    let batch_ms = start.elapsed().as_secs_f64() * 1000.;
    if expected != actual {
        return Err("SHA256 batch mismatch".into());
    }
    println!(
        "SHA_READ_REFERENCE PASS bytes={} digest={} reference_ms={reference_ms:.3} batch_ms={batch_ms:.3}",
        actual.0, actual.1
    );
    Ok(())
}

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

// Offline CI probe only: private, immutable fixtures with no concurrent writers.
#[cfg(all(
    any(target_os = "linux", target_os = "windows"),
    target_pointer_width = "64"
))]
fn mapped_reference(root: &Path) -> Result<(), String> {
    let start = Instant::now();
    let ordinary = MixedModel::load(root, PAYLOAD_BYTES)?;
    let ordinary_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    // SAFETY: CI completes writes in its private fixture before this call.
    let mapped = unsafe { MixedModel::load_mapped(root, PAYLOAD_BYTES) }?;
    let mapped_ms = start.elapsed().as_secs_f64() * 1000.;
    let mut q = 0;
    let mut f = 0;
    let mut elements = 0;
    let mut blocks = 0;
    let mut floats = 0;
    for (name, shape) in rpmodel::package::expected_shapes() {
        if let Some(a) = ordinary.matrix(&name) {
            let b = mapped.matrix(&name).ok_or("missing mapped matrix")?;
            let a = a.as_input(&name, &shape);
            let b = b.as_input(&name, &shape);
            if a.codes != b.codes
                || a.scales.len() != b.scales.len()
                || a.scales
                    .iter()
                    .zip(b.scales)
                    .any(|(x, y)| x.to_bits() != y.to_bits())
            {
                return Err(format!("mapped matrix mismatch: {name}"));
            }
            q += 1;
            elements += a.codes.len();
            blocks += a.scales.len();
        } else {
            let a = ordinary.auxiliary(&name).ok_or("missing auxiliary")?;
            let b = mapped.auxiliary(&name).ok_or("missing mapped auxiliary")?;
            if a.shape() != b.shape()
                || a.data().len() != b.data().len()
                || a.data()
                    .iter()
                    .zip(b.data())
                    .any(|(x, y)| x.to_bits() != y.to_bits())
            {
                return Err(format!("mapped auxiliary mismatch: {name}"));
            }
            f += 1;
            floats += a.data().len();
        }
    }
    if (q, f, elements, blocks, floats) != (97, 159, 77_320_704, 1_208_136, 687_593) {
        return Err("mapped inventory mismatch".into());
    }
    println!(
        "MAPPED_REFERENCE PASS q={q} f={f} codes={elements} scales={blocks} floats={floats} bitwise=true ordinary_ms={ordinary_ms:.3} mapped_ms={mapped_ms:.3}"
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
        [mode, path] if mode == "sha-reference" => sha_reference(Path::new(path))?,
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            target_pointer_width = "64"
        ))]
        [mode, root] if mode == "mapped-reference" => mapped_reference(Path::new(root))?,
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            target_pointer_width = "64"
        ))]
        [mode, root] if mode == "verify-mapped" => {
            // SAFETY: same controlled immutable fixture as mapped-reference.
            unsafe { MixedModel::load_mapped(Path::new(root), PAYLOAD_BYTES) }?;
        }
        _ => return Err(
            "usage: mixed_model_reference convert SOURCE DEST | verify DEST | batch-reference DEST | sha-reference FILE"
                .into(),
        ),
    }
    Ok(())
}
