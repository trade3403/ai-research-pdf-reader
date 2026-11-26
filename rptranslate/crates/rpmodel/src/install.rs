//! Verified flat-package installation into an exclusively owned model store.
//! This layer validates catalog identity and bytes, not inference architecture.
//! No archive extraction, activation, overwrite, or concurrent writers.
use crate::catalog::{self, Entry, Precision, Version};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Catalog(catalog::Error),
    IncompatibleEngine,
    UnsafeStore,
    AlreadyInstalled,
    PendingStage,
    Validation(String),
    Io,
    Cleanup,
}

/// The caller must own the store and source directories, including their
/// ancestors, and exclude other writers for the entire operation. This is
/// not a sandbox against hostile concurrent changes or a power-loss journal.
#[derive(Debug)]
pub struct Store {
    root: PathBuf,
    engine: Version,
}

fn present(path: &Path) -> Result<bool, Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Error::Io),
    }
}

impl Store {
    /// Open an existing directory; never create or follow a store symlink.
    pub fn open(root: &Path, engine: Version) -> Result<Self, Error> {
        let metadata = fs::symlink_metadata(root).map_err(|_| Error::Io)?;
        if !metadata.is_dir() || catalog::is_link(&metadata) {
            return Err(Error::UnsafeStore);
        }
        Ok(Self {
            root: root.into(),
            engine,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    fn key(&self, entry: &Entry) -> Result<String, Error> {
        entry.validate().map_err(Error::Catalog)?;
        if entry.min_engine > self.engine {
            return Err(Error::IncompatibleEngine);
        }
        let Version(major, minor, patch) = entry.version;
        let precision = match entry.precision {
            Precision::Fp32 => "fp32",
            Precision::Int8 => "int8",
        };
        Ok(format!(
            "model-{}-{major}.{minor}.{patch}-{precision}",
            entry.id
        ))
    }

    pub fn installed_path(&self, entry: &Entry) -> Result<PathBuf, Error> {
        Ok(self.root.join(self.key(entry)?))
    }

    /// Verify an existing installation against the same trusted catalog.
    /// A directory name or a successful copy alone is not a trust signal.
    pub fn verify_installed(&self, entry: &Entry) -> Result<(), Error> {
        entry
            .verify_directory(&self.installed_path(entry)?)
            .map_err(Error::Catalog)
    }

    /// Import an already materialized offline or downloaded flat package.
    /// Exact file set and SHA256 must match a separately trusted catalog.
    /// Stage is a sibling of the destination, so publication uses rename
    /// on one filesystem. A pre-existing stage is never deleted or reused.
    pub fn import(&self, entry: &Entry, source: &Path) -> Result<PathBuf, Error> {
        self.import_checked(entry, source, |_| Ok(()))
    }

    /// Validate the staged model with an architecture-specific loader before
    /// publication. The validator must be read-only and close all handles.
    pub fn import_checked(
        &self,
        entry: &Entry,
        source: &Path,
        validate: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<PathBuf, Error> {
        let key = self.key(entry)?;
        let target = self.root.join(&key);
        let stage = self.root.join(format!(".stage-{key}"));
        if present(&target)? {
            return Err(Error::AlreadyInstalled);
        }
        if present(&stage)? {
            return Err(Error::PendingStage);
        }
        entry.verify_directory(source).map_err(Error::Catalog)?;
        fs::create_dir(&stage).map_err(|_| Error::Io)?;
        let result = self.copy_and_commit(entry, source, &stage, &target, validate);
        if result.is_err() && fs::remove_dir_all(&stage).is_err() {
            return Err(Error::Cleanup);
        }
        result.map(|()| target)
    }

    fn copy_and_commit(
        &self,
        entry: &Entry,
        source: &Path,
        stage: &Path,
        target: &Path,
        validate: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), Error> {
        for (name, asset) in &entry.files {
            let source_path = source.join(name);
            let metadata = fs::symlink_metadata(&source_path).map_err(|_| Error::Io)?;
            if !metadata.is_file() || catalog::is_link(&metadata) || metadata.len() != asset.bytes {
                return Err(Error::Catalog(catalog::Error::Integrity));
            }
            let mut input = File::open(source_path)
                .map_err(|_| Error::Io)?
                .take(asset.bytes + 1);
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(stage.join(name))
                .map_err(|_| Error::Io)?;
            if io::copy(&mut input, &mut output).map_err(|_| Error::Io)? != asset.bytes {
                return Err(Error::Catalog(catalog::Error::Integrity));
            }
            output.sync_all().map_err(|_| Error::Io)?;
        }
        self.publish_checked(entry, stage, target, validate)
    }

    /// Recover only a complete verified stage. Failure leaves it untouched.
    pub fn recover_checked(
        &self,
        entry: &Entry,
        validate: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<PathBuf, Error> {
        let key = self.key(entry)?;
        let target = self.root.join(&key);
        let stage = self.root.join(format!(".stage-{key}"));
        if present(&target)? {
            return Err(Error::AlreadyInstalled);
        }
        self.publish_checked(entry, &stage, &target, validate)?;
        Ok(target)
    }

    /// Explicitly isolate an abandoned stage without deleting its contents.
    /// No overwrite, and do not follow a symlink or reparse point.
    pub fn quarantine_stage(&self, entry: &Entry) -> Result<PathBuf, Error> {
        let key = self.key(entry)?;
        let stage = self.root.join(format!(".stage-{key}"));
        let dest = self.root.join(format!(".quarantine-{key}"));
        let meta = fs::symlink_metadata(&stage).map_err(|_| Error::Io)?;
        if !meta.is_dir() || catalog::is_link(&meta) {
            return Err(Error::UnsafeStore);
        }
        if present(&dest)? {
            return Err(Error::AlreadyInstalled);
        }
        fs::rename(stage, &dest).map_err(|_| Error::Io)?;
        Ok(dest)
    }

    fn publish_checked(
        &self,
        entry: &Entry,
        stage: &Path,
        target: &Path,
        validate: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), Error> {
        entry.verify_directory(stage).map_err(Error::Catalog)?;
        validate(stage).map_err(Error::Validation)?;
        if present(target)? {
            return Err(Error::AlreadyInstalled);
        }
        // No hostile concurrent writers: std rename may replace a target
        // on some platforms. The store's exclusive ownership is mandatory.
        fs::rename(stage, target).map_err(|_| Error::Io)
    }
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
                "rpmodel-install-{}-{}",
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

        fn source(&self) -> PathBuf {
            self.0.join("source")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(precision: Precision) -> Entry {
        Entry {
            id: "fixture".into(),
            version: Version(1, 2, 3),
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
        }
    }

    #[test]
    fn imports_both_precisions_to_distinct_verified_paths() {
        let root = Scratch::new();
        let store = root.store();
        for precision in [Precision::Fp32, Precision::Int8] {
            let entry = entry(precision);
            let path = store.import(&entry, &root.source()).unwrap();
            assert_eq!(path, store.installed_path(&entry).unwrap());
            assert_eq!(store.verify_installed(&entry), Ok(()));
            assert_eq!(fs::read(path.join("data.bin")).unwrap(), b"abc");
        }
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 2);
        assert_eq!(fs::read(root.source().join("data.bin")).unwrap(), b"abc");
    }

    #[test]
    fn rejects_corrupt_missing_and_extra_source_files() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let file = root.source().join("data.bin");
        fs::write(&file, b"abd").unwrap();
        assert!(store.import(&entry, &root.source()).is_err());
        fs::remove_file(&file).unwrap();
        assert!(store.import(&entry, &root.source()).is_err());
        fs::write(&file, b"abc").unwrap();
        fs::write(root.source().join("extra.bin"), b"x").unwrap();
        assert!(store.import(&entry, &root.source()).is_err());
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 0);
    }

    #[test]
    fn never_overwrites_existing_install_even_if_corrupted() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Fp32);
        let path = store.import(&entry, &root.source()).unwrap();
        fs::write(path.join("data.bin"), b"abd").unwrap();
        assert_eq!(
            store.import(&entry, &root.source()),
            Err(Error::AlreadyInstalled)
        );
        assert!(store.verify_installed(&entry).is_err());
        assert_eq!(fs::read(path.join("data.bin")).unwrap(), b"abd");
    }

    #[test]
    fn existing_stage_is_preserved_for_explicit_recovery() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let stage = store.root.join(".stage-model-fixture-1.2.3-int8");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("data.bin"), b"a").unwrap();
        assert_eq!(
            store.import(&entry, &root.source()),
            Err(Error::PendingStage)
        );
        assert_eq!(fs::read(stage.join("data.bin")).unwrap(), b"a");
        assert!(!store.installed_path(&entry).unwrap().exists());
    }

    #[test]
    fn rejects_incompatible_engine_and_unsafe_identity() {
        let root = Scratch::new();
        let store = root.store();
        let mut entry = entry(Precision::Int8);
        entry.min_engine = Version(2, 0, 0);
        assert_eq!(
            store.import(&entry, &root.source()),
            Err(Error::IncompatibleEngine)
        );
        entry.id = "../escape".into();
        assert_eq!(
            store.installed_path(&entry),
            Err(Error::Catalog(catalog::Error::InvalidName))
        );
    }

    #[test]
    fn copy_failure_does_not_publish_target() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let stage = store.root.join(".manual-stage");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("data.bin"), b"existing").unwrap();
        let target = store.installed_path(&entry).unwrap();
        assert_eq!(
            store.copy_and_commit(&entry, &root.source(), &stage, &target, |_| Ok(())),
            Err(Error::Io)
        );
        assert!(!target.exists());
        assert_eq!(fs::read(stage.join("data.bin")).unwrap(), b"existing");
    }

    #[test]
    fn rejects_non_directory_store() {
        let root = Scratch::new();
        assert_eq!(
            Store::open(&root.source().join("data.bin"), Version(1, 0, 0)).unwrap_err(),
            Error::UnsafeStore
        );
    }

    #[test]
    fn validator_failure_never_publishes_and_cleans_new_stage() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let result = store.import_checked(&entry, &root.source(), |_| Err("profile".into()));
        assert_eq!(result, Err(Error::Validation("profile".into())));
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 0);
    }

    #[test]
    fn recovers_only_complete_verified_stage() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let stage = store.root.join(".stage-model-fixture-1.2.3-int8");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("data.bin"), b"a").unwrap();
        assert!(store
            .recover_checked(&entry, |_| panic!("bad bytes"))
            .is_err());
        assert_eq!(fs::read(stage.join("data.bin")).unwrap(), b"a");
        fs::write(stage.join("data.bin"), b"abc").unwrap();
        assert_eq!(
            store.recover_checked(&entry, |_| Err("profile".into())),
            Err(Error::Validation("profile".into()))
        );
        assert!(stage.exists());
        let path = store.recover_checked(&entry, |_| Ok(())).unwrap();
        assert!(!stage.exists());
        assert_eq!(path, store.installed_path(&entry).unwrap());
        assert_eq!(store.verify_installed(&entry), Ok(()));
        assert_eq!(
            store.recover_checked(&entry, |_| Ok(())),
            Err(Error::AlreadyInstalled)
        );
    }

    #[test]
    fn quarantine_preserves_stage_and_never_overwrites() {
        let root = Scratch::new();
        let store = root.store();
        let entry = entry(Precision::Int8);
        let stage = store.root.join(".stage-model-fixture-1.2.3-int8");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("data.bin"), b"a").unwrap();
        let dest = store.quarantine_stage(&entry).unwrap();
        assert!(!stage.exists());
        assert_eq!(fs::read(dest.join("data.bin")).unwrap(), b"a");
        fs::create_dir(&stage).unwrap();
        assert_eq!(store.quarantine_stage(&entry), Err(Error::AlreadyInstalled));
        assert!(stage.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_store_and_source_asset() {
        use std::os::unix::fs::symlink;
        let root = Scratch::new();
        let linked = root.0.join("linked");
        symlink(root.0.join("store"), &linked).unwrap();
        assert_eq!(
            Store::open(&linked, Version(1, 0, 0)).unwrap_err(),
            Error::UnsafeStore
        );
        let file = root.source().join("data.bin");
        fs::remove_file(&file).unwrap();
        fs::write(root.0.join("outside"), b"abc").unwrap();
        symlink(root.0.join("outside"), file).unwrap();
        let store = root.store();
        assert!(store
            .import(&entry(Precision::Int8), &root.source())
            .is_err());
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 0);
    }
}
