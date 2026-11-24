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
            let mut property_seen = false;
            for item in child.children().filter(|n| n.is_element()) {
                if named(item, ns, "rPr") {
                    if property_seen {
                        warn(&mut document, "Duplicate run property container");
                        continue;
                    }
                    property_seen = true;
                    run.properties = parse_run_node(item, ns);
                    for message in &run.properties.diagnostics {
                        warn(&mut document, message.clone());
                    }
                } else if named(item, ns, "t") {
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


// C2.0 first independent increment: typed direct run properties.
// Style inheritance and attachment to the document tree are subsequent increments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectRunProperties {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub size_half_points: Option<u32>,
    pub color_rgb: Option<[u8; 3]>,
    pub automatic_color: bool,
    pub ascii_font: Option<String>,
    pub east_asia_font: Option<String>,
    pub style_id: Option<String>,
    pub diagnostics: Vec<String>,
}

fn on_off(node: Node<'_, '_>, ns: &str) -> Option<bool> {
    match node.attribute((ns, "val")) {
        None | Some("true" | "1" | "on") => Some(true),
        Some("false" | "0" | "off") => Some(false),
        _ => None,
    }
}

/// Parse a standalone w:rPr element, without applying defaults or inheritance.
/// Unknown and malformed properties remain explicit diagnostics.
pub fn parse_direct_run_properties(bytes: &[u8], limits: Limits) -> Result<DirectRunProperties, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("run property namespace"))?;
    if !matches!(ns, WORD | WORD_STRICT) || !named(root, ns, "rPr") {
        return Err(Error::Invalid("run property root"));
    }
    Ok(parse_run_node(root, ns))
}

fn parse_run_node(root: Node<'_, '_>, ns: &str) -> DirectRunProperties {
    let mut out = DirectRunProperties::default();
    let mut seen = std::collections::HashSet::new();
    for node in root.children().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        if node.tag_name().namespace() != Some(ns) {
            out.diagnostics.push(format!("Unsupported foreign run property: {name}"));
            continue;
        }
        if !seen.insert(name) {
            out.diagnostics.push(format!("Duplicate run property: {name}"));
            continue;
        }
        let val = node.attribute((ns, "val"));
        let valid = match name {
            "b" => { out.bold = on_off(node, ns); out.bold.is_some() }
            "i" => { out.italic = on_off(node, ns); out.italic.is_some() }
            "sz" => {
                out.size_half_points = val.and_then(|s| s.parse::<u32>().ok()).filter(|v| *v > 0);
                out.size_half_points.is_some()
            }
            "color" => {
                if node.attribute((ns, "themeColor")).is_some() {
                    out.diagnostics.push("Theme color resolution not implemented".into());
                }
                if val == Some("auto") {
                    out.automatic_color = true;
                    true
                } else {
                    out.color_rgb = val.filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                        .and_then(|s| u32::from_str_radix(s, 16).ok())
                        .map(|v| [(v >> 16) as u8, (v >> 8) as u8, v as u8]);
                    out.color_rgb.is_some()
                }
            }
            "rFonts" => {
                out.ascii_font = node.attribute((ns, "ascii")).filter(|s| !s.is_empty()).map(str::to_owned);
                out.east_asia_font = node.attribute((ns, "eastAsia")).filter(|s| !s.is_empty()).map(str::to_owned);
                for attr in node.attributes() {
                    if attr.namespace() != Some(ns) || !matches!(attr.name(), "ascii" | "eastAsia") {
                        out.diagnostics.push(format!("Unsupported font attribute: {}", attr.name()));
                    }
                }
                true
            }
            "rStyle" => {
                out.style_id = val.filter(|s| !s.is_empty()).map(str::to_owned);
                out.style_id.is_some()
            }
            _ => {
                out.diagnostics.push(format!("Unsupported run property: {name}"));
                continue;
            }
        };
        if !valid { out.diagnostics.push(format!("Invalid run property: {name}")); }
    }
    out
}

#[cfg(test)]
mod direct_property_tests {
    use super::*;
    fn parse(inner: &str) -> DirectRunProperties {
        let input = format!("<w:rPr xmlns:w=\"{WORD}\">{inner}</w:rPr>");
        parse_direct_run_properties(input.as_bytes(), Limits::default()).unwrap()
    }
    #[test]
    fn direct_values_and_explicit_false() {
        let p = parse("<w:b/><w:i w:val=\"0\"/><w:sz w:val=\"24\"/><w:color w:val=\"12aBff\"/>");
        assert_eq!(p.bold, Some(true));
        assert_eq!(p.italic, Some(false));
        assert_eq!(p.size_half_points, Some(24));
        assert_eq!(p.color_rgb, Some([18,171,255]));
        assert!(p.diagnostics.is_empty());
    }
    #[test]
    fn absent_and_auto_are_distinct() {
        assert_eq!(parse("").bold, None);
        assert!(parse("<w:color w:val=\"auto\"/>").automatic_color);
        assert!(!parse("").automatic_color);
    }
    #[test]
    fn malformed_duplicate_and_unknown_are_diagnosed() {
        let p = parse("<w:b w:val=\"maybe\"/><w:b/><w:sz w:val=\"0\"/><w:color w:val=\"中abc\"/><w:u/>");
        assert_eq!(p.bold, None);
        assert_eq!(p.size_half_points, None);
        assert_eq!(p.color_rgb, None);
        assert_eq!(p.diagnostics.len(), 5);
    }
    #[test]
    fn font_and_style_ids_preserve_unicode() {
        let p = parse("<w:rFonts w:ascii=\"Arial\" w:eastAsia=\"宋体\"/><w:rStyle w:val=\"Strong\"/>");
        assert_eq!(p.ascii_font.as_deref(), Some("Arial"));
        assert_eq!(p.east_asia_font.as_deref(), Some("宋体"));
        assert_eq!(p.style_id.as_deref(), Some("Strong"));
    }
    #[test]
    fn strict_namespace_and_limits() {
        let s = format!("<w:rPr xmlns:w=\"{WORD_STRICT}\"><w:b/></w:rPr>");
        assert_eq!(parse_direct_run_properties(s.as_bytes(), Limits::default()).unwrap().bold, Some(true));
        let limits = Limits { max_part_bytes: 1, ..Limits::default() };
        assert_eq!(parse_direct_run_properties(s.as_bytes(), limits), Err(Error::Limit("part bytes")));
        assert!(parse_direct_run_properties(b"<rPr/>", Limits::default()).is_err());
    }
}
