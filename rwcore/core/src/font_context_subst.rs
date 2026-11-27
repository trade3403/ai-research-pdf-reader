//! Standalone GSUB type 5 formats 1/2/3. Match only; nested execution is separate.
use crate::{Error,GlyphCoverage,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct ContextLimits{
    pub max_sets:usize,pub max_rules:usize,pub max_input:usize,
    pub max_total_input:usize,pub max_actions:usize,pub max_bytes:usize,
}
impl Default for ContextLimits{
    fn default()->Self{Self{max_sets:65535,max_rules:65535,max_input:1024,
        max_total_input:262144,max_actions:262144,max_bytes:1024*1024}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ContextAction{pub sequence_index:u16,pub lookup_index:u16}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ContextRule{pub remaining:Vec<u16>,pub actions:Vec<ContextAction>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ContextMatch<'a>{pub input_len:usize,pub actions:&'a [ContextAction]}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ContextSubstitution{coverage:GlyphCoverage,sets:Vec<Option<Vec<ContextRule>>>}
impl ContextSubstitution{
    /// Stored rule sets in font order, including null and unreachable sets.
    pub fn rule_sets(&self)->&[Option<Vec<ContextRule>>]{&self.sets}
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn rules(&self,id:u16)->Option<&[ContextRule]>{
        self.coverage.index(id).and_then(|i|self.sets.get(i as usize)?.as_deref())
    }
    /// First match in font order. Empty action list is a match, not a miss.
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{
        let (&first,tail)=glyphs.split_first()?;
        for r in self.rules(first)?{
            if tail.starts_with(&r.remaining){
                return Some(ContextMatch{input_len:r.remaining.len()+1,actions:&r.actions});
            }
        }
        None
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("context offset"))?).ok_or(Error::Invalid("context truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
fn charge(total:&mut usize,n:usize,max:usize)->Result<(),Error>{
    *total=total.checked_add(n).ok_or(Error::Limit("context budget"))?;
    if *total>max{return Err(Error::Limit("context budget"));}
    Ok(())
}
/// Parse a glyph-based context subtable. Rule-set offsets may be null;
/// rule offsets are relative to the containing set. Cross references are
/// checked against the owning lookup count. Action order is preserved.
/// No nested lookup is executed and cycles are not evaluated here.
pub fn parse_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("context bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("context format"));}
    let cov=usize::from(word(b,2)?);
    let n=usize::from(word(b,4)?);
    if n>l.max_sets{return Err(Error::Limit("context sets"));}
    let header=6+2*n;
    if cov<header||cov>b.len(){return Err(Error::Invalid("context coverage offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_sets,l.max_sets.min(65536) as u32)?;
    if coverage.len() as usize!=n{return Err(Error::Invalid("context set count"));}
    let sets=parse_context_sets(b,n,6,header,glyph_count,lookup_count,l)?;
    Ok(ContextSubstitution{coverage,sets})
}

fn parse_context_sets(b:&[u8],n:usize,offset_base:usize,header:usize,symbol_bound:u32,lookup_count:usize,l:ContextLimits)->Result<Vec<Option<Vec<ContextRule>>>,Error>{
    let mut sets=Vec::new();
        sets.try_reserve_exact(n).map_err(|_|Error::Limit("context allocation"))?;
    let(mut rules_total,mut input_total,mut actions_total)=(0,0,0);
    for i in 0..n{
        let at=usize::from(word(b,offset_base+2*i)?);
        if at==0{sets.push(None);continue;}
        if at<header{return Err(Error::Invalid("context set offset"));}
        let count=usize::from(word(b,at)?);
        charge(&mut rules_total,count,l.max_rules)?;
        let set_header=2+2*count;
        if at+set_header>b.len(){return Err(Error::Invalid("context truncated set"));}
        let mut rules=Vec::new();
        rules.try_reserve_exact(count).map_err(|_|Error::Limit("context allocation"))?;
        for j in 0..count{
            let rel=usize::from(word(b,at+2+2*j)?);
            if rel<set_header{return Err(Error::Invalid("context rule offset"));}
            let p=at+rel;
            let input=usize::from(word(b,p)?);
            let actions=usize::from(word(b,p+2)?);
            if input==0{return Err(Error::Invalid("context empty input"));}
            if input>l.max_input{return Err(Error::Limit("context input"));}
            charge(&mut input_total,input,l.max_total_input)?;
            charge(&mut actions_total,actions,l.max_actions)?;
            let start=p+4+2*(input-1);
            if start+4*actions>b.len(){return Err(Error::Invalid("context truncated rule"));}
            let mut remaining=Vec::new();
        remaining.try_reserve_exact(input-1).map_err(|_|Error::Limit("context allocation"))?;
            for k in 0..input-1{
                let id=word(b,p+4+2*k)?;
                if u32::from(id)>=symbol_bound{return Err(Error::Invalid("context input glyph"));}
                remaining.push(id);
            }
            let mut refs=Vec::new();
        refs.try_reserve_exact(actions).map_err(|_|Error::Limit("context allocation"))?;
            for k in 0..actions{
                let q=start+4*k;
                let sequence_index=word(b,q)?;
                let lookup_index=word(b,q+2)?;
                if usize::from(sequence_index)>=input||usize::from(lookup_index)>=lookup_count{
                    return Err(Error::Invalid("context action index"));
                }
                refs.push(ContextAction{sequence_index,lookup_index});
            }
            rules.push(ContextRule{remaining,actions:refs});
        }
        sets.push(Some(rules));
    }
    Ok(sets)
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassContextSubstitution{
    coverage:GlyphCoverage,classes:crate::GlyphClassDef,sets:Vec<Option<Vec<ContextRule>>>,
}
impl ClassContextSubstitution{
    pub(crate) fn class_for_match(&self,id:u16)->Option<u16>{self.classes.class(id)}
    /// Stored rule sets in font order, including null and unreachable sets.
    pub fn rule_sets(&self)->&[Option<Vec<ContextRule>>]{&self.sets}
    pub fn rules(&self,id:u16)->Option<&[ContextRule]>{
        self.coverage.index(id)?;
        self.sets.get(usize::from(self.classes.class(id)?))?.as_deref()
    }
    /// Rules store trailing class IDs (not glyph IDs). First glyph must be covered.
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{
        let (&first,tail)=glyphs.split_first()?;
        for r in self.rules(first)?{
            if tail.len()>=r.remaining.len()&&r.remaining.iter().zip(tail).all(|(&c,&g)|self.classes.class(g)==Some(c)){
                return Some(ContextMatch{input_len:r.remaining.len()+1,actions:&r.actions});
            }
        }
        None
    }
}
/// Parse class-based context format 2. Class zero includes unassigned glyphs.
/// Lookup execution remains separate.
pub fn parse_class_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ClassContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("context bytes"));}
    if word(b,0)?!=2{return Err(Error::Unsupported("class context format"));}
    let cov=usize::from(word(b,2)?);
    let cls=usize::from(word(b,4)?);
    let n=usize::from(word(b,6)?);
    if n==0{return Err(Error::Invalid("class context sets"));}
    if n>l.max_sets{return Err(Error::Limit("context sets"));}
    let header=8+2*n;
    if cov<header||cls<header||cov>b.len()||cls>b.len(){return Err(Error::Invalid("class context offset"));}
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.max_sets,65536)?;
    let classes=crate::parse_glyph_class_def(&b[cls..],glyph_count,n as u32,l.max_sets,65536)?;
    let sets=parse_context_sets(b,n,8,header,n as u32,lookup_count,l)?;
    Ok(ClassContextSubstitution{coverage,classes,sets})
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CoverageContextSubstitution{
    coverages:Vec<GlyphCoverage>,actions:Vec<ContextAction>,
}
impl CoverageContextSubstitution{
    pub fn input_len(&self)->usize{self.coverages.len()}
    pub fn coverages(&self)->&[GlyphCoverage]{&self.coverages}
    pub fn actions(&self)->&[ContextAction]{&self.actions}
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{
        if glyphs.len()<self.coverages.len(){return None;}
        if !self.coverages.iter().zip(glyphs).all(|(c,&g)|c.index(g).is_some()){return None;}
        Some(ContextMatch{input_len:self.coverages.len(),actions:&self.actions})
    }
}
/// Type 5 format 3: one coverage per input position. The max_sets budget
/// bounds cumulative coverage records and max_total_input bounds cumulative
/// expanded coverage glyphs. Shared offsets are charged per reference.
/// Actions preserve font order; no nested execution.
pub fn parse_coverage_context_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<CoverageContextSubstitution,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("context bytes"));}
    if word(b,0)?!=3{return Err(Error::Unsupported("coverage context format"));}
    let n=usize::from(word(b,2)?);
    let action_count=usize::from(word(b,4)?);
    if n==0{return Err(Error::Invalid("context empty input"));}
    if n>l.max_input||n>l.max_total_input||l.max_rules==0||action_count>l.max_actions{
        return Err(Error::Limit("coverage context budget"));
    }
    let action_start=6+2*n;
    let header=action_start+4*action_count;
    if header>b.len(){return Err(Error::Invalid("coverage context truncated header"));}
    let mut actions=Vec::new();
    actions.try_reserve_exact(action_count).map_err(|_|Error::Limit("context allocation"))?;
    for i in 0..action_count{
        let p=action_start+4*i;
        let sequence_index=word(b,p)?;
        let lookup_index=word(b,p+2)?;
        if usize::from(sequence_index)>=n||usize::from(lookup_index)>=lookup_count{
            return Err(Error::Invalid("context action index"));
        }
        actions.push(ContextAction{sequence_index,lookup_index});
    }
    let mut coverages=Vec::new();
    coverages.try_reserve_exact(n).map_err(|_|Error::Limit("context allocation"))?;
    let(mut records,mut expanded)=(0usize,0usize);
    for i in 0..n{
        let at=usize::from(word(b,6+2*i)?);
        if at<header||at>=b.len(){return Err(Error::Invalid("coverage context offset"));}
        let count=usize::from(word(b,at+2)?);
        charge(&mut records,count,l.max_sets)?;
        let remaining=l.max_total_input-expanded;
        let c=parse_glyph_coverage(&b[at..],glyph_count,count,remaining.min(65536) as u32)?;
        charge(&mut expanded,c.len() as usize,l.max_total_input)?;
        coverages.push(c);
    }
    Ok(CoverageContextSubstitution{coverages,actions})
}
