//! Standalone GSUB type 6 format 1 (glyph-based chained context).
//! Matches a contiguous glyph stream without executing nested lookups.
use crate::{ContextAction,ContextLimits,ContextMatch,Error,GlyphCoverage,parse_glyph_coverage};

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ChainContextRule {
    /// Closest preceding glyph first (OpenType storage order).
    pub backtrack:Vec<u16>,
    /// Input after the first covered glyph.
    pub remaining:Vec<u16>,
    /// Following glyphs in forward order, after the complete input.
    pub lookahead:Vec<u16>,
    pub actions:Vec<ContextAction>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ChainContextSubstitution {
    coverage:GlyphCoverage,
    sets:Vec<Option<Vec<ChainContextRule>>>,
}
impl ChainContextSubstitution {
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn rules(&self,id:u16)->Option<&[ChainContextRule]>{
        self.coverage.index(id).and_then(|i|self.sets.get(i as usize)?.as_deref())
    }
    /// First matching rule in font order. Position is the first input glyph.
    /// Backtrack is compared in reverse stream order, lookahead in forward order.
    /// Empty action lists still match. No skipping or GDEF filtering.
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<ContextMatch<'_>>{
        let first=*glyphs.get(position)?;
        let before=&glyphs[..position];
        let tail=&glyphs[position+1..];
        for r in self.rules(first)? {
            if before.len()<r.backtrack.len() ||
                !r.backtrack.iter().zip(before.iter().rev()).all(|(a,b)|a==b) ||
                !tail.starts_with(&r.remaining) {continue;}
            if tail[r.remaining.len()..].starts_with(&r.lookahead) {
                return Some(ContextMatch{input_len:r.remaining.len()+1,actions:&r.actions});
            }
        }
        None
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("chain offset"))?)
        .ok_or(Error::Invalid("chain truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
fn take(b:&[u8],p:&mut usize)->Result<u16,Error>{
    let v=word(b,*p)?;
    *p+=2;
    Ok(v)
}
fn charge(total:&mut usize,n:usize,max:usize)->Result<(),Error>{
    *total=total.checked_add(n).ok_or(Error::Limit("chain budget"))?;
    if *total>max{return Err(Error::Limit("chain budget"));}
    Ok(())
}
fn read_glyphs(b:&[u8],p:&mut usize,n:usize,bound:u32)->Result<Vec<u16>,Error>{
    if b.len().saturating_sub(*p)/2<n{return Err(Error::Invalid("chain truncated glyphs"));}
    let mut ids=Vec::with_capacity(n);
    for _ in 0..n{
        let g=take(b,p)?;
        if u32::from(g)>=bound{return Err(Error::Invalid("chain glyph"));}
        ids.push(g);
    }
    Ok(ids)
}
/// Parse format 1. ContextLimits::max_input bounds the complete per-rule
/// backtrack+input+lookahead length; max_total_input charges all these glyph
/// slots cumulatively (including the implicit first input). Shared offsets
/// are charged per reference. Action sequence indices address input only.
pub fn parse_chain_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ChainContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("chain bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("chain context format"));}
    let cov=usize::from(word(b,2)?);
    let n=usize::from(word(b,4)?);
    if n>l.max_sets{return Err(Error::Limit("chain sets"));}
    let header=6+2*n;
    if header>b.len()||cov<header||cov>=b.len(){return Err(Error::Invalid("chain coverage offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_sets,l.max_sets.min(65536) as u32)?;
    if coverage.len() as usize!=n{return Err(Error::Invalid("chain set count"));}
    let mut sets=Vec::with_capacity(n);
    let(mut rules_total,mut glyph_total,mut action_total)=(0,0,0);
    for i in 0..n{
        let at=usize::from(word(b,6+2*i)?);
        if at==0{sets.push(None);continue;}
        if at<header{return Err(Error::Invalid("chain set offset"));}
        let count=usize::from(word(b,at)?);
        charge(&mut rules_total,count,l.max_rules)?;
        let sh=2+2*count;
        if at+sh>b.len(){return Err(Error::Invalid("chain truncated set"));}
        let mut rules=Vec::with_capacity(count);
        for j in 0..count{
            let rel=usize::from(word(b,at+2+2*j)?);
            if rel<sh{return Err(Error::Invalid("chain rule offset"));}
            let mut p=at+rel;
            let back=usize::from(take(b,&mut p)?);
            let mut span=0;
            charge(&mut span,back,l.max_input)?;
            charge(&mut glyph_total,back,l.max_total_input)?;
            let backtrack=read_glyphs(b,&mut p,back,glyph_count)?;
            let input=usize::from(take(b,&mut p)?);
            if input==0{return Err(Error::Invalid("chain empty input"));}
            charge(&mut span,input,l.max_input)?;
            charge(&mut glyph_total,input,l.max_total_input)?;
            let remaining=read_glyphs(b,&mut p,input-1,glyph_count)?;
            let ahead=usize::from(take(b,&mut p)?);
            charge(&mut span,ahead,l.max_input)?;
            charge(&mut glyph_total,ahead,l.max_total_input)?;
            let lookahead=read_glyphs(b,&mut p,ahead,glyph_count)?;
            let count=usize::from(take(b,&mut p)?);
            charge(&mut action_total,count,l.max_actions)?;
            if b.len().saturating_sub(p)/4<count{return Err(Error::Invalid("chain truncated actions"));}
            let mut actions=Vec::with_capacity(count);
            for _ in 0..count{
                let sequence_index=take(b,&mut p)?;
                let lookup_index=take(b,&mut p)?;
                if usize::from(sequence_index)>=input||usize::from(lookup_index)>=lookup_count{
                    return Err(Error::Invalid("chain action index"));
                }
                actions.push(ContextAction{sequence_index,lookup_index});
            }
            rules.push(ChainContextRule{backtrack,remaining,lookahead,actions});
        }
        sets.push(Some(rules));
    }
    Ok(ChainContextSubstitution{coverage,sets})
}
