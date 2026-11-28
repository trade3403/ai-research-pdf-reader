//! C2.2a: bounded, renderer-neutral section properties.
use crate::opc::{xml, named, Error, Limits};
use roxmltree::Node;
use std::collections::{BTreeSet, BTreeMap};

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const STRICT_REL: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderFooterVariant { Default, First, Even }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderFooterKind { Header, Footer }

/// Explicit page numbering attributes, not evaluated page fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageNumberProperties {
    pub start: Option<i32>,
    pub format: Option<String>,
    pub chapter_style: Option<u8>,
    pub chapter_separator: Option<String>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation { Portrait, Landscape }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionBreak { NextPage, Continuous, EvenPage, OddPage, NextColumn }

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionProperties {
    pub break_type: Option<SectionBreak>,
    pub width_twips: Option<u32>,
    pub height_twips: Option<u32>,
    pub orientation: Option<Orientation>,
    pub margins: PageMargins,
    pub columns: Option<Columns>,
    pub headers: BTreeMap<HeaderFooterVariant, String>,
    pub footers: BTreeMap<HeaderFooterVariant, String>,
    pub title_page: Option<bool>,
    pub page_number: Option<PageNumberProperties>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageMargins {
    pub top: Option<i32>,
    pub bottom: Option<i32>,
    pub left: Option<u32>,
    pub right: Option<u32>,
    pub header: Option<u32>,
    pub footer: Option<u32>,
    pub gutter: Option<u32>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Columns {
    pub count: Option<u16>,
    pub spacing_twips: Option<u32>,
    pub equal_width: Option<bool>,
    pub separator: Option<bool>,
    pub explicit: Vec<Column>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Column {
    pub width_twips: Option<u32>,
    pub spacing_twips: Option<u32>,
}

fn number<T: std::str::FromStr>(s: &str) -> Result<T, Error> {
    s.parse().map_err(|_| Error::Invalid("invalid section integer"))
}
fn positive(s: &str) -> Result<u32, Error> {
    let n: u32 = number(s)?;
    if n == 0 { return Err(Error::Invalid("zero page or column dimension")); }
    Ok(n)
}
fn boolean(s: &str) -> Result<bool, Error> {
    match s {
        "true" | "1" | "on" => Ok(true),
        "false" | "0" | "off" => Ok(false),
        _ => Err(Error::Invalid("invalid section boolean")),
    }
}
fn attributes(node: Node<'_, '_>, ns: &str, warnings: &mut Vec<String>,
    mut read: impl FnMut(&str, &str) -> Result<bool, Error>) -> Result<(), Error>
{
    for a in node.attributes() {
        if a.namespace() != Some(ns) || !read(a.name(), a.value())? {
            warnings.push(format!("Unsupported section attribute: {}/{}", node.tag_name().name(), a.name()));
        }
    }
    Ok(())
}
fn parse_columns(node: Node<'_, '_>, ns: &str, warnings: &mut Vec<String>) -> Result<Columns, Error> {
    let mut out = Columns::default();
    attributes(node, ns, warnings, |key, value| {
        match key {
            "num" => {
                let n: u16 = number(value)?;
                if !(1..=64).contains(&n) { return Err(Error::Limit("section column count")); }
                out.count = Some(n);
            }
            "space" => out.spacing_twips = Some(number(value)?),
            "equalWidth" => out.equal_width = Some(boolean(value)?),
            "sep" => out.separator = Some(boolean(value)?),
            _ => return Ok(false),
        }
        Ok(true)
    })?;
    for child in node.children().filter(|n| n.is_element()) {
        if !named(child, ns, "col") {
            warnings.push(format!("Unsupported columns element: {}", child.tag_name().name()));
            continue;
        }
        if out.explicit.len() >= 64 { return Err(Error::Limit("section column count")); }
        let mut col = Column::default();
        attributes(child, ns, warnings, |key, value| {
            match key {
                "w" => col.width_twips = Some(positive(value)?),
                "space" => col.spacing_twips = Some(number(value)?),
                _ => return Ok(false),
            }
            Ok(true)
        })?;
        out.explicit.push(col);
    }
    if out.equal_width == Some(false) {
        if let Some(count) = out.count {
            if usize::from(count) != out.explicit.len() {
                warnings.push("Explicit column count differs from declared count".into());
            }
        }
    }
    Ok(out)
}

/// Parse a standalone sectPr under existing part and XML node limits.
/// All missing attributes remain unspecified. Dimensions are raw integer twips.
/// Strict universal measures expressed with physical units are not supported.
pub fn parse_section(bytes: &[u8], limits: Limits) -> Result<SectionProperties, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("section namespace"))?;
    if !matches!(ns, WORD | STRICT) || !named(root, ns, "sectPr") {
        return Err(Error::Invalid("section root"));
    }
    parse_section_node(root, ns)
}

pub(crate) fn parse_section_node(root: Node<'_, '_>, ns: &str) -> Result<SectionProperties, Error> {
    let mut out = SectionProperties::default();
    let mut seen = BTreeSet::new();
    for child in root.children().filter(|n| n.is_element()) {
        let key = child.tag_name().name();
        if child.tag_name().namespace() == Some(ns) && matches!(key, "headerReference" | "footerReference") {
            let variant = match child.attribute((ns, "type")) {
                Some("default") => HeaderFooterVariant::Default,
                Some("first") => HeaderFooterVariant::First,
                Some("even") => HeaderFooterVariant::Even,
                _ => return Err(Error::Invalid("invalid header/footer variant")),
            };
            let rel_ns = if ns == STRICT { STRICT_REL } else { REL };
            let id = child.attribute((rel_ns, "id")).filter(|s| !s.is_empty() && !s.chars().any(char::is_whitespace))
                .ok_or(Error::Invalid("missing or invalid header/footer relationship ID"))?;
            if id.len() > 1024 { return Err(Error::Limit("header/footer relationship ID")); }
            let map = if key == "headerReference" { &mut out.headers } else { &mut out.footers };
            if map.insert(variant, id.to_owned()).is_some() {
                return Err(Error::Invalid("duplicate header/footer variant"));
            }
            continue;
        }
        if child.tag_name().namespace() != Some(ns) || !matches!(key, "type" | "pgSz" | "pgMar" | "cols" | "titlePg" | "pgNumType") {
            out.diagnostics.push(format!("Unsupported section property: {key}"));
            continue;
        }
        if !seen.insert(key) { return Err(Error::Invalid("duplicate section property")); }
        match key {
            "type" => {
                out.break_type = Some(match child.attribute((ns, "val")) {
                    Some("nextPage") => SectionBreak::NextPage,
                    Some("continuous") => SectionBreak::Continuous,
                    Some("evenPage") => SectionBreak::EvenPage,
                    Some("oddPage") => SectionBreak::OddPage,
                    Some("nextColumn") => SectionBreak::NextColumn,
                    _ => return Err(Error::Invalid("invalid section break type")),
                });
            }
            "pgSz" => attributes(child, ns, &mut out.diagnostics, |key, value| {
                match key {
                    "w" => out.width_twips = Some(positive(value)?),
                    "h" => out.height_twips = Some(positive(value)?),
                    "orient" => out.orientation = Some(match value {
                        "portrait" => Orientation::Portrait,
                        "landscape" => Orientation::Landscape,
                        _ => return Err(Error::Invalid("invalid page orientation")),
                    }),
                    _ => return Ok(false),
                }
                Ok(true)
            })?,
            "pgMar" => attributes(child, ns, &mut out.diagnostics, |key, value| {
                match key {
                    "top" => out.margins.top = Some(number(value)?),
                    "bottom" => out.margins.bottom = Some(number(value)?),
                    "left" => out.margins.left = Some(number(value)?),
                    "right" => out.margins.right = Some(number(value)?),
                    "header" => out.margins.header = Some(number(value)?),
                    "footer" => out.margins.footer = Some(number(value)?),
                    "gutter" => out.margins.gutter = Some(number(value)?),
                    _ => return Ok(false),
                }
                Ok(true)
            })?,
            "cols" => out.columns = Some(parse_columns(child, ns, &mut out.diagnostics)?),
            "titlePg" => out.title_page = Some(match child.attribute((ns, "val")) {
                Some(value) => boolean(value)?,
                None => true,
            }),
            "pgNumType" => {
                let mut props = PageNumberProperties::default();
                attributes(child, ns, &mut out.diagnostics, |key, value| {
                    match key {
                        "start" => props.start = Some(number(value)?),
                        "fmt" if !value.is_empty() => props.format = Some(value.to_owned()),
                        "fmt" => return Err(Error::Invalid("empty page number format")),
                        "chapStyle" => {
                            let level: u8 = number(value)?;
                            if !(1..=9).contains(&level) { return Err(Error::Invalid("invalid chapter style level")); }
                            props.chapter_style = Some(level);
                        }
                        "chapSep" => {
                            if !matches!(value, "hyphen" | "period" | "colon" | "emDash" | "enDash") {
                                return Err(Error::Invalid("invalid chapter separator"));
                            }
                            props.chapter_separator = Some(value.to_owned());
                        }
                        _ => return Ok(false),
                    }
                    Ok(true)
                })?;
                out.page_number = Some(props);
            }
            _ => unreachable!(),
        }
    }
    Ok(out)
}


/// Source location of a section's terminating properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionEnd { Paragraph, Body, Implicit }

/// A half-open range in Document::blocks. The paragraph containing sectPr
/// belongs to the section it terminates. Body sectPr is metadata, not content.
/// Empty ranges are allowed; absent final properties remain unspecified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSection {
    pub blocks: std::ops::Range<usize>,
    pub properties: Option<SectionProperties>,
    pub end: SectionEnd,
}


/// Resolve an explicit section reference against the main document's own
/// relationships. No inheritance, part loading, external fetch or field evaluation.
pub fn resolve_header_footer<'a>(
    graph: &'a crate::PackageGraph,
    section: &SectionProperties,
    kind: HeaderFooterKind,
    variant: HeaderFooterVariant,
) -> Result<Option<&'a str>, Error> {
    use crate::{ResourceKind, Target};
    let (refs, expected, mime) = match kind {
        HeaderFooterKind::Header => (&section.headers, ResourceKind::Header, "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"),
        HeaderFooterKind::Footer => (&section.footers, ResourceKind::Footer, "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"),
    };
    let Some(id) = refs.get(&variant) else { return Ok(None); };
    let mut matches = graph.relationships.iter().filter(|r| r.source == graph.main_part && &r.id == id);
    let rel = matches.next().ok_or(Error::Invalid("missing header/footer relationship"))?;
    if matches.next().is_some() { return Err(Error::Invalid("ambiguous header/footer relationship")); }
    if rel.kind != expected { return Err(Error::Invalid("header/footer relationship kind")); }
    let Target::Internal(name) = &rel.target else {
        return Err(Error::Unsupported("external header/footer reference"));
    };
    if graph.parts.get(name).and_then(|m| m.as_deref()) != Some(mime) {
        return Err(Error::Invalid("header/footer content type"));
    }
    Ok(Some(name.as_str()))
}
