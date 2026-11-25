//! Strict RPModel v1 package validation for the audited Marian revision.
//! Hashes detect corruption; a self-authored manifest is not a signature.
use crate::{
    config::{self, MarianConfig},
    safetensors::{self, Dtype, Parser, SafeTensorReader},
    sha256::digest_reader,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub const REVISION: &str = "a4193836671069f1f80ce341f9227a850ffb52d4";
pub const SOURCE_SHA256: &str = "69a1d6ec829cee349360b3b677ac0aa99a7d88822d1a6578370029efffdca3f5";
pub const PAYLOAD_BYTES: u64 = 312033188;
pub const TENSOR_COUNT: usize = 256;
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_WEIGHTS: u64 = PAYLOAD_BYTES + 8 * 1024 * 1024 + 8;
const PINNED_FILES: [(&str, u64, &str); 5] = [
    (
        "config.json",
        1403,
        "bcae8ed74fed77fb51c58462b62397fee6b1a1a34aece79183a0dd02ad329e71",
    ),
    (
        "generation_config.json",
        293,
        "524dc91570823feb51fbd33a35e5aee3050a4441bdf65e66f6f5e945e5838e40",
    ),
    (
        "source.spm",
        806435,
        "5775ddc9e3ff2fae91554da56468ad35ff56edaba870fea74447bc7234bfdaa8",
    ),
    (
        "target.spm",
        804600,
        "81dc94efa84e4025ef38d25d5d07429fe41e3eb29d44003f1db6fe98487b0052",
    ),
    (
        "vocab.json",
        1617791,
        "f1832ca38f7aa158b9a944ff583e4901e6b37d9d4ad9241f03838e636e9cfb03",
    ),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
    Manifest,
    Profile,
    Integrity,
    Limit,
    Index,
    Reader(safetensors::Error),
    Config(config::Error),
}

#[derive(Debug, PartialEq, Eq)]
struct FileInfo {
    bytes: u64,
    sha256: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Field {
    Text(String),
    Number(u64),
    Aliases(BTreeMap<String, String>),
    Files(BTreeMap<String, FileInfo>),
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn file_info(p: &mut Parser<'_>) -> Result<FileInfo, safetensors::Error> {
    let mut fields = p.object(2, |p, key| match key {
        "bytes" => Ok(Field::Number(p.number()?)),
        "sha256" => Ok(Field::Text(p.string()?)),
        _ => Err(safetensors::Error::Header),
    })?;
    let Some(Field::Number(bytes)) = fields.remove("bytes") else {
        return Err(safetensors::Error::Header);
    };
    let Some(Field::Text(sha256)) = fields.remove("sha256") else {
        return Err(safetensors::Error::Header);
    };
    if !valid_hash(&sha256) {
        return Err(safetensors::Error::Header);
    }
    Ok(FileInfo { bytes, sha256 })
}

fn manifest(input: &str) -> Result<BTreeMap<String, FileInfo>, Error> {
    if input.len() as u64 > MAX_MANIFEST {
        return Err(Error::Limit);
    }
    let mut p = Parser { rest: input };
    let mut fields = p
        .object(11, |p, key| match key {
            "format" | "source" | "revision" | "license" | "checkpoint_sha256" | "dtype" => {
                Ok(Field::Text(p.string()?))
            }
            "version" | "tensor_count" | "payload_bytes" => Ok(Field::Number(p.number()?)),
            "aliases" => Ok(Field::Aliases(p.object(2, |p, _| p.string())?)),
            "files" => Ok(Field::Files(p.object(6, |p, _| file_info(p))?)),
            _ => Err(safetensors::Error::Header),
        })
        .map_err(|_| Error::Manifest)?;
    p.ws();
    if !p.rest.is_empty() {
        return Err(Error::Manifest);
    }
    for (key, value) in [
        ("format", "RPModel"),
        ("source", "Helsinki-NLP/opus-mt-en-zh"),
        ("revision", REVISION),
        ("license", "Apache-2.0"),
        ("checkpoint_sha256", SOURCE_SHA256),
        ("dtype", "F32"),
    ] {
        if fields.remove(key) != Some(Field::Text(value.into())) {
            return Err(Error::Profile);
        }
    }
    for (key, value) in [
        ("version", 1),
        ("tensor_count", TENSOR_COUNT as u64),
        ("payload_bytes", PAYLOAD_BYTES),
    ] {
        if fields.remove(key) != Some(Field::Number(value)) {
            return Err(Error::Profile);
        }
    }
    let Some(Field::Aliases(aliases)) = fields.remove("aliases") else {
        return Err(Error::Manifest);
    };
    if aliases
        != BTreeMap::from([
            (
                "model.encoder.embed_tokens.weight".into(),
                "model.shared.weight".into(),
            ),
            (
                "model.decoder.embed_tokens.weight".into(),
                "model.shared.weight".into(),
            ),
        ])
    {
        return Err(Error::Profile);
    }
    let Some(Field::Files(files)) = fields.remove("files") else {
        return Err(Error::Manifest);
    };
    if files.len() != 6 {
        return Err(Error::Manifest);
    }
    for (name, bytes, sha256) in PINNED_FILES {
        if files.get(name)
            != Some(&FileInfo {
                bytes,
                sha256: sha256.into(),
            })
        {
            return Err(Error::Integrity);
        }
    }
    let weights = files.get("weights.safetensors").ok_or(Error::Manifest)?;
    if weights.bytes <= PAYLOAD_BYTES || weights.bytes > MAX_WEIGHTS {
        return Err(Error::Limit);
    }
    Ok(files)
}

/// The only aliases accepted for this profile. LM head also uses shared table
/// in the later inference adapter; no separate lm_head is in the source file.
pub fn canonical_name(name: &str) -> &str {
    match name {
        "model.encoder.embed_tokens.weight" | "model.decoder.embed_tokens.weight" => {
            "model.shared.weight"
        }
        _ => name,
    }
}

pub fn expected_shapes() -> BTreeMap<String, Vec<u64>> {
    let mut out = BTreeMap::from([
        ("model.shared.weight".into(), vec![65001, 512]),
        ("final_logits_bias".into(), vec![1, 65001]),
    ]);
    for side in ["encoder", "decoder"] {
        let prefix = format!("model.{side}.");
        out.insert(format!("{prefix}embed_positions.weight"), vec![512, 512]);
        for i in 0..6 {
            let layer = format!("{prefix}layers.{i}.");
            let attention: &[&str] = if side == "encoder" {
                &["self_attn"]
            } else {
                &["self_attn", "encoder_attn"]
            };
            for attn in attention {
                for projection in ["q_proj", "k_proj", "v_proj", "out_proj"] {
                    out.insert(format!("{layer}{attn}.{projection}.weight"), vec![512, 512]);
                    out.insert(format!("{layer}{attn}.{projection}.bias"), vec![512]);
                }
                for field in ["weight", "bias"] {
                    out.insert(format!("{layer}{attn}_layer_norm.{field}"), vec![512]);
                }
            }
            for (name, shape) in [
                ("fc1.weight", vec![2048, 512]),
                ("fc1.bias", vec![2048]),
                ("fc2.weight", vec![512, 2048]),
                ("fc2.bias", vec![512]),
                ("final_layer_norm.weight", vec![512]),
                ("final_layer_norm.bias", vec![512]),
            ] {
                out.insert(format!("{layer}{name}"), shape);
            }
        }
    }
    out
}

fn validate_index<R: Read + Seek>(reader: &SafeTensorReader<R>) -> Result<(), Error> {
    let tensors = reader.tensors();
    let metadata = reader.metadata();
    let expected = expected_shapes();
    if tensors.len() != TENSOR_COUNT || metadata.len() != 4 {
        return Err(Error::Index);
    }
    for (key, value) in [
        ("format", "pt"),
        ("rpmodel_format", "1"),
        ("source_revision", REVISION),
        ("source_sha256", SOURCE_SHA256),
    ] {
        if metadata.get(key).map(String::as_str) != Some(value) {
            return Err(Error::Index);
        }
    }
    for (name, shape) in expected {
        let info = tensors.get(&name).ok_or(Error::Index)?;
        if info.dtype != Dtype::F32 || info.shape != shape {
            return Err(Error::Index);
        }
    }
    Ok(())
}

fn regular_file(root: &Path, name: &str) -> Result<File, Error> {
    let path = root.join(name);
    if !fs::symlink_metadata(&path)
        .map_err(|_| Error::Io)?
        .file_type()
        .is_file()
    {
        return Err(Error::Io);
    }
    File::open(path).map_err(|_| Error::Io)
}

/// Open the verified fixed-profile index.
pub fn open(root: &Path) -> Result<SafeTensorReader<File>, Error> {
    Ok(open_profile(root)?.1)
}

/// Parse typed configuration from the exact verified config bytes and return
/// the weight reader. Stable local files required; no signature guarantee.
pub fn open_profile(root: &Path) -> Result<(MarianConfig, SafeTensorReader<File>), Error> {
    let mut input = String::new();
    regular_file(root, "manifest.json")?
        .take(MAX_MANIFEST + 1)
        .read_to_string(&mut input)
        .map_err(|_| Error::Manifest)?;
    let files = manifest(&input)?;
    let mut weights = None;
    let mut model_config = None;
    let mut generation_config = None;
    for (name, info) in files {
        let mut file = regular_file(root, &name)?;
        if file.metadata().map_err(|_| Error::Io)?.len() != info.bytes {
            return Err(Error::Integrity);
        }
        let (bytes, sha) = digest_reader(&mut file, info.bytes).map_err(|_| Error::Integrity)?;
        if bytes != info.bytes || sha != info.sha256 {
            return Err(Error::Integrity);
        }
        if name == "weights.safetensors" {
            file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
            weights = Some(file);
        } else if name == "config.json" || name == "generation_config.json" {
            file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
            let mut text = String::new();
            file.take(info.bytes + 1).read_to_string(&mut text).map_err(|_| Error::Io)?;
            if name == "config.json" {
                model_config = Some(text);
            } else {
                generation_config = Some(text);
            }
        }
    }
    let reader = SafeTensorReader::open(weights.ok_or(Error::Manifest)?).map_err(Error::Reader)?;
    validate_index(&reader)?;
    let config = MarianConfig::parse(
        &model_config.ok_or(Error::Manifest)?,
        &generation_config.ok_or(Error::Manifest)?,
    ).map_err(Error::Config)?;
    Ok((config, reader))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        let mut files = PINNED_FILES
            .iter()
            .map(|(name, bytes, sha)| format!(r#""{name}":{{"bytes":{bytes},"sha256":"{sha}"}}"#))
            .collect::<Vec<_>>();
        files.push(format!(
            r#""weights.safetensors":{{"bytes":{},"sha256":"{}"}}"#,
            PAYLOAD_BYTES + 1024,
            "a".repeat(64)
        ));
        format!(
            r#"{{"format":"RPModel","version":1,"source":"Helsinki-NLP/opus-mt-en-zh","revision":"{REVISION}","license":"Apache-2.0","checkpoint_sha256":"{SOURCE_SHA256}","dtype":"F32","tensor_count":256,"payload_bytes":{PAYLOAD_BYTES},"aliases":{{"model.encoder.embed_tokens.weight":"model.shared.weight","model.decoder.embed_tokens.weight":"model.shared.weight"}},"files":{{{}}}}}"#,
            files.join(",")
        )
    }

    #[test]
    fn strict_manifest_rejects_invalid_profiles_and_paths() {
        let text = fixture();
        assert_eq!(manifest(&text).unwrap().len(), 6);
        for bad in [
            text.replace(r#""version":1"#, r#""version":2"#),
            text.replace("config.json", "../config.json"),
            text.replace("weights.safetensors", "../weights.safetensors"),
            text.replace(r#""version":1"#, r#""version":1,"version":1"#),
            text.replace("model.shared.weight", "untrusted.weight"),
            text.replace(&"a".repeat(64), &"g".repeat(64)),
            text.clone() + "garbage",
        ] {
            assert!(manifest(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn exact_canonical_shape_inventory() {
        let shapes = expected_shapes();
        assert_eq!(shapes.len(), TENSOR_COUNT);
        assert_eq!(
            shapes
                .values()
                .map(|s| s.iter().product::<u64>() * 4)
                .sum::<u64>(),
            PAYLOAD_BYTES
        );
        assert_eq!(
            canonical_name("model.decoder.embed_tokens.weight"),
            "model.shared.weight"
        );
        assert_eq!(
            shapes[canonical_name("model.encoder.embed_tokens.weight")],
            [65001, 512]
        );
        assert!(!shapes.contains_key("model.encoder.embed_tokens.weight"));
    }
}
