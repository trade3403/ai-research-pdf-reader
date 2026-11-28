use rwcore::{validate_zip, resolve_target, Error, Limits};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

fn fixture() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in ["a.xml", "b.xml"] {
        writer.start_file(name, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
        writer.write_all(b"test").unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn locate(data: &[u8], sig: &[u8]) -> usize {
    data.windows(sig.len()).position(|v| v==sig).unwrap()
}
fn set32(data: &mut [u8], p: usize, v: u32) {
    data[p..p+4].copy_from_slice(&v.to_le_bytes());
}
fn zip_error(data: &[u8]) {
    assert!(matches!(validate_zip(data, Limits::default()), Err(Error::Zip(_))));
}
#[test]
fn accepts_normal_zip32() {
    assert_eq!(validate_zip(&fixture(), Limits::default()), Ok(2));
}
#[test]
fn rejects_truncated_archive() {
    let mut data = fixture();
    data.pop();
    zip_error(&data);
}
#[test]
fn rejects_central_offset_mismatch() {
    let mut data = fixture();
    let e = locate(&data,b"PK\x05\x06");
    set32(&mut data,e+16,0);
    zip_error(&data);
}
#[test]
fn rejects_local_name_mismatch() {
    let mut data = fixture();
    data[30] = b'z';
    zip_error(&data);
}
#[test]
fn rejects_local_crc_mismatch() {
    let mut data = fixture();
    data[14] ^= 1;
    zip_error(&data);
}
#[test]
fn rejects_overlapping_local_offsets() {
    let mut data = fixture();
    let p = locate(&data,b"PK\x01\x02");
    let q = p+4+locate(&data[p+4..],b"PK\x01\x02");
    set32(&mut data,q+42,0);
    zip_error(&data);
}
#[test]
fn rejects_duplicate_raw_central_names() {
    let mut data = fixture();
    let p = locate(&data,b"PK\x01\x02");
    let q = p+4+locate(&data[p+4..],b"PK\x01\x02");
    // Make the second local and central names both identical to the first.
    let local = u32::from_le_bytes(data[q+42..q+46].try_into().unwrap()) as usize;
    data[q+46] = b'a';
    data[local+30] = b'a';
    assert_eq!(validate_zip(&data,Limits::default()),Err(Error::DuplicatePart("a.xml".into())));
}
#[test]
fn rejects_encryption_flag() {
    let mut data = fixture();
    let p = locate(&data,b"PK\x01\x02");
    data[p+8] |= 1;
    assert!(matches!(validate_zip(&data,Limits::default()),Err(Error::Unsupported(_))));
}
#[test]
fn rejects_oversized_declared_entry() {
    let mut data = fixture();
    let p = locate(&data,b"PK\x01\x02");
    set32(&mut data,p+24,9*1024*1024);
    assert_eq!(validate_zip(&data,Limits::default()),Err(Error::Limit("part bytes")));
}
#[test]
fn rejects_excessive_declared_expansion_ratio() {
    let mut data = fixture();
    let p = locate(&data,b"PK\x01\x02");
    set32(&mut data,p+24,2*1024*1024);
    assert_eq!(validate_zip(&data,Limits::default()),Err(Error::Limit("compression ratio")));
}
#[test]
fn accepts_safe_percent_encoded_segments() {
    assert_eq!(resolve_target("word/document.xml","media/a%20b.png").unwrap(),"word/media/a%20b.png");
    assert_eq!(resolve_target("word/document.xml","media/%e4%b8%ad.png").unwrap(),"word/media/%E4%B8%AD.png");
}
#[test]
fn rejects_unsafe_or_invalid_percent_sequences() {
    for target in ["%2Ffoo","%5cfoo","%00","%252e","%2e%2e/a","%41","%FF","%E4%B8","%","%2"] {
        assert!(resolve_target("word/document.xml",target).is_err(),"{target}");
    }
}
#[test]
fn rejects_trailing_unindexed_bytes() {
    let mut data = fixture();
    data.push(0);
    zip_error(&data);
}
