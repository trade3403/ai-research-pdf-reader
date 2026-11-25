//! Renderer-independent initial document model.
//! Text is Unicode; layout units are integer twips (1/1440 inch).

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    /// Parsed styles associated with the main document; direct formatting remains unchanged.
    pub styles: Option<crate::opc::StyleCatalog>,
    pub numbering: Option<crate::numbering::NumberingCatalog>,
    /// Ordered top-level content ranges; section properties terminate their own range.
    pub sections: Vec<crate::section::DocumentSection>,
    /// Loaded only by read_docx_with_stories.
    pub header_footer: Option<crate::stories::StoryCatalog>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Paragraph),
    Table(Box<crate::Table>),
    Unsupported { name: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub properties: crate::opc::DirectParagraphProperties,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Run {
    /// Plain-text projection, retained for existing text consumers.
    pub text: String,
    pub properties: crate::opc::DirectRunProperties,
    /// Ordered image anchors at UTF-8 byte boundaries in text.
    pub pictures: Vec<InlinePictureAt>,
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


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlinePictureAt {
    pub text_offset: usize,
    pub picture: crate::InlinePicture,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunContent<'a> {
    Text(&'a str),
    Picture(&'a crate::InlinePicture),
}
impl Run {
    /// Borrow ordered text slices and picture nodes without copying text.
    /// Equal offsets retain source order. Public model edits must keep offsets
    /// sorted and on UTF-8 boundaries; inconsistent offsets fail explicitly.
    pub fn inline_content(&self)->Result<Vec<RunContent<'_>>,crate::Error>{
        let mut out=Vec::new();
        let mut cursor=0;
        for p in &self.pictures{
            if p.text_offset<cursor || !self.text.is_char_boundary(p.text_offset){
                return Err(crate::Error::Invalid("inline picture text offset"));
            }
            if p.text_offset>cursor{
                out.push(RunContent::Text(&self.text[cursor..p.text_offset]));
            }
            out.push(RunContent::Picture(&p.picture));
            cursor=p.text_offset;
        }
        if cursor<self.text.len(){out.push(RunContent::Text(&self.text[cursor..]));}
        Ok(out)
    }
}
