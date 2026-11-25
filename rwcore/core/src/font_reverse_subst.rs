//! Standalone GSUB Type 8 Format 1 reverse chained single substitution.
//! Contiguous logical glyph stream, no GDEF skipping or lookup scheduling.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};

#[derive(Debug,Clone,Copy)]
pub struct ReverseSubstLimits{
    pub max_bytes:usize,
    pub max_positions:usize,
    pub max_records:usize,
    pub max_covered:usize,
    pub max_replacements:usize,
}
impl Default for ReverseSubstLimits{
    fn default()->Self{Self{max_bytes:1024*1024,max_positions:1024,max_records:65535,max_covered:262144,max_replacements:65535}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ReverseSubstitution{
    input:GlyphCoverage,
    backtrack:Vec<GlyphCoverage>,
    lookahead:Vec<GlyphCoverage>,
    replacements:Vec<u16>,
    glyph_count:u32,
}
impl ReverseSubstitution{
    pub fn input(&self)->&GlyphCoverage{&self.input}
    pub fn backtrack(&self)->&[GlyphCoverage]{&self.backtrack}
    pub fn lookahead(&self)->&[GlyphCoverage]{&self.lookahead}
    pub fn replacements(&self)->&[u16]{&self.replacements}
    /// Nearest preceding glyph first; following glyphs in forward order.
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<u16>{
        let id=*glyphs.get(position)?;
        let index=self.input.index(id)? as usize;
        let before=&glyphs[..position];
        let after=&glyphs[position+1..];
        if before.len()<self.backtrack.len()||after.len()<self.lookahead.len(){return None;}
        if !self.backtrack.iter().zip(before.iter().rev()).all(|(c,&g)|c.index(g).is_some())||
            !self.lookahead.iter().zip(after).all(|(c,&g)|c.index(g).is_some()){return None;}
        self.replacements.get(index).copied()
    }
    /// One reverse pass on a contiguous logical run, visiting each position
    /// once. Earlier positions see already-substituted later positions.
    /// Return the number of matched positions (including identity mappings).
    /// Validate the full run and conservative coverage-probe budget before
    /// mutation. Errors leave the run unchanged; length never changes.
    pub fn replace_reverse(&self,glyphs:&mut [u16],max_glyphs:usize,max_probes:usize)->Result<usize,Error>{
        if glyphs.len()>max_glyphs{return Err(Error::Limit("reverse run length"));}
        let probes=glyphs.len().checked_mul(1+self.backtrack.len()+self.lookahead.len()).ok_or(Error::Limit("reverse probes"))?;
        if probes>max_probes{return Err(Error::Limit("reverse probes"));}
        if glyphs.iter().any(|&g|u32::from(g)>=self.glyph_count){return Err(Error::Invalid("reverse run glyph"));}
        let mut matched=0;
        for i in (0..glyphs.len()).rev(){
            if let Some(out)=self.match_at(glyphs,i){glyphs[i]=out;matched+=1;}
        }
        Ok(matched)
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let s=b.get(p..p.checked_add(2).ok_or(Error::Limit("reverse offset"))?).ok_or(Error::Invalid("reverse truncated"))?;
    Ok(u16::from_be_bytes([s[0],s[1]]))
}
fn take(b:&[u8],p:&mut usize)->Result<u16,Error>{let v=word(b,*p)?;*p+=2;Ok(v)}
fn charge(total:&mut usize,n:usize,max:usize)->Result<(),Error>{
    *total=total.checked_add(n).ok_or(Error::Limit("reverse budget"))?;
    if *total>max{return Err(Error::Limit("reverse budget"));}
    Ok(())
}
fn offsets(b:&[u8],p:&mut usize,span:&mut usize,l:ReverseSubstLimits)->Result<Vec<u16>,Error>{
    let n=usize::from(take(b,p)?);
    charge(span,n,l.max_positions)?;
    if b.len().saturating_sub(*p)/2<n{return Err(Error::Invalid("reverse offsets truncated"));}
    (0..n).map(|_|take(b,p)).collect()
}
/// Offsets are relative to this subtable; all payloads must follow the
/// complete header and substitute array. Budgets charge aliased coverages
/// per reference. max_positions includes the single input position.
pub fn parse_reverse_substitution(b:&[u8],glyph_count:u32,l:ReverseSubstLimits)->Result<ReverseSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("reverse bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("reverse format"));}
    let cov=word(b,2)?;
    let mut p=4;
    let mut span=0;
    charge(&mut span,1,l.max_positions)?;
    let back=offsets(b,&mut p,&mut span,l)?;
    let ahead=offsets(b,&mut p,&mut span,l)?;
    let n=usize::from(take(b,&mut p)?);
    if n>l.max_replacements{return Err(Error::Limit("reverse replacements"));}
    if b.len().saturating_sub(p)/2<n{return Err(Error::Invalid("reverse substitutes truncated"));}
    let mut replacements=Vec::with_capacity(n);
    for _ in 0..n{
        let id=take(b,&mut p)?;
        if u32::from(id)>=glyph_count{return Err(Error::Invalid("reverse output glyph"));}
        replacements.push(id);
    }
    let header=p;
    let(mut records,mut covered)=(0usize,0usize);
    let mut decode=|offset:u16|->Result<GlyphCoverage,Error>{
        let at=usize::from(offset);
        if at<header||at>=b.len(){return Err(Error::Invalid("reverse coverage offset"));}
        let count=usize::from(word(b,at+2)?);
        charge(&mut records,count,l.max_records)?;
        let c=parse_glyph_coverage(&b[at..],glyph_count,count,(l.max_covered-covered).min(65536) as u32)?;
        charge(&mut covered,c.len() as usize,l.max_covered)?;
        Ok(c)
    };
    let input=decode(cov)?;
    if input.len() as usize!=n{return Err(Error::Invalid("reverse substitute count"));}
    let backtrack=back.into_iter().map(&mut decode).collect::<Result<Vec<_>,_>>()?;
    let lookahead=ahead.into_iter().map(&mut decode).collect::<Result<Vec<_>,_>>()?;
    Ok(ReverseSubstitution{input,backtrack,lookahead,replacements,glyph_count})
}
