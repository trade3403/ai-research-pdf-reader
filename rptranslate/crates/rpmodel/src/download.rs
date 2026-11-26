//! Bounded HTTPS download adapter. TLS is handled by a trusted curl binary.
//! The Rust core owns the catalog, byte limits, integrity and staging policy.
use crate::{
    catalog::{self, Asset, Catalog, Entry},
    online_catalog::{self, Origin},
    resume::{self, PartialDownload, Progress, Range},
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
    Resume(resume::Error),
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

const MAX_RANGE_HEADER: usize = 64 * 1024;

/// curl --include returns HTTP headers followed by the decoded body.
/// Reject ambiguous or unexpected framing before any staging write.
fn range_response(data: &[u8], range: Range) -> Result<(&str, &[u8]), Error> {
    let boundary = data
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .ok_or(Error::Framing)?;
    if boundary > MAX_RANGE_HEADER {
        return Err(Error::Framing);
    }
    let header = std::str::from_utf8(&data[..boundary]).map_err(|_| Error::Framing)?;
    let mut lines = header.split("\r\n");
    let mut status = lines.next().ok_or(Error::Framing)?.split_ascii_whitespace();
    if status.next() != Some("HTTP/1.1") || status.next() != Some("206") {
        return Err(Error::HttpStatus);
    }
    let mut content_range = None;
    let mut length = None;
    let mut encoding = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(Error::Framing)?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b < 32 && b != b'\t')
        {
            return Err(Error::Framing);
        }
        let value = value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("content-range") {
            if content_range.replace(value).is_some() {
                return Err(Error::Framing);
            }
        } else if name.eq_ignore_ascii_case("content-length") {
            if length.replace(value).is_some() {
                return Err(Error::Framing);
            }
        } else if name.eq_ignore_ascii_case("content-encoding") && encoding.replace(value).is_some()
        {
            return Err(Error::Framing);
        }
    }
    if encoding.is_some_and(|v| !v.eq_ignore_ascii_case("identity")) {
        return Err(Error::Framing);
    }
    let content_range = content_range.ok_or(Error::Framing)?;
    let count = range.end - range.start + 1;
    let body = &data[boundary + 4..];
    if content_range != format!("bytes {}-{}/{}", range.start, range.end, range.total)
        || body.len() as u64 != count
        || length.is_some_and(|v| v.parse::<u64>().ok() != Some(count))
    {
        return Err(Error::Framing);
    }
    Ok((content_range, body))
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

    /// Perform one bounded range request. Stop between chunks to pause.
    /// The caller owns the partial file and must keep the same trusted pin.
    /// A final successful step includes whole-file SHA256 verification.
    pub fn fetch_next(
        &self,
        entry: &Entry,
        name: &str,
        partial: &mut PartialDownload,
        max_bytes: u64,
    ) -> Result<Progress, Error> {
        let url = self.origin.asset_url(entry, name).map_err(Error::Online)?;
        let asset = entry.files.get(name).ok_or(Error::Configuration)?;
        if partial.asset() != asset {
            return Err(Error::Configuration);
        }
        self.fetch_next_url(&url, partial, max_bytes)
    }

    fn fetch_next_url(
        &self,
        url: &str,
        partial: &mut PartialDownload,
        max_bytes: u64,
    ) -> Result<Progress, Error> {
        let Some(range) = partial.next_range(max_bytes).map_err(Error::Resume)? else {
            partial.verify_complete().map_err(Error::Resume)?;
            return Ok(partial.progress());
        };
        let count = range.end - range.start + 1;
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
                "--http1.1",
                "--connect-timeout",
                "15",
                "--max-time",
                "120",
                "--max-filesize",
                &count.to_string(),
                "--header",
                "Accept-Encoding: identity",
                "--header",
                &format!("Range: {}", range.header()),
                "--include",
                "--output",
                "-",
                "--url",
                url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::Transport)?;
        let limit = count + MAX_RANGE_HEADER as u64 + 4;
        let mut data = Vec::new();
        let read = child
            .stdout
            .take()
            .ok_or(Error::Transport)
            .and_then(|stdout| {
                stdout
                    .take(limit + 1)
                    .read_to_end(&mut data)
                    .map_err(|_| Error::Transport)
            });
        if read.is_err() || data.len() as u64 > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Framing);
        }
        if !child.wait().map_err(|_| Error::Transport)?.success() {
            return Err(Error::Transport);
        }
        let (header, body) = range_response(&data, range)?;
        let progress = partial
            .append(range, 206, header, body)
            .map_err(Error::Resume)?;
        if progress.received == progress.total {
            partial.verify_complete().map_err(Error::Resume)?;
        }
        Ok(progress)
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
    fn range_headers_and_binary_body() {
        let range = Range {
            start: 1,
            end: 2,
            total: 3,
        };
        let data = b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 1-2/3\r\nContent-Length: 2\r\n\r\n\x00\xff";
        let (header, body) = range_response(data, range).unwrap();
        assert_eq!(header, "bytes 1-2/3");
        assert_eq!(body, [0, 255]);
    }

    #[test]
    fn range_rejects_ambiguous_or_wrong_responses() {
        let range = Range {
            start: 0,
            end: 1,
            total: 3,
        };
        for bad in [
            "HTTP/1.1 200 OK\r\nContent-Range: bytes 0-1/3\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/4\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/3\r\ncontent-range: bytes 0-1/3\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/3\r\nContent-Encoding: gzip\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/3\r\nContent-Length: 3\r\n\r\nab",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/3\r\n\r\na",
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-1/3\r\n\r\nabc",
        ] {
            assert!(range_response(bad.as_bytes(), range).is_err());
        }
    }

    #[test]
    fn range_header_budget_is_bounded() {
        let range = Range {
            start: 0,
            end: 0,
            total: 1,
        };
        let data = format!(
            "HTTP/1.1 206 Partial Content\r\nX-Pad: {}\r\nContent-Range: bytes 0-0/1\r\n\r\na",
            "x".repeat(MAX_RANGE_HEADER)
        );
        assert_eq!(range_response(data.as_bytes(), range), Err(Error::Framing));
    }

    #[test]
    #[ignore = "explicit online CI probe only"]
    fn actual_https_range_fixture() {
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
        let root = std::env::temp_dir().join(format!(
            "rpmodel-https-range-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("fixture.part");
        let mut partial = PartialDownload::create(&path, asset.clone()).unwrap();
        let first = downloader.fetch_next_url(url, &mut partial, 32).unwrap();
        assert_eq!(first.received, 32);
        drop(partial);
        let mut partial = PartialDownload::resume(&path, asset).unwrap();
        let second = downloader.fetch_next_url(url, &mut partial, 32).unwrap();
        assert_eq!(second.received, 64);
        let third = downloader.fetch_next_url(url, &mut partial, 32).unwrap();
        assert_eq!(third.received, 77);
        assert_eq!(partial.verify_complete(), Ok(()));
        drop(partial);
        fs::remove_dir_all(root).unwrap();
        println!("HTTPS_RANGE_FIXTURE PASS bytes=77 chunks=3 reopened=true");
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
