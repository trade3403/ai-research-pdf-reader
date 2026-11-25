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
    read_docx_impl(bytes, limits, false)
}

/// Opt-in strict loading of referenced header/footer stories and page settings.
/// Ordinary read_docx remains a metadata-only reader for these resources.
pub fn read_docx_with_stories(bytes: &[u8], limits: Limits) -> Result<Document, Error> {
    read_docx_impl(bytes, limits, true)
}

fn read_docx_impl(bytes: &[u8], limits: Limits, load_stories: bool) -> Result<Document, Error> {
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
    use crate::package::{ResourceKind, Target};
    let mut styles = graph.relationships.iter().filter(|r|
        r.source == graph.main_part && r.kind == ResourceKind::Styles);
    if let Some(rel) = styles.next() {
        if styles.next().is_some() {
            return Err(Error::Invalid("multiple main document styles relationships"));
        }
        match &rel.target {
            Target::Internal(name) => {
                let bytes = part(&mut zip, name, limits)?;
                document.styles = Some(parse_styles(&bytes, limits)?);
            }
            Target::External(_) => warn(&mut document, "External styles relationship ignored"),
        }
    }
    let mut numbering = graph.relationships.iter().filter(|r|
        r.source == graph.main_part && r.kind == ResourceKind::Numbering);
    if let Some(rel) = numbering.next() {
        if numbering.next().is_some() {
            return Err(Error::Invalid("multiple main document numbering relationships"));
        }
        match &rel.target {
            Target::Internal(name) => {
                if graph.parts.get(name).and_then(|mime| mime.as_deref()) != Some(crate::numbering::NUMBERING_MIME) {
                    return Err(Error::Invalid("numbering content type"));
                }
                let bytes = part(&mut zip, name, limits)?;
                document.numbering = Some(crate::numbering::parse_numbering(&bytes, limits)?);
            }
            Target::External(_) => warn(&mut document, "External numbering relationship ignored"),
        }
    }
    use crate::section::{DocumentSection, SectionEnd, parse_section_node};
    let mut section_start = 0;
    let mut final_section_seen = false;
    for node in body.children().filter(|n| n.is_element()) {
        if final_section_seen {
            return Err(Error::Invalid("content after final section properties"));
        }
        if named(node, ns, "sectPr") {
            let properties = parse_section_node(node, ns)?;
            for message in &properties.diagnostics { warn(&mut document, message.clone()); }
            document.sections.push(DocumentSection {
                blocks: section_start..document.blocks.len(),
                properties: Some(properties),
                end: SectionEnd::Body,
            });
            final_section_seen = true;
            continue;
        }
        if named(node, ns, "tbl") {
            let table = crate::table::parse_table_node(node, ns)?;
            for message in &table.diagnostics { warn(&mut document, message.clone()); }
            document.blocks.push(Block::Table(Box::new(table)));
            continue;
        }
        if !named(node, ns, "p") {
            let name = node.tag_name().name().to_owned();
            warn(&mut document, format!("Unsupported body element: {name}"));
            document.blocks.push(Block::Unsupported { name });
            continue;
        }
        let mut paragraph = Paragraph::default();
        let mut paragraph_properties_seen = false;
        let mut section_properties = None;
        for child in node.children().filter(|n| n.is_element()) {
            if named(child, ns, "pPr") {
                if paragraph_properties_seen {
                    warn(&mut document, "Duplicate paragraph property container");
                    continue;
                }
                paragraph_properties_seen = true;
                let mut endings = child.children().filter(|n| named(*n, ns, "sectPr"));
                if let Some(ending) = endings.next() {
                    if endings.next().is_some() {
                        return Err(Error::Invalid("duplicate paragraph section properties"));
                    }
                    let properties = parse_section_node(ending, ns)?;
                    for message in &properties.diagnostics { warn(&mut document, message.clone()); }
                    section_properties = Some(properties);
                }
                paragraph.properties = parse_paragraph_node_impl(child, ns, true);
                for message in &paragraph.properties.diagnostics {
                    warn(&mut document, message.clone());
                }
                continue;
            }
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
        if let Some(properties) = section_properties {
            let end = document.blocks.len();
            document.sections.push(DocumentSection {
                blocks: section_start..end,
                properties: Some(properties),
                end: SectionEnd::Paragraph,
            });
            section_start = end;
        }
    }
    if !final_section_seen {
        document.sections.push(DocumentSection {
            blocks: section_start..document.blocks.len(),
            properties: None,
            end: SectionEnd::Implicit,
        });
    }
    if load_stories {
        document.header_footer = Some(crate::stories::load(&mut zip, &graph, &document.sections, limits)?);
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

pub(crate) fn parse_run_node(root: Node<'_, '_>, ns: &str) -> DirectRunProperties {
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


/// Direct paragraph formatting only; absent values are not defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirectParagraphProperties {
    pub style_id: Option<String>,
    pub alignment: Option<ParagraphAlignment>,
    pub before_twips: Option<u32>,
    pub after_twips: Option<u32>,
    pub line: Option<i32>,
    pub line_rule: Option<LineRule>,
    pub left_twips: Option<i32>,
    pub right_twips: Option<i32>,
    pub first_line_twips: Option<u32>,
    pub hanging_twips: Option<u32>,
    pub keep_next: Option<bool>,
    pub keep_lines: Option<bool>,
    pub page_break_before: Option<bool>,
    /// List instance; zero explicitly suppresses inherited numbering.
    pub num_id: Option<u32>,
    /// List depth 0..=8; absent remains unspecified.
    pub num_level: Option<u8>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphAlignment { Left, Right, Center, Both, Start, End }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineRule { Auto, Exact, AtLeast }

pub(crate) fn parse_paragraph_node(root: Node<'_, '_>, ns: &str) -> DirectParagraphProperties {
    parse_paragraph_node_impl(root, ns, false)
}

fn parse_paragraph_node_impl(root: Node<'_, '_>, ns: &str, section_handled: bool) -> DirectParagraphProperties {
    let mut out = DirectParagraphProperties::default();
    let mut seen = std::collections::HashSet::new();
    for node in root.children().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        if section_handled && named(node, ns, "sectPr") { continue; }
        if node.tag_name().namespace() != Some(ns) {
            out.diagnostics.push(format!("Unsupported foreign paragraph property: {name}"));
            continue;
        }
        if !seen.insert(name) {
            out.diagnostics.push(format!("Duplicate paragraph property: {name}"));
            continue;
        }
        let val = node.attribute((ns, "val"));
        let valid = match name {
            "pStyle" => {
                out.style_id = val.filter(|s| !s.is_empty()).map(str::to_owned);
                out.style_id.is_some()
            }
            "jc" => {
                out.alignment = match val {
                    Some("left") => Some(ParagraphAlignment::Left),
                    Some("right") => Some(ParagraphAlignment::Right),
                    Some("center") => Some(ParagraphAlignment::Center),
                    Some("both") => Some(ParagraphAlignment::Both),
                    Some("start") => Some(ParagraphAlignment::Start),
                    Some("end") => Some(ParagraphAlignment::End),
                    _ => None,
                };
                out.alignment.is_some()
            }
            "keepNext" => { out.keep_next = on_off(node, ns); out.keep_next.is_some() }
            "keepLines" => { out.keep_lines = on_off(node, ns); out.keep_lines.is_some() }
            "pageBreakBefore" => { out.page_break_before = on_off(node, ns); out.page_break_before.is_some() }
            "numPr" => {
                let mut children = std::collections::HashSet::new();
                for item in node.children().filter(|n| n.is_element()) {
                    let key = item.tag_name().name();
                    if item.tag_name().namespace() != Some(ns) || !matches!(key, "numId" | "ilvl") {
                        out.diagnostics.push(format!("Unsupported numbering property: {key}"));
                        continue;
                    }
                    if !children.insert(key) {
                        out.diagnostics.push(format!("Duplicate numbering property: {key}"));
                        continue;
                    }
                    let value = item.attribute((ns, "val"));
                    let valid = match key {
                        "numId" => {
                            out.num_id = value.and_then(|s| s.parse::<u32>().ok());
                            out.num_id.is_some()
                        }
                        "ilvl" => {
                            out.num_level = value.and_then(|s| s.parse::<u8>().ok()).filter(|v| *v <= 8);
                            out.num_level.is_some()
                        }
                        _ => unreachable!(),
                    };
                    if !valid { out.diagnostics.push(format!("Invalid numbering property: {key}")); }
                }
                // Keep an empty container diagnostic; no implicit cancellation.
                if !node.children().any(|n| n.is_element()) {
                    out.diagnostics.push("Empty numbering property container".into());
                }
                true
            }
            "spacing" | "ind" => {
                for a in node.attributes() {
                    let v = a.value();
                    let ok = if a.namespace() != Some(ns) { false } else {
                        match (name, a.name()) {
                            ("spacing", "before") => { out.before_twips = v.parse().ok(); out.before_twips.is_some() }
                            ("spacing", "after") => { out.after_twips = v.parse().ok(); out.after_twips.is_some() }
                            ("spacing", "line") => { out.line = v.parse().ok(); out.line.is_some() }
                            ("spacing", "lineRule") => {
                                out.line_rule = match v { "auto" => Some(LineRule::Auto), "exact" => Some(LineRule::Exact), "atLeast" => Some(LineRule::AtLeast), _ => None };
                                out.line_rule.is_some()
                            }
                            ("ind", "left") => { out.left_twips = v.parse().ok(); out.left_twips.is_some() }
                            ("ind", "right") => { out.right_twips = v.parse().ok(); out.right_twips.is_some() }
                            ("ind", "firstLine") => { out.first_line_twips = v.parse().ok(); out.first_line_twips.is_some() }
                            ("ind", "hanging") => { out.hanging_twips = v.parse().ok(); out.hanging_twips.is_some() }
                            _ => false,
                        }
                    };
                    if !ok { out.diagnostics.push(format!("Unsupported or invalid paragraph attribute: {name}/{}", a.name())); }
                }
                if out.first_line_twips.is_some() && out.hanging_twips.is_some() && name == "ind" {
                    out.diagnostics.push("Conflicting firstLine and hanging indentation".into());
                }
                true
            }
            _ => {
                out.diagnostics.push(format!("Unsupported paragraph property: {name}"));
                continue;
            }
        };
        if !valid { out.diagnostics.push(format!("Invalid paragraph property: {name}")); }
    }
    out
}


/// A style definition preserves direct properties, not effective formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleDefinition {
    pub kind: String,
    pub based_on: Option<String>,
    pub is_default: bool,
    pub paragraph: DirectParagraphProperties,
    pub run: DirectRunProperties,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleCatalog {
    pub styles: std::collections::BTreeMap<String, StyleDefinition>,
    pub default_paragraph: DirectParagraphProperties,
    pub default_run: DirectRunProperties,
}
impl StyleCatalog {
    /// Parent-first chain, with a fixed depth budget and explicit invalid links.
    pub fn chain(&self, id: &str) -> Result<Vec<&StyleDefinition>, Error> {
        let mut chain = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut current = Some(id);
        let mut kind: Option<&str> = None;
        while let Some(key) = current {
            if !seen.insert(key) { return Err(Error::Invalid("cyclic style inheritance")); }
            if chain.len() >= 128 { return Err(Error::Limit("style inheritance depth")); }
            let style = self.styles.get(key).ok_or(Error::Invalid("missing base style"))?;
            if kind.is_some_and(|k| k != style.kind.as_str()) {
                return Err(Error::Invalid("cross-kind style inheritance"));
            }
            kind = Some(style.kind.as_str());
            chain.push(style);
            current = style.based_on.as_deref();
        }
        chain.reverse();
        Ok(chain)
    }
}
/// Parse standalone styles.xml. Does not apply defaults, toggle semantics or formatting.
pub fn parse_styles(bytes: &[u8], limits: Limits) -> Result<StyleCatalog, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("styles namespace"))?;
    if !matches!(ns, WORD | WORD_STRICT) || !named(root, ns, "styles") {
        return Err(Error::Invalid("styles root"));
    }
    let mut catalog = StyleCatalog::default();
    let mut defaults = root.children().filter(|n| named(*n, ns, "docDefaults"));
    if let Some(d) = defaults.next() {
        if defaults.next().is_some() { return Err(Error::Invalid("duplicate docDefaults")); }
        for (wrapper, property) in [("pPrDefault", "pPr"), ("rPrDefault", "rPr")] {
            let mut wrappers = d.children().filter(|n| named(*n, ns, wrapper));
            if let Some(w) = wrappers.next() {
                if wrappers.next().is_some() { return Err(Error::Invalid("duplicate default wrapper")); }
                let mut props = w.children().filter(|n| named(*n, ns, property));
                if let Some(p) = props.next() {
                    if props.next().is_some() { return Err(Error::Invalid("duplicate default properties")); }
                    if property == "pPr" { catalog.default_paragraph = parse_paragraph_node(p, ns); }
                    else { catalog.default_run = parse_run_node(p, ns); }
                }
            }
        }
    }
    for node in root.children().filter(|n| named(*n, ns, "style")) {
        let id = node.attribute((ns, "styleId")).filter(|s| !s.is_empty())
            .ok_or(Error::Invalid("missing style ID"))?;
        let kind = node.attribute((ns, "type")).ok_or(Error::Invalid("missing style type"))?;
        let is_default = match node.attribute((ns, "default")) {
            None | Some("0" | "false" | "off") => false,
            Some("1" | "true" | "on") => true,
            _ => return Err(Error::Invalid("invalid default style flag")),
        };
        let mut style = StyleDefinition { kind: kind.to_owned(), based_on: None, is_default,
            paragraph: Default::default(), run: Default::default() };
        let mut seen = std::collections::BTreeSet::new();
        for child in node.children().filter(|n| n.is_element()) {
            let name = child.tag_name().name();
            if child.tag_name().namespace() != Some(ns) || !matches!(name, "basedOn" | "pPr" | "rPr") { continue; }
            if !seen.insert(name) { return Err(Error::Invalid("duplicate style property container")); }
            match name {
                "basedOn" => style.based_on = Some(child.attribute((ns, "val"))
                    .filter(|s| !s.is_empty()).ok_or(Error::Invalid("missing base style ID"))?.to_owned()),
                "pPr" => style.paragraph = parse_paragraph_node(child, ns),
                "rPr" => style.run = parse_run_node(child, ns),
                _ => unreachable!(),
            }
        }
        if catalog.styles.insert(id.to_owned(), style).is_some() {
            return Err(Error::Invalid("duplicate style ID"));
        }
    }
    Ok(catalog)
}

#[cfg(test)]
mod style_catalog_tests {
    use super::*;
    fn catalog(body: &str) -> Result<StyleCatalog, Error> {
        parse_styles(format!("<w:styles xmlns:w='{WORD}'>{body}</w:styles>").as_bytes(), Limits::default())
    }
    #[test]
    fn parent_first_and_direct_properties() {
        let c = catalog("<w:style w:type='paragraph' w:styleId='A' w:default='1'><w:rPr><w:b/></w:rPr></w:style><w:style w:type='paragraph' w:styleId='B'><w:basedOn w:val='A'/><w:pPr><w:jc w:val='center'/></w:pPr></w:style>").unwrap();
        let chain = c.chain("B").unwrap();
        assert_eq!(chain.len(), 2);
        assert!(chain[0].is_default);
        assert_eq!(chain[0].run.bold, Some(true));
        assert_eq!(chain[1].paragraph.alignment, Some(ParagraphAlignment::Center));
    }
    #[test]
    fn rejects_duplicate_ids() {
        assert!(catalog("<w:style w:type='paragraph' w:styleId='A'/><w:style w:type='paragraph' w:styleId='A'/>").is_err());
    }
    #[test]
    fn rejects_cycles_and_missing_links() {
        let c = catalog("<w:style w:type='paragraph' w:styleId='A'><w:basedOn w:val='A'/></w:style><w:style w:type='paragraph' w:styleId='B'><w:basedOn w:val='C'/></w:style>").unwrap();
        assert_eq!(c.chain("A"), Err(Error::Invalid("cyclic style inheritance")));
        assert_eq!(c.chain("B"), Err(Error::Invalid("missing base style")));
    }
    #[test]
    fn rejects_cross_kind_links() {
        let c = catalog("<w:style w:type='character' w:styleId='A'/><w:style w:type='paragraph' w:styleId='B'><w:basedOn w:val='A'/></w:style>").unwrap();
        assert_eq!(c.chain("B"), Err(Error::Invalid("cross-kind style inheritance")));
    }
    #[test]
    fn bounds_inheritance_depth() {
        let mut body = String::new();
        for i in 0..129 {
            body.push_str(&format!("<w:style w:type='paragraph' w:styleId='s{i}'>"));
            if i > 0 { body.push_str(&format!("<w:basedOn w:val='s{}'/>", i-1)); }
            body.push_str("</w:style>");
        }
        let c = catalog(&body).unwrap();
        assert_eq!(c.chain("s128"), Err(Error::Limit("style inheritance depth")));
    }
}
