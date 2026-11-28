//! Standalone GSUB type 1 subtable, no lookup scheduling or GDEF filtering.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct SingleSubstLimits{pub max_records:usize,pub max_glyphs:u32,pub max_bytes:usize}
impl Default for SingleSubstLimits{fn default()->Self{Self{max_records:65535,max_glyphs:65536,max_bytes:1024*1024}}}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SingleSubstitution{coverage:GlyphCoverage,replacements:Vec<u16>}
impl SingleSubstitution{
    pub fn len(&self)->usize{self.replacements.len()}
    pub fn is_empty(&self)->bool{self.replacements.is_empty()}
    pub fn substitute(&self,id:u16)->Option<u16>{
        self.coverage.index(id).and_then(|i|self.replacements.get(i as usize).copied())
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("single offset"))?;
    let a=b.get(p..end).ok_or(Error::Invalid("single truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
/// All output IDs are validated before returning a rule. Format 1 uses
/// modulo-65536 addition as required by OpenType, then checks face bounds.
/// Input starts at the subtable, and may include trailing GSUB bytes.
pub fn parse_single_substitution(b:&[u8],glyph_count:u32,l:SingleSubstLimits)->Result<SingleSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("single bytes"));}
    let format=word(b,0)?;
    if !matches!(format,1|2){return Err(Error::Unsupported("single format"));}
    let at=usize::from(word(b,2)?);
    let value=word(b,4)?;
    let size=if format==1{6}else{6+2*usize::from(value)};
    if at<size||at>b.len(){return Err(Error::Invalid("single coverage offset"));}
    let coverage=parse_glyph_coverage(&b[at..],glyph_count,l.max_records,l.max_glyphs)?;
    let mut replacements=Vec::with_capacity(coverage.len() as usize);
    if format==1{
        // Scan the bounded 16-bit glyph domain, preserving Coverage order.
        for id in 0..glyph_count{
            if coverage.index(id as u16).is_some(){
                let out=(id as u16).wrapping_add(value);
                if u32::from(out)>=glyph_count{return Err(Error::Invalid("single output glyph"));}
                replacements.push(out);
            }
        }
    }else{
        if u32::from(value)!=coverage.len(){return Err(Error::Invalid("single replacement count"));}
        for i in 0..usize::from(value){
            let out=word(b,6+2*i)?;
            if u32::from(out)>=glyph_count{return Err(Error::Invalid("single output glyph"));}
            replacements.push(out);
        }
    }
    Ok(SingleSubstitution{coverage,replacements})
}
