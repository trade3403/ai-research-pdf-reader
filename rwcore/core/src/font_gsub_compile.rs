//! Compile selected GSUB lookup targets without executing substitutions.
//! Nested references remain indices into the full source LookupList.
use crate::{Error,Gdef,GsubLimits,GsubLayout,GsubPlan,ExtensionLimits,ExtensionTarget,
    LookupFilter,parse_gsub_layout,parse_extension_substitution,parse_direct_gsub_target};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Clone,Copy)]
pub struct GsubCompileLimits{
    pub layout:GsubLimits,
    pub target:ExtensionLimits,
    pub max_lookups:usize,
    pub max_subtables_per_lookup:usize,
    pub max_total_subtables:usize,
    /// Conservative work charge: sum of target suffix lengths, plus 8
    /// bytes per extension wrapper. Aliases charged per reference.
    /// This is NOT the exact owned memory size or a certified table extent.
    pub max_total_target_bytes:usize,
    pub max_plan_refs:usize,
}
impl Default for GsubCompileLimits{
    fn default()->Self{Self{layout:Default::default(),target:Default::default(),
        max_lookups:16384,max_subtables_per_lookup:4096,max_total_subtables:16384,
        max_total_target_bytes:512*1024*1024,max_plan_refs:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CompiledGsubSubtable{
    source_offset:usize,
    target_offset:usize,
    target:ExtensionTarget,
}
impl CompiledGsubSubtable{
    pub fn source_offset(&self)->usize{self.source_offset}
    pub fn target_offset(&self)->usize{self.target_offset}
    pub fn target(&self)->&ExtensionTarget{&self.target}
}
#[derive(Debug,Clone)]
pub struct CompiledGsubLookup<'a>{
    index:u16,
    declared_kind:u16,
    effective_kind:u16,
    filter:LookupFilter<'a>,
    subtables:Vec<CompiledGsubSubtable>,
}
impl<'a> CompiledGsubLookup<'a>{
    pub fn index(&self)->u16{self.index}
    pub fn declared_kind(&self)->u16{self.declared_kind}
    pub fn effective_kind(&self)->u16{self.effective_kind}
    pub fn filter(&self)->&LookupFilter<'a>{&self.filter}
    pub fn subtables(&self)->&[CompiledGsubSubtable]{&self.subtables}
}
fn charge(budget:&mut usize,n:usize,what:&'static str)->Result<(),Error>{
    *budget=budget.checked_sub(n).ok_or(Error::Limit(what))?;
    Ok(())
}
fn validate_domain(glyph_count:u32,gdef:Option<&Gdef>)->Result<(),Error>{
    if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("gsub compile glyph domain"));}
    if gdef.is_some_and(|g|g.glyph_count()!=glyph_count){return Err(Error::Invalid("gsub compile GDEF domain"));}
    Ok(())
}
fn compile<'a>(b:&[u8],layout:&GsubLayout,indices:&[u16],glyph_count:u32,
    gdef:Option<&'a Gdef>,l:GsubCompileLimits)->Result<Vec<CompiledGsubLookup<'a>>,Error>{
    validate_domain(glyph_count,gdef)?;
    if indices.len()>l.max_lookups{return Err(Error::Limit("gsub compile lookups"));}
    if indices.windows(2).any(|w|w[0]>=w[1]){return Err(Error::Invalid("gsub compile lookup order"));}
    let mut count_budget=l.max_total_subtables;
    let mut byte_budget=l.max_total_target_bytes;
    let mut out=Vec::new();
    for &index in indices{
        let lookup=layout.lookups.get(usize::from(index)).ok_or(Error::Invalid("gsub compile lookup index"))?;
        let filter=LookupFilter::from_gsub_lookup(lookup,glyph_count,gdef)?;
        if lookup.subtables.is_empty(){return Err(Error::Invalid("gsub compile empty lookup"));}
        if lookup.subtables.len()>l.max_subtables_per_lookup{return Err(Error::Limit("gsub compile per lookup subtables"));}
        charge(&mut count_budget,lookup.subtables.len(),"gsub compile subtables")?;
        let mut subtables=Vec::new();
        let mut effective=None;
        for &at in &lookup.subtables{
            let source=b.get(at..).ok_or(Error::Invalid("gsub compile source offset"))?;
            let (kind,target_at,bytes)=if lookup.kind==7{
                let ext=parse_extension_substitution(source,l.target.max_bytes)?;
                charge(&mut byte_budget,8,"gsub compile target bytes")?;
                (ext.kind(),at.checked_add(ext.offset()).ok_or(Error::Limit("gsub compile target offset"))?,ext.target())
            }else{(lookup.kind,at,source)};
            if effective.is_some_and(|k|k!=kind){return Err(Error::Invalid("gsub compile mixed targets"));}
            effective=Some(kind);
            charge(&mut byte_budget,bytes.len(),"gsub compile target bytes")?;
            let target=parse_direct_gsub_target(kind,bytes,glyph_count,layout.lookups.len(),l.target)?;
            subtables.push(CompiledGsubSubtable{source_offset:at,target_offset:target_at,target});
        }
        out.push(CompiledGsubLookup{index,declared_kind:lookup.kind,
            effective_kind:effective.ok_or(Error::Invalid("gsub compile empty lookup"))?,filter,subtables});
    }
    Ok(out)
}
/// Compile explicitly selected ascending unique lookup indices from the
/// supplied GSUB bytes. The directory is parsed internally (not supplied
/// separately by the caller). No implicit recursive compilation.
pub fn compile_gsub_lookups<'a>(b:&[u8],indices:&[u16],glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GsubCompileLimits)->Result<Vec<CompiledGsubLookup<'a>>,Error>{
    let layout=parse_gsub_layout(b,l.layout)?;
    compile(b,&layout,indices,glyph_count,gdef,l)
}
/// Validate the base plan's selected language, feature membership and
/// exact lookup provenance against the supplied GSUB. A compatible plan
/// from another font is not a cryptographic font-identity guarantee.
/// Unavailable tags are diagnostic only, not execution inputs.
pub fn compile_gsub_plan<'a>(b:&[u8],plan:&GsubPlan,glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GsubCompileLimits)->Result<Vec<CompiledGsubLookup<'a>>,Error>{
    let layout=parse_gsub_layout(b,l.layout)?;
    if plan.lookups.len()>l.max_lookups||plan.features.len()>l.max_plan_refs{return Err(Error::Limit("gsub compile plan size"));}
    if plan.features.windows(2).any(|w|w[0]>=w[1]){return Err(Error::Invalid("gsub compile feature order"));}
    let script=layout.scripts.iter().find(|s|s.tag==plan.script).ok_or(Error::Invalid("gsub compile plan script"))?;
    let lang=match plan.language{
        Some(t)=>script.languages.iter().find(|(tag,_)|*tag==t).map(|(_,l)|l),
        None=>script.default_language.as_ref(),
    }.ok_or(Error::Invalid("gsub compile plan language"))?;
    if plan.required_feature!=lang.required_feature||
        lang.required_feature.is_some_and(|r|plan.features.binary_search(&r).is_err())||
        plan.base_only_variations_present!=layout.directory.feature_variations.is_some(){
        return Err(Error::Invalid("gsub compile plan metadata"));
    }
    let mut budget=l.max_plan_refs;
    charge(&mut budget,lang.features.len(),"gsub compile plan refs")?;
    charge(&mut budget,plan.features.len(),"gsub compile plan refs")?;
    let allowed:BTreeSet<_>=lang.required_feature.into_iter().chain(lang.features.iter().copied()).collect();
    let mut expected:BTreeMap<u16,BTreeSet<u16>>=BTreeMap::new();
    for &id in &plan.features{
        if !allowed.contains(&id){return Err(Error::Invalid("gsub compile feature membership"));}
        let feature=layout.features.get(usize::from(id)).ok_or(Error::Invalid("gsub compile feature index"))?;
        charge(&mut budget,feature.lookups.len(),"gsub compile plan refs")?;
        for &i in &feature.lookups{
            expected.entry(i).or_default().insert(id);
            if expected.len()>l.max_lookups{return Err(Error::Limit("gsub compile lookups"));}
        }
    }
    if expected.len()!=plan.lookups.len(){return Err(Error::Invalid("gsub compile plan lookup count"));}
    let mut indices=Vec::new();
    for ((index,features),entry) in expected.into_iter().zip(&plan.lookups){
        charge(&mut budget,entry.features.len(),"gsub compile plan refs")?;
        if index!=entry.index||!features.iter().copied().eq(entry.features.iter().copied()){
            return Err(Error::Invalid("gsub compile plan provenance"));
        }
        indices.push(index);
    }
    compile(b,&layout,&indices,glyph_count,gdef,l)
}
