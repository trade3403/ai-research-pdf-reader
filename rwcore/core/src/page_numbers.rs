//! C2.2d-5: checked numbering of caller-supplied disjoint physical page ranges.
use crate::{DocumentSection, Error, FieldKind, StoryField};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    physical_before: u32,
    count: u32,
    start: u32,
    format: String,
    chapter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageNumberPlan {
    spans: Vec<Span>,
    total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageNumberContext<'a> {
    pub physical_page: u32,
    pub displayed_page: u32,
    pub document_pages: u32,
    pub section_pages: u32,
    pub page_format: &'a str,
    pub chapter_numbering: bool,
}

impl PageNumberPlan {
    /// Each section must occupy a nonempty, disjoint physical page range.
    /// The caller supplies counts from layout, including any assigned blank pages.
    /// Shared pages at continuous breaks and blank-page ownership are not inferred.
    /// Absent start continues the previous displayed number, initially 1.
    /// Absent format is decimal. Explicit zero is allowed; negative starts are
    /// outside this subset. No allocation proportional to page count.
    pub fn new(sections: &[DocumentSection], counts: &[u32]) -> Result<Self, Error> {
        if sections.len() != counts.len() || sections.is_empty() {
            return Err(Error::Invalid("page count section mismatch"));
        }
        let mut spans = Vec::with_capacity(sections.len());
        let mut total = 0u32;
        let mut previous: Option<u32> = None;
        for (section, &count) in sections.iter().zip(counts) {
            if count == 0 { return Err(Error::Invalid("zero section page count")); }
            let props = section.properties.as_ref().and_then(|p| p.page_number.as_ref());
            let start = match props.and_then(|p| p.start) {
                Some(v) => u32::try_from(v).map_err(|_| Error::Unsupported("negative page number start"))?,
                None => match previous {
                    None => 1,
                    Some(v) => v.checked_add(1).ok_or(Error::Limit("page number overflow"))?,
                },
            };
            previous = Some(start.checked_add(count - 1).ok_or(Error::Limit("page number overflow"))?);
            spans.push(Span {
                physical_before: total, count, start,
                format: props.and_then(|p| p.format.as_deref()).unwrap_or("decimal").to_owned(),
                chapter: props.is_some_and(|p| p.chapter_style.is_some() || p.chapter_separator.is_some()),
            });
            total = total.checked_add(count).ok_or(Error::Limit("document page count overflow"))?;
        }
        Ok(Self { spans, total })
    }

    pub fn total_pages(&self) -> u32 { self.total }

    /// section and page_index are zero-based; physical_page is one-based.
    pub fn context(&self, section: usize, page_index: u32) -> Result<PageNumberContext<'_>, Error> {
        let s = self.spans.get(section).ok_or(Error::Invalid("page section index"))?;
        if page_index >= s.count { return Err(Error::Invalid("section page index")); }
        Ok(PageNumberContext {
            physical_page: s.physical_before + page_index + 1,
            displayed_page: s.start + page_index,
            document_pages: self.total,
            section_pages: s.count,
            page_format: &s.format,
            chapter_numbering: s.chapter,
        })
    }
}

/// Supported field subset: PAGE, NUMPAGES, SECTIONPAGES, optional single numeric
/// format switch and nonnumeric MERGEFORMAT/CHARFORMAT switches. Unknown
/// commands and switches are rejected; cached text is never changed.
/// PAGE uses section format by default; count fields default to decimal.
pub fn evaluate_page_field(field: &StoryField, context: PageNumberContext<'_>) -> Result<String, Error> {
    if field.instruction.len() > 16 * 1024 { return Err(Error::Limit("story field text")); }
    if context.physical_page == 0 || context.physical_page > context.document_pages ||
        context.section_pages == 0 || context.section_pages > context.document_pages {
        return Err(Error::Invalid("invalid page context"));
    }
    let mut tokens = field.instruction.split_whitespace();
    let command = tokens.next().unwrap_or_default();
    let (kind, value, default_format) = if command.eq_ignore_ascii_case("PAGE") {
        if context.chapter_numbering { return Err(Error::Unsupported("chapter page numbering")); }
        (FieldKind::Page, context.displayed_page, context.page_format)
    } else if command.eq_ignore_ascii_case("NUMPAGES") {
        (FieldKind::NumPages, context.document_pages, "decimal")
    } else if command.eq_ignore_ascii_case("SECTIONPAGES") {
        (FieldKind::SectionPages, context.section_pages, "decimal")
    } else {
        return Err(Error::Unsupported("page field command"));
    };
    if field.kind != kind { return Err(Error::Invalid("field kind mismatch")); }
    let mut explicit = None;
    while let Some(token) = tokens.next() {
        if token != r"\*" { return Err(Error::Unsupported("page field switch")); }
        let arg = tokens.next().ok_or(Error::Invalid("missing field switch argument"))?;
        if arg.eq_ignore_ascii_case("MERGEFORMAT") || arg.eq_ignore_ascii_case("CHARFORMAT") { continue; }
        let format = match arg {
            "ROMAN" => "upperRoman",
            "roman" => "lowerRoman",
            "ALPHABETIC" => "upperLetter",
            "alphabetic" => "lowerLetter",
            s if s.eq_ignore_ascii_case("Arabic") => "decimal",
            _ => return Err(Error::Unsupported("page field number format")),
        };
        if explicit.replace(format).is_some() { return Err(Error::Invalid("duplicate field number format")); }
    }
    crate::numbering::format_value(value, explicit.unwrap_or(default_format))
}
