use rwcore::{inspect_package, read_docx, resolve_target, Block, Error, Limits, ResourceKind, Target};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

const CT: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="png" ContentType="image/png"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/manuscript/main.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const ROOT: &str = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="manuscript/main.xml"/>"#;
const MAIN: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>科研论文</w:t></w:r></w:p></w:body></w:document>"#;
const RESOURCES: &str = r#"<Relationship Id="s" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="i" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image.png"/><Relationship Id="f" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/><Relationship Id="h" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org/" TargetMode="External"/>"#;
fn rels(children: &str) -> String {
    format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{children}</Relationships>"#)
}
fn fixture(root: &str, resources: &str, extra: &[(&str, &str)]) -> Vec<u8> {
    let root = rels(root);
    let resources = rels(resources);
    let mut entries = vec![
        ("[Content_Types].xml", CT), ("_rels/.rels", root.as_str()),
        ("manuscript/main.xml", MAIN), ("manuscript/_rels/main.xml.rels", resources.as_str()),
        ("manuscript/styles.xml", "<styles/>"), ("manuscript/fontTable.xml", "<fonts/>"),
        ("media/image.png", "placeholder")];
    entries.extend_from_slice(extra);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer.start_file(name, SimpleFileOptions::default()).unwrap();
        writer.write_all(data.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn nonconventional_main_and_resource_catalog() {
    let bytes = fixture(ROOT, RESOURCES, &[]);
    let graph = inspect_package(&bytes, Limits::default()).unwrap();
    assert_eq!(graph.main_part, "manuscript/main.xml");
    assert_eq!(graph.parts["media/image.png"].as_deref(), Some("image/png"));
    for kind in [ResourceKind::Styles, ResourceKind::Image, ResourceKind::FontTable] {
        assert!(graph.relationships.iter().any(|r| r.kind == kind));
    }
    assert!(graph.relationships.iter().any(|r| r.target == Target::External("https://example.org/".into())));
    let document = read_docx(&bytes, Limits::default()).unwrap();
    let Block::Paragraph(p) = &document.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.text(), "科研论文");
}
#[test]
fn safe_relative_and_absolute_targets() {
    assert_eq!(resolve_target("word/document.xml", "../media/a.png").unwrap(), "media/a.png");
    assert_eq!(resolve_target("word/document.xml", "/word/styles.xml").unwrap(), "word/styles.xml");
    assert_eq!(resolve_target("", "./word/document.xml").unwrap(), "word/document.xml");
}
#[test]
fn reject_ambiguous_and_escaping_uris() {
    for target in ["../../a", "%2e%2e/a", "https://example.org", "//host/a", "a\\b", "a#fragment", "a?query", "a//b", "/_rels/.rels"] {
        assert!(resolve_target("word/document.xml", target).is_err(), "{target}");
    }
}
#[test]
fn dangling_relationship_is_error() {
    let resources = RESOURCES.replace("styles.xml", "missing.xml");
    assert_eq!(inspect_package(&fixture(ROOT, &resources, &[]), Limits::default()),
        Err(Error::MissingNamedPart("manuscript/missing.xml".into())));
}
#[test]
fn duplicate_relationship_id_is_error() {
    let resources = RESOURCES.replace("Id=\"i\"", "Id=\"s\"");
    assert_eq!(inspect_package(&fixture(ROOT, &resources, &[]), Limits::default()),
        Err(Error::Invalid("duplicate relationship ID")));
}
#[test]
fn root_ambiguity_is_error() {
    let root = format!("{ROOT}{}", ROOT.replace("rId1", "rId2"));
    assert_eq!(inspect_package(&fixture(&root, RESOURCES, &[]), Limits::default()),
        Err(Error::Invalid("multiple officeDocument relationships")));
}
#[test]
fn orphan_relationship_part_is_error() {
    let bytes = fixture(ROOT, RESOURCES, &[("manuscript/_rels/missing.xml.rels", "<ignored/>")]);
    assert_eq!(inspect_package(&bytes, Limits::default()), Err(Error::Invalid("orphan relationship part")));
}
#[test]
fn case_alias_is_error() {
    let bytes = fixture(ROOT, RESOURCES, &[("MEDIA/IMAGE.PNG", "alias")]);
    assert!(matches!(inspect_package(&bytes, Limits::default()), Err(Error::DuplicatePart(_))));
}
#[test]
fn cycles_are_finite_edges_not_recursion() {
    let cycle = rels(r#"<Relationship Id="back" Type="urn:test:back" Target="main.xml"/>"#);
    let bytes = fixture(ROOT, RESOURCES, &[("manuscript/_rels/styles.xml.rels", &cycle)]);
    let graph = inspect_package(&bytes, Limits::default()).unwrap();
    assert_eq!(graph.relationships.len(), 6);
}
#[test]
fn unknown_target_mode_is_error() {
    let resources = RESOURCES.replace("TargetMode=\"External\"", "TargetMode=\"Unknown\"");
    assert_eq!(inspect_package(&fixture(ROOT, &resources, &[]), Limits::default()),
        Err(Error::Invalid("invalid TargetMode")));
}
