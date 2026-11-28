//! GDEF MarkGlyphSetsDef format 1, with 32-bit relative coverage offsets.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};

#[derive(Debug,Clone,Copy)]
pub struct MarkGlyphSetLimits{
    pub max_bytes:usize,
    pub max_sets:usize,
    pub max_records:usize,
    pub max_covered:usize,
}
impl Default for MarkGlyphSetLimits{
    fn default()->Self{Self{max_bytes:1024*1024,max_sets:65535,max_records:65535,max_covered:262144}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MarkGlyphSets{
    sets:Vec<GlyphCoverage>,
    glyph_count:u32,
}
impl MarkGlyphSets{
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn coverage(&self,index:usize)->Option<&GlyphCoverage>{self.sets.get(index)}
    /// None: invalid set index or glyph outside the font domain.
    /// Some(false): valid set and glyph, but not a member.
    /// Membership alone does not imply GDEF Mark glyph classification.
    pub fn contains(&self,index:usize,glyph:u16)->Option<bool>{
        if u32::from(glyph)>=self.glyph_count{return None;}
        Some(self.coverage(index)?.index(glyph).is_some())
    }
}
fn slice(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("mark sets offset"))?).ok_or(Error::Invalid("mark sets truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    Ok(u16::from_be_bytes(slice(b,p,2)?.try_into().unwrap()))
}
/// Coverage offsets are non-null and relative to the start of this subtable.
/// Trailing parent-table bytes are permitted. Shared coverages are charged
/// per reference against cumulative record and expanded glyph limits.
pub fn parse_mark_glyph_sets(b:&[u8],glyph_count:u32,l:MarkGlyphSetLimits)->Result<MarkGlyphSets,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("mark sets bytes"));}
    if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("mark sets glyph domain"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("mark sets format"));}
    let n=usize::from(word(b,2)?);
    if n>l.max_sets{return Err(Error::Limit("mark sets count"));}
    let header=4+n*4;
    slice(b,0,header)?;
    let mut sets=Vec::with_capacity(n);
    let mut records=l.max_records;
    let mut covered=l.max_covered;
    for i in 0..n{
        let raw=u32::from_be_bytes(slice(b,4+i*4,4)?.try_into().unwrap());
        let at=usize::try_from(raw).map_err(|_|Error::Limit("mark sets coverage offset"))?;
        if at<header{return Err(Error::Invalid("mark sets coverage offset"));}
        let c=b.get(at..).ok_or(Error::Invalid("mark sets coverage offset"))?;
        let count=usize::from(word(c,2)?);
        records=records.checked_sub(count).ok_or(Error::Limit("mark sets records"))?;
        let cov=parse_glyph_coverage(c,glyph_count,count,covered.min(65536) as u32)?;
        covered=covered.checked_sub(cov.len() as usize).ok_or(Error::Limit("mark sets covered glyphs"))?;
        sets.push(cov);
    }
    Ok(MarkGlyphSets{sets,glyph_count})
}
