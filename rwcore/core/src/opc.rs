//! Minimal OPC reader. Only the conventional Word main part is accepted in C1.0.
use crate::model::{Block, Diagnostic, Document, Paragraph, Run};
use roxmltree::{Document as Xml, Node};
use std::{fmt, io::{Cursor, Read}};
use zip::ZipArchive;

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WORD_STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const OFFICE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const OFFICE_STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument";
const MAIN_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_archive_bytes: usize,
    pub max_entries: usize,
    pub max_part_bytes: usize,
    pub max_xml_nodes: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_archive_bytes: 32 * 1024 * 1024, max_entries: 4096,
            max_part_bytes: 8 * 1024 * 1024, max_xml_nodes: 200_000 }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Limit(&'static str),
    Zip(String),
    Io(String),
    MissingPart(&'static str),
    DuplicatePart(String),
    Xml(String),
    Unsupported(&'static str),
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for Error {}

fn xml<'a>(bytes: &'a [u8], limits: Limits) -> Result<Xml<'a>, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::Unsupported("non-UTF-8 XML"))?;
    Xml::parse_with_options(s, roxmltree::ParsingOptions {
        allow_dtd: false, nodes_limit: limits.max_xml_nodes, ..Default::default()
    }).map_err(|e| Error::Xml(e.to_string()))
}

fn part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &'static str, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut file = zip.by_name(name).map_err(|_| Error::MissingPart(name))?;
    if file.is_dir() { return Err(Error::Invalid("part is directory")); }
    if file.size() > limits.max_part_bytes as u64 { return Err(Error::Limit("part bytes")); }
    let bound = (limits.max_part_bytes as u64).saturating_add(1);
    let mut bytes = Vec::new();
    (&mut file).take(bound).read_to_end(&mut bytes).map_err(|e| Error::Io(e.to_string()))?;
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    Ok(bytes)
}

fn named(node: Node<'_, '_>, ns: &str, name: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(ns) && node.tag_name().name() == name
}

fn warn(document: &mut Document, message: impl Into<String>) {
    document.diagnostics.push(Diagnostic { message: message.into() });
}

/// No file extraction, network access, macro execution or writes.
/// Not a complete OPC implementation: nonconventional targets are explicit errors.
pub fn read_docx(bytes: &[u8], limits: Limits) -> Result<Document, Error> {
    if bytes.len() > limits.max_archive_bytes { return Err(Error::Limit("archive bytes")); }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::Zip(e.to_string()))?;
    if zip.len() > limits.max_entries { return Err(Error::Limit("archive entries")); }
    let mut seen = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i).map_err(|e| Error::Zip(e.to_string()))?;
        let name = entry.name().to_owned();
        if name.starts_with('/') || name.contains('\\') ||
            name.split('/').any(|s| s == ".." || s == ".") {
            return Err(Error::Invalid("unsafe ZIP path"));
        }
        if !seen.insert(name.clone()) { return Err(Error::DuplicatePart(name)); }
    }
    let ct_bytes = part(&mut zip, "[Content_Types].xml", limits)?;
    let ct = xml(&ct_bytes, limits)?;
    if !named(ct.root_element(), CT, "Types") { return Err(Error::Invalid("content types root")); }
    let mut types = ct.root_element().children().filter(|n|
        named(*n, CT, "Override") && n.attribute("PartName") == Some("/word/document.xml"));
    if types.next().and_then(|n| n.attribute("ContentType")) != Some(MAIN_TYPE) || types.next().is_some() {
        return Err(Error::Unsupported("main part content type"));
    }
    let rel_bytes = part(&mut zip, "_rels/.rels", limits)?;
    let rel = xml(&rel_bytes, limits)?;
    if !named(rel.root_element(), REL, "Relationships") { return Err(Error::Invalid("relationships root")); }
    let mut offices = rel.root_element().children().filter(|n|
        named(*n, REL, "Relationship") && matches!(n.attribute("Type"), Some(OFFICE | OFFICE_STRICT)));
    let main = offices.next().ok_or(Error::Invalid("officeDocument relationship"))?;
    if offices.next().is_some() { return Err(Error::Invalid("multiple officeDocument relationships")); }
    if !matches!(main.attribute("TargetMode"), None | Some("Internal")) ||
        !matches!(main.attribute("Target"), Some("word/document.xml" | "/word/document.xml")) {
        return Err(Error::Unsupported("external or nonconventional main part"));
    }
    let main_bytes = part(&mut zip, "word/document.xml", limits)?;
    let tree = xml(&main_bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("document namespace"))?;
    if !matches!(ns, WORD | WORD_STRICT) || !named(root, ns, "document") {
        return Err(Error::Invalid("Word document root"));
    }
    let mut bodies = root.children().filter(|n| named(*n, ns, "body"));
    let body = bodies.next().ok_or(Error::Invalid("missing body"))?;
    if bodies.next().is_some() { return Err(Error::Invalid("duplicate body")); }

    let mut document = Document::default();
    for node in body.children().filter(|n| n.is_element()) {
        if !named(node, ns, "p") {
            let name = node.tag_name().name().to_owned();
            warn(&mut document, format!("Unsupported body element: {name}"));
            document.blocks.push(Block::Unsupported { name });
            continue;
        }
        let mut paragraph = Paragraph::default();
        for child in node.children().filter(|n| n.is_element()) {
            if !named(child, ns, "r") {
                warn(&mut document, format!("Unsupported paragraph element: {}", child.tag_name().name()));
                continue;
            }
            let mut run = Run::default();
            for item in child.children().filter(|n| n.is_element()) {
                if named(item, ns, "t") {
                    for text in item.children().filter(|n| n.is_text()) {
                        run.text.push_str(text.text().unwrap_or_default());
                    }
                    if item.children().any(|n| n.is_element()) {
                        warn(&mut document, "Unsupported nested text element");
                    }
                } else if named(item, ns, "tab") {
                    run.text.push('\t');
                } else if named(item, ns, "br") && matches!(item.attribute((ns, "type")), None | Some("textWrapping")) {
                    run.text.push('\n');
                } else {
                    warn(&mut document, format!("Unsupported run element: {}", item.tag_name().name()));
                }
            }
            paragraph.runs.push(run);
        }
        document.blocks.push(Block::Paragraph(paragraph));
    }
    Ok(document)
}
