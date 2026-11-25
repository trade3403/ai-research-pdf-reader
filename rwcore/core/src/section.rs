//! C2.2a: bounded, renderer-neutral section properties.
use crate::opc::{xml, named, Error, Limits};
use roxmltree::Node;
use std::collections::BTreeSet;

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";

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
        if child.tag_name().namespace() != Some(ns) || !matches!(key, "type" | "pgSz" | "pgMar" | "cols") {
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
            _ => unreachable!(),
        }
    }
    Ok(out)
}
