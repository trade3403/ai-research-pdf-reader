//! Bounded Word XML reader using the C1.1 OPC package graph.
use crate::model::{Block, Diagnostic, Document, Paragraph, Run};
use roxmltree::{Document as Xml, Node};
use std::{fmt, io::{Cursor, Read}};
use zip::ZipArchive;

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WORD_STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";

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
    MissingNamedPart(String),
    Xml(String),
    Unsupported(&'static str),
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for Error {}

pub(crate) fn xml<'a>(bytes: &'a [u8], limits: Limits) -> Result<Xml<'a>, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::Unsupported("non-UTF-8 XML"))?;
    Xml::parse_with_options(s, roxmltree::ParsingOptions {
        allow_dtd: false, nodes_limit: limits.max_xml_nodes, ..Default::default()
    }).map_err(|e| Error::Xml(e.to_string()))
}

pub(crate) fn part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut file = zip.by_name(name).map_err(|_| match name {
        "[Content_Types].xml" => Error::MissingPart("[Content_Types].xml"),
        "_rels/.rels" => Error::MissingPart("_rels/.rels"),
        "word/document.xml" => Error::MissingPart("word/document.xml"),
        _ => Error::MissingNamedPart(name.to_owned()),
    })?;
    if file.is_dir() { return Err(Error::Invalid("part is directory")); }
    if file.size() > limits.max_part_bytes as u64 { return Err(Error::Limit("part bytes")); }
    let expected = file.size();
    let bound = expected.saturating_add(1);
    let mut bytes = Vec::new();
    (&mut file).take(bound).read_to_end(&mut bytes).map_err(|e| Error::Io(e.to_string()))?;
    if bytes.len() as u64 != expected { return Err(Error::Invalid("expanded part size mismatch")); }
    Ok(bytes)
}

pub(crate) fn named(node: Node<'_, '_>, ns: &str, name: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(ns) && node.tag_name().name() == name
}

fn warn(document: &mut Document, message: impl Into<String>) {
    document.diagnostics.push(Diagnostic { message: message.into() });
}

pub(crate) fn open_archive(bytes: &[u8], limits: Limits) -> Result<ZipArchive<Cursor<&[u8]>>, Error> {
    let expected_count = crate::zip_guard::validate_zip(bytes, limits)?;
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::Zip(e.to_string()))?;
    if zip.len() != expected_count { return Err(Error::Invalid("ZIP reader entry count disagreement")); }
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
    Ok(zip)
}

/// No file extraction, network access, macro execution or writes.
pub fn read_docx(bytes: &[u8], limits: Limits) -> Result<Document, Error> {
    let mut zip = open_archive(bytes, limits)?;
    let graph = crate::package::graph(&mut zip, limits)?;
    let main_bytes = part(&mut zip, &graph.main_part, limits)?;
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
