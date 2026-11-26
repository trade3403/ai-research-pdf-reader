//! Bounded execution of GSUB Type 1 roots (including Type 7 wrappers).
//! Logical glyph order and length are preserved. This is not full shaping.
use crate::{CompiledGsubProgram,Error,ExtensionTarget};

#[derive(Debug,Clone,Copy)]
pub struct GsubSingleExecLimits{
    pub max_glyphs:usize,
    pub max_roots:usize,
    pub max_visits:usize,
    pub max_probes:usize,
    pub max_matches:usize,
}
impl Default for GsubSingleExecLimits{
    fn default()->Self{Self{max_glyphs:262144,max_roots:16384,
        max_visits:16*1024*1024,max_probes:32*1024*1024,max_matches:16*1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubSingleExecResult{
    pub glyphs:Vec<u16>,
    pub visits:usize,
    pub probes:usize,
    /// Includes identity substitutions: a matching subtable wins even
    /// if its replacement equals the input.
    pub matches:usize,
    pub changes:usize,
}
fn tick(counter:&mut usize,limit:usize,what:&'static str)->Result<(),Error>{
    if *counter>=limit{return Err(Error::Limit(what));}
    *counter+=1;
    Ok(())
}
/// Execute all scheduled roots, in original ascending LookupList order.
/// Reject any unsupported root before execution, including on empty runs.
/// The caller's immutable input never changes, and errors return no output.
/// Each lookup visits each position once; later lookups see earlier edits.
/// Eligibility is evaluated on the current glyph, not the original glyph.
pub fn execute_gsub_single(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    limits:GsubSingleExecLimits)->Result<GsubSingleExecResult,Error>{
    if glyphs.len()>limits.max_glyphs||program.roots().len()>limits.max_roots{
        return Err(Error::Limit("gsub single execution size"));
    }
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){
        return Err(Error::Invalid("gsub single execution glyph"));
    }
    for &id in program.roots(){
        let l=program.lookup(id).ok_or(Error::Invalid("gsub execution missing root"))?;
        if l.effective_kind()!=1{return Err(Error::Unsupported("gsub single execution root kind"));}
    }
    let mut out=GsubSingleExecResult{glyphs:Vec::new(),visits:0,probes:0,matches:0,changes:0};
    out.glyphs.try_reserve_exact(glyphs.len()).map_err(|_|Error::Limit("gsub single allocation"))?;
    out.glyphs.extend_from_slice(glyphs);
    for &id in program.roots(){
        let lookup=program.lookup(id).ok_or(Error::Invalid("gsub execution missing root"))?;
        for glyph in &mut out.glyphs{
            tick(&mut out.visits,limits.max_visits,"gsub single visits")?;
            if !lookup.filter().eligibility(*glyph)?.is_eligible(){continue;}
            for sub in lookup.subtables(){
                tick(&mut out.probes,limits.max_probes,"gsub single probes")?;
                let ExtensionTarget::Single(rule)=sub.target() else{
                    return Err(Error::Invalid("gsub single target kind"));
                };
                if let Some(replacement)=rule.substitute(*glyph){
                    tick(&mut out.matches,limits.max_matches,"gsub single matches")?;
                    if replacement!=*glyph{out.changes+=1;}
                    *glyph=replacement;
                    break;
                }
            }
        }
    }
    Ok(out)
}


/// Bounded mixed Type 1/2 execution. Glyph cap applies to the input and
/// every intermediate/output run. Emission counts include copied glyphs,
/// not just replacement glyphs, across all root passes.
#[derive(Debug,Clone,Copy)]
pub struct GsubMixedExecLimits{
    pub max_glyphs:usize,
    pub max_roots:usize,
    pub max_visits:usize,
    pub max_probes:usize,
    pub max_matches:usize,
    pub max_emitted:usize,
}
impl Default for GsubMixedExecLimits{
    fn default()->Self{Self{max_glyphs:262144,max_roots:16384,
        max_visits:16*1024*1024,max_probes:32*1024*1024,
        max_matches:16*1024*1024,max_emitted:32*1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubMixedExecResult{
    pub glyphs:Vec<u16>,
    pub visits:usize,
    pub probes:usize,
    pub matches:usize,
    /// Number of matched input positions whose output differs from the
    /// one-element input, including expansions.
    pub changes:usize,
    pub emitted:usize,
}
fn emit(next:&mut Vec<u16>,seq:&[u16],emitted:&mut usize,
    limits:GsubMixedExecLimits)->Result<(),Error>{
    let len=next.len().checked_add(seq.len()).ok_or(Error::Limit("gsub mixed run length"))?;
    let total=emitted.checked_add(seq.len()).ok_or(Error::Limit("gsub mixed emissions"))?;
    if len>limits.max_glyphs{return Err(Error::Limit("gsub mixed run length"));}
    if total>limits.max_emitted{return Err(Error::Limit("gsub mixed emissions"));}
    next.try_reserve(seq.len()).map_err(|_|Error::Limit("gsub mixed allocation"))?;
    next.extend_from_slice(seq);
    *emitted=total;
    Ok(())
}
/// Execute effective Type 1 and Type 2 roots (including extension
/// wrappers) in LookupList order. Each pass scans only its input: newly
/// emitted glyphs cannot be processed by that same root. Later roots see
/// every emitted glyph and recompute eligibility. No deletion, cluster
/// propagation, nested context, or unsupported-root skipping.
pub fn execute_gsub_mixed(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    limits:GsubMixedExecLimits)->Result<GsubMixedExecResult,Error>{
    if glyphs.len()>limits.max_glyphs||program.roots().len()>limits.max_roots{
        return Err(Error::Limit("gsub mixed execution size"));
    }
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){
        return Err(Error::Invalid("gsub mixed execution glyph"));
    }
    for &id in program.roots(){
        let l=program.lookup(id).ok_or(Error::Invalid("gsub execution missing root"))?;
        if !matches!(l.effective_kind(),1|2){return Err(Error::Unsupported("gsub mixed execution root kind"));}
    }
    let mut out=GsubMixedExecResult{glyphs:Vec::new(),visits:0,probes:0,matches:0,changes:0,emitted:0};
    out.glyphs.try_reserve_exact(glyphs.len()).map_err(|_|Error::Limit("gsub mixed allocation"))?;
    out.glyphs.extend_from_slice(glyphs);
    let mut next=Vec::new();
    for &id in program.roots(){
        let lookup=program.lookup(id).ok_or(Error::Invalid("gsub execution missing root"))?;
        next.clear();
        for &glyph in &out.glyphs{
            tick(&mut out.visits,limits.max_visits,"gsub mixed visits")?;
            let mut single=None;
            let mut multiple=None;
            if lookup.filter().eligibility(glyph)?.is_eligible(){
                for sub in lookup.subtables(){
                    tick(&mut out.probes,limits.max_probes,"gsub mixed probes")?;
                    match sub.target(){
                        ExtensionTarget::Single(rule)=>{
                            if let Some(v)=rule.substitute(glyph){single=Some(v);break;}
                        }
                        ExtensionTarget::Multiple(rule)=>{
                            if let Some(seq)=rule.substitute(glyph){multiple=Some(seq);break;}
                        }
                        _=>return Err(Error::Invalid("gsub mixed target kind")),
                    }
                }
            }
            let one=single.map(|v|[v]);
            let replacement=one.as_ref().map(|v|v.as_slice()).or(multiple);
            if let Some(seq)=replacement{
                tick(&mut out.matches,limits.max_matches,"gsub mixed matches")?;
                if seq!=[glyph]{out.changes+=1;}
                emit(&mut next,seq,&mut out.emitted,limits)?;
            }else{
                emit(&mut next,&[glyph],&mut out.emitted,limits)?;
            }
        }
        std::mem::swap(&mut next,&mut out.glyphs);
    }
    Ok(out)
}
