//! Persistent per-model/per-precision activation and explicit rollback.
//! Trusted catalogs remain the source of identity and integrity, not pointers.
//! Requires an exclusively controlled store, no concurrent writers or readers
//! of mutable model files. No power-loss durability guarantee.
use crate::{
    catalog::{self, Catalog, Entry, Precision, Version},
    install::{self, Store},
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const MAX_RECORD: u64 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub current: Version,
    pub previous: Option<Version>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Store(install::Error),
    Record,
    Io,
    Pending,
    MissingVersion,
    NoRollback,
    Cleanup,
}

fn version_text(Version(a, b, c): Version) -> String {
    format!("{a}.{b}.{c}")
}

fn parse_version(input: &str) -> Result<Version, Error> {
    let mut parts = input.split('.');
    let mut next = || {
        parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .ok_or(Error::Record)
    };
    let version = Version(next()?, next()?, next()?);
    if parts.next().is_some() || version_text(version) != input {
        return Err(Error::Record);
    }
    Ok(version)
}

fn encode(selection: Selection) -> String {
    let previous = selection.previous.map_or_else(|| "-".into(), version_text);
    format!(
        "RPActive1\n{}\n{previous}\n",
        version_text(selection.current)
    )
}

fn decode(input: &str) -> Result<Selection, Error> {
    let mut lines = input.split('\n');
    if lines.next() != Some("RPActive1") {
        return Err(Error::Record);
    }
    let current = parse_version(lines.next().ok_or(Error::Record)?)?;
    let previous = match lines.next() {
        Some("-") => None,
        Some(value) => Some(parse_version(value)?),
        None => return Err(Error::Record),
    };
    let selection = Selection { current, previous };
    if previous == Some(current) || encode(selection) != input {
        return Err(Error::Record);
    }
    Ok(selection)
}

fn present(path: &Path) -> Result<bool, Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Error::Io),
    }
}

fn record_path(store: &Store, family: &Entry) -> Result<PathBuf, Error> {
    store.installed_path(family).map_err(Error::Store)?;
    let precision = match family.precision {
        Precision::Fp32 => "fp32",
        Precision::Int8 => "int8",
    };
    Ok(store
        .root()
        .join(format!(".active-{}-{precision}", family.id)))
}

fn read_record(path: &Path) -> Result<Option<Selection>, Error> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Io),
    };
    if !meta.is_file() || catalog::is_link(&meta) || meta.len() > MAX_RECORD {
        return Err(Error::Record);
    }
    let mut text = String::new();
    File::open(path)
        .map_err(|_| Error::Io)?
        .take(MAX_RECORD + 1)
        .read_to_string(&mut text)
        .map_err(|_| Error::Record)?;
    if text.len() as u64 > MAX_RECORD {
        return Err(Error::Record);
    }
    decode(&text).map(Some)
}

fn write_record(path: &Path, selection: Selection) -> Result<(), Error> {
    let stage = path.with_extension("pending");
    if present(&stage)? {
        return Err(Error::Pending);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .map_err(|_| Error::Io)?;
    let result = file
        .write_all(encode(selection).as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| fs::rename(&stage, path));
    if result.is_err() {
        if fs::remove_file(&stage).is_err() {
            return Err(Error::Cleanup);
        }
        return Err(Error::Io);
    }
    Ok(())
}

fn resolve<'a>(catalog: &'a Catalog, family: &Entry, version: Version) -> Result<&'a Entry, Error> {
    catalog
        .entries()
        .iter()
        .find(|entry| {
            entry.id == family.id && entry.precision == family.precision && entry.version == version
        })
        .ok_or(Error::MissingVersion)
}

/// Read the strict record; does not imply the selected model is healthy.
pub fn selection(store: &Store, family: &Entry) -> Result<Option<Selection>, Error> {
    read_record(&record_path(store, family)?)
}

/// Resolve the current version against a trusted catalog and recheck bytes.
/// Never silently substitute another version or precision.
pub fn active<'a>(
    store: &Store,
    catalog: &'a Catalog,
    family: &Entry,
) -> Result<Option<&'a Entry>, Error> {
    let Some(selected) = selection(store, family)? else {
        return Ok(None);
    };
    let entry = resolve(catalog, family, selected.current)?;
    store.verify_installed(entry).map_err(Error::Store)?;
    Ok(Some(entry))
}

fn verify(
    store: &Store,
    entry: &Entry,
    validate: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), Error> {
    store.verify_installed(entry).map_err(Error::Store)?;
    let path = store.installed_path(entry).map_err(Error::Store)?;
    validate(&path).map_err(|e| Error::Store(install::Error::Validation(e)))
}

/// Activate an already installed, explicitly selected trusted entry.
/// No modification of the old model; validate before committing the pointer.
pub fn activate(
    store: &Store,
    entry: &Entry,
    validate: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<Selection, Error> {
    let path = record_path(store, entry)?;
    let old = read_record(&path)?;
    verify(store, entry, validate)?;
    if let Some(old) = old.filter(|s| s.current == entry.version) {
        return Ok(old);
    }
    let selected = Selection {
        current: entry.version,
        previous: old.map(|s| s.current),
    };
    write_record(&path, selected)?;
    Ok(selected)
}

/// Roll back only to the recorded previous version, using its trusted pin.
/// On any validation error the original selection remains unchanged.
pub fn rollback(
    store: &Store,
    catalog: &Catalog,
    family: &Entry,
    validate: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<Selection, Error> {
    let path = record_path(store, family)?;
    let old = read_record(&path)?.ok_or(Error::NoRollback)?;
    let previous = old.previous.ok_or(Error::NoRollback)?;
    let entry = resolve(catalog, family, previous)?;
    verify(store, entry, validate)?;
    let selected = Selection {
        current: previous,
        previous: Some(old.current),
    };
    write_record(&path, selected)?;
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Asset;
    use std::collections::BTreeMap;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rpmodel-active-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("store")).unwrap();
            fs::create_dir(root.join("source")).unwrap();
            fs::write(root.join("source/data.bin"), b"abc").unwrap();
            Self(root)
        }

        fn store(&self) -> Store {
            Store::open(&self.0.join("store"), Version(1, 0, 0)).unwrap()
        }

        fn install(&self, store: &Store, version: u32, precision: Precision) -> Entry {
            let entry = Entry {
                id: "fixture".into(),
                version: Version(version, 0, 0),
                precision,
                min_engine: Version(1, 0, 0),
                files: BTreeMap::from([(
                    "data.bin".into(),
                    Asset {
                        bytes: 3,
                        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                            .into(),
                    },
                )]),
            };
            store.import(&entry, &self.0.join("source")).unwrap();
            entry
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn activation_upgrade_reopen_and_rollback() {
        let root = Scratch::new();
        let store = root.store();
        let a = root.install(&store, 1, Precision::Int8);
        let b = root.install(&store, 2, Precision::Int8);
        let catalog = Catalog::new(vec![a.clone(), b.clone()]).unwrap();
        assert_eq!(selection(&store, &a), Ok(None));
        activate(&store, &a, |_| Ok(())).unwrap();
        let switched = activate(&store, &b, |_| Ok(())).unwrap();
        assert_eq!(switched.current, b.version);
        assert_eq!(switched.previous, Some(a.version));
        let store = root.store();
        assert_eq!(active(&store, &catalog, &a).unwrap(), Some(&b));
        assert_eq!(activate(&store, &b, |_| Ok(())).unwrap(), switched);
        let rolled = rollback(&store, &catalog, &a, |_| Ok(())).unwrap();
        assert_eq!(rolled.current, a.version);
        assert_eq!(active(&store, &catalog, &a).unwrap(), Some(&a));
        assert_eq!(store.verify_installed(&b), Ok(()));
    }

    #[test]
    fn validation_failure_keeps_old_selection() {
        let root = Scratch::new();
        let store = root.store();
        let a = root.install(&store, 1, Precision::Int8);
        let b = root.install(&store, 2, Precision::Int8);
        let original = activate(&store, &a, |_| Ok(())).unwrap();
        assert_eq!(
            activate(&store, &b, |_| Err("profile".into())),
            Err(Error::Store(install::Error::Validation("profile".into())))
        );
        assert_eq!(selection(&store, &a), Ok(Some(original)));
        fs::write(store.installed_path(&b).unwrap().join("data.bin"), b"abd").unwrap();
        assert!(activate(&store, &b, |_| panic!("bad bytes")).is_err());
        assert_eq!(selection(&store, &a), Ok(Some(original)));
    }

    #[test]
    fn rollback_rejects_missing_catalog_and_corrupt_previous() {
        let root = Scratch::new();
        let store = root.store();
        let a = root.install(&store, 1, Precision::Int8);
        let b = root.install(&store, 2, Precision::Int8);
        activate(&store, &a, |_| Ok(())).unwrap();
        let original = activate(&store, &b, |_| Ok(())).unwrap();
        let limited = Catalog::new(vec![b.clone()]).unwrap();
        assert_eq!(
            rollback(&store, &limited, &a, |_| Ok(())),
            Err(Error::MissingVersion)
        );
        let catalog = Catalog::new(vec![a.clone(), b]).unwrap();
        fs::write(store.installed_path(&a).unwrap().join("data.bin"), b"abd").unwrap();
        assert!(rollback(&store, &catalog, &a, |_| Ok(())).is_err());
        assert_eq!(selection(&store, &a), Ok(Some(original)));
    }

    #[test]
    fn precision_isolation_and_no_implicit_rollback() {
        let root = Scratch::new();
        let store = root.store();
        let a = root.install(&store, 1, Precision::Fp32);
        let b = root.install(&store, 2, Precision::Int8);
        let catalog = Catalog::new(vec![a.clone(), b.clone()]).unwrap();
        activate(&store, &a, |_| Ok(())).unwrap();
        assert_eq!(active(&store, &catalog, &b), Ok(None));
        assert_eq!(
            rollback(&store, &catalog, &a, |_| Ok(())),
            Err(Error::NoRollback)
        );
        activate(&store, &b, |_| Ok(())).unwrap();
        assert_eq!(active(&store, &catalog, &a).unwrap(), Some(&a));
        assert_eq!(active(&store, &catalog, &b).unwrap(), Some(&b));
    }

    #[test]
    fn rejects_noncanonical_and_oversized_records() {
        for text in [
            "RPActive1\n01.0.0\n-\n",
            "RPActive1\n1.0.0\n1.0.0\n",
            "RPActive1\n1.0.0\n-\nextra",
            "RPActive1\n1.0.0\n../escape\n",
        ] {
            assert_eq!(decode(text), Err(Error::Record));
        }
        let root = Scratch::new();
        let store = root.store();
        let entry = root.install(&store, 1, Precision::Int8);
        let path = record_path(&store, &entry).unwrap();
        fs::write(path, vec![b'x'; 129]).unwrap();
        assert_eq!(selection(&store, &entry), Err(Error::Record));
        assert_eq!(activate(&store, &entry, |_| Ok(())), Err(Error::Record));
    }

    #[test]
    fn pending_record_is_never_overwritten() {
        let root = Scratch::new();
        let store = root.store();
        let a = root.install(&store, 1, Precision::Int8);
        let b = root.install(&store, 2, Precision::Int8);
        let old = activate(&store, &a, |_| Ok(())).unwrap();
        let pending = record_path(&store, &a).unwrap().with_extension("pending");
        fs::write(&pending, b"interrupted").unwrap();
        assert_eq!(activate(&store, &b, |_| Ok(())), Err(Error::Pending));
        assert_eq!(fs::read(pending).unwrap(), b"interrupted");
        assert_eq!(selection(&store, &a), Ok(Some(old)));
    }
}
