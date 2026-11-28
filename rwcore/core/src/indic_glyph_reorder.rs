//! Project a restricted Devanagari prebase plan onto post-GSUB occurrences.
//! Caller supplies the completed logical GSUB ledger from the same source
//! run and explicitly selects this policy. No font feature success is
//! inferred. Actual pipeline scheduling and GPOS remain separate.
use crate::*;
#[derive(Debug,Clone,Copy)]
pub struct IndicGlyphReorderLimits{
    pub plan:IndicReorderLimits,pub max_glyphs:usize,pub max_work:usize,
}
impl Default for IndicGlyphReorderLimits{
    fn default()->Self{Self{plan:Default::default(),max_glyphs:65536,max_work:4194304}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicGlyphReorderResult{
    pub plan:IndicReorderPlan,
    /// New physical glyph occurrence -> old physical occurrence.
    pub order:Vec<usize>,
    pub inverse:Vec<usize>,
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("Indic glyph reorder allocation"))
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("Indic glyph reorder work"))}
fn mul(a:usize,b:usize)->Result<usize,Error>{a.checked_mul(b).ok_or(Error::Limit("Indic glyph reorder work"))}
/// Apply only simple single-base prebase candidates. Keep expansion order
/// within a moved matra group and within the remaining glyphs. A glyph
/// combining the matra with another scalar, a cross-cluster contraction,
/// a missing source, or a nonmonotonic cluster stream is rejected.
/// Complex/deferred/broken clusters reject the whole request rather than
/// silently emit a partially shaped result. Other clusters remain stable.
/// All validation and allocation precede the atomic ledger edit.
pub fn reorder_devanagari_glyphs(text:&str,source:&SourceRun,ledger:&mut ProvenanceLedger,
    p:&IndicProperties,scripts:&UnicodeScriptProperties,l:IndicGlyphReorderLimits)->Result<IndicGlyphReorderResult,Error>{
    let plan=plan_devanagari_reorder(text,p,scripts,l.plan)?;
    let n=ledger.glyphs().len();let ns=source.scalars().len();
    if n>l.max_glyphs{return Err(Error::Limit("Indic glyph reorder glyphs"));}
    let cost=add(add(mul(n,6)?,mul(ns,3)?)?,mul(ledger.reference_count(),2)?)?;
    if cost>l.max_work{return Err(Error::Limit("Indic glyph reorder work"));}
    if source.byte_len()!=text.len()||ns!=plan.logical.scalars.len()||ledger.scalar_count()!=ns||
        source.scalars().iter().zip(&plan.logical.scalars).any(|(a,b)|
            (a.character,a.byte_start,a.byte_end)!=(b.character,b.byte_start,b.byte_end)){
        return Err(Error::Invalid("Indic glyph reorder source identity"));
    }
    let mut owner=Vec::new();let mut seen=Vec::new();let mut group=Vec::new();
    let mut order=Vec::new();let mut inverse=Vec::new();
    reserve(&mut owner,ns)?;reserve(&mut seen,ns)?;reserve(&mut group,n)?;
    reserve(&mut order,n)?;reserve(&mut inverse,n)?;
    for (i,c) in plan.clusters.iter().enumerate(){
        if matches!(c.status,IndicReorderStatus::NeedsFont|IndicReorderStatus::Unsupported|IndicReorderStatus::Broken){
            return Err(Error::Unsupported("Indic glyph reorder deferred cluster"));
        }
        owner.resize(c.scalar_end,i);
    }
    seen.resize(ns,false);
    let mut previous=None;
    for g in ledger.glyphs(){
        let first=*g.sources().first().ok_or(Error::Invalid("Indic glyph reorder empty sources"))?;
        let ci=*owner.get(first).ok_or(Error::Invalid("Indic glyph reorder source index"))?;
        if previous.is_some_and(|old|old>ci){return Err(Error::Invalid("Indic glyph reorder cluster order"));}
        previous=Some(ci);
        let c=&plan.clusters[ci];let mut matra=false;let mut other=false;
        for &s in g.sources(){
            if owner.get(s)!=Some(&ci){return Err(Error::Unsupported("Indic glyph reorder cross-cluster glyph"));}
            seen[s]=true;
            if c.status==IndicReorderStatus::SimplePrebaseCandidate&&c.prebase_matra==Some(s){matra=true;}
            else{other=true;}
        }
        if matra&&other{return Err(Error::Unsupported("Indic glyph reorder mixed matra provenance"));}
        group.push((ci,matra));
    }
    if seen.contains(&false){return Err(Error::Invalid("Indic glyph reorder missing source"));}
    let mut at=0;
    while at<n{
        let start=at;let ci=group[at].0;at+=1;
        while at<n&&group[at].0==ci{at+=1;}
        order.extend((start..at).filter(|&i|group[i].1));
        order.extend((start..at).filter(|&i|!group[i].1));
    }
    inverse.resize(n,0);
    for (i,&old) in order.iter().enumerate(){inverse[old]=i;}
    ledger.reorder(&order)?;
    Ok(IndicGlyphReorderResult{plan,order,inverse})
}
