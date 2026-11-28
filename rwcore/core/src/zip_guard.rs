//! C1.2: ZIP32 structural preflight before handing bytes to the decompressor.
//! Reject unsupported container features instead of interpreting them ambiguously.
use crate::{Error, Limits};
use std::collections::BTreeSet;

const MAX_TOTAL: u64 = 128 * 1024 * 1024;
const RATIO: u64 = 1000;
const RATIO_GRACE: u64 = 1024 * 1024;
fn bad(reason: &str) -> Error { Error::Zip(reason.to_owned()) }
fn span(b: &[u8], p: usize, n: usize) -> Result<&[u8], Error> {
    let end = p.checked_add(n).ok_or_else(|| bad("offset overflow"))?;
    b.get(p..end).ok_or_else(|| bad("truncated ZIP structure"))
}
fn u16_at(b: &[u8], p: usize) -> Result<u16, Error> {
    let s = span(b,p,2)?;
    Ok(u16::from_le_bytes([s[0],s[1]]))
}
fn u32_at(b: &[u8], p: usize) -> Result<u32, Error> {
    let s = span(b,p,4)?;
    Ok(u32::from_le_bytes([s[0],s[1],s[2],s[3]]))
}
fn extras(mut data: &[u8]) -> Result<(), Error> {
    while !data.is_empty() {
        let id = u16_at(data,0)?;
        let len = u16_at(data,2)? as usize;
        span(data,4,len)?;
        // Alternate Unicode path, ZIP64 and encryption fields are not accepted.
        if matches!(id, 0x0001 | 0x7075 | 0x9901 | 0x0017) {
            return Err(Error::Unsupported("ZIP64, alternate path or encryption extra field"));
        }
        data = &data[4+len..];
    }
    Ok(())
}

/// Returns the number of validated central-directory records.
pub fn validate_zip(bytes: &[u8], limits: Limits) -> Result<usize, Error> {
    if bytes.len() > limits.max_archive_bytes { return Err(Error::Limit("archive bytes")); }
    if bytes.len() < 22 { return Err(bad("missing ZIP end record")); }
    let last = bytes.len()-22;
    let first = last.saturating_sub(65535);
    let eocd = (first..=last).rev().find(|&p| {
        bytes.get(p..p+4) == Some(&b"PK\x05\x06"[..]) &&
            u16_at(bytes,p+20).ok().map(|n| p+22+n as usize) == Some(bytes.len())
    }).ok_or_else(|| bad("missing ZIP end record"))?;
    if u16_at(bytes,eocd+4)? != 0 || u16_at(bytes,eocd+6)? != 0 {
        return Err(Error::Unsupported("multi-disk ZIP"));
    }
    let count = u16_at(bytes,eocd+10)? as usize;
    let cd_size = u32_at(bytes,eocd+12)? as usize;
    let cd_start = u32_at(bytes,eocd+16)? as usize;
    if count == 65535 || cd_size == u32::MAX as usize || cd_start == u32::MAX as usize {
        return Err(Error::Unsupported("ZIP64"));
    }
    if u16_at(bytes,eocd+8)? as usize != count { return Err(bad("entry count mismatch")); }
    if count > limits.max_entries { return Err(Error::Limit("archive entries")); }
    if cd_start.checked_add(cd_size) != Some(eocd) {
        return Err(bad("central directory bounds"));
    }
    let mut p = cd_start;
    let mut names = BTreeSet::new();
    let mut ranges = Vec::with_capacity(count);
    let mut total = 0u64;
    for _ in 0..count {
        let h = span(bytes,p,46)?;
        if &h[..4] != b"PK\x01\x02" { return Err(bad("central header signature")); }
        let flags = u16_at(h,8)?;
        let method = u16_at(h,10)?;
        if flags & !0x080e != 0 || !matches!(method,0|8) || (method == 0 && flags & 6 != 0) {
            return Err(Error::Unsupported("ZIP flags or compression method"));
        }
        let crc = u32_at(h,16)?;
        let compressed = u32_at(h,20)? as usize;
        let plain = u32_at(h,24)? as usize;
        let name_len = u16_at(h,28)? as usize;
        let extra_len = u16_at(h,30)? as usize;
        let comment_len = u16_at(h,32)? as usize;
        let local = u32_at(h,42)? as usize;
        if u16_at(h,34)? != 0 || [compressed,plain,local].contains(&(u32::MAX as usize)) {
            return Err(Error::Unsupported("ZIP64 or multi-disk entry"));
        }
        if plain > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
        total = total.checked_add(plain as u64).ok_or(Error::Limit("total expanded bytes"))?;
        if total > MAX_TOTAL { return Err(Error::Limit("total expanded bytes")); }
        if plain as u64 > RATIO_GRACE + (compressed as u64)*RATIO {
            return Err(Error::Limit("compression ratio"));
        }
        if method == 0 && compressed != plain { return Err(bad("stored size mismatch")); }
        let name_raw = span(bytes,p+46,name_len)?;
        let name = std::str::from_utf8(name_raw).map_err(|_| Error::Unsupported("non-UTF-8 ZIP name"))?;
        if !name.is_ascii() || name.is_empty() || name.contains('\\') || name.starts_with('/') ||
            name.bytes().any(|b| b.is_ascii_control()) ||
            name.trim_end_matches('/').split('/').any(|s| s.is_empty() || s=="." || s=="..") {
            return Err(Error::Invalid("unsafe ZIP path"));
        }
        if !names.insert(name.to_ascii_lowercase()) { return Err(Error::DuplicatePart(name.to_owned())); }
        extras(span(bytes,p+46+name_len,extra_len)?)?;
        p = p.checked_add(46+name_len+extra_len+comment_len).ok_or_else(|| bad("central offset overflow"))?;
        if p > eocd { return Err(bad("central record extends past directory")); }

        if local >= cd_start { return Err(bad("local header outside data area")); }
        let l = span(bytes,local,30)?;
        if &l[..4] != b"PK\x03\x04" || u16_at(l,6)? != flags || u16_at(l,8)? != method {
            return Err(bad("local header mismatch"));
        }
        let ln = u16_at(l,26)? as usize;
        let le = u16_at(l,28)? as usize;
        if span(bytes,local+30,ln)? != name_raw { return Err(bad("local name mismatch")); }
        extras(span(bytes,local+30+ln,le)?)?;
        let payload = local.checked_add(30+ln+le).ok_or_else(|| bad("local offset overflow"))?;
        let data_end = payload.checked_add(compressed).ok_or_else(|| bad("payload offset overflow"))?;
        if data_end > cd_start { return Err(bad("payload overlaps directory")); }
        let expected = [crc, compressed as u32, plain as u32];
        let actual = [u32_at(l,14)?,u32_at(l,18)?,u32_at(l,22)?];
        let end = if flags & 8 == 0 {
            if actual != expected { return Err(bad("local CRC/size mismatch")); }
            data_end
        } else {
            if actual.iter().zip(expected).any(|(&a,b)| a != 0 && a != b) {
                return Err(bad("local descriptor placeholder mismatch"));
            }
            // Try both descriptor forms (CRC may equal the optional signature).
            let mut matched = None;
            for skip in [0usize,4] {
                if skip == 4 && u32_at(bytes,data_end).ok() != Some(0x08074b50) { continue; }
                let d = data_end+skip;
                if d.checked_add(12).is_some_and(|n| n <= cd_start) &&
                    [u32_at(bytes,d)?,u32_at(bytes,d+4)?,u32_at(bytes,d+8)?] == expected {
                    if matched.is_some() { return Err(bad("ambiguous data descriptor")); }
                    matched = Some(d+12);
                }
            }
            matched.ok_or_else(|| bad("invalid data descriptor"))?
        };
        ranges.push((local,end));
    }
    if p != eocd { return Err(bad("central directory size mismatch")); }
    ranges.sort_unstable();
    let mut end = 0;
    for (start,next) in ranges {
        if start != end || next < start { return Err(bad("overlapping or unindexed local records")); }
        end = next;
    }
    if end != cd_start { return Err(bad("unindexed local data")); }
    Ok(count)
}
