//! C2.2d-4: bounded, inert field metadata from direct story paragraphs.
use crate::{Error, opc::named};
use roxmltree::Node;

const MAX_INSTRUCTION: usize = 16 * 1024;
const MAX_CACHE: usize = 64 * 1024;
const MAX_FIELDS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind { Page, NumPages, SectionPages, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldEncoding { Simple, Complex }

/// Source order within a top-level story block. Cached text is not evaluated.
/// None means a complex field had no result separator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryField {
    pub block_index: usize,
    pub encoding: FieldEncoding,
    pub kind: FieldKind,
    pub instruction: String,
    pub cached_text: Option<String>,
}

fn classify(instruction: &str) -> FieldKind {
    let command = instruction.split_whitespace().next().unwrap_or_default();
    if command.eq_ignore_ascii_case("PAGE") { FieldKind::Page }
    else if command.eq_ignore_ascii_case("NUMPAGES") { FieldKind::NumPages }
    else if command.eq_ignore_ascii_case("SECTIONPAGES") { FieldKind::SectionPages }
    else { FieldKind::Other }
}

fn append(dst: &mut String, text: &str, max: usize) -> Result<(), Error> {
    if dst.len().checked_add(text.len()).is_none_or(|n| n > max) {
        return Err(Error::Limit("story field text"));
    }
    dst.push_str(text);
    Ok(())
}

fn text_children(node: Node<'_, '_>, dst: &mut String, max: usize) -> Result<(), Error> {
    for child in node.children() {
        if child.is_text() {
            append(dst, child.text().unwrap_or_default(), max)?;
        } else if child.is_element() {
            return Err(Error::Unsupported("nested field text"));
        }
    }
    Ok(())
}

fn cached_item(node: Node<'_, '_>, ns: &str, cache: &mut String) -> Result<(), Error> {
    if named(node, ns, "t") {
        text_children(node, cache, MAX_CACHE)?;
    } else if named(node, ns, "tab") {
        append(cache, "\t", MAX_CACHE)?;
    } else if named(node, ns, "br") && matches!(node.attribute((ns,"type")), None | Some("textWrapping")) {
        append(cache, "\n", MAX_CACHE)?;
    }
    Ok(())
}

struct Pending {
    instruction: String,
    cache: Option<String>,
}

fn push(out: &mut Vec<StoryField>, block_index: usize, encoding: FieldEncoding,
    instruction: String, cached_text: Option<String>) -> Result<(), Error>
{
    if out.len() >= MAX_FIELDS { return Err(Error::Limit("story field count")); }
    if instruction.trim().is_empty() { return Err(Error::Invalid("empty field instruction")); }
    out.push(StoryField { block_index, encoding, kind: classify(&instruction), instruction, cached_text });
    Ok(())
}

/// Supported subset: simple fields with direct runs, and complex fields with
/// begin/separate/end in direct runs of a single paragraph. Nested fields,
/// cross-paragraph fields and malformed markers are rejected explicitly.
/// Unsupported wrappers are not traversed; no field instruction is executed.
pub(crate) fn collect_paragraph_fields(root: Node<'_, '_>, ns: &str, block_index: usize,
    out: &mut Vec<StoryField>) -> Result<(), Error>
{
    let mut pending: Option<Pending> = None;
    for child in root.children().filter(|n| n.is_element()) {
        if named(child, ns, "fldSimple") {
            if pending.is_some() { return Err(Error::Unsupported("nested story fields")); }
            let instruction = child.attribute((ns,"instr")).ok_or(Error::Invalid("missing simple field instruction"))?;
            if instruction.len() > MAX_INSTRUCTION { return Err(Error::Limit("story field text")); }
            let mut cache = String::new();
            for run in child.children().filter(|n| n.is_element()) {
                if !named(run, ns, "r") { return Err(Error::Unsupported("simple field child")); }
                for item in run.children().filter(|n| n.is_element()) {
                    if named(item, ns, "fldChar") || named(item, ns, "instrText") || named(item, ns, "fldSimple") {
                        return Err(Error::Unsupported("nested story fields"));
                    }
                    cached_item(item, ns, &mut cache)?;
                }
            }
            push(out, block_index, FieldEncoding::Simple, instruction.to_owned(), Some(cache))?;
            continue;
        }
        if !named(child, ns, "r") { continue; }
        for item in child.children().filter(|n| n.is_element()) {
            if named(item, ns, "fldChar") {
                match item.attribute((ns, "fldCharType")) {
                    Some("begin") => {
                        if pending.is_some() { return Err(Error::Unsupported("nested story fields")); }
                        pending = Some(Pending { instruction: String::new(), cache: None });
                    }
                    Some("separate") => {
                        let field = pending.as_mut().ok_or(Error::Invalid("orphan field separator"))?;
                        if field.cache.is_some() { return Err(Error::Invalid("duplicate field separator")); }
                        field.cache = Some(String::new());
                    }
                    Some("end") => {
                        let field = pending.take().ok_or(Error::Invalid("orphan field end"))?;
                        push(out, block_index, FieldEncoding::Complex, field.instruction, field.cache)?;
                    }
                    _ => return Err(Error::Invalid("invalid field marker")),
                }
            } else if named(item, ns, "instrText") {
                let field = pending.as_mut().ok_or(Error::Invalid("orphan field instruction"))?;
                if field.cache.is_some() { return Err(Error::Invalid("instruction after field separator")); }
                text_children(item, &mut field.instruction, MAX_INSTRUCTION)?;
            } else if let Some(cache) = pending.as_mut().and_then(|f| f.cache.as_mut()) {
                cached_item(item, ns, cache)?;
            }
        }
    }
    if pending.is_some() { return Err(Error::Unsupported("unclosed or cross-paragraph story field")); }
    Ok(())
}
