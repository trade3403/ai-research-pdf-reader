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
        let filter=LookupFilter::from_gsub_lookup(lookup,glyph_count,gdef)?;
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
