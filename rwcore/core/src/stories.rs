//! C2.2d-2: bounded read-only header/footer story loading.
use crate::{Block, Paragraph, Run, Error, Limits, PackageGraph, ResourceKind, Target};
use crate::opc::{xml, named, part, parse_run_node, parse_paragraph_node};
use crate::section::{DocumentSection, HeaderFooterKind, resolve_header_footer};
use std::{collections::BTreeMap, io::Cursor};
use zip::ZipArchive;

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const SETTINGS_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml";
const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub kind: HeaderFooterKind,
    pub blocks: Vec<Block>,
    /// Field metadata, independent of the cached paragraph text.
    pub fields: Vec<crate::fields::StoryField>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageSettings {
    pub even_and_odd_headers: Option<bool>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryCatalog {
    /// One parsed copy per canonical package part.
    pub parts: BTreeMap<String, Story>,
    /// Main-document relationship IDs mapped to loaded internal part names.
    pub headers: BTreeMap<String, String>,
    pub footers: BTreeMap<String, String>,
    pub settings: Option<PageSettings>,
}

fn namespace<'a>(root: roxmltree::Node<'a, '_>, name: &str) -> Result<&'a str, Error> {
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("story/settings namespace"))?;
    if !matches!(ns, WORD | STRICT) || !named(root, ns, name) {
        return Err(Error::Invalid("story/settings root"));
    }
    Ok(ns)
}

/// Parse top-level paragraphs, direct formatting, text, tabs and text breaks.
/// Field instructions and cached results are retained separately; nested tables and unsupported drawings remain diagnostics;
/// unsupported top-level blocks retain their names.
pub fn parse_story(bytes: &[u8], kind: HeaderFooterKind, limits: Limits) -> Result<Story, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let name = match kind { HeaderFooterKind::Header => "hdr", HeaderFooterKind::Footer => "ftr" };
    let ns = namespace(root, name)?;
    let mut out = Story { kind, blocks: Vec::new(), fields: Vec::new(), diagnostics: Vec::new() };
    for node in root.children().filter(|n| n.is_element()) {
        if !named(node, ns, "p") {
            let name = node.tag_name().name().to_owned();
            out.diagnostics.push(format!("Unsupported story block: {name}"));
            out.blocks.push(Block::Unsupported { name });
            continue;
        }
        crate::fields::collect_paragraph_fields(node, ns, out.blocks.len(), &mut out.fields)?;
        let mut p = Paragraph::default();
        let mut properties_seen = false;
        for child in node.children().filter(|n| n.is_element()) {
            if named(child, ns, "pPr") {
                if properties_seen {
                    out.diagnostics.push("Duplicate story paragraph property container".into());
                    continue;
                }
                properties_seen = true;
                p.properties = parse_paragraph_node(child, ns);
                out.diagnostics.extend(p.properties.diagnostics.iter().cloned());
            } else if named(child, ns, "r") {
                let mut run = Run::default();
                let mut properties_seen = false;
                for item in child.children().filter(|n| n.is_element()) {
                    if named(item, ns, "rPr") {
                        if properties_seen {
                            out.diagnostics.push("Duplicate story run property container".into());
                            continue;
                        }
                        properties_seen = true;
                        run.properties = parse_run_node(item, ns);
                        out.diagnostics.extend(run.properties.diagnostics.iter().cloned());
                    } else if named(item, ns, "t") {
                        for text in item.children().filter(|n| n.is_text()) {
                            run.text.push_str(text.text().unwrap_or_default());
                        }
                        if item.children().any(|n| n.is_element()) {
                            out.diagnostics.push("Unsupported nested story text element".into());
                        }
                    } else if named(item, ns, "tab") {
                        run.text.push('\t');
                    } else if named(item, ns, "br") && matches!(item.attribute((ns, "type")), None | Some("textWrapping")) {
                        run.text.push('\n');
                    } else if named(item, ns, "drawing") {
                        if let Err(e)=crate::drawing_inline::append_run_picture(&mut run,item){
                            out.diagnostics.push(format!("Unsupported or invalid inline drawing: {e}"));
                        }
                    } else {
                        out.diagnostics.push(format!("Unsupported story run element: {}", item.tag_name().name()));
                    }
                }
                p.runs.push(run);
            } else {
                out.diagnostics.push(format!("Unsupported story paragraph element: {}", child.tag_name().name()));
            }
        }
        out.blocks.push(Block::Paragraph(p));
    }
    Ok(out)
}

pub fn parse_page_settings(bytes: &[u8], limits: Limits) -> Result<PageSettings, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = namespace(root, "settings")?;
    let mut out = PageSettings::default();
    for child in root.children().filter(|n| n.is_element()) {
        if !named(child, ns, "evenAndOddHeaders") {
            out.diagnostics.push(format!("Unsupported page setting: {}", child.tag_name().name()));
            continue;
        }
        if out.even_and_odd_headers.is_some() {
            return Err(Error::Invalid("duplicate evenAndOddHeaders"));
        }
        out.even_and_odd_headers = Some(match child.attribute((ns, "val")) {
            None | Some("1" | "true" | "on") => true,
            Some("0" | "false" | "off") => false,
            _ => return Err(Error::Invalid("invalid evenAndOddHeaders")),
        });
    }
    Ok(out)
}

fn charge(total: &mut usize, bytes: usize) -> Result<(), Error> {
    *total = total.checked_add(bytes).ok_or(Error::Limit("story aggregate bytes"))?;
    if *total > MAX_TOTAL_BYTES { return Err(Error::Limit("story aggregate bytes")); }
    Ok(())
}

pub(crate) fn load(zip: &mut ZipArchive<Cursor<&[u8]>>, graph: &PackageGraph,
    sections: &[DocumentSection], limits: Limits) -> Result<StoryCatalog, Error>
{
    let mut out = StoryCatalog::default();
    let mut total = 0usize;
    for section in sections {
        let Some(props) = section.properties.as_ref() else { continue; };
        for (kind, refs, ids) in [
            (HeaderFooterKind::Header, &props.headers, &mut out.headers),
            (HeaderFooterKind::Footer, &props.footers, &mut out.footers),
        ] {
            for (variant, id) in refs {
                if ids.contains_key(id) { continue; }
                let name = resolve_header_footer(graph, props, kind, *variant)?
                    .ok_or(Error::Invalid("missing header/footer reference"))?;
                if !out.parts.contains_key(name) {
                    let bytes = part(zip, name, limits)?;
                    charge(&mut total, bytes.len())?;
                    let story = parse_story(&bytes, kind, limits)?;
                    out.parts.insert(name.to_owned(), story);
                }
                ids.insert(id.clone(), name.to_owned());
            }
        }
    }
    let mut settings = graph.relationships.iter().filter(|r| r.source == graph.main_part && r.kind == ResourceKind::Settings);
    if let Some(rel) = settings.next() {
        if settings.next().is_some() { return Err(Error::Invalid("multiple main document settings relationships")); }
        let Target::Internal(name) = &rel.target else { return Err(Error::Unsupported("external settings reference")); };
        if graph.parts.get(name).and_then(|m| m.as_deref()) != Some(SETTINGS_MIME) {
            return Err(Error::Invalid("settings content type"));
        }
        let bytes = part(zip, name, limits)?;
        charge(&mut total, bytes.len())?;
        out.settings = Some(parse_page_settings(&bytes, limits)?);
    }
    Ok(out)
}


use crate::section::HeaderFooterVariant;

/// A resolved part with the zero-based source section that defined its reference.
/// The original story is borrowed, not duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryBinding<'a> {
    pub part_name: &'a str,
    pub story: &'a Story,
    pub source_section: usize,
}

/// Pagination supplies the physical page context. This engine deliberately
/// does not infer parity from displayed page numbers or continuous breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageContext {
    pub first_in_section: bool,
    pub even_page: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionStories<'a> {
    pub headers: BTreeMap<HeaderFooterVariant, StoryBinding<'a>>,
    pub footers: BTreeMap<HeaderFooterVariant, StoryBinding<'a>>,
    pub title_page: bool,
    pub even_and_odd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedStories<'a> {
    pub variant: HeaderFooterVariant,
    pub header: Option<StoryBinding<'a>>,
    pub footer: Option<StoryBinding<'a>>,
}

impl<'a> SectionStories<'a> {
    /// First-page selection takes precedence over even-page selection.
    /// Missing active variants are blank; no fallback to another variant.
    pub fn select(&self, context: PageContext) -> SelectedStories<'a> {
        let variant = if self.title_page && context.first_in_section {
            HeaderFooterVariant::First
        } else if self.even_and_odd && context.even_page {
            HeaderFooterVariant::Even
        } else {
            HeaderFooterVariant::Default
        };
        SelectedStories {
            variant,
            header: self.headers.get(&variant).copied(),
            footer: self.footers.get(&variant).copied(),
        }
    }
}

impl StoryCatalog {
    fn binding(&self, kind: HeaderFooterKind, id: &str, section: usize)
        -> Result<StoryBinding<'_>, Error>
    {
        let ids = match kind {
            HeaderFooterKind::Header => &self.headers,
            HeaderFooterKind::Footer => &self.footers,
        };
        let name = ids.get(id).ok_or(Error::Invalid("unloaded header/footer reference"))?;
        let story = self.parts.get(name).ok_or(Error::Invalid("missing loaded header/footer part"))?;
        if story.kind != kind { return Err(Error::Invalid("loaded header/footer kind")); }
        Ok(StoryBinding { part_name: name, story, source_section: section })
    }

    /// Resolve same-kind, same-variant references in one forward pass.
    /// All six slots inherit independently, even if a slot was inactive in
    /// the preceding section. An explicit empty story replaces inherited text.
    /// Missing titlePg is false, not inherited. The even/odd switch is global.
    /// An inconsistent catalog fails explicitly, with no partial output.
    pub fn section_stories<'a>(&'a self, sections: &[DocumentSection])
        -> Result<Vec<SectionStories<'a>>, Error>
    {
        let mut effective = SectionStories {
            even_and_odd: self.settings.as_ref()
                .and_then(|settings| settings.even_and_odd_headers).unwrap_or(false),
            ..Default::default()
        };
        let mut out = Vec::with_capacity(sections.len());
        for (index, section) in sections.iter().enumerate() {
            effective.title_page = section.properties.as_ref()
                .and_then(|props| props.title_page).unwrap_or(false);
            if let Some(props) = section.properties.as_ref() {
                for (variant, id) in &props.headers {
                    effective.headers.insert(*variant, self.binding(HeaderFooterKind::Header, id, index)?);
                }
                for (variant, id) in &props.footers {
                    effective.footers.insert(*variant, self.binding(HeaderFooterKind::Footer, id, index)?);
                }
            }
            out.push(effective.clone());
        }
        Ok(out)
    }
}

impl crate::Document {
    pub fn section_stories(&self) -> Result<Vec<SectionStories<'_>>, Error> {
        self.header_footer.as_ref().ok_or(Error::Invalid("header/footer stories not loaded"))?
            .section_stories(&self.sections)
    }
}
