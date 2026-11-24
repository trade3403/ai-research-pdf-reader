//! Effective formatting for the currently supported Word property subset.
use crate::{Paragraph, Run, Error};
use crate::opc::{StyleCatalog, StyleDefinition, DirectParagraphProperties, DirectRunProperties};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveFormatting {
    pub paragraph: DirectParagraphProperties,
    pub run: DirectRunProperties,
}

fn chain<'a>(catalog: &'a StyleCatalog, explicit: Option<&str>, kind: &str) -> Result<Vec<&'a StyleDefinition>, Error> {
    let id = match explicit {
        Some(id) => id,
        None => {
            let mut defaults = catalog.styles.iter().filter(|(_, s)| s.kind == kind && s.is_default);
            let Some((id, _)) = defaults.next() else { return Ok(Vec::new()); };
            if defaults.next().is_some() { return Err(Error::Invalid("multiple default styles of kind")); }
            id.as_str()
        }
    };
    let style = catalog.styles.get(id).ok_or(Error::Invalid("missing selected style"))?;
    if style.kind != kind { return Err(Error::Invalid("wrong selected style kind")); }
    catalog.chain(id)
}

fn merge_paragraph(out: &mut DirectParagraphProperties, layer: &DirectParagraphProperties) {
    macro_rules! copy { ($($f:ident),*) => { $(if layer.$f.is_some() { out.$f = layer.$f; })* }; }
    copy!(alignment, before_twips, after_twips, line, line_rule, left_twips, right_twips,
          keep_next, keep_lines, page_break_before);
    if let Some(v) = layer.hanging_twips {
        out.hanging_twips = Some(v);
        out.first_line_twips = None;
    } else if let Some(v) = layer.first_line_twips {
        out.first_line_twips = Some(v);
        out.hanging_twips = None;
    }
    out.diagnostics.extend(layer.diagnostics.iter().cloned());
}
fn toggle(current: &mut Option<bool>, value: Option<bool>, style: bool) {
    match (style, value) {
        (true, Some(true)) => *current = Some(!current.unwrap_or(false)),
        (true, Some(false)) | (_, None) => {},
        (false, Some(v)) => *current = Some(v),
    }
}
fn merge_run(out: &mut DirectRunProperties, layer: &DirectRunProperties, style: bool) {
    toggle(&mut out.bold, layer.bold, style);
    toggle(&mut out.italic, layer.italic, style);
    if layer.size_half_points.is_some() { out.size_half_points = layer.size_half_points; }
    if let Some(rgb) = layer.color_rgb {
        out.color_rgb = Some(rgb);
        out.automatic_color = false;
    } else if layer.automatic_color {
        out.automatic_color = true;
        out.color_rgb = None;
    }
    if layer.ascii_font.is_some() { out.ascii_font.clone_from(&layer.ascii_font); }
    if layer.east_asia_font.is_some() { out.east_asia_font.clone_from(&layer.east_asia_font); }
    out.diagnostics.extend(layer.diagnostics.iter().cloned());
}

fn paragraph_base(catalog: &StyleCatalog, paragraph: &Paragraph) -> Result<EffectiveFormatting, Error> {
    let mut out = EffectiveFormatting::default();
    merge_paragraph(&mut out.paragraph, &catalog.default_paragraph);
    merge_run(&mut out.run, &catalog.default_run, false);
    for style in chain(catalog, paragraph.properties.style_id.as_deref(), "paragraph")? {
        merge_paragraph(&mut out.paragraph, &style.paragraph);
        merge_run(&mut out.run, &style.run, true);
    }
    merge_paragraph(&mut out.paragraph, &paragraph.properties);
    out.paragraph.style_id.clone_from(&paragraph.properties.style_id);
    Ok(out)
}

fn effective_run(catalog: &StyleCatalog, base: &DirectRunProperties, run: &Run)
    -> Result<DirectRunProperties, Error>
{
    let mut out = base.clone();
    for style in chain(catalog, run.properties.style_id.as_deref(), "character")? {
        merge_run(&mut out, &style.run, true);
    }
    merge_run(&mut out, &run.properties, false);
    out.style_id.clone_from(&run.properties.style_id);
    Ok(out)
}

/// Compute a paragraph/run pair without altering original direct properties.
/// docDefaults -> paragraph style chain -> character style chain -> direct.
/// In style layers b/i=true toggles and false is a no-op; direct values are absolute.
/// Table/numbering styles, themes, and script-specific formatting are not resolved.
/// Invalid style chains fail explicitly. Unspecified values remain unspecified.
pub fn resolve_formatting(catalog: &StyleCatalog, paragraph: &Paragraph, run: &Run)
    -> Result<EffectiveFormatting, Error>
{
    let mut out = paragraph_base(catalog, paragraph)?;
    out.run = effective_run(catalog, &out.run, run)?;
    Ok(out)
}

/// Borrowed view: keeps source text and direct properties intact.
#[derive(Debug)]
pub struct FormattedParagraph<'a> {
    pub source: &'a Paragraph,
    pub properties: DirectParagraphProperties,
    base_run: DirectRunProperties,
    catalog: Option<&'a StyleCatalog>,
}

impl<'a> FormattedParagraph<'a> {
    /// Resolve individual runs lazily; one malformed run does not hide later runs.
    pub fn runs(&self) -> impl Iterator<Item = Result<FormattedRun<'a>, Error>> + '_ {
        self.source.runs.iter().map(move |run| {
            let empty = StyleCatalog::default();
            let properties = effective_run(self.catalog.unwrap_or(&empty), &self.base_run, run)?;
            Ok(FormattedRun { source: run, properties })
        })
    }
}

#[derive(Debug)]
pub struct FormattedRun<'a> {
    pub source: &'a Run,
    pub properties: DirectRunProperties,
}

#[derive(Debug)]
pub enum FormattedBlock<'a> {
    Paragraph(FormattedParagraph<'a>),
    Unsupported { name: &'a str },
}

impl crate::Document {
    /// Top-level blocks in source order, including empty paragraphs and unsupported
    /// elements. Paragraph styles are resolved once per visited paragraph.
    /// Missing catalogs use empty defaults; explicit unresolved IDs return errors.
    pub fn formatted_blocks(&self) -> impl Iterator<Item = Result<FormattedBlock<'_>, Error>> + '_ {
        self.blocks.iter().map(move |block| {
            match block {
                crate::Block::Paragraph(source) => {
                    let empty = StyleCatalog::default();
                    let base = paragraph_base(self.styles.as_ref().unwrap_or(&empty), source)?;
                    Ok(FormattedBlock::Paragraph(FormattedParagraph {
                        source, properties: base.paragraph, base_run: base.run,
                        catalog: self.styles.as_ref(),
                    }))
                }
                crate::Block::Unsupported { name } => Ok(FormattedBlock::Unsupported { name }),
            }
        })
    }
}
