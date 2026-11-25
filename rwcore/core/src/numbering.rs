//! C2.1: bounded numbering definitions and initial list label calculation.
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


impl NumberingCatalog {
    /// Full level replacement followed by a separate startOverride, if present.
    pub fn effective_level(&self, num_id: u32, index: u8) -> Result<NumberingLevel, Error> {
        if num_id == 0 || index > 8 { return Err(Error::Invalid("invalid numbering reference")); }
        let instance = self.instances.get(&num_id).ok_or(Error::Invalid("missing numbering instance"))?;
        let abstract_num = self.abstracts.get(&instance.abstract_id).ok_or(Error::Invalid("missing abstract numbering definition"))?;
        let over = instance.overrides.get(&index);
        let mut level = over.and_then(|o| o.level.as_ref()).or_else(|| abstract_num.levels.get(&index))
            .ok_or(Error::Invalid("missing numbering level"))?.clone();
        if let Some(start) = over.and_then(|o| o.start) { level.start = Some(start); }
        Ok(level)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberingLabel {
    pub text: String,
    /// Layout suffix is separate from the label text.
    pub suffix: String,
    pub value: u32,
}

/// One ordered traversal with independent state per list instance.
/// Failed label generation never advances counters.
#[derive(Debug, Default)]
pub struct NumberingCounter {
    counters: BTreeMap<u32, [Option<u32>; 9]>,
}

fn start_value(level: &NumberingLevel) -> u32 { level.start.unwrap_or(0) }

pub(crate) fn format_value(value: u32, kind: &str) -> Result<String, Error> {
    match kind {
        "decimal" => Ok(value.to_string()),
        "decimalZero" => Ok(format!("{value:02}")),
        "upperLetter" | "lowerLetter" => {
            if value == 0 { return Err(Error::Invalid("zero letter number")); }
            // Repeated-letter scheme: A..Z, AA..ZZ, AAA...
            let repeats = (value - 1) / 26 + 1;
            if repeats > 256 { return Err(Error::Limit("letter numbering length")); }
            let base = if kind == "upperLetter" { b'A' } else { b'a' };
            let ch = char::from(base + ((value - 1) % 26) as u8);
            Ok(ch.to_string().repeat(repeats as usize))
        }
        "upperRoman" | "lowerRoman" => {
            if !(1..=3999).contains(&value) { return Err(Error::Unsupported("Roman number outside 1..3999")); }
            let mut n = value;
            let mut text = String::new();
            for (amount, symbol) in [(1000,"M"),(900,"CM"),(500,"D"),(400,"CD"),(100,"C"),(90,"XC"),(50,"L"),(40,"XL"),(10,"X"),(9,"IX"),(5,"V"),(4,"IV"),(1,"I")] {
                while n >= amount { n -= amount; text.push_str(symbol); }
            }
            if kind == "lowerRoman" { text.make_ascii_lowercase(); }
            Ok(text)
        }
        _ => Err(Error::Unsupported("numbering format")),
    }
}

impl NumberingCounter {
    /// Advance an explicit effective reference; zero suppresses numbering.
    /// Initial restart subset: restart when the trigger level or a higher
    /// ancestor appears, except when explicitly disabled by restart=0.
    /// Unvisited ancestors use their start values for display only.
    /// Unsupported metadata is not interpreted; callers must inspect diagnostics.
    pub fn next_label(&mut self, catalog: &NumberingCatalog, num_id: u32, index: u8)
        -> Result<Option<NumberingLabel>, Error>
    {
        if num_id == 0 { return Ok(None); }
        let level = catalog.effective_level(num_id, index)?;
        let mut state = self.counters.get(&num_id).copied().unwrap_or([None; 9]);
        let i = usize::from(index);
        let value = match state[i] {
            Some(v) => v.checked_add(1).ok_or(Error::Limit("numbering counter overflow"))?,
            None => start_value(&level),
        };
        state[i] = Some(value);
        let kind = level.format.as_deref().unwrap_or("decimal");
        let pattern = level.text.as_deref().unwrap_or("");
        if pattern.len() > 4096 { return Err(Error::Limit("numbering template length")); }
        let suffix = match level.suffix.as_deref().unwrap_or("tab") {
            s @ ("tab" | "space" | "nothing") => s.to_owned(),
            _ => return Err(Error::Unsupported("numbering suffix")),
        };
        let mut text = String::new();
        if kind == "bullet" {
            text.push_str(pattern);
        } else if kind != "none" {
            format_value(value, kind)?;
            let mut chars = pattern.chars().peekable();
            // Cache formatted references to avoid repeated clones of large level data.
            let mut rendered: [Option<String>; 9] = Default::default();
            while let Some(ch) = chars.next() {
                if ch == '%' {
                    if let Some(digit @ '1'..='9') = chars.peek().copied() {
                        chars.next();
                        let k = digit as u8 - b'1';
                        if k <= index {
                            let slot = &mut rendered[usize::from(k)];
                            if slot.is_none() {
                                let referenced = catalog.effective_level(num_id, k)?;
                                let v = state[usize::from(k)].unwrap_or_else(|| start_value(&referenced));
                                *slot = Some(format_value(v, referenced.format.as_deref().unwrap_or("decimal"))?);
                            }
                            text.push_str(slot.as_deref().unwrap_or_default());
                        }
                    } else { text.push(ch); }
                } else { text.push(ch); }
                if text.len() > 16384 { return Err(Error::Limit("numbering label length")); }
            }
        }
        let instance = catalog.instances.get(&num_id).ok_or(Error::Invalid("missing numbering instance"))?;
        let abstract_num = catalog.abstracts.get(&instance.abstract_id).ok_or(Error::Invalid("missing abstract numbering definition"))?;
        for k in index+1..=8 {
            if !abstract_num.levels.contains_key(&k) &&
                !instance.overrides.get(&k).is_some_and(|o| o.level.is_some()) { continue; }
            let deeper = catalog.effective_level(num_id, k)?;
            let trigger = deeper.restart.unwrap_or(k);
            if trigger > k { return Err(Error::Invalid("numbering restart level")); }
            if trigger != 0 && index < trigger {
                state[usize::from(k)] = None;
            }
        }
        self.counters.insert(num_id, state);
        Ok(Some(NumberingLabel { text, suffix, value }))
    }
}


/// A top-level paragraph with resolved formatting and a separate list label.
/// An invalid numbering reference does not discard the paragraph or its runs.
#[derive(Debug)]
pub struct NumberedParagraph<'a> {
    pub formatted: crate::FormattedParagraph<'a>,
    pub label: Result<Option<NumberingLabel>, Error>,
}

#[derive(Debug)]
pub enum NumberedBlock<'a> {
    Paragraph(NumberedParagraph<'a>),
    Unsupported { name: &'a str },
}

impl crate::Document {
    /// Resolve labels lazily in body order, with new counters per traversal.
    /// A missing level on a nonzero reference defaults to level zero in this
    /// subset. A zero ID suppresses numbering without consuming a counter.
    /// Unsupported blocks are preserved but not traversed; their possible
    /// internal list items cannot yet contribute to counters.
    /// Numbering errors are separate from formatting errors and leave counters
    /// unchanged. Source text and direct formatting are not modified.
    pub fn numbered_blocks(&self) -> impl Iterator<Item = Result<NumberedBlock<'_>, Error>> + '_ {
        let mut counter = NumberingCounter::default();
        self.formatted_blocks().map(move |block| {
            match block? {
                crate::FormattedBlock::Unsupported { name } => Ok(NumberedBlock::Unsupported { name }),
                crate::FormattedBlock::Paragraph(formatted) => {
                    let properties = &formatted.properties;
                    let label = match properties.num_id {
                        Some(0) => Ok(None),
                        None if properties.num_level.is_some() => Err(Error::Invalid("numbering level without instance")),
                        None => Ok(None),
                        Some(id) => match self.numbering.as_ref() {
                            Some(catalog) => counter.next_label(catalog, id, properties.num_level.unwrap_or(0)),
                            None => Err(Error::Invalid("missing numbering catalog")),
                        },
                    };
                    Ok(NumberedBlock::Paragraph(NumberedParagraph { formatted, label }))
                }
            }
        })
    }
}
