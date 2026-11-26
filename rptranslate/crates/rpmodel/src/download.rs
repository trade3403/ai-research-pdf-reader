//! Bounded HTTPS download adapter. TLS is handled by a trusted curl binary.
//! The Rust core owns the catalog, byte limits, integrity and staging policy.
use crate::{
    catalog::{self, Asset, Catalog, Entry},
    online_catalog::{self, Origin},
};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MAX_CATALOG_BYTES: u64 = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Transport,
    HttpStatus,
    Framing,
    Io,
    Cleanup,
    Catalog(catalog::Error),
    Online(online_catalog::Error),
}

struct Tee<'a, R, W> {
    reader: R,
    writer: &'a mut W,
}

impl<R: Read, W: Write> Read for Tee<'_, R, W> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(buffer)?;
        self.writer.write_all(&buffer[..count])?;
        Ok(count)
    }
}

/// curl writes the three-digit final HTTP status after the response body.
/// Accept only the pinned body followed by exactly "200" and EOF.
fn receive(mut input: impl Read, output: &mut impl Write, asset: &Asset) -> Result<(), Error> {
    asset
        .verify(Tee {
            reader: (&mut input).take(asset.bytes),
            writer: output,
        })
        .map_err(Error::Catalog)?;
    let mut status = [0; 3];
    input.read_exact(&mut status).map_err(|_| Error::Framing)?;
    if status != *b"200" {
        return Err(Error::HttpStatus);
    }
    let mut extra = [0; 1];
    if input.read(&mut extra).map_err(|_| Error::Io)? != 0 {
        return Err(Error::Framing);
    }
    Ok(())
}

#[derive(Debug)]
pub struct Downloader {
    curl: PathBuf,
    origin: Origin,
}

impl Downloader {
    /// The executable and process environment must be trusted. No PATH lookup.
    /// Direct HTTPS only; no redirects, proxy, user curl config or custom TLS.
    pub fn new(curl: &Path, origin: Origin) -> Result<Self, Error> {
        if !curl.is_absolute() || !curl.is_file() {
            return Err(Error::Configuration);
        }
        Ok(Self {
            curl: curl.into(),
            origin,
        })
    }

    fn fetch(&self, url: &str, asset: &Asset, output: &mut impl Write) -> Result<(), Error> {
        if !asset.valid() {
            return Err(Error::Catalog(catalog::Error::InvalidAsset));
        }
        let mut child = Command::new(&self.curl)
            .args([
                "--disable",
                "--globoff",
                "--silent",
                "--fail",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--noproxy",
                "*",
                "--connect-timeout",
                "15",
                "--max-time",
                "600",
                "--max-filesize",
                &asset.bytes.to_string(),
                "--header",
                "Accept-Encoding: identity",
                "--output",
                "-",
                "--write-out",
                "%{http_code}",
                "--url",
                url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::Transport)?;
        let result = match child.stdout.take() {
            Some(stdout) => receive(stdout, output, asset),
            None => Err(Error::Transport),
        };
        if result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait().map_err(|_| Error::Transport)?;
        result?;
        if !status.success() {
            return Err(Error::Transport);
        }
        Ok(())
    }

    pub fn fetch_catalog(&self, trusted: &Asset) -> Result<Catalog, Error> {
        if trusted.bytes > MAX_CATALOG_BYTES {
            return Err(Error::Online(online_catalog::Error::Limit));
        }
        let mut data = Vec::new();
        self.fetch(&self.origin.catalog_url(), trusted, &mut data)?;
        online_catalog::parse_pinned(&data, trusted).map_err(Error::Online)
    }

    /// Download a catalog asset to a new staging file, not an installed model.
    /// Caller must own the staging directory and exclude concurrent mutation.
    /// On normal failure the newly created file is removed; never overwrite.
    pub fn fetch_asset(&self, entry: &Entry, name: &str, staging: &Path) -> Result<(), Error> {
        let url = self.origin.asset_url(entry, name).map_err(Error::Online)?;
        let asset = entry.files.get(name).ok_or(Error::Configuration)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(staging)
            .map_err(|_| Error::Io)?;
        let result = self
            .fetch(&url, asset, &mut file)
            .and_then(|()| file.sync_all().map_err(|_| Error::Io));
        drop(file);
        if result.is_err() && fs::remove_file(staging).is_err() {
            return Err(Error::Cleanup);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin() -> Asset {
        Asset {
            bytes: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }
    }

    #[test]
    fn exact_body_and_http_success() {
        let mut output = Vec::new();
        assert_eq!(receive(b"abc200".as_slice(), &mut output, &pin()), Ok(()));
        assert_eq!(output, b"abc");
    }

    #[test]
    fn rejects_http_errors_and_redirects() {
        for status in [b"abc302", b"abc206", b"abc404", b"abc500"] {
            assert_eq!(
                receive(status.as_slice(), &mut Vec::new(), &pin()),
                Err(Error::HttpStatus)
            );
        }
    }

    #[test]
    fn rejects_corruption_truncation_and_extra_bytes() {
        assert_eq!(
            receive(b"abd200".as_slice(), &mut Vec::new(), &pin()),
            Err(Error::Catalog(catalog::Error::Integrity))
        );
        assert_eq!(
            receive(b"ab".as_slice(), &mut Vec::new(), &pin()),
            Err(Error::Catalog(catalog::Error::Integrity))
        );
        assert_eq!(
            receive(b"abc20".as_slice(), &mut Vec::new(), &pin()),
            Err(Error::Framing)
        );
        let mut output = Vec::new();
        assert_eq!(
            receive(b"abc200x".as_slice(), &mut output, &pin()),
            Err(Error::Framing)
        );
        assert_eq!(output, b"abc");
    }

    #[test]
    fn rejects_relative_executable() {
        assert!(matches!(
            Downloader::new(Path::new("curl"), Origin::new("example.org").unwrap()),
            Err(Error::Configuration)
        ));
    }

    #[test]
    #[ignore = "explicit online CI probe only"]
    fn actual_https_pinned_fixture() {
        let binary = std::env::var_os("RP_CURL").expect("absolute trusted curl path");
        let downloader = Downloader::new(
            Path::new(&binary),
            Origin::new("raw.githubusercontent.com").unwrap(),
        )
        .unwrap();
        let asset = Asset {
            bytes: 77,
            sha256: "5cdbd96cac32e26a8900d732a4a8be550b1d1a10762e35fecdcd75ec41f81fd5".into(),
        };
        let url = concat!(
            "https://raw.githubusercontent.com/trade3403/ai-research-pdf-reader/",
            "eeb648d70e871805a0b45f7d8236c5890c6c9b21/rptranslate/crates/rpmodel/Cargo.toml"
        );
        let mut data = Vec::new();
        downloader.fetch(url, &asset, &mut data).unwrap();
        assert_eq!(
            data,
            b"[package]\nname = \"rpmodel\"\nversion.workspace = true\nedition.workspace = true\n"
        );
        let mut damaged = asset;
        damaged.sha256 = "0".repeat(64);
        assert_eq!(
            downloader.fetch(url, &damaged, &mut Vec::new()),
            Err(Error::Catalog(catalog::Error::Integrity))
        );
        println!("HTTPS_PINNED_FIXTURE PASS bytes=77 corruption_rejected=true");
    }
}
