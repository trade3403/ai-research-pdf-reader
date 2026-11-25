//! Fixed-profile RPModel-Mixed v1: RPQ8 block64 matrices and F32 auxiliaries.
//! Owns verified weights; neural network adapter is a separate next stage.
//! Hashes check integrity, not publisher authenticity. Stable local files required.
use crate::{
    quantized_table::{LoadError, QuantizedTable},
    tensor::Tensor,
};
use rpmodel::{
    config::MarianConfig,
    package,
    quantized::{self, TensorInput},
    safetensors::{Dtype, SafeTensorReader},
    sha256::digest_reader,
};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

pub const BLOCK: usize = 64;
pub const Q_COUNT: usize = 97;
pub const F_COUNT: usize = 159;
pub const Q_BYTES: usize = 82_153_248;
pub const F_BYTES: usize = 2_750_372;
pub const PAYLOAD_BYTES: usize = Q_BYTES + F_BYTES;
const Q_FILE: &str = "weights.rpq";
const F_FILE: &str = "auxiliary.safetensors";
const MANIFEST: &str = "mixed.manifest";
const ASSETS: [&str; 5] = [
    "config.json",
    "generation_config.json",
    "source.spm",
    "target.spm",
    "vocab.json",
];
type Shapes = BTreeMap<String, Vec<u64>>;

fn shapes() -> (Shapes, Shapes) {
    package::expected_shapes()
        .into_iter()
        .partition(|(name, shape)| {
            name == "model.shared.weight"
                || (shape.len() == 2 && name.contains(".layers.") && name.ends_with(".weight"))
        })
}

fn q_bytes(shape: &[u64]) -> usize {
    let n = shape.iter().product::<u64>() as usize;
    n + n / BLOCK * 4
}

fn f_bytes(shape: &[u64]) -> usize {
    shape.iter().product::<u64>() as usize * 4
}

fn debug_error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn open_file(root: &Path, name: &str) -> Result<File, String> {
    let path = root.join(name);
    if !fs::symlink_metadata(&path)
        .map_err(debug_error)?
        .file_type()
        .is_file()
    {
        return Err("expected regular file".into());
    }
    File::open(path).map_err(debug_error)
}

fn create_file(root: &Path, name: &str) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .map_err(debug_error)
}

fn hash_file(file: &mut File, cap: usize) -> Result<(u64, String), String> {
    let (len, hash) = digest_reader(&mut *file, cap as u64).map_err(debug_error)?;
    file.seek(SeekFrom::Start(0)).map_err(debug_error)?;
    Ok((len, hash))
}

fn manifest(q: &(u64, String), f: &(u64, String)) -> String {
    format!(
        "RPModel-Mixed 1\nrevision {}\nsource_sha256 {}\nblock 64\nq_tensors 97\nf_tensors 159\nq_payload 82153248\nf_payload 2750372\n{} {} {}\n{} {} {}\n",
        package::REVISION,
        package::SOURCE_SHA256,
        Q_FILE,
        q.0,
        q.1,
        F_FILE,
        f.0,
        f.1
    )
}

fn write_aux(
    root: &Path,
    source: &mut SafeTensorReader<File>,
    expected: &Shapes,
) -> Result<(), String> {
    let mut fields = Vec::new();
    let mut offset = 0;
    for (name, shape) in expected {
        let end = offset + f_bytes(shape);
        // Names and dimensions are from the fixed audited inventory.
        fields.push(format!(
            r#""{name}":{{"dtype":"F32","shape":{shape:?},"data_offsets":[{offset},{end}]}}"#
        ));
        offset = end;
    }
    let mut header = format!("{{{}}}", fields.join(","));
    while !header.len().is_multiple_of(8) {
        header.push(' ');
    }
    let mut out = BufWriter::new(create_file(root, F_FILE)?);
    out.write_all(&(header.len() as u64).to_le_bytes())
        .map_err(debug_error)?;
    out.write_all(header.as_bytes()).map_err(debug_error)?;
    for (name, shape) in expected {
        let data = source
            .read_f32(name, 0, f_bytes(shape) / 4)
            .map_err(debug_error)?;
        for v in data {
            if !v.is_finite() {
                return Err("nonfinite auxiliary weight".into());
            }
            out.write_all(&v.to_le_bytes()).map_err(debug_error)?;
        }
    }
    out.flush().map_err(debug_error)?;
    Ok(())
}

/// Convert an already verified RPModel v1 to a new directory. Manifest is
/// written last. On failure caller must discard the incomplete destination.
/// Conversion retains packed matrices, not the whole original FP32 model.
pub fn convert(source: &Path, dest: &Path, budget: usize) -> Result<(), String> {
    if budget < PAYLOAD_BYTES {
        return Err("mixed weight budget".into());
    }
    let (_, mut reader) = package::open_profile(source).map_err(debug_error)?;
    let (quantized_shapes, float_shapes) = shapes();
    fs::create_dir(dest).map_err(debug_error)?;
    for name in ASSETS {
        let data = package::verified_asset(source, name).map_err(debug_error)?;
        create_file(dest, name)?
            .write_all(&data)
            .map_err(debug_error)?;
    }
    let mut tables = BTreeMap::new();
    for (name, shape) in &quantized_shapes {
        let table = QuantizedTable::from_fp32(&mut reader, name, BLOCK, q_bytes(shape))
            .map_err(debug_error)?;
        tables.insert(name, table);
    }
    let inputs: Vec<TensorInput<'_>> = tables
        .iter()
        .map(|(&name, table)| table.as_input(name, &quantized_shapes[name]))
        .collect();
    let mut q = create_file(dest, Q_FILE)?;
    quantized::write_container(&mut q, &inputs, (Q_BYTES + 1024 * 1024) as u64)
        .map_err(debug_error)?;
    drop(inputs);
    drop(tables);
    write_aux(dest, &mut reader, &float_shapes)?;
    let mut f = open_file(dest, F_FILE)?;
    q.seek(SeekFrom::Start(0)).map_err(debug_error)?;
    let manifest = manifest(
        &hash_file(&mut q, Q_BYTES + 1024 * 1024)?,
        &hash_file(&mut f, F_BYTES + 1024 * 1024)?,
    );
    create_file(dest, MANIFEST)?
        .write_all(manifest.as_bytes())
        .map_err(debug_error)?;
    Ok(())
}

/// Owns exactly 97 packed matrices and 159 FP32 tensors, with shared
/// embedding stored once. No file handles remain after loading.
pub struct MixedModel {
    config: MarianConfig,
    quantized: BTreeMap<String, QuantizedTable>,
    auxiliary: BTreeMap<String, Tensor>,
}

impl MixedModel {
    pub fn load(root: &Path, budget: usize) -> Result<Self, String> {
        if budget < PAYLOAD_BYTES {
            return Err("mixed weight budget".into());
        }
        let mut q = open_file(root, Q_FILE)?;
        let mut f = open_file(root, F_FILE)?;
        let q_info = hash_file(&mut q, Q_BYTES + 1024 * 1024)?;
        let f_info = hash_file(&mut f, F_BYTES + 1024 * 1024)?;
        Self::load_verified(
            root,
            BufReader::new(q),
            f,
            q_info,
            f_info,
            QuantizedTable::load,
        )
    }

    /// Opt-in mapped input for caller-controlled immutable model packages.
    /// Inference arrays are still owned and the mappings are released.
    ///
    /// # Safety
    /// Both weight files must remain immutable and untruncated for the
    /// entire call, including through other handles or processes.
    #[cfg(all(
        any(target_os = "linux", target_os = "windows"),
        target_pointer_width = "64"
    ))]
    pub unsafe fn load_mapped(root: &Path, budget: usize) -> Result<Self, String> {
        use rpmodel::{mapped::MappedFile, sha256::digest_slice};
        use std::io::Cursor;

        if budget < PAYLOAD_BYTES {
            return Err("mixed weight budget".into());
        }
        let q_file = open_file(root, Q_FILE)?;
        let f_file = open_file(root, F_FILE)?;
        let q = unsafe { MappedFile::open(&q_file, Q_BYTES + 1024 * 1024) }.map_err(debug_error)?;
        let f = unsafe { MappedFile::open(&f_file, F_BYTES + 1024 * 1024) }.map_err(debug_error)?;
        let q_info =
            digest_slice(q.as_bytes(), (Q_BYTES + 1024 * 1024) as u64).map_err(debug_error)?;
        let f_info =
            digest_slice(f.as_bytes(), (F_BYTES + 1024 * 1024) as u64).map_err(debug_error)?;
        Self::load_verified(
            root,
            Cursor::new(q.as_bytes()),
            Cursor::new(f.as_bytes()),
            q_info,
            f_info,
            |reader, name, budget| QuantizedTable::load_slice(reader, name, budget),
        )
    }

    fn load_verified<R: Read + Seek, S: Read + Seek>(
        root: &Path,
        q: R,
        f: S,
        q_info: (u64, String),
        f_info: (u64, String),
        mut load_table: impl FnMut(
            &mut quantized::Reader<R>,
            &str,
            usize,
        ) -> Result<QuantizedTable, LoadError>,
    ) -> Result<Self, String> {
        let mut manifest_text = String::new();
        open_file(root, MANIFEST)?
            .take(1025)
            .read_to_string(&mut manifest_text)
            .map_err(debug_error)?;
        if manifest_text.len() > 1024 {
            return Err("manifest limit".into());
        }
        if manifest_text != manifest(&q_info, &f_info) {
            return Err("mixed manifest integrity/profile".into());
        }
        let mut assets = BTreeMap::new();
        for name in ASSETS {
            assets.insert(
                name,
                package::verified_asset(root, name).map_err(debug_error)?,
            );
        }
        let model_json = std::str::from_utf8(&assets["config.json"]).map_err(debug_error)?;
        let generation_json =
            std::str::from_utf8(&assets["generation_config.json"]).map_err(debug_error)?;
        let config = MarianConfig::parse(model_json, generation_json).map_err(debug_error)?;
        drop(assets);
        let mut q = quantized::Reader::open(q, q_info.0).map_err(debug_error)?;
        let mut f = SafeTensorReader::open(f).map_err(debug_error)?;
        let (q_shapes, f_shapes) = shapes();
        if q.tensors().len() != Q_COUNT || f.tensors().len() != F_COUNT || !f.metadata().is_empty()
        {
            return Err("mixed tensor count".into());
        }
        for (name, shape) in &q_shapes {
            let info = q.tensors().get(name).ok_or("missing packed tensor")?;
            if &info.shape != shape
                || info.block_size != BLOCK as u32
                || info.bytes != q_bytes(shape) as u64
            {
                return Err("packed tensor profile".into());
            }
        }
        for (name, shape) in &f_shapes {
            let info = f.tensors().get(name).ok_or("missing auxiliary tensor")?;
            if &info.shape != shape || info.dtype != Dtype::F32 {
                return Err("auxiliary tensor profile".into());
            }
        }
        let mut quantized = BTreeMap::new();
        let mut auxiliary = BTreeMap::new();
        for (name, shape) in q_shapes {
            let table = load_table(&mut q, &name, q_bytes(&shape)).map_err(debug_error)?;
            quantized.insert(name, table);
        }
        for (name, shape) in f_shapes {
            let data = f
                .read_f32(&name, 0, f_bytes(&shape) / 4)
                .map_err(debug_error)?;
            let dims: Vec<usize> = shape.iter().map(|&v| v as usize).collect();
            auxiliary.insert(name, Tensor::new(&dims, data).map_err(debug_error)?);
        }
        let model = Self {
            config,
            quantized,
            auxiliary,
        };
        if model.payload_bytes() != PAYLOAD_BYTES {
            return Err("mixed payload mismatch".into());
        }
        Ok(model)
    }

    pub fn config(&self) -> &MarianConfig {
        &self.config
    }

    pub fn matrix(&self, name: &str) -> Option<&QuantizedTable> {
        self.quantized.get(package::canonical_name(name))
    }

    pub fn auxiliary(&self, name: &str) -> Option<&Tensor> {
        self.auxiliary.get(name)
    }

    pub fn counts(&self) -> (usize, usize) {
        (self.quantized.len(), self.auxiliary.len())
    }

    pub fn payload_bytes(&self) -> usize {
        self.quantized
            .values()
            .map(QuantizedTable::payload_bytes)
            .sum::<usize>()
            + self
                .auxiliary
                .values()
                .map(|v| v.data().len() * 4)
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_inventory_and_budget() {
        let (q, f) = shapes();
        assert_eq!((q.len(), f.len()), (Q_COUNT, F_COUNT));
        assert_eq!(q.values().map(|s| q_bytes(s)).sum::<usize>(), Q_BYTES);
        assert_eq!(f.values().map(|s| f_bytes(s)).sum::<usize>(), F_BYTES);
        assert!(q
            .values()
            .all(|s| s.len() == 2 && s[1].is_multiple_of(BLOCK as u64)));
        assert!(f.contains_key("model.encoder.embed_positions.weight"));
        assert!(MixedModel::load(Path::new("missing"), PAYLOAD_BYTES - 1).is_err());
    }
}
