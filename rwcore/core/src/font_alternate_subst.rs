//! Standalone GSUB type 3 format 1. Caller explicitly selects an alternate.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct AlternateSubstLimits{
    pub max_records:usize,pub max_covered:u32,pub max_alternates:usize,
    pub max_total_glyphs:usize,pub max_bytes:usize,
}
impl Default for AlternateSubstLimits{
    fn default()->Self{Self{max_records:65535,max_covered:65536,max_alternates:1024,
        max_total_glyphs:262144,max_bytes:1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct AlternateSubstitution{coverage:GlyphCoverage,sets:Vec<Vec<u16>>}
impl AlternateSubstitution{
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn alternates(&self,id:u16)->Option<&[u16]>{
        self.coverage.index(id).and_then(|i|self.sets.get(i as usize).map(Vec::as_slice))
    }
    /// Zero-based explicit selection. A coverage miss returns None;
    /// an invalid selection on a covered glyph is an error.
    pub fn select(&self,id:u16,index:usize)->Result<Option<u16>,Error>{
        match self.alternates(id){
            None=>Ok(None),
            Some(set)=>set.get(index).copied().map(Some).ok_or(Error::Invalid("alternate selection")),
        }
    }
    /// Replace one caller-selected glyph in place. Errors and misses
    /// preserve the run. No feature-value policy or lookup scheduling.
    pub fn replace_at(&self,run:&mut [u16],position:usize,index:usize)->Result<bool,Error>{
        let id=*run.get(position).ok_or(Error::Invalid("alternate run position"))?;
        if let Some(out)=self.select(id,index)?{run[position]=out;Ok(true)}else{Ok(false)}
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("alternate offset"))?).ok_or(Error::Invalid("alternate truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
/// Parse a type 3 subtable with trailing GSUB bytes allowed. Preserve
/// stored candidate order, including duplicate candidate IDs if present.
/// Shared set offsets are charged once per reference.
pub fn parse_alternate_substitution(b:&[u8],glyph_count:u32,l:AlternateSubstLimits)->Result<AlternateSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("alternate bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("alternate format"));}
    let cov=usize::from(word(b,2)?);
    let n=usize::from(word(b,4)?);
    if n>l.max_records{return Err(Error::Limit("alternate records"));}
    let header=6+2*n;
    if cov<header||cov>b.len(){return Err(Error::Invalid("alternate coverage offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_records,l.max_covered)?;
    if coverage.len() as usize!=n{return Err(Error::Invalid("alternate set count"));}
    let mut sets=Vec::with_capacity(n);
    let mut total=0usize;
    for i in 0..n{
        let at=usize::from(word(b,6+2*i)?);
        if at<header{return Err(Error::Invalid("alternate set offset"));}
        let len=usize::from(word(b,at)?);
        if len==0{return Err(Error::Invalid("alternate empty set"));}
        total=total.checked_add(len).ok_or(Error::Limit("alternate total"))?;
        if len>l.max_alternates||total>l.max_total_glyphs{return Err(Error::Limit("alternate set budget"));}
        let end=at+2+2*len;
        if end>b.len(){return Err(Error::Invalid("alternate truncated set"));}
        let mut set=Vec::with_capacity(len);
        for j in 0..len{
            let id=word(b,at+2+2*j)?;
            if u32::from(id)>=glyph_count{return Err(Error::Invalid("alternate output glyph"));}
            set.push(id);
        }
        sets.push(set);
    }
    Ok(AlternateSubstitution{coverage,sets})
}
