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
