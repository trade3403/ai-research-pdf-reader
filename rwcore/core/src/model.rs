//! Renderer-independent initial document model.
//! Text is Unicode; layout units are integer twips (1/1440 inch).

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    /// Parsed styles associated with the main document; direct formatting remains unchanged.
    pub styles: Option<crate::opc::StyleCatalog>,
    pub numbering: Option<crate::numbering::NumberingCatalog>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Paragraph),
    Unsupported { name: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub properties: crate::opc::DirectParagraphProperties,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub properties: crate::opc::DirectRunProperties,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Twips(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    pub width: Twips,
    pub height: Twips,
}

impl Paragraph {
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
}
