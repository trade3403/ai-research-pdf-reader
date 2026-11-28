//! Logical scalar provenance foundation, not Unicode grapheme segmentation.
use crate::{Error, UnicodeCmap};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceLimits {
    pub max_bytes: usize,
    pub max_scalars: usize,
    pub max_glyphs: usize,
}
impl Default for SourceLimits {
    fn default()->Self { Self{max_bytes:1024*1024,max_scalars:65536,max_glyphs:65536} }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ScalarSource {
    pub character: char,
    pub byte_start: usize,
    pub byte_end: usize,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GlyphSource {
    pub glyph_id: u16,
    pub scalar_index: usize,
    pub missing: bool,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SourceRun {
    scalars: Vec<ScalarSource>,
    glyphs: Vec<GlyphSource>,
    byte_len: usize,
}
impl SourceRun {
    pub fn scalars(&self)->&[ScalarSource]{&self.scalars}
    pub fn glyphs(&self)->&[GlyphSource]{&self.glyphs}
    pub fn byte_len(&self)->usize{self.byte_len}
    /// Any byte within a scalar maps to that scalar; end of text does not.
    pub fn scalar_at_byte(&self,offset:usize)->Option<usize>{
        if offset>=self.byte_len{return None;}
        let i=self.scalars.partition_point(|s|s.byte_end<=offset);
        self.scalars.get(i).filter(|s|s.byte_start<=offset).map(|_|i)
    }
    /// Half-open scalar range to half-open UTF-8 byte range, including empty ranges.
    pub fn byte_range(&self,start:usize,end:usize)->Result<std::ops::Range<usize>,Error>{
        if start>end||end>self.scalars.len(){return Err(Error::Invalid("source scalar range"));}
        let boundary=|i:usize|self.scalars.get(i).map_or(self.byte_len,|s|s.byte_start);
        Ok(boundary(start)..boundary(end))
    }
}
/// One logical source record per Unicode scalar. No normalization, bidi,
/// grapheme breaking or variation-selector interpretation. Missing cmap
/// entries map to glyph zero when explicitly permitted.
pub fn map_source_run(text:&str,cmap:&UnicodeCmap<'_>,allow_missing:bool,l:SourceLimits)->Result<SourceRun,Error>{
    if text.len()>l.max_bytes{return Err(Error::Limit("source bytes"));}
    let n=text.chars().count();
    if n>l.max_scalars||n>l.max_glyphs{return Err(Error::Limit("source records"));}
    let mut scalars=Vec::new();let mut glyphs=Vec::new();
    scalars.try_reserve_exact(n).map_err(|_|Error::Limit("source allocation"))?;
    glyphs.try_reserve_exact(n).map_err(|_|Error::Limit("source allocation"))?;
    for (i,(start,ch)) in text.char_indices().enumerate(){
        let id=cmap.glyph_for_char(ch)?;
        if id.is_none()&&!allow_missing{return Err(Error::Invalid("source missing glyph"));}
        scalars.push(ScalarSource{character:ch,byte_start:start,byte_end:start+ch.len_utf8()});
        glyphs.push(GlyphSource{glyph_id:id.unwrap_or(0),scalar_index:i,missing:id.is_none()});
    }
    Ok(SourceRun{scalars,glyphs,byte_len:text.len()})
}
