//! Standalone GSUB type 2 format 1. No lookup scheduling or GDEF filtering.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct MultipleSubstLimits{
    pub max_records:usize,pub max_covered:u32,pub max_sequence:usize,
    pub max_total_glyphs:usize,pub max_bytes:usize,
}
impl Default for MultipleSubstLimits{
    fn default()->Self{Self{max_records:65535,max_covered:65536,max_sequence:1024,
        max_total_glyphs:262144,max_bytes:1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MultipleSubstitution{coverage:GlyphCoverage,sequences:Vec<Vec<u16>>}
impl MultipleSubstitution{
    pub fn len(&self)->usize{self.sequences.len()}
    pub fn is_empty(&self)->bool{self.sequences.is_empty()}
    pub fn substitute(&self,id:u16)->Option<&[u16]>{
        self.coverage.index(id).and_then(|i|self.sequences.get(i as usize).map(Vec::as_slice))
    }
    /// Replace one glyph at a caller-selected position, without recursive
    /// substitution. Returns number of output glyphs on a match, None on miss.
    /// Errors and misses leave the input unchanged.
    pub fn replace_at(&self,run:&mut Vec<u16>,index:usize,max_output:usize)->Result<Option<usize>,Error>{
        let id=*run.get(index).ok_or(Error::Invalid("multiple run index"))?;
        if run.len()>max_output{return Err(Error::Limit("multiple run length"));}
        let Some(seq)=self.substitute(id) else{return Ok(None)};
        let n=run.len().checked_sub(1).and_then(|n|n.checked_add(seq.len())).ok_or(Error::Limit("multiple run length"))?;
        if n>max_output{return Err(Error::Limit("multiple run length"));}
        run.try_reserve(seq.len().saturating_sub(1)).map_err(|_|Error::Limit("multiple allocation"))?;
        run.splice(index..index+1,seq.iter().copied());
        Ok(Some(seq.len()))
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("multiple offset"))?).ok_or(Error::Invalid("multiple truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
/// Input begins at the subtable; trailing GSUB bytes are allowed. Empty
/// sequences are rejected (OpenType MultipleSubst requires glyphCount > 0).
/// Shared sequence offsets are allowed but charged once per reference.
pub fn parse_multiple_substitution(b:&[u8],glyph_count:u32,l:MultipleSubstLimits)->Result<MultipleSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("multiple bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("multiple format"));}
    let cov=usize::from(word(b,2)?);
    let n=usize::from(word(b,4)?);
    if n>l.max_records{return Err(Error::Limit("multiple records"));}
    let header=6+2*n;
    if cov<header||cov>b.len(){return Err(Error::Invalid("multiple coverage offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_records,l.max_covered)?;
    if coverage.len() as usize!=n{return Err(Error::Invalid("multiple sequence count"));}
    let mut sequences=Vec::with_capacity(n);
    let mut total=0usize;
    for i in 0..n{
        let at=usize::from(word(b,6+2*i)?);
        if at<header{return Err(Error::Invalid("multiple sequence offset"));}
        let len=usize::from(word(b,at)?);
        if len==0{return Err(Error::Invalid("multiple empty sequence"));}
        total=total.checked_add(len).ok_or(Error::Limit("multiple total"))?;
        if len>l.max_sequence||total>l.max_total_glyphs{return Err(Error::Limit("multiple sequence budget"));}
        let end=at+2+2*len;
        if end>b.len(){return Err(Error::Invalid("multiple truncated sequence"));}
        let mut seq=Vec::with_capacity(len);
        for j in 0..len{
            let id=word(b,at+2+2*j)?;
            if u32::from(id)>=glyph_count{return Err(Error::Invalid("multiple output glyph"));}
            seq.push(id);
        }
        sequences.push(seq);
    }
    Ok(MultipleSubstitution{coverage,sequences})
}
