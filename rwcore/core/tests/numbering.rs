use rwcore::{parse_numbering, read_docx, Error, Limits};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

const NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const BODY: &str = "<w:abstractNum w:abstractNumId='0'><w:multiLevelType w:val='multilevel'/><w:lvl w:ilvl='0'><w:start w:val='1'/><w:numFmt w:val='decimal'/><w:lvlText w:val='%1.'/><w:pPr><w:ind w:left='720'/></w:pPr></w:lvl><w:lvl w:ilvl='1'><w:numFmt w:val='lowerLetter'/><w:lvlText w:val='%1.%2'/></w:lvl></w:abstractNum><w:num w:numId='7'><w:abstractNumId w:val='0'/><w:lvlOverride w:ilvl='0'><w:startOverride w:val='3'/></w:lvlOverride></w:num>";
fn xml(body: &str) -> String { format!("<w:numbering xmlns:w='{NS}'>{body}</w:numbering>") }

#[test]
fn definitions_and_overrides() {
    let c = parse_numbering(xml(BODY).as_bytes(), Limits::default()).unwrap();
    assert_eq!(c.instances[&7].abstract_id, 0);
    assert_eq!(c.instances[&7].overrides[&0].start, Some(3));
    assert_eq!(c.abstracts[&0].levels[&1].text.as_deref(), Some("%1.%2"));
    assert_eq!(c.abstracts[&0].levels[&0].paragraph.left_twips, Some(720));
    assert!(c.diagnostics.is_empty());
}
#[test]
fn rejects_duplicates_dangling_references_and_depth() {
    for body in [
        "<w:abstractNum w:abstractNumId='1'/><w:abstractNum w:abstractNumId='1'/>",
        "<w:num w:numId='1'><w:abstractNumId w:val='99'/></w:num>",
        "<w:abstractNum w:abstractNumId='0'><w:lvl w:ilvl='9'/></w:abstractNum>",
        "<w:abstractNum w:abstractNumId='0'><w:lvl w:ilvl='1'/><w:lvl w:ilvl='1'/></w:abstractNum>",
        "<w:abstractNum w:abstractNumId='0'/><w:num w:numId='0'><w:abstractNumId w:val='0'/></w:num>",
    ] { assert!(parse_numbering(xml(body).as_bytes(), Limits::default()).is_err(), "{body}"); }
}
#[test]
fn full_override_and_unsupported_metadata() {
    let b = "<w:abstractNum w:abstractNumId='0'><w:styleLink w:val='List'/></w:abstractNum><w:num w:numId='1'><w:abstractNumId w:val='0'/><w:lvlOverride w:ilvl='2'><w:lvl w:ilvl='2'><w:numFmt w:val='bullet'/><w:lvlText w:val='•'/><w:rPr><w:b/></w:rPr></w:lvl></w:lvlOverride></w:num>";
    let c = parse_numbering(xml(b).as_bytes(), Limits::default()).unwrap();
    let l = c.instances[&1].overrides[&2].level.as_ref().unwrap();
    assert_eq!(l.text.as_deref(), Some("•"));
    assert_eq!(l.run.bold, Some(true));
    assert_eq!(c.diagnostics.len(), 1);
    assert!(parse_numbering(xml(&b.replace("w:lvl w:ilvl='2'", "w:lvl w:ilvl='3'")).as_bytes(), Limits::default()).is_err());
}
#[test]
fn strict_namespace_and_limits() {
    let strict = xml(BODY).replace(NS, "http://purl.oclc.org/ooxml/wordprocessingml/main");
    assert!(parse_numbering(strict.as_bytes(), Limits::default()).is_ok());
    assert_eq!(parse_numbering(strict.as_bytes(), Limits { max_part_bytes: 2, ..Limits::default() }), Err(Error::Limit("part bytes")));
    assert!(parse_numbering(b"<numbering/>", Limits::default()).is_err());
}

fn fixture(rel: &str, mime: &str) -> Vec<u8> {
    let ct = format!("<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Override PartName='/custom/main.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/><Override PartName='/custom/lists.xml' ContentType='{mime}'/></Types>");
    let root = "<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='r' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument' Target='custom/main.xml'/></Relationships>";
    let main = format!("<w:document xmlns:w='{NS}'><w:body><w:p/></w:body></w:document>");
    let relationships = format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'>{rel}</Relationships>");
    let lists = xml(BODY);
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    for (name,data) in [("[Content_Types].xml",ct.as_str()),("_rels/.rels",root),("custom/main.xml",main.as_str()),("custom/_rels/main.xml.rels",relationships.as_str()),("custom/lists.xml",lists.as_str())] {
        z.start_file(name, SimpleFileOptions::default()).unwrap();
        z.write_all(data.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}
const REL: &str = "<Relationship Id='n' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering' Target='lists.xml'/>";
const MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";

#[test]
fn relationship_loading_and_unrelated_part() {
    let d = read_docx(&fixture(REL, MIME), Limits::default()).unwrap();
    assert_eq!(d.numbering.unwrap().instances[&7].abstract_id, 0);
    assert!(read_docx(&fixture("", MIME), Limits::default()).unwrap().numbering.is_none());
}
#[test]
fn rejects_wrong_mime_and_duplicate_relationships() {
    assert_eq!(read_docx(&fixture(REL, "application/xml"), Limits::default()), Err(Error::Invalid("numbering content type")));
    let rel = format!("{REL}{}", REL.replace("Id='n'", "Id='n2'"));
    assert_eq!(read_docx(&fixture(&rel, MIME), Limits::default()), Err(Error::Invalid("multiple main document numbering relationships")));
}
#[test]
fn external_numbering_is_inert() {
    let rel = "<Relationship Id='n' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering' Target='https://example.org/lists.xml' TargetMode='External'/>";
    let d = read_docx(&fixture(rel, MIME), Limits::default()).unwrap();
    assert!(d.numbering.is_none());
    assert!(d.diagnostics.iter().any(|d| d.message.contains("External numbering")));
}
