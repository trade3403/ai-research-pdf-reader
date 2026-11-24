//! C1.1: bounded OPC package catalog and relationship graph.
use crate::opc::{Error, Limits, xml, named, part, open_archive};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use zip::ZipArchive;

const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const OFFICE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships/";
const MAIN_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Internal(String),
    External(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    MainDocument, Styles, Numbering, Settings, Theme, Image, FontTable, Font,
    Header, Footer, Footnotes, Endnotes, Comments, Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// Empty source denotes the package root.
    pub source: String,
    pub id: String,
    pub kind: ResourceKind,
    pub relationship_type: String,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageGraph {
    pub main_part: String,
    /// Part names without leading slash, mapped to declared MIME types.
    pub parts: BTreeMap<String, Option<String>>,
    pub relationships: Vec<Relationship>,
}

fn kind(value: &str) -> ResourceKind {
    use ResourceKind::*;
    match value.strip_prefix(OFFICE).or_else(|| value.strip_prefix(STRICT)) {
        Some("officeDocument") => MainDocument,
        Some("styles") => Styles,
        Some("numbering") => Numbering,
        Some("settings") => Settings,
        Some("theme") => Theme,
        Some("image") => Image,
        Some("fontTable") => FontTable,
        Some("font") => Font,
        Some("header") => Header,
        Some("footer") => Footer,
        Some("footnotes") => Footnotes,
        Some("endnotes") => Endnotes,
        Some("comments") => Comments,
        _ => Other,
    }
}

// Deliberately conservative URI subset. Encoded names, queries and fragments
// require a future standards-complete canonicalizer; never guess their meaning.
fn safe_segment(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~!$&'()+,;=@".contains(&b))
        && !s.ends_with('.')
}
pub fn resolve_target(source: &str, target: &str) -> Result<String, Error> {
    if target.is_empty() || target.starts_with("//") || target.ends_with('/') {
        return Err(Error::Invalid("invalid internal target"));
    }
    let mut segments = Vec::new();
    if !source.is_empty() {
        if !source.split('/').all(safe_segment) {
            return Err(Error::Invalid("invalid source part"));
        }
        if !target.starts_with('/') {
            segments.extend(source.split('/'));
            segments.pop();
        }
    }
    for s in target.strip_prefix('/').unwrap_or(target).split('/') {
        match s {
            "." => {},
            ".." => { segments.pop().ok_or(Error::Invalid("target escapes package"))?; },
            _ if safe_segment(s) => segments.push(s),
            _ => return Err(Error::Unsupported("internal URI outside supported subset")),
        }
    }
    if segments.is_empty() { return Err(Error::Invalid("empty resolved target")); }
    let result = segments.join("/");
    if result == "[Content_Types].xml" || relationship_source(&result).is_some() {
        return Err(Error::Invalid("relationship targets metadata"));
    }
    Ok(result)
}

fn relationship_source(name: &str) -> Option<String> {
    if name == "_rels/.rels" { return Some(String::new()); }
    let (dir, file) = name.rsplit_once('/')?;
    let source_dir = if dir == "_rels" { "" } else { dir.strip_suffix("/_rels")? };
    let source = file.strip_suffix(".rels")?;
    if source.is_empty() { return None; }
    Some(if source_dir.is_empty() { source.to_owned() } else { format!("{source_dir}/{source}") })
}

fn required<'a, 'input>(node: roxmltree::Node<'a, 'input>, key: &str) -> Result<&'a str, Error> {
    node.attribute(key).filter(|s| !s.is_empty()).ok_or(Error::Invalid("missing required OPC attribute"))
}

pub fn inspect_package(bytes: &[u8], limits: Limits) -> Result<PackageGraph, Error> {
    let mut archive = open_archive(bytes, limits)?;
    graph(&mut archive, limits)
}

pub(crate) fn graph(archive: &mut ZipArchive<Cursor<&[u8]>>, limits: Limits) -> Result<PackageGraph, Error> {
    let mut names = BTreeSet::new();
    let mut folded = BTreeSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| Error::Zip(e.to_string()))?;
        if entry.is_dir() { continue; }
        let name = entry.name().to_owned();
        if name != "[Content_Types].xml" && !name.split('/').all(safe_segment) {
            return Err(Error::Unsupported("part name outside supported URI subset"));
        }
        if !folded.insert(name.to_ascii_lowercase()) { return Err(Error::DuplicatePart(name)); }
        names.insert(name);
    }
    let content_bytes = part(archive, "[Content_Types].xml", limits)?;
    let content = xml(&content_bytes, limits)?;
    if !named(content.root_element(), CT, "Types") { return Err(Error::Invalid("content types root")); }
    let mut overrides = BTreeMap::new();
    let mut defaults = BTreeMap::new();
    for n in content.root_element().children().filter(|n| n.is_element()) {
        let mime = required(n, "ContentType")?.to_owned();
        if named(n, CT, "Override") {
            let raw = required(n, "PartName")?;
            if !raw.starts_with('/') { return Err(Error::Invalid("override part name")); }
            let name = raw[1..].to_owned();
            if !names.contains(&name) { return Err(Error::Invalid("override target missing")); }
            if overrides.insert(name, mime).is_some() { return Err(Error::Invalid("duplicate content type override")); }
        } else if named(n, CT, "Default") {
            let ext = required(n, "Extension")?.to_ascii_lowercase();
            if defaults.insert(ext, mime).is_some() { return Err(Error::Invalid("duplicate content type default")); }
        } else { return Err(Error::Invalid("unknown content type element")); }
    }
    let parts: BTreeMap<_, _> = names.iter().filter(|n| n.as_str() != "[Content_Types].xml")
        .map(|name| {
            let mime = overrides.get(name).or_else(|| name.rsplit_once('.').and_then(|(_, ext)| defaults.get(&ext.to_ascii_lowercase())));
            (name.clone(), mime.cloned())
        }).collect();
    if !names.contains("_rels/.rels") { return Err(Error::MissingPart("_rels/.rels")); }
    let mut relationships = Vec::new();
    let mut metadata_bytes = content_bytes.len();
    for name in &names {
        let Some(source) = relationship_source(name) else { continue };
        if !source.is_empty() && !parts.contains_key(&source) {
            return Err(Error::Invalid("orphan relationship part"));
        }
        let bytes = part(archive, name, limits)?;
        metadata_bytes = metadata_bytes.checked_add(bytes.len()).ok_or(Error::Limit("metadata bytes"))?;
        if metadata_bytes > 32 * 1024 * 1024 { return Err(Error::Limit("metadata bytes")); }
        let doc = xml(&bytes, limits)?;
        if !named(doc.root_element(), REL, "Relationships") { return Err(Error::Invalid("relationships root")); }
        let mut ids = BTreeSet::new();
        for n in doc.root_element().children().filter(|n| n.is_element()) {
            if !named(n, REL, "Relationship") { return Err(Error::Invalid("unknown relationship element")); }
            if relationships.len() >= 50_000 { return Err(Error::Limit("relationship count")); }
            let id = required(n, "Id")?.to_owned();
            if !ids.insert(id.clone()) { return Err(Error::Invalid("duplicate relationship ID")); }
            let ty = required(n, "Type")?;
            let raw = required(n, "Target")?;
            let target = match n.attribute("TargetMode") {
                None | Some("Internal") => Target::Internal(resolve_target(&source, raw)?),
                Some("External") => Target::External(raw.to_owned()),
                _ => return Err(Error::Invalid("invalid TargetMode")),
            };
            relationships.push(Relationship { source: source.clone(), id, kind: kind(ty),
                relationship_type: ty.to_owned(), target });
        }
    }
    let mut roots = relationships.iter().filter(|r| r.source.is_empty() && r.kind == ResourceKind::MainDocument);
    let root = roots.next().ok_or(Error::Invalid("officeDocument relationship"))?;
    if roots.next().is_some() { return Err(Error::Invalid("multiple officeDocument relationships")); }
    let Target::Internal(main_part) = &root.target else { return Err(Error::Unsupported("external main part")); };
    if !parts.contains_key(main_part) {
        return if main_part == "word/document.xml" { Err(Error::MissingPart("word/document.xml")) }
            else { Err(Error::MissingNamedPart(main_part.clone())) };
    }
    if parts.get(main_part).and_then(|v| v.as_deref()) != Some(MAIN_TYPE) {
        return Err(Error::Unsupported("main part content type"));
    }
    for r in &relationships {
        if let Target::Internal(name) = &r.target {
            if !parts.contains_key(name) { return Err(Error::MissingNamedPart(name.clone())); }
        }
    }
    Ok(PackageGraph { main_part: main_part.clone(), parts, relationships })
}
