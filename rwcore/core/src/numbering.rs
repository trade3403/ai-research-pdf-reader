//! C2.1: bounded numbering definitions, not yet a label generator.
use crate::opc::{xml, named, Error, Limits};
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet};

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
pub const NUMBERING_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NumberingCatalog {
    pub abstracts: BTreeMap<u32, AbstractNumbering>,
    pub instances: BTreeMap<u32, NumberingInstance>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbstractNumbering {
    pub levels: BTreeMap<u8, NumberingLevel>,
    pub multi_level_type: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NumberingLevel {
    pub start: Option<u32>,
    pub format: Option<String>,
    pub text: Option<String>,
    pub restart: Option<u8>,
    pub suffix: Option<String>,
    pub paragraph_style: Option<String>,
    pub paragraph: crate::opc::DirectParagraphProperties,
    pub run: crate::opc::DirectRunProperties,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelOverride {
    pub start: Option<u32>,
    pub level: Option<NumberingLevel>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingInstance {
    pub abstract_id: u32,
    pub overrides: BTreeMap<u8, LevelOverride>,
}

fn unsigned(node: Node<'_, '_>, ns: &str, attr: &str) -> Result<u32, Error> {
    node.attribute((ns, attr)).and_then(|s| s.parse().ok())
        .ok_or(Error::Invalid("invalid numbering integer"))
}
fn depth(node: Node<'_, '_>, ns: &str) -> Result<u8, Error> {
    let v = unsigned(node, ns, "ilvl")?;
    if v > 8 { return Err(Error::Invalid("invalid numbering level")); }
    Ok(v as u8)
}
fn val(node: Node<'_, '_>, ns: &str) -> Result<String, Error> {
    node.attribute((ns, "val")).map(str::to_owned)
        .ok_or(Error::Invalid("missing numbering value"))
}
fn unique<'a>(seen: &mut BTreeSet<&'a str>, key: &'a str) -> Result<(), Error> {
    if !seen.insert(key) { return Err(Error::Invalid("duplicate numbering property")); }
    Ok(())
}
fn parse_level(node: Node<'_, '_>, ns: &str, warnings: &mut Vec<String>) -> Result<NumberingLevel, Error> {
    let mut out = NumberingLevel::default();
    let mut seen = BTreeSet::new();
    for child in node.children().filter(|n| n.is_element()) {
        let key = child.tag_name().name();
        if child.tag_name().namespace() != Some(ns) {
            warnings.push(format!("Unsupported foreign numbering level property: {key}"));
            continue;
        }
        unique(&mut seen, key)?;
        match key {
            "start" => out.start = Some(unsigned(child, ns, "val")?),
            "numFmt" => {
                out.format = Some(val(child, ns)?);
                if child.attribute((ns, "format")).is_some() {
                    warnings.push("Custom numbering format not interpreted".into());
                }
            }
            "lvlText" => {
                out.text = Some(val(child, ns)?);
                if child.attribute((ns, "null")).is_some() {
                    warnings.push("Numbering null-text flag not interpreted".into());
                }
            }
            "lvlRestart" => {
                let v = unsigned(child, ns, "val")?;
                if v > 9 { return Err(Error::Invalid("invalid numbering restart")); }
                out.restart = Some(v as u8);
            }
            "suff" => out.suffix = Some(val(child, ns)?),
            "pStyle" => out.paragraph_style = Some(val(child, ns)?),
            "pPr" => out.paragraph = crate::opc::parse_paragraph_node(child, ns),
            "rPr" => out.run = crate::opc::parse_run_node(child, ns),
            _ => warnings.push(format!("Unsupported numbering level property: {key}")),
        }
    }
    Ok(out)
}

/// Parse a UTF-8 numbering part under existing XML/part budgets.
/// Keep explicit definitions and overrides; no counter or rendering semantics.
pub fn parse_numbering(bytes: &[u8], limits: Limits) -> Result<NumberingCatalog, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("numbering namespace"))?;
    if !matches!(ns, WORD | STRICT) || !named(root, ns, "numbering") {
        return Err(Error::Invalid("numbering root"));
    }
    let mut catalog = NumberingCatalog::default();
    for node in root.children().filter(|n| n.is_element()) {
        if named(node, ns, "abstractNum") {
            let id = unsigned(node, ns, "abstractNumId")?;
            let mut a = AbstractNumbering::default();
            for child in node.children().filter(|n| n.is_element()) {
                if named(child, ns, "lvl") {
                    let index = depth(child, ns)?;
                    if a.levels.contains_key(&index) { return Err(Error::Invalid("duplicate abstract numbering level")); }
                    a.levels.insert(index, parse_level(child, ns, &mut catalog.diagnostics)?);
                } else if named(child, ns, "multiLevelType") {
                    if a.multi_level_type.is_some() { return Err(Error::Invalid("duplicate multiLevelType")); }
                    a.multi_level_type = Some(val(child, ns)?);
                } else {
                    catalog.diagnostics.push(format!("Unsupported abstract numbering property: {}", child.tag_name().name()));
                }
            }
            if catalog.abstracts.insert(id, a).is_some() { return Err(Error::Invalid("duplicate abstract numbering ID")); }
        } else if named(node, ns, "num") {
            let id = unsigned(node, ns, "numId")?;
            if id == 0 { return Err(Error::Invalid("zero numbering instance ID")); }
            let mut abstract_id = None;
            let mut overrides = BTreeMap::new();
            for child in node.children().filter(|n| n.is_element()) {
                if named(child, ns, "abstractNumId") {
                    if abstract_id.is_some() { return Err(Error::Invalid("duplicate abstract numbering reference")); }
                    abstract_id = Some(unsigned(child, ns, "val")?);
                } else if named(child, ns, "lvlOverride") {
                    let index = depth(child, ns)?;
                    let mut o = LevelOverride::default();
                    for field in child.children().filter(|n| n.is_element()) {
                        if named(field, ns, "startOverride") {
                            if o.start.is_some() { return Err(Error::Invalid("duplicate start override")); }
                            o.start = Some(unsigned(field, ns, "val")?);
                        } else if named(field, ns, "lvl") {
                            if o.level.is_some() { return Err(Error::Invalid("duplicate level override")); }
                            if depth(field, ns)? != index { return Err(Error::Invalid("mismatched override level")); }
                            o.level = Some(parse_level(field, ns, &mut catalog.diagnostics)?);
                        } else {
                            catalog.diagnostics.push(format!("Unsupported level override property: {}", field.tag_name().name()));
                        }
                    }
                    if overrides.insert(index, o).is_some() { return Err(Error::Invalid("duplicate numbering override")); }
                } else {
                    catalog.diagnostics.push(format!("Unsupported numbering instance property: {}", child.tag_name().name()));
                }
            }
            let instance = NumberingInstance {
                abstract_id: abstract_id.ok_or(Error::Invalid("missing abstract numbering reference"))?,
                overrides,
            };
            if catalog.instances.insert(id, instance).is_some() { return Err(Error::Invalid("duplicate numbering instance ID")); }
        } else {
            catalog.diagnostics.push(format!("Unsupported numbering element: {}", node.tag_name().name()));
        }
    }
    for instance in catalog.instances.values() {
        if !catalog.abstracts.contains_key(&instance.abstract_id) {
            return Err(Error::Invalid("missing abstract numbering definition"));
        }
    }
    Ok(catalog)
}
