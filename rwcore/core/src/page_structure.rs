//! C2.2e: composition of static section geometry, numbering and stories.
use crate::{Document, Error, GutterSide, PageGeometry, PageNumberContext,
    PageNumberPlan, PageContext, SectionStories, SelectedStories, StoryBinding,
    StoryField, compute_page_geometry, evaluate_page_field};

/// One record per section, never one allocation per physical page.
/// The document is borrowed and remains unchanged.
#[derive(Debug)]
pub struct PageStructurePlan<'a> {
    document: &'a Document,
    geometry: Vec<PageGeometry>,
    stories: Vec<SectionStories<'a>>,
    numbering: PageNumberPlan,
}

#[derive(Debug)]
pub struct StructuredPage<'a> {
    pub section_index: usize,
    pub page_index: u32,
    /// All blocks belonging to the section, not a laid-out page's content.
    pub section_blocks: std::ops::Range<usize>,
    pub geometry: &'a PageGeometry,
    pub numbers: PageNumberContext<'a>,
    pub stories: SelectedStories<'a>,
}

#[derive(Debug)]
pub struct EvaluatedField<'a> {
    pub source: &'a StoryField,
    pub value: Result<String, Error>,
}

impl<'a> PageStructurePlan<'a> {
    /// Counts and gutter sides must correspond to each source section.
    /// Caller supplies the final disjoint page ranges; this does not paginate
    /// or infer parity of shared pages. Requires explicit section geometry and
    /// a loaded story catalog, even if it contains no header/footer references.
    /// All source blocks must be covered exactly once by ordered section ranges.
    pub fn new(document: &'a Document, counts: &[u32], gutter_sides: &[GutterSide])
        -> Result<Self, Error>
    {
        if gutter_sides.len() != document.sections.len() {
            return Err(Error::Invalid("gutter side section mismatch"));
        }
        let mut next = 0;
        for section in &document.sections {
            if section.blocks.start != next || section.blocks.end < next ||
                section.blocks.end > document.blocks.len() {
                return Err(Error::Invalid("invalid section block coverage"));
            }
            next = section.blocks.end;
        }
        if next != document.blocks.len() {
            return Err(Error::Invalid("invalid section block coverage"));
        }
        let numbering = PageNumberPlan::new(&document.sections, counts)?;
        let stories = document.section_stories()?;
        let mut geometry = Vec::with_capacity(document.sections.len());
        for (section, &side) in document.sections.iter().zip(gutter_sides) {
            let props = section.properties.as_ref()
                .ok_or(Error::Invalid("missing section geometry properties"))?;
            geometry.push(compute_page_geometry(props, side)?);
        }
        Ok(Self { document, geometry, stories, numbering })
    }

    pub fn total_pages(&self) -> u32 { self.numbering.total_pages() }

    /// Both indices are zero-based. Story choice uses physical parity, never
    /// the displayed page number. Field evaluation is separate and lazy.
    pub fn page(&self, section: usize, page_index: u32) -> Result<StructuredPage<'_>, Error> {
        let numbers = self.numbering.context(section, page_index)?;
        let stories = self.stories[section].select(PageContext {
            first_in_section: page_index == 0,
            even_page: numbers.physical_page % 2 == 0,
        });
        Ok(StructuredPage {
            section_index: section, page_index,
            section_blocks: self.document.sections[section].blocks.clone(),
            geometry: &self.geometry[section], numbers, stories,
        })
    }
}

impl<'a> StructuredPage<'a> {
    fn fields(&self, story: Option<StoryBinding<'a>>) -> impl Iterator<Item = EvaluatedField<'a>> {
        let numbers = self.numbers;
        story.into_iter().flat_map(|binding| binding.story.fields.iter())
            .map(move |source| EvaluatedField { source, value: evaluate_page_field(source, numbers) })
    }

    /// Errors are per field; one unsupported instruction does not discard the
    /// other fields or the story. The original cached results remain available.
    pub fn header_fields(&self) -> impl Iterator<Item = EvaluatedField<'a>> {
        self.fields(self.stories.header)
    }

    pub fn footer_fields(&self) -> impl Iterator<Item = EvaluatedField<'a>> {
        self.fields(self.stories.footer)
    }
}
