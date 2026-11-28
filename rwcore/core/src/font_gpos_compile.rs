//! Compile selected GPOS lookup targets without executing positioning.
//! Nested references remain indices into the full source LookupList.
use crate::{Error,Gdef,GposLimits,GposLayout,GposExtensionLimits,GposExtensionTarget,
    LookupFilter,parse_gpos_layout,parse_extension_positioning,parse_direct_gpos_target};


#[derive(Debug,Clone,Copy)]
pub struct GposCompileLimits{
    pub layout:GposLimits,
    pub target:GposExtensionLimits,
    pub max_lookups:usize,
    pub max_subtables_per_lookup:usize,
    pub max_total_subtables:usize,
    /// Conservative work charge: sum of target suffix lengths, plus 8
    /// bytes per extension wrapper. Aliases charged per reference.
    /// This is NOT the exact owned memory size or a certified table extent.
    pub max_total_target_bytes:usize,
    pub max_plan_refs:usize,
}
impl Default for GposCompileLimits{
    fn default()->Self{Self{layout:Default::default(),target:Default::default(),
        max_lookups:16384,max_subtables_per_lookup:4096,max_total_subtables:16384,
        max_total_target_bytes:512*1024*1024,max_plan_refs:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CompiledGposSubtable{
    source_offset:usize,
    target_offset:usize,
    target:GposExtensionTarget,
}
impl CompiledGposSubtable{
    pub fn source_offset(&self)->usize{self.source_offset}
    pub fn target_offset(&self)->usize{self.target_offset}
    pub fn target(&self)->&GposExtensionTarget{&self.target}
}
#[derive(Debug,Clone)]
pub struct CompiledGposLookup<'a>{
    index:u16,
    declared_kind:u16,
    effective_kind:u16,
    filter:LookupFilter<'a>,
    subtables:Vec<CompiledGposSubtable>,
}
impl<'a> CompiledGposLookup<'a>{
    pub fn index(&self)->u16{self.index}
    pub fn declared_kind(&self)->u16{self.declared_kind}
    pub fn effective_kind(&self)->u16{self.effective_kind}
    pub fn filter(&self)->&LookupFilter<'a>{&self.filter}
    pub fn subtables(&self)->&[CompiledGposSubtable]{&self.subtables}
}
fn charge(budget:&mut usize,n:usize,what:&'static str)->Result<(),Error>{
    *budget=budget.checked_sub(n).ok_or(Error::Limit(what))?;
    Ok(())
}
fn validate_domain(glyph_count:u32,gdef:Option<&Gdef>)->Result<(),Error>{
    if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("gpos compile glyph domain"));}
    if gdef.is_some_and(|g|g.glyph_count()!=glyph_count){return Err(Error::Invalid("gpos compile GDEF domain"));}
    Ok(())
}
fn compile<'a>(b:&[u8],layout:&GposLayout,indices:&[u16],glyph_count:u32,
    gdef:Option<&'a Gdef>,l:GposCompileLimits)->Result<Vec<CompiledGposLookup<'a>>,Error>{
    validate_domain(glyph_count,gdef)?;
    if indices.len()>l.max_lookups{return Err(Error::Limit("gpos compile lookups"));}
    if indices.windows(2).any(|w|w[0]>=w[1]){return Err(Error::Invalid("gpos compile lookup order"));}
    let mut count_budget=l.max_total_subtables;
    let mut byte_budget=l.max_total_target_bytes;
    let mut out=Vec::new();
    out.try_reserve_exact(indices.len()).map_err(|_|Error::Limit("gpos compile allocation"))?;
    for &index in indices{
        let lookup=layout.lookups.get(usize::from(index)).ok_or(Error::Invalid("gpos compile lookup index"))?;
        let filter=LookupFilter::from_gpos_lookup(lookup,glyph_count,gdef)?;
        if lookup.subtables.is_empty(){return Err(Error::Invalid("gpos compile empty lookup"));}
        if lookup.subtables.len()>l.max_subtables_per_lookup{return Err(Error::Limit("gpos compile per lookup subtables"));}
        charge(&mut count_budget,lookup.subtables.len(),"gpos compile subtables")?;
        let mut subtables=Vec::new();
        subtables.try_reserve_exact(lookup.subtables.len()).map_err(|_|Error::Limit("gpos compile allocation"))?;
        let mut effective=None;
        for &at in &lookup.subtables{
            let source=b.get(at..).ok_or(Error::Invalid("gpos compile source offset"))?;
            let (kind,target_at,bytes)=if lookup.kind==9{
                let ext=parse_extension_positioning(source,l.target.max_bytes)?;
                charge(&mut byte_budget,8,"gpos compile target bytes")?;
                (ext.kind(),at.checked_add(ext.offset()).ok_or(Error::Limit("gpos compile target offset"))?,ext.target())
            }else{(lookup.kind,at,source)};
            if effective.is_some_and(|k|k!=kind){return Err(Error::Invalid("gpos compile mixed targets"));}
            effective=Some(kind);
            charge(&mut byte_budget,bytes.len(),"gpos compile target bytes")?;
            let target=parse_direct_gpos_target(kind,bytes,glyph_count,layout.lookups.len(),l.target)?;
            subtables.push(CompiledGposSubtable{source_offset:at,target_offset:target_at,target});
        }
        out.push(CompiledGposLookup{index,declared_kind:lookup.kind,
            effective_kind:effective.ok_or(Error::Invalid("gpos compile empty lookup"))?,filter,subtables});
    }
    Ok(out)
}
/// Compile explicitly selected ascending unique lookup indices from the
/// supplied GPOS bytes. The directory is parsed internally (not supplied
/// separately by the caller). No implicit recursive compilation.
pub fn compile_gpos_lookups<'a>(b:&[u8],indices:&[u16],glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GposCompileLimits)->Result<Vec<CompiledGposLookup<'a>>,Error>{
    let layout=parse_gpos_layout(b,l.layout)?;
    compile(b,&layout,indices,glyph_count,gdef,l)
}

use crate::GposPlan;
use std::collections::{BTreeMap,BTreeSet};
/// Validate the base plan's selected language, feature membership and
/// exact lookup provenance against the supplied GPOS. A compatible plan
/// from another font is not a cryptographic font-identity guarantee.
/// Unavailable tags are diagnostic only, not execution inputs.
pub fn compile_gpos_plan<'a>(b:&[u8],plan:&GposPlan,glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GposCompileLimits)->Result<Vec<CompiledGposLookup<'a>>,Error>{
    let layout=parse_gpos_layout(b,l.layout)?;
    let indices=validate_plan(&layout,plan,l)?;
    compile(b,&layout,&indices,glyph_count,gdef,l)
}
fn validate_plan(layout:&GposLayout,plan:&GposPlan,l:GposCompileLimits)->Result<Vec<u16>,Error>{

    if plan.lookups.len()>l.max_lookups||plan.features.len()>l.max_plan_refs{return Err(Error::Limit("gpos compile plan size"));}
    if plan.features.windows(2).any(|w|w[0]>=w[1]){return Err(Error::Invalid("gpos compile feature order"));}
    let script=layout.scripts.iter().find(|s|s.tag==plan.script).ok_or(Error::Invalid("gpos compile plan script"))?;
    let lang=match plan.language{
        Some(t)=>script.languages.iter().find(|(tag,_)|*tag==t).map(|(_,l)|l),
        None=>script.default_language.as_ref(),
    }.ok_or(Error::Invalid("gpos compile plan language"))?;
    if plan.required_feature!=lang.required_feature||
        lang.required_feature.is_some_and(|r|plan.features.binary_search(&r).is_err())||
        plan.base_only_variations_present!=layout.directory.feature_variations.is_some(){
        return Err(Error::Invalid("gpos compile plan metadata"));
    }
    let mut budget=l.max_plan_refs;
    charge(&mut budget,lang.features.len(),"gpos compile plan refs")?;
    charge(&mut budget,plan.features.len(),"gpos compile plan refs")?;
    let allowed:BTreeSet<_>=lang.required_feature.into_iter().chain(lang.features.iter().copied()).collect();
    let mut expected:BTreeMap<u16,BTreeSet<u16>>=BTreeMap::new();
    for &id in &plan.features{
        if !allowed.contains(&id){return Err(Error::Invalid("gpos compile feature membership"));}
        let feature=layout.features.get(usize::from(id)).ok_or(Error::Invalid("gpos compile feature index"))?;
        let lookups=feature.lookups.as_slice();
        charge(&mut budget,lookups.len(),"gpos compile plan refs")?;
        for &i in lookups{
            expected.entry(i).or_default().insert(id);
            if expected.len()>l.max_lookups{return Err(Error::Limit("gpos compile lookups"));}
        }
    }
    if expected.len()!=plan.lookups.len(){return Err(Error::Invalid("gpos compile plan lookup count"));}
    let mut indices=Vec::new();
    for ((index,features),entry) in expected.into_iter().zip(&plan.lookups){
        charge(&mut budget,entry.features.len(),"gpos compile plan refs")?;
        if index!=entry.index||!features.iter().copied().eq(entry.features.iter().copied()){
            return Err(Error::Invalid("gpos compile plan provenance"));
        }
        indices.push(index);
    }
    Ok(indices)
}


use crate::{GposDependency,GposDependencyLimits,GposClosureLimits,GposDependencyClosure,analyze_gpos_dependency_closure};
#[derive(Debug,Clone,Copy,Default)]
pub struct GposProgramLimits{
    pub compile:GposCompileLimits,
    pub dependencies:GposDependencyLimits,
    pub closure:GposClosureLimits,
}
/// Static compiled program. Only roots are scheduled at top level.
/// Dependency-only targets are available by original LookupList index.
#[derive(Debug,Clone)]
pub struct CompiledGposProgram<'a>{
    glyph_count:u32,
    roots:Vec<u16>,
    validated_plan:Option<GposPlan>,
    lookups:Vec<CompiledGposLookup<'a>>,
    dependencies:Vec<GposDependency>,
    closure:GposDependencyClosure,
}
impl<'a> CompiledGposProgram<'a>{
    pub fn glyph_count(&self)->u32{self.glyph_count}
    pub fn roots(&self)->&[u16]{&self.roots}
    pub fn validated_plan(&self)->Option<&GposPlan>{self.validated_plan.as_ref()}
    pub fn lookups(&self)->&[CompiledGposLookup<'a>]{&self.lookups}
    pub fn lookup(&self,id:u16)->Option<&CompiledGposLookup<'a>>{
        self.lookups.binary_search_by_key(&id,|l|l.index()).ok().map(|i|&self.lookups[i])
    }
    pub fn dependencies(&self)->&[GposDependency]{&self.dependencies}
    pub fn closure(&self)->&GposDependencyClosure{&self.closure}
}
fn compile_program<'a>(b:&[u8],layout:&GposLayout,roots:&[u16],glyph_count:u32,
    gdef:Option<&'a Gdef>,l:GposProgramLimits)->Result<CompiledGposProgram<'a>,Error>{
    validate_domain(glyph_count,gdef)?;
    let n=layout.lookups.len();
    if n>l.closure.max_lookup_count||roots.len()>l.closure.max_roots{
        return Err(Error::Limit("gpos program domain"));
    }
    if roots.windows(2).any(|w|w[0]>=w[1]){return Err(Error::Invalid("gpos program root order"));}
    let mut pending=BTreeSet::new();
    let mut seen=BTreeSet::new();
    for &id in roots{
        if usize::from(id)>=n{return Err(Error::Invalid("gpos program root index"));}
        pending.insert(id);seen.insert(id);
    }
    let cap=l.compile.max_lookups.min(l.closure.max_reachable);
    if seen.len()>cap{return Err(Error::Limit("gpos program lookups"));}
    let mut budget=l.compile;
    let mut dep=l.dependencies;
    dep.max_actions=dep.max_actions.min(l.closure.max_actions);
    let mut compiled=BTreeMap::new();
    let mut edges=Vec::new();
    while let Some(id)=pending.pop_first(){
        let mut one=compile(b,layout,&[id],glyph_count,gdef,budget)?;
        let lookup=one.pop().ok_or(Error::Invalid("gpos program lookup"))?;
        charge(&mut budget.max_lookups,1,"gpos program lookups")?;
        charge(&mut budget.max_total_subtables,lookup.subtables().len(),"gpos program subtables")?;
        for sub in lookup.subtables(){
            if lookup.declared_kind()==9{charge(&mut budget.max_total_target_bytes,8,"gpos program bytes")?;}
            charge(&mut budget.max_total_target_bytes,b.len()-sub.target_offset(),"gpos program bytes")?;
        }
        let (found,remaining)=crate::font_gpos_dependencies::discover_with_scan(std::slice::from_ref(&lookup),n,dep)?;
        dep.max_scan_records=remaining;
        charge(&mut dep.max_actions,found.len(),"gpos program actions")?;
        for e in &found{
            let target=e.action.lookup_index;
            if seen.insert(target){
                if seen.len()>cap{return Err(Error::Limit("gpos program lookups"));}
                pending.insert(target);
            }
        }
        edges.try_reserve(found.len()).map_err(|_|Error::Limit("gpos program allocation"))?;
        edges.extend(found);
        compiled.insert(id,lookup);
    }
    let sources:Vec<_>=compiled.keys().copied().collect();
    // Discovery order can differ from original LookupList order.
    // Stable sort retains every source's original ordered actions.
    edges.sort_by_key(|e|e.site.lookup_index);
    let closure=analyze_gpos_dependency_closure(n,&sources,&edges,roots,l.closure)?;
    if !closure.is_complete()||closure.reachable!=sources{return Err(Error::Invalid("gpos program incomplete"));}
    Ok(CompiledGposProgram{glyph_count,roots:roots.to_vec(),validated_plan:None,lookups:compiled.into_values().collect(),dependencies:edges,closure})
}
pub fn compile_gpos_program<'a>(b:&[u8],roots:&[u16],glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GposProgramLimits)->Result<CompiledGposProgram<'a>,Error>{
    let layout=parse_gpos_layout(b,l.compile.layout)?;
    compile_program(b,&layout,roots,glyph_count,gdef,l)
}
pub fn compile_gpos_plan_program<'a>(b:&[u8],plan:&GposPlan,glyph_count:u32,gdef:Option<&'a Gdef>,
    l:GposProgramLimits)->Result<CompiledGposProgram<'a>,Error>{
    let layout=parse_gpos_layout(b,l.compile.layout)?;
    let roots=validate_plan(&layout,plan,l.compile)?;
    let mut program=compile_program(b,&layout,&roots,glyph_count,gdef,l)?;
    program.validated_plan=Some(plan.clone());
    Ok(program)
}

