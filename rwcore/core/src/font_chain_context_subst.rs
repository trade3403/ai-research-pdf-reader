//! Standalone GSUB type 6 formats 1/2/3 (glyph/class/coverage-based chained context).
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
    /// Stored rule sets in font order, including null and unreachable sets.
    pub fn rule_sets(&self)->&[Option<Vec<ChainContextRule>>]{&self.sets}
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
    let mut ids=Vec::new();
    ids.try_reserve_exact(n).map_err(|_|Error::Limit("chain allocation"))?;
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
    let sets=parse_chain_sets(b,n,6,header,[glyph_count;3],lookup_count,l)?;
    Ok(ChainContextSubstitution{coverage,sets})
}

fn parse_chain_sets(b:&[u8],n:usize,offset_base:usize,header:usize,bounds:[u32;3],lookup_count:usize,l:ContextLimits)->Result<Vec<Option<Vec<ChainContextRule>>>,Error>{
    let mut sets=Vec::new();
    sets.try_reserve_exact(n).map_err(|_|Error::Limit("chain allocation"))?;
    let(mut rules_total,mut glyph_total,mut action_total)=(0,0,0);
    for i in 0..n{
        let at=usize::from(word(b,offset_base+2*i)?);
        if at==0{sets.push(None);continue;}
        if at<header{return Err(Error::Invalid("chain set offset"));}
        let count=usize::from(word(b,at)?);
        charge(&mut rules_total,count,l.max_rules)?;
        let sh=2+2*count;
        if at+sh>b.len(){return Err(Error::Invalid("chain truncated set"));}
        let mut rules=Vec::new();
    rules.try_reserve_exact(count).map_err(|_|Error::Limit("chain allocation"))?;
        for j in 0..count{
            let rel=usize::from(word(b,at+2+2*j)?);
            if rel<sh{return Err(Error::Invalid("chain rule offset"));}
            let mut p=at+rel;
            let back=usize::from(take(b,&mut p)?);
            let mut span=0;
            charge(&mut span,back,l.max_input)?;
            charge(&mut glyph_total,back,l.max_total_input)?;
            let backtrack=read_glyphs(b,&mut p,back,bounds[0])?;
            let input=usize::from(take(b,&mut p)?);
            if input==0{return Err(Error::Invalid("chain empty input"));}
            charge(&mut span,input,l.max_input)?;
            charge(&mut glyph_total,input,l.max_total_input)?;
            let remaining=read_glyphs(b,&mut p,input-1,bounds[1])?;
            let ahead=usize::from(take(b,&mut p)?);
            charge(&mut span,ahead,l.max_input)?;
            charge(&mut glyph_total,ahead,l.max_total_input)?;
            let lookahead=read_glyphs(b,&mut p,ahead,bounds[2])?;
            let count=usize::from(take(b,&mut p)?);
            charge(&mut action_total,count,l.max_actions)?;
            if b.len().saturating_sub(p)/4<count{return Err(Error::Invalid("chain truncated actions"));}
            let mut actions=Vec::new();
    actions.try_reserve_exact(count).map_err(|_|Error::Limit("chain allocation"))?;
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
    Ok(sets)
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassChainContextSubstitution {
    coverage:GlyphCoverage,
    backtrack:crate::GlyphClassDef,
    input:crate::GlyphClassDef,
    lookahead:crate::GlyphClassDef,
    sets:Vec<Option<Vec<ChainContextRule>>>,
}
impl ClassChainContextSubstitution {
    pub(crate) fn backtrack_class(&self,g:u16)->Option<u16>{self.backtrack.class(g)}
    pub(crate) fn input_class(&self,g:u16)->Option<u16>{self.input.class(g)}
    pub(crate) fn lookahead_class(&self,g:u16)->Option<u16>{self.lookahead.class(g)}
    /// Stored rule sets in font order, including null and unreachable sets.
    pub fn rule_sets(&self)->&[Option<Vec<ChainContextRule>>]{&self.sets}
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    /// Rules contain class IDs in their respective independent class domains.
    pub fn rules(&self,id:u16)->Option<&[ChainContextRule]>{
        self.coverage.index(id)?;
        self.sets.get(usize::from(self.input.class(id)?))?.as_deref()
    }
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<ContextMatch<'_>>{
        let first=*glyphs.get(position)?;
        let before=&glyphs[..position];
        let tail=&glyphs[position+1..];
        for r in self.rules(first)?{
            if before.len()<r.backtrack.len()||tail.len()<r.remaining.len()||
                !r.backtrack.iter().zip(before.iter().rev()).all(|(&c,&g)|self.backtrack.class(g)==Some(c))||
                !r.remaining.iter().zip(tail).all(|(&c,&g)|self.input.class(g)==Some(c)){continue;}
            let ahead=&tail[r.remaining.len()..];
            if ahead.len()>=r.lookahead.len()&&r.lookahead.iter().zip(ahead).all(|(&c,&g)|self.lookahead.class(g)==Some(c)){
                return Some(ContextMatch{input_len:r.remaining.len()+1,actions:&r.actions});
            }
        }
        None
    }
}
/// Format 2: independent backtrack/input/lookahead ClassDefs; input classes
/// index nullable sets. Backtrack and lookahead class IDs are full uint16
/// domains (a class with no members is simply unmatchable). Class zero is
/// implicit only for in-face glyphs. Matching is contiguous and actions
/// are returned, not executed. Shared rule offsets consume the same budgets
/// as format 1; each ClassDef is bounded separately by max_sets records.
pub fn parse_class_chain_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ClassChainContextSubstitution,Error>{
    decode_class_chain_context(b,glyph_count,lookup_count,l,false)
}
pub(crate) fn decode_class_chain_context(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits,allow_trailing_only:bool)->Result<ClassChainContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("chain bytes"));}
    if word(b,0)?!=2{return Err(Error::Unsupported("class chain format"));}
    let cov=usize::from(word(b,2)?);
    let back=usize::from(word(b,4)?);
    let input=usize::from(word(b,6)?);
    let ahead=usize::from(word(b,8)?);
    let n=usize::from(word(b,10)?);
    if n==0{return Err(Error::Invalid("class chain sets"));}
    if n>l.max_sets{return Err(Error::Limit("chain sets"));}
    let header=12+2*n;
    if header>b.len()||[cov,back,input,ahead].iter().any(|&p|p<header||p>=b.len()){
        return Err(Error::Invalid("class chain offset"));
    }
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_sets,65536)?;
    let backtrack=crate::parse_glyph_class_def(&b[back..],glyph_count,65536,l.max_sets,65536)?;
    let input=crate::parse_glyph_class_def(&b[input..],glyph_count,if allow_trailing_only{65536}else{n as u32},l.max_sets,65536)?;
    let lookahead=crate::parse_glyph_class_def(&b[ahead..],glyph_count,65536,l.max_sets,65536)?;
    let mut bound=n as u32;
    let mut present=Vec::new();
    if allow_trailing_only{
        present.try_reserve_exact(65536).map_err(|_|Error::Limit("chain class allocation"))?;
        present.resize(65536,false);
        for g in 0..glyph_count.min(65536){
            let c=input.class(g as u16).ok_or(Error::Invalid("chain input class domain"))?;
            present[usize::from(c)]=true;
            bound=bound.max(u32::from(c)+1);
            if coverage.index(g as u16).is_some()&&usize::from(c)>=n{
                return Err(Error::Invalid("chain initial class set"));
            }
        }
    }
    let sets=parse_chain_sets(b,n,12,header,[65536,bound,65536],lookup_count,l)?;
    if allow_trailing_only{
        for rs in sets.iter().flatten(){for r in rs{for &c in &r.remaining{
            if usize::from(c)>=n&&!present[usize::from(c)]{return Err(Error::Invalid("chain trailing class undefined"));}
        }}}
    }
    Ok(ClassChainContextSubstitution{coverage,backtrack,input,lookahead,sets})
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CoverageChainContextSubstitution {
    backtrack:Vec<GlyphCoverage>,
    input:Vec<GlyphCoverage>,
    lookahead:Vec<GlyphCoverage>,
    actions:Vec<ContextAction>,
}
impl CoverageChainContextSubstitution {
    pub fn backtrack(&self)->&[GlyphCoverage]{&self.backtrack}
    pub fn input(&self)->&[GlyphCoverage]{&self.input}
    pub fn lookahead(&self)->&[GlyphCoverage]{&self.lookahead}
    pub fn actions(&self)->&[ContextAction]{&self.actions}
    pub fn input_len(&self)->usize{self.input.len()}
    /// Backtrack coverages are nearest-first; input and lookahead are forward.
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<ContextMatch<'_>>{
        let tail=glyphs.get(position..)?;
        let before=&glyphs[..position];
        if before.len()<self.backtrack.len()||tail.len()<self.input.len(){return None;}
        let (input,ahead)=tail.split_at(self.input.len());
        if ahead.len()<self.lookahead.len(){return None;}
        if !self.backtrack.iter().zip(before.iter().rev()).all(|(c,&g)|c.index(g).is_some())||
            !self.input.iter().zip(input).all(|(c,&g)|c.index(g).is_some())||
            !self.lookahead.iter().zip(ahead).all(|(c,&g)|c.index(g).is_some()){return None;}
        Some(ContextMatch{input_len:self.input.len(),actions:&self.actions})
    }
}
fn read_coverage_offsets(b:&[u8],p:&mut usize,span:&mut usize,l:ContextLimits)->Result<Vec<u16>,Error>{
    let n=usize::from(take(b,p)?);
    charge(span,n,l.max_input)?;
    if *span>l.max_total_input{return Err(Error::Limit("chain coverage positions"));}
    read_glyphs(b,p,n,65536)
}
/// Format 3: all three segments contain per-position Coverage offsets relative
/// to the subtable. The complete header (including actions) must precede the
/// coverage payloads. Shared offsets are charged on each reference. max_sets
/// bounds cumulative coverage records; max_total_input bounds both positions
/// and cumulative expanded coverage glyphs independently. No nested execution.
pub fn parse_coverage_chain_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<CoverageChainContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("chain bytes"));}
    if word(b,0)?!=3{return Err(Error::Unsupported("coverage chain format"));}
    if l.max_rules==0{return Err(Error::Limit("chain rules"));}
    let mut p=2;
    let mut span=0;
    let back=read_coverage_offsets(b,&mut p,&mut span,l)?;
    let input=read_coverage_offsets(b,&mut p,&mut span,l)?;
    if input.is_empty(){return Err(Error::Invalid("chain empty input"));}
    let ahead=read_coverage_offsets(b,&mut p,&mut span,l)?;
    let n=usize::from(take(b,&mut p)?);
    if n>l.max_actions{return Err(Error::Limit("chain actions"));}
    if b.len().saturating_sub(p)/4<n{return Err(Error::Invalid("chain truncated actions"));}
    let mut actions=Vec::new();
    actions.try_reserve_exact(n).map_err(|_|Error::Limit("chain allocation"))?;
    for _ in 0..n{
        let sequence_index=take(b,&mut p)?;
        let lookup_index=take(b,&mut p)?;
        if usize::from(sequence_index)>=input.len()||usize::from(lookup_index)>=lookup_count{
            return Err(Error::Invalid("chain action index"));
        }
        actions.push(ContextAction{sequence_index,lookup_index});
    }
    let header=p;
    let(mut records,mut expanded)=(0usize,0usize);
    let mut decode=|offsets:Vec<u16>|->Result<Vec<GlyphCoverage>,Error>{
        let mut result=Vec::new();
    result.try_reserve_exact(offsets.len()).map_err(|_|Error::Limit("chain allocation"))?;
        for offset in offsets{
            let at=usize::from(offset);
            if at<header||at>=b.len(){return Err(Error::Invalid("chain coverage offset"));}
            let count=usize::from(word(b,at+2)?);
            charge(&mut records,count,l.max_sets)?;
            let remaining=l.max_total_input-expanded;
            let c=parse_glyph_coverage(&b[at..],glyph_count,count,remaining.min(65536) as u32)?;
            charge(&mut expanded,c.len() as usize,l.max_total_input)?;
            result.push(c);
        }
        Ok(result)
    };
    let backtrack=decode(back)?;
    let input=decode(input)?;
    let lookahead=decode(ahead)?;
    Ok(CoverageChainContextSubstitution{backtrack,input,lookahead,actions})
}
