//! Typed model catalog and bounded integrity checks for independent deployment.
//! A catalog must come from a trusted source; hashes alone are not signatures.
use crate::sha256::digest_reader;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::Path,
};

const MAX_ENTRIES: usize = 1024;
const MAX_FILES: usize = 64;
const MAX_PACKAGE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precision {
    Fp32,
    Int8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub version: Version,
    pub precision: Precision,
    pub min_engine: Version,
    pub files: BTreeMap<String, Asset>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidName,
    InvalidAsset,
    Limit,
    Duplicate,
    Integrity,
    Io,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn asset_name(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 96
        || !name.as_bytes()[0].is_ascii_lowercase()
        || name.ends_with('.')
        || name.contains("..")
        || !name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
    {
        return false;
    }
    let stem = name.split('.').next().unwrap_or("");
    !(matches!(stem, "con" | "prn" | "aux" | "nul")
        || (stem.len() == 4
            && (stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.as_bytes()[3].is_ascii_digit()))
}

impl Asset {
    fn valid(&self) -> bool {
        self.bytes > 0
            && self.bytes <= MAX_PACKAGE_BYTES
            && self.sha256.len() == 64
            && self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    /// Check the exact length and digest, streaming with a hard byte limit.
    pub fn verify(&self, input: impl Read) -> Result<(), Error> {
        if !self.valid() {
            return Err(Error::InvalidAsset);
        }
        let (bytes, hash) = digest_reader(input, self.bytes).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData {
                Error::Integrity
            } else {
                Error::Io
            }
        })?;
        if bytes != self.bytes || hash != self.sha256 {
            return Err(Error::Integrity);
        }
        Ok(())
    }
}

impl Entry {
    pub fn validate(&self) -> Result<u64, Error> {
        if !identifier(&self.id) {
            return Err(Error::InvalidName);
        }
        if self.files.is_empty() || self.files.len() > MAX_FILES {
            return Err(Error::Limit);
        }
        let mut total = 0_u64;
        for (name, asset) in &self.files {
            if !asset_name(name) {
                return Err(Error::InvalidName);
            }
            if !asset.valid() {
                return Err(Error::InvalidAsset);
            }
            total = total
                .checked_add(asset.bytes)
                .filter(|&n| n <= MAX_PACKAGE_BYTES)
                .ok_or(Error::Limit)?;
        }
        Ok(total)
    }

    /// Verify an already materialized flat package, not its model architecture.
    /// Caller must control the directory and exclude concurrent mutation,
    /// including changes to ancestor directories. No installation is performed.
    pub fn verify_directory(&self, root: &Path) -> Result<(), Error> {
        self.validate()?;
        let metadata = fs::symlink_metadata(root).map_err(|_| Error::Io)?;
        if !metadata.is_dir() || is_link(&metadata) {
            return Err(Error::Integrity);
        }
        let mut seen = BTreeSet::new();
        for item in fs::read_dir(root).map_err(|_| Error::Io)? {
            let item = item.map_err(|_| Error::Io)?;
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| Error::InvalidName)?;
            let asset = self.files.get(&name).ok_or(Error::Integrity)?;
            let metadata = fs::symlink_metadata(item.path()).map_err(|_| Error::Io)?;
            if !metadata.is_file() || is_link(&metadata) || metadata.len() != asset.bytes {
                return Err(Error::Integrity);
            }
            asset.verify(File::open(item.path()).map_err(|_| Error::Io)?)?;
            seen.insert(name);
        }
        if seen.len() != self.files.len() {
            return Err(Error::Integrity);
        }
        Ok(())
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[derive(Debug)]
pub struct Catalog {
    entries: Vec<Entry>,
}

impl Catalog {
    pub fn new(entries: Vec<Entry>) -> Result<Self, Error> {
        if entries.len() > MAX_ENTRIES {
            return Err(Error::Limit);
        }
        let mut keys = BTreeSet::new();
        for entry in &entries {
            entry.validate()?;
            if !keys.insert((&entry.id, entry.version, entry.precision)) {
                return Err(Error::Duplicate);
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// No implicit precision fallback and no incompatible engine upgrade.
    pub fn select(&self, id: &str, precision: Precision, engine: Version) -> Option<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.id == id && e.precision == precision && e.min_engine <= engine)
            .max_by_key(|e| e.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(version: Version, precision: Precision) -> Entry {
        Entry {
            id: "opus-en-zh".into(),
            version,
            precision,
            min_engine: Version(0, 1, 0),
            files: BTreeMap::from([(
                "weights.bin".into(),
                Asset {
                    bytes: 3,
                    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .into(),
                },
            )]),
        }
    }

    #[test]
    fn selects_numeric_version_and_exact_precision() {
        let a = entry(Version(1, 9, 0), Precision::Int8);
        let b = entry(Version(1, 10, 0), Precision::Int8);
        let c = entry(Version(2, 0, 0), Precision::Fp32);
        let mut future = entry(Version(3, 0, 0), Precision::Int8);
        future.min_engine = Version(1, 0, 0);
        let catalog = Catalog::new(vec![a, b, c, future]).unwrap();
        let engine = Version(0, 1, 0);
        assert_eq!(catalog.entries().len(), 4);
        assert_eq!(
            catalog
                .select("opus-en-zh", Precision::Int8, engine)
                .unwrap()
                .version,
            Version(1, 10, 0)
        );
        assert_eq!(
            catalog
                .select("opus-en-zh", Precision::Fp32, engine)
                .unwrap()
                .version,
            Version(2, 0, 0)
        );
        assert!(catalog.select("missing", Precision::Int8, engine).is_none());
        assert!(catalog
            .select("opus-en-zh", Precision::Int8, Version(0, 0, 9))
            .is_none());
    }

    #[test]
    fn rejects_duplicate_and_unsafe_names() {
        let a = entry(Version(1, 0, 0), Precision::Int8);
        assert_eq!(
            Catalog::new(vec![a.clone(), a.clone()]).unwrap_err(),
            Error::Duplicate
        );
        for name in [
            "../x", "a/b", "a\\b", "a:stream", "con.txt", "com1", "x.", "A.bin",
        ] {
            let mut bad = a.clone();
            let asset = bad.files.pop_first().unwrap().1;
            bad.files.insert(name.into(), asset);
            assert_eq!(bad.validate(), Err(Error::InvalidName));
        }
        let mut bad = a;
        bad.id = "../escape".into();
        assert_eq!(bad.validate(), Err(Error::InvalidName));
    }

    #[test]
    fn rejects_bad_hash_and_package_budget() {
        let mut a = entry(Version(1, 0, 0), Precision::Fp32);
        a.files.get_mut("weights.bin").unwrap().sha256 = "x".repeat(64);
        assert_eq!(a.validate(), Err(Error::InvalidAsset));
        let mut a = entry(Version(1, 0, 0), Precision::Fp32);
        let mut asset = a.files["weights.bin"].clone();
        asset.bytes = MAX_PACKAGE_BYTES;
        a.files.insert("extra.bin".into(), asset);
        assert_eq!(a.validate(), Err(Error::Limit));
    }

    #[test]
    fn verifies_exact_length_and_hash() {
        let a = entry(Version(1, 0, 0), Precision::Fp32);
        let asset = &a.files["weights.bin"];
        assert_eq!(asset.verify(b"abc".as_slice()), Ok(()));
        for data in [b"ab".as_slice(), b"abd".as_slice(), b"abcd".as_slice()] {
            assert_eq!(asset.verify(data), Err(Error::Integrity));
        }
    }

    #[test]
    fn verifies_directory_and_rejects_extra_missing_or_changed_files() {
        let root = std::env::temp_dir().join(format!(
            "rpmodel-catalog-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let a = entry(Version(1, 0, 0), Precision::Fp32);
        assert_eq!(a.verify_directory(&root), Err(Error::Integrity));
        fs::write(root.join("weights.bin"), b"abc").unwrap();
        assert_eq!(a.verify_directory(&root), Ok(()));
        fs::write(root.join("extra.bin"), b"abc").unwrap();
        assert_eq!(a.verify_directory(&root), Err(Error::Integrity));
        fs::remove_file(root.join("extra.bin")).unwrap();
        fs::write(root.join("weights.bin"), b"abd").unwrap();
        assert_eq!(a.verify_directory(&root), Err(Error::Integrity));
        fs::remove_dir_all(root).unwrap();
    }
}
