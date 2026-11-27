//! Native GSUB -> post-substitution metrics -> GPOS pipeline.
//! Explicit compiled programs and logical SourceRun; no implicit script
//! shaping, normalization, bidi or glyph fallback. Font identity is a caller
//! contract: equal glyph domains alone do not establish same-font identity.
use crate::*;

#[derive(Debug,Clone,Copy)]
pub struct JointShapeLimits{
    pub gsub:GsubProgramExecLimits,
    pub provenance:ProvenanceLimits,
    pub gpos:GposMixedRunLimits,
    pub intervals:GposIntervalLimits,
    pub max_output_glyphs:usize,
}
impl Default for JointShapeLimits{
    fn default()->Self{Self{gsub:Default::default(),provenance:Default::default(),
        gpos:Default::default(),intervals:Default::default(),max_output_glyphs:65536}}
}
#[derive(Debug)]
pub struct JointShapeResult{
    pub substitution:GsubProgramExecResult,
    pub provenance:ProvenanceLedger,
    pub design_advances:Vec<(i32,i32)>,
    pub positioning:GposCursiveRun,
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("joint shape allocation"))
}
/// Execute full GSUB with scalar provenance and then selected GPOS on the
/// resulting glyph IDs. Nominal horizontal widths are read after GSUB.
/// Design-only; no device scaling or Unicode mark-width normalization.
/// Type 5 GPOS is deliberately rejected until occurrence/component lineage
/// exists. MarkToMark after ligature contraction is also rejected rather
/// than conflating distinct ligature components in an unmapped mark chain.
pub fn execute_joint_shape(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,choices:&[GsubAlternateSelection],
    flow:GposHorizontalFlow,limits:JointShapeLimits,
)->Result<JointShapeResult,Error>{
    let domain=u32::from(metrics.maxp().glyph_count);
    if gsub.glyph_count()!=domain||gpos.glyph_count()!=domain{
        return Err(Error::Invalid("joint shape glyph domain"));
    }
    if gpos.lookups().iter().any(|l|l.effective_kind()==5){
        return Err(Error::Unsupported("joint shape ligature component lineage"));
    }
    if run.glyphs().len()>limits.max_output_glyphs{
        return Err(Error::Limit("joint shape input glyphs"));
    }
    // Bound expansion before the first stage, not merely before GPOS.
    let mut sl=limits.gsub;
    let cap=limits.max_output_glyphs.min(limits.gpos.marks.context.run.max_glyphs)
        .min(limits.gpos.attachments.max_glyphs).min(limits.intervals.max_glyphs);
    sl.expansion.context.max_glyphs=sl.expansion.context.max_glyphs.min(cap);
    let (substitution,provenance)=execute_gsub_program_with_provenance(gsub,run,choices,sl,limits.provenance)?;
    if substitution.ligatures>0&&gpos.lookups().iter().any(|l|l.effective_kind()==6){
        return Err(Error::Unsupported("joint shape mark component lineage"));
    }
    let glyphs=&substitution.expansion.execution.glyphs;
    let n=glyphs.len();
    if n>cap{return Err(Error::Limit("joint shape output glyphs"));}
    if provenance.glyphs().len()!=n||provenance.glyphs().iter().zip(glyphs).any(|(g,id)|g.glyph_id!=*id){
        return Err(Error::Invalid("joint shape provenance alignment"));
    }
    let mut design_advances=Vec::new();reserve(&mut design_advances,n)?;
    for &id in glyphs{design_advances.push((i32::from(metrics.glyph(id)?.advance_width),0));}
    let mut prior=Vec::new();reserve(&mut prior,n)?;prior.resize(n,GposRunAdjustment::default());
    let mut pixels=Vec::new();reserve(&mut pixels,n)?;pixels.resize(n,(0,0));
    let mut sources=Vec::new();reserve(&mut sources,n)?;sources.resize(n,GposLigatureSource::Unmapped);
    let positioning=execute_gpos_mixed_joint(gpos,glyphs,&prior,&design_advances,&pixels,&sources,
        GposRunMode::DesignOnly,flow,limits.gpos,limits.intervals)?;
    Ok(JointShapeResult{substitution,provenance,design_advances,positioning})
}
