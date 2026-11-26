//! Persistent, bounded range assembly. A transport supplies the response body.
//! Partial bytes are not authenticated until the entire asset digest matches.
use crate::catalog::{self, Asset};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub const MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: u64,
    pub end: u64,
    pub total: u64,
}

impl Range {
    pub fn header(&self) -> String {
        format!("bytes={}-{}", self.start, self.end)
    }

    fn content_range(&self) -> String {
        format!("bytes {}-{}/{}", self.start, self.end, self.total)
    }

    fn count(&self) -> Option<u64> {
        self.end.checked_sub(self.start)?.checked_add(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub received: u64,
    pub total: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
    UnsafeFile,
    Range,
    Response,
    Incomplete,
    Rollback,
    Catalog(catalog::Error),
}

/// Exclusive access to a caller-controlled staging file and its ancestors
/// is required. Never pass an installed model as staging. The trusted asset
/// pin must remain the same across restarts; existing bytes are untrusted.
#[derive(Debug)]
pub struct PartialDownload {
    file: File,
    asset: Asset,
    received: u64,
    usable: bool,
}

impl PartialDownload {
    pub fn create(path: &Path, asset: Asset) -> Result<Self, Error> {
        if !asset.valid() {
            return Err(Error::Catalog(catalog::Error::InvalidAsset));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| Error::Io)?;
        Ok(Self {
            file,
            asset,
            received: 0,
            usable: true,
        })
    }

    pub fn resume(path: &Path, asset: Asset) -> Result<Self, Error> {
        if !asset.valid() {
            return Err(Error::Catalog(catalog::Error::InvalidAsset));
        }
        let metadata = fs::symlink_metadata(path).map_err(|_| Error::Io)?;
        if !metadata.is_file() || catalog::is_link(&metadata) || metadata.len() > asset.bytes {
            return Err(Error::UnsafeFile);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| Error::Io)?;
        Ok(Self {
            file,
            asset,
            received: metadata.len(),
            usable: true,
        })
    }

    pub fn progress(&self) -> Progress {
        Progress {
            received: self.received,
            total: self.asset.bytes,
        }
    }

    /// Pause between chunks by not requesting the next range. Reopen the
    /// staging file with the same trusted pin to resume in a later process.
    pub fn next_range(&self, max_bytes: u64) -> Result<Option<Range>, Error> {
        if !self.usable {
            return Err(Error::Rollback);
        }
        if max_bytes == 0 || max_bytes > MAX_CHUNK_BYTES {
            return Err(Error::Range);
        }
        if self.received == self.asset.bytes {
            return Ok(None);
        }
        let count = max_bytes.min(self.asset.bytes - self.received);
        Ok(Some(Range {
            start: self.received,
            end: self.received + count - 1,
            total: self.asset.bytes,
        }))
    }

    /// Only HTTP 206 with the exact canonical Content-Range and exact
    /// response length can advance the staging file. No implicit HTTP 200
    /// fallback, even for the first chunk. A failed response writes nothing.
    pub fn append(
        &mut self,
        range: Range,
        status: u16,
        content_range: &str,
        body: impl Read,
    ) -> Result<Progress, Error> {
        let count = range.count().ok_or(Error::Range)?;
        if self.next_range(count)? != Some(range) {
            return Err(Error::Range);
        }
        if status != 206 || content_range != range.content_range() {
            return Err(Error::Response);
        }
        let mut bytes = Vec::with_capacity(count as usize);
        body.take(count + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Io)?;
        if bytes.len() as u64 != count {
            return Err(Error::Response);
        }
        self.file
            .seek(SeekFrom::Start(self.received))
            .map_err(|_| Error::Io)?;
        if self
            .file
            .write_all(&bytes)
            .and_then(|()| self.file.sync_all())
            .is_err()
        {
            if self
                .file
                .set_len(self.received)
                .and_then(|()| self.file.sync_all())
                .is_err()
            {
                self.usable = false;
                return Err(Error::Rollback);
            }
            return Err(Error::Io);
        }
        self.received += count;
        Ok(self.progress())
    }

    /// Completion by byte count alone is not authenticity. Always call
    /// this before handing the file to the future installer.
    pub fn verify_complete(&mut self) -> Result<(), Error> {
        if !self.usable {
            return Err(Error::Rollback);
        }
        if self.received != self.asset.bytes {
            return Err(Error::Incomplete);
        }
        self.file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
        self.asset.verify(&mut self.file).map_err(Error::Catalog)
    }

    /// Explicitly discard a corrupt partial asset. Never silently restart
    /// after a server ignores Range or after a digest mismatch.
    pub fn restart(&mut self) -> Result<(), Error> {
        if self
            .file
            .set_len(0)
            .and_then(|()| self.file.sync_all())
            .is_err()
        {
            self.usable = false;
            return Err(Error::Rollback);
        }
        self.received = 0;
        self.usable = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rpmodel-resume-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn file(&self) -> PathBuf {
            self.0.join("asset.part")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pin() -> Asset {
        Asset {
            bytes: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }
    }

    #[test]
    fn resumes_after_close_and_verifies_whole_asset() {
        let root = Scratch::new();
        let path = root.file();
        let mut download = PartialDownload::create(&path, pin()).unwrap();
        let first = download.next_range(2).unwrap().unwrap();
        assert_eq!(first.header(), "bytes=0-1");
        download
            .append(first, 206, "bytes 0-1/3", b"ab".as_slice())
            .unwrap();
        assert_eq!(download.verify_complete(), Err(Error::Incomplete));
        drop(download);
        let mut download = PartialDownload::resume(&path, pin()).unwrap();
        assert_eq!(download.progress().received, 2);
        let last = download.next_range(2).unwrap().unwrap();
        assert_eq!(last.header(), "bytes=2-2");
        download
            .append(last, 206, "bytes 2-2/3", b"c".as_slice())
            .unwrap();
        assert_eq!(download.next_range(2), Ok(None));
        assert_eq!(download.verify_complete(), Ok(()));
        assert_eq!(fs::read(path).unwrap(), b"abc");
    }

    #[test]
    fn rejects_wrong_status_and_content_range_without_writes() {
        let root = Scratch::new();
        let path = root.file();
        let mut download = PartialDownload::create(&path, pin()).unwrap();
        let range = download.next_range(2).unwrap().unwrap();
        for (status, header) in [
            (200, "bytes 0-1/3"),
            (206, "bytes 0-1/4"),
            (206, "bytes 1-2/3"),
        ] {
            assert_eq!(
                download.append(range, status, header, b"ab".as_slice()),
                Err(Error::Response)
            );
        }
        assert_eq!(download.progress().received, 0);
        assert_eq!(fs::metadata(path).unwrap().len(), 0);
    }

    #[test]
    fn rejects_short_long_and_stale_chunks() {
        let root = Scratch::new();
        let mut download = PartialDownload::create(&root.file(), pin()).unwrap();
        let range = download.next_range(2).unwrap().unwrap();
        for data in [b"a".as_slice(), b"abc".as_slice()] {
            assert_eq!(
                download.append(range, 206, "bytes 0-1/3", data),
                Err(Error::Response)
            );
        }
        download
            .append(range, 206, "bytes 0-1/3", b"ab".as_slice())
            .unwrap();
        assert_eq!(
            download.append(range, 206, "bytes 0-1/3", b"ab".as_slice()),
            Err(Error::Range)
        );
        assert_eq!(download.progress().received, 2);
    }

    #[test]
    fn corrupted_complete_asset_requires_explicit_restart() {
        let root = Scratch::new();
        let path = root.file();
        fs::write(&path, b"abd").unwrap();
        let mut download = PartialDownload::resume(&path, pin()).unwrap();
        assert_eq!(
            download.verify_complete(),
            Err(Error::Catalog(catalog::Error::Integrity))
        );
        download.restart().unwrap();
        assert_eq!(download.progress().received, 0);
        let range = download.next_range(3).unwrap().unwrap();
        download
            .append(range, 206, "bytes 0-2/3", b"abc".as_slice())
            .unwrap();
        assert_eq!(download.verify_complete(), Ok(()));
    }

    #[test]
    fn rejects_existing_target_oversized_partial_and_invalid_budget() {
        let root = Scratch::new();
        let path = root.file();
        fs::write(&path, b"abcd").unwrap();
        assert_eq!(
            PartialDownload::create(&path, pin()).unwrap_err(),
            Error::Io
        );
        assert_eq!(
            PartialDownload::resume(&path, pin()).unwrap_err(),
            Error::UnsafeFile
        );
        assert_eq!(fs::read(&path).unwrap(), b"abcd");
        fs::remove_file(&path).unwrap();
        let download = PartialDownload::create(&path, pin()).unwrap();
        assert_eq!(download.next_range(0), Err(Error::Range));
        assert_eq!(download.next_range(MAX_CHUNK_BYTES + 1), Err(Error::Range));
    }

    #[test]
    fn interrupted_body_preserves_existing_prefix() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("connection lost"))
            }
        }
        let root = Scratch::new();
        let path = root.file();
        fs::write(&path, b"a").unwrap();
        let mut download = PartialDownload::resume(&path, pin()).unwrap();
        let range = download.next_range(2).unwrap().unwrap();
        assert_eq!(
            download.append(range, 206, "bytes 1-2/3", Broken),
            Err(Error::Io)
        );
        assert_eq!(fs::read(path).unwrap(), b"a");
        assert_eq!(download.progress().received, 1);
    }
}
