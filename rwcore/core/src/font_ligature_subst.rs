//! Standalone GSUB type 4 format 1, contiguous glyphs only.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct LigatureLimits{
    pub max_sets:usize,pub max_rules:usize,pub max_components:usize,
    pub max_total_components:usize,pub max_bytes:usize,
}
impl Default for LigatureLimits{
    fn default()->Self{Self{max_sets:65535,max_rules:65535,max_components:1024,
        max_total_components:262144,max_bytes:1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct LigatureRule{pub output:u16,pub remaining:Vec<u16>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct LigatureMatch{pub output:u16,pub consumed:usize}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct LigatureSubstitution{coverage:GlyphCoverage,sets:Vec<Vec<LigatureRule>>}
impl LigatureSubstitution{
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn rules(&self,id:u16)->Option<&[LigatureRule]>{
        self.coverage.index(id).and_then(|i|self.sets.get(i as usize).map(Vec::as_slice))
    }
    /// Match the first rule in font order, not necessarily the longest.
    pub fn match_prefix(&self,glyphs:&[u16])->Option<LigatureMatch>{
        let (&first,tail)=glyphs.split_first()?;
        for r in self.rules(first)?{
            if tail.starts_with(&r.remaining){
                return Some(LigatureMatch{output:r.output,consumed:r.remaining.len()+1});
            }
        }
        None
    }
    /// Replace a contiguous match without reprocessing the output.
    /// Invalid positions and misses do not alter the run.
    pub fn replace_at(&self,run:&mut Vec<u16>,position:usize)->Result<Option<LigatureMatch>,Error>{
        if position>=run.len(){return Err(Error::Invalid("ligature run position"));}
        let Some(m)=self.match_prefix(&run[position..]) else{return Ok(None)};
        run[position]=m.output;
        run.drain(position+1..position+m.consumed);
        Ok(Some(m))
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("ligature offset"))?).ok_or(Error::Invalid("ligature truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
/// Decode type 4 format 1. Set offsets are relative to the subtable;
/// ligature offsets are relative to their set. Shared offsets are charged
/// per reference. This contiguous-matching subset requires at least two
/// input components per rule. GDEF filtering and cluster merging are separate.
pub fn parse_ligature_substitution(b:&[u8],glyph_count:u32,l:LigatureLimits)->Result<LigatureSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("ligature bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("ligature format"));}
    let cov=usize::from(word(b,2)?);
    let n=usize::from(word(b,4)?);
    if n>l.max_sets{return Err(Error::Limit("ligature sets"));}
    let header=6+2*n;
    if cov<header||cov>b.len(){return Err(Error::Invalid("ligature coverage offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_sets,l.max_sets.min(65536) as u32)?;
    if coverage.len() as usize!=n{return Err(Error::Invalid("ligature set count"));}
    let mut sets=Vec::with_capacity(n);
    let mut total_rules=0usize;
    let mut total_components=0usize;
    for i in 0..n{
        let at=usize::from(word(b,6+2*i)?);
        if at<header{return Err(Error::Invalid("ligature set offset"));}
        let count=usize::from(word(b,at)?);
        if count==0{return Err(Error::Invalid("ligature empty set"));}
        total_rules=total_rules.checked_add(count).ok_or(Error::Limit("ligature rules"))?;
        if total_rules>l.max_rules{return Err(Error::Limit("ligature rules"));}
        let set_header=2+2*count;
        if at+set_header>b.len(){return Err(Error::Invalid("ligature truncated set"));}
        let mut rules=Vec::with_capacity(count);
        for j in 0..count{
            let rel=usize::from(word(b,at+2+2*j)?);
            if rel<set_header{return Err(Error::Invalid("ligature rule offset"));}
            let p=at+rel;
            let output=word(b,p)?;
            let components=usize::from(word(b,p+2)?);
            if u32::from(output)>=glyph_count||components<2{return Err(Error::Invalid("ligature output or component count"));}
            total_components=total_components.checked_add(components).ok_or(Error::Limit("ligature components"))?;
            if components>l.max_components||total_components>l.max_total_components{return Err(Error::Limit("ligature component budget"));}
            if p+4+2*(components-1)>b.len(){return Err(Error::Invalid("ligature truncated components"));}
            let mut remaining=Vec::with_capacity(components-1);
            for k in 0..components-1{
                let id=word(b,p+4+2*k)?;
                if u32::from(id)>=glyph_count{return Err(Error::Invalid("ligature input glyph"));}
                remaining.push(id);
            }
            rules.push(LigatureRule{output,remaining});
        }
        sets.push(rules);
    }
    Ok(LigatureSubstitution{coverage,sets})
}
