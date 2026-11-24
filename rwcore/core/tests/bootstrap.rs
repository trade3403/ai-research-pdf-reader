use rwcore::{read_docx, Block, Error, Limits};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

const CT: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const REL: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
const WORD: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t xml:space="preserve">中文 &amp; SCI </w:t><w:tab/><w:t>论文</w:t><w:br/><w:t>second</w:t></w:r></w:p><w:tbl/><w:sectPr/></w:body></w:document>"#;

fn package(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for &(name, text) in entries {
        zip.start_file(name, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn fixture(xml: &str, rel: &str) -> Vec<u8> {
    package(&[("[Content_Types].xml", CT), ("_rels/.rels", rel), ("word/document.xml", xml)])
}

#[test]
fn paragraph_text_and_diagnostics() {
    let document = read_docx(&fixture(WORD, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &document.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.text(), "中文 & SCI \t论文\nsecond");
    assert_eq!(document.blocks.len(), 3);
    assert_eq!(document.diagnostics.len(), 2);
}

#[test]
fn rejects_invalid_zip() {
    assert!(matches!(read_docx(b"invalid", Limits::default()), Err(Error::Zip(_))));
}
#[test]
fn rejects_missing_main_part() {
    let bytes = package(&[("[Content_Types].xml", CT), ("_rels/.rels", REL)]);
    assert_eq!(read_docx(&bytes, Limits::default()), Err(Error::MissingPart("word/document.xml")));
}
#[test]
fn rejects_malformed_xml() {
    assert!(matches!(read_docx(&fixture("<broken", REL), Limits::default()), Err(Error::Xml(_))));
}
#[test]
fn rejects_external_relationship() {
    let rel = REL.replace("Target=", "TargetMode=\"External\" Target=");
    assert!(matches!(read_docx(&fixture(WORD, &rel), Limits::default()), Err(Error::Unsupported(_))));
}
#[test]
fn rejects_dtd() {
    let xml = format!("<!DOCTYPE w:document [<!ENTITY foo 'x'>]>{WORD}");
    assert!(matches!(read_docx(&fixture(&xml, REL), Limits::default()), Err(Error::Xml(_))));
}
#[test]
fn archive_size_limit() {
    let limits = Limits { max_archive_bytes: 1, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("archive bytes")));
}
#[test]
fn part_size_limit() {
    let limits = Limits { max_part_bytes: 10, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("part bytes")));
}
#[test]
fn entry_count_limit() {
    let limits = Limits { max_entries: 2, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("archive entries")));
}
#[test]
fn node_limit() {
    let limits = Limits { max_xml_nodes: 1, ..Limits::default() };
    assert!(matches!(read_docx(&fixture(WORD, REL), limits), Err(Error::Xml(_))));
}
