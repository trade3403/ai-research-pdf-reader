//! Versioned online catalog wire format. Trust is supplied out of band.
use crate::{
    catalog::{self, Asset, Catalog, Entry, Precision, Version},
    safetensors::{Error as JsonError, Parser},
};
use std::collections::BTreeMap;

const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 1024;
const MAX_FILES: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Format,
    Origin,
    Catalog(catalog::Error),
}

/// An explicitly configured DNS host, without credentials, port or URL path.
/// This is not a DNS/IP security policy: the transport must enforce TLS.
#[derive(Debug, Clone)]
pub struct Origin(String);

impl Origin {
    pub fn new(host: &str) -> Result<Self, Error> {
        if host.len() > 253 || !host.contains('.') || host.ends_with('.') {
            return Err(Error::Origin);
        }
        for label in host.split('.') {
            if label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
                })
            {
                return Err(Error::Origin);
            }
        }
        if !host
            .rsplit('.')
            .next()
            .unwrap_or("")
            .bytes()
            .any(|b| b.is_ascii_lowercase())
        {
            return Err(Error::Origin);
        }
        Ok(Self(host.into()))
    }

    pub fn catalog_url(&self) -> String {
        format!("https://{}/catalog-v1.json", self.0)
    }

    /// Derive the URL, never accept a URL or path from catalog JSON.
    pub fn asset_url(&self, entry: &Entry, name: &str) -> Result<String, Error> {
        entry.validate().map_err(Error::Catalog)?;
        if !entry.files.contains_key(name) {
            return Err(Error::Format);
        }
        let Version(major, minor, patch) = entry.version;
        let precision = match entry.precision {
            Precision::Fp32 => "fp32",
            Precision::Int8 => "int8",
        };
        Ok(format!(
            "https://{}/models/{}/{major}.{minor}.{patch}/{precision}/{name}",
            self.0, entry.id
        ))
    }
}

enum Field {
    Text(String),
    Number(u64),
    Files(BTreeMap<String, Asset>),
    Entries(Vec<Entry>),
}

fn version(value: &str) -> Result<Version, JsonError> {
    let mut parts = value.split('.');
    let mut number = || {
        let part = parts.next().ok_or(JsonError::Header)?;
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(JsonError::Header);
        }
        part.parse::<u32>().map_err(|_| JsonError::Limit)
    };
    let result = Version(number()?, number()?, number()?);
    if parts.next().is_some() {
        return Err(JsonError::Header);
    }
    Ok(result)
}

fn asset(p: &mut Parser<'_>) -> Result<Asset, JsonError> {
    let mut fields = p.object(2, |p, key| match key {
        "bytes" => Ok(Field::Number(p.number()?)),
        "sha256" => Ok(Field::Text(p.string()?)),
        _ => Err(JsonError::Header),
    })?;
    let Some(Field::Number(bytes)) = fields.remove("bytes") else {
        return Err(JsonError::Header);
    };
    let Some(Field::Text(sha256)) = fields.remove("sha256") else {
        return Err(JsonError::Header);
    };
    Ok(Asset { bytes, sha256 })
}

fn entry(p: &mut Parser<'_>) -> Result<Entry, JsonError> {
    let mut fields = p.object(5, |p, key| match key {
        "id" | "version" | "precision" | "min_engine" => Ok(Field::Text(p.string()?)),
        "files" => Ok(Field::Files(p.object(MAX_FILES, |p, _| asset(p))?)),
        _ => Err(JsonError::Header),
    })?;
    let Some(Field::Text(id)) = fields.remove("id") else {
        return Err(JsonError::Header);
    };
    let Some(Field::Text(release)) = fields.remove("version") else {
        return Err(JsonError::Header);
    };
    let Some(Field::Text(engine)) = fields.remove("min_engine") else {
        return Err(JsonError::Header);
    };
    let Some(Field::Text(precision)) = fields.remove("precision") else {
        return Err(JsonError::Header);
    };
    let precision = match precision.as_str() {
        "fp32" => Precision::Fp32,
        "int8" => Precision::Int8,
        _ => return Err(JsonError::Header),
    };
    let Some(Field::Files(files)) = fields.remove("files") else {
        return Err(JsonError::Header);
    };
    Ok(Entry {
        id,
        version: version(&release)?,
        precision,
        min_engine: version(&engine)?,
        files,
    })
}

fn entries(p: &mut Parser<'_>) -> Result<Vec<Entry>, JsonError> {
    p.expect('[')?;
    let mut result = Vec::new();
    if p.take(']') {
        return Ok(result);
    }
    loop {
        if result.len() == MAX_ENTRIES {
            return Err(JsonError::Limit);
        }
        result.push(entry(p)?);
        if p.take(']') {
            return Ok(result);
        }
        p.expect(',')?;
    }
}

/// Verify an independently trusted length and digest BEFORE decoding JSON.
/// A digest supplied by the same untrusted server is not a trust anchor.
pub fn parse_pinned(input: &[u8], trusted: &Asset) -> Result<Catalog, Error> {
    if input.len() > MAX_CATALOG_BYTES || trusted.bytes > MAX_CATALOG_BYTES as u64 {
        return Err(Error::Limit);
    }
    trusted.verify(input).map_err(Error::Catalog)?;
    let mut p = Parser {
        rest: std::str::from_utf8(input).map_err(|_| Error::Format)?,
    };
    let mut fields = p
        .object(3, |p, key| match key {
            "format" => Ok(Field::Text(p.string()?)),
            "version" => Ok(Field::Number(p.number()?)),
            "entries" => Ok(Field::Entries(entries(p)?)),
            _ => Err(JsonError::Header),
        })
        .map_err(|e| match e {
            JsonError::Limit => Error::Limit,
            _ => Error::Format,
        })?;
    p.ws();
    if !p.rest.is_empty()
        || !matches!(fields.remove("format"), Some(Field::Text(s)) if s == "RPCatalog")
        || !matches!(fields.remove("version"), Some(Field::Number(1)))
    {
        return Err(Error::Format);
    }
    let Some(Field::Entries(entries)) = fields.remove("entries") else {
        return Err(Error::Format);
    };
    Catalog::new(entries).map_err(Error::Catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha256::digest_slice;

    const ENTRY: &str = r#"{"id":"opus-en-zh","version":"1.10.0","precision":"int8","min_engine":"0.1.0","files":{"weights.bin":{"bytes":3,"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}}"#;

    fn document(entries: &str) -> String {
        format!(r#"{{"format":"RPCatalog","version":1,"entries":[{entries}]}}"#)
    }

    fn pin(input: &[u8]) -> Asset {
        let (bytes, sha256) = digest_slice(input, MAX_CATALOG_BYTES as u64).unwrap();
        Asset { bytes, sha256 }
    }

    fn parse(input: &str) -> Result<Catalog, Error> {
        parse_pinned(input.as_bytes(), &pin(input.as_bytes()))
    }

    #[test]
    fn pinned_catalog_selects_and_derives_urls() {
        let catalog = parse(&document(ENTRY)).unwrap();
        let entry = catalog
            .select("opus-en-zh", Precision::Int8, Version(0, 1, 0))
            .unwrap();
        let origin = Origin::new("models.example.org").unwrap();
        assert_eq!(
            origin.catalog_url(),
            "https://models.example.org/catalog-v1.json"
        );
        assert_eq!(
            origin.asset_url(entry, "weights.bin").unwrap(),
            "https://models.example.org/models/opus-en-zh/1.10.0/int8/weights.bin"
        );
        assert_eq!(origin.asset_url(entry, "../other"), Err(Error::Format));
        assert!(catalog
            .select("opus-en-zh", Precision::Fp32, Version(0, 1, 0))
            .is_none());
    }

    #[test]
    fn rejects_changed_bytes_before_parse() {
        let text = document(ENTRY);
        let mut bytes = text.into_bytes();
        let trusted = pin(&bytes);
        bytes[0] = b'!';
        assert_eq!(
            parse_pinned(&bytes, &trusted).unwrap_err(),
            Error::Catalog(catalog::Error::Integrity)
        );
        assert_eq!(
            parse_pinned(&bytes[..bytes.len() - 1], &trusted).unwrap_err(),
            Error::Catalog(catalog::Error::Integrity)
        );
    }

    #[test]
    fn rejects_schema_and_version_errors() {
        let text = document(ENTRY);
        for bad in [
            text.replace("RPCatalog", "other"),
            text.replace(r#""version":1"#, r#""version":2"#),
            text.replace("1.10.0", "01.10.0"),
            text.replace("1.10.0", "4294967296.0.0"),
            text.replace("int8", "int4"),
            text.replace(r#""id":"opus-en-zh""#, r#""id":"opus-en-zh","id":"x""#),
            format!("{text} trailing"),
        ] {
            assert!(parse(&bad).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_entries_and_unsafe_asset_names() {
        assert_eq!(
            parse(&document(&format!("{ENTRY},{ENTRY}"))).unwrap_err(),
            Error::Catalog(catalog::Error::Duplicate)
        );
        let bad = document(&ENTRY.replace("weights.bin", "../weights.bin"));
        assert_eq!(
            parse(&bad).unwrap_err(),
            Error::Catalog(catalog::Error::InvalidName)
        );
    }

    #[test]
    fn rejects_untrusted_origin_syntax() {
        for host in [
            "http://example.org",
            "example.org:443",
            "user@example.org",
            "example.org/path",
            "example.org?x=1",
            "127.0.0.1",
            "localhost",
            "example..org",
            "example.org.",
            "-example.org",
        ] {
            assert!(matches!(Origin::new(host), Err(Error::Origin)));
        }
    }

    #[test]
    fn rejects_catalog_over_budget_and_invalid_utf8() {
        let input = vec![0; MAX_CATALOG_BYTES + 1];
        let trusted = pin(b"abc");
        assert_eq!(parse_pinned(&input, &trusted).unwrap_err(), Error::Limit);
        let invalid = [0xff];
        assert_eq!(
            parse_pinned(&invalid, &pin(&invalid)).unwrap_err(),
            Error::Format
        );
    }
}
