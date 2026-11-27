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

/// Bounds for same-face mapping, planning, compiling and execution.
/// Parsing and execution have separate explicit resource limits.
#[derive(Debug,Clone,Copy,Default)]
pub struct FaceShapeLimits{
    pub cmap:CmapLimits,
    pub metrics:FontMetricLimits,
    pub source:SourceLimits,
    pub gdef:GdefLimits,
    pub mark_sets:MarkGlyphSetLimits,
    pub substitution_plan:GsubPlanLimits,
    pub positioning_plan:GposPlanLimits,
    pub substitution_program:GsubProgramLimits,
    pub positioning_program:GposProgramLimits,
    pub joint:JointShapeLimits,
}
/// Caller explicitly selects script, language, feature tags and flow.
/// Missing requested features are either reported or rejected; required
/// features are selected by each table's own LangSys. No implicit shaping
/// stages, default feature lists, bidi or normalization.
#[derive(Debug,Clone,Copy)]
pub struct FaceShapeOptions<'a>{
    pub script:[u8;4],
    pub language:Option<[u8;4]>,
    pub substitution_tags:&'a [[u8;4]],
    pub positioning_tags:&'a [[u8;4]],
    pub alternates:&'a [GsubAlternateSelection],
    pub flow:GposHorizontalFlow,
    pub allow_missing_glyphs:bool,
    pub require_requested_features:bool,
}
#[derive(Debug)]
pub struct FaceShapeResult{
    pub source:SourceRun,
    pub substitution_plan:GsubPlan,
    pub positioning_plan:GposPlan,
    pub units_per_em:u16,
    pub shaped:JointShapeResult,
}
/// Construct all layout inputs directly from one immutable FontFace.
/// Both GSUB and GPOS tables and selectable LangSys must exist in this
/// initial API. An empty selected feature set is valid; absent tables
/// are not silently substituted. Variation plans are base-only and their
/// flags are exposed. Type 5 positioning retains the joint-core rejection.
pub fn shape_font_face(face:&FontFace<'_>,text:&str,o:FaceShapeOptions<'_>,l:FaceShapeLimits)
    ->Result<FaceShapeResult,Error>{
    // Fail early before font-dependent allocations if the request exceeds
    // the source byte or feature-tag budgets.
    if text.len()>l.source.max_bytes{
        return Err(Error::Limit("face shape source bytes"));
    }
    if o.substitution_tags.len()>l.substitution_plan.max_requested_tags||
       o.positioning_tags.len()>l.positioning_plan.max_requested_tags{
        return Err(Error::Limit("face shape requested tags"));
    }
    let sb=face.table(*b"GSUB").ok_or(Error::Unsupported("face shape missing GSUB"))?;
    let pb=face.table(*b"GPOS").ok_or(Error::Unsupported("face shape missing GPOS"))?;
    let metrics=parse_font_metrics(face,l.metrics)?;
    let cmap=parse_unicode_cmap(face,l.cmap)?;
    let domain=u32::from(metrics.maxp().glyph_count);
    if u32::from(cmap.glyph_count())!=domain{
        return Err(Error::Invalid("face shape glyph domain"));
    }
    let mut source_limits=l.source;
    source_limits.max_glyphs=source_limits.max_glyphs.min(l.joint.max_output_glyphs)
        .min(l.joint.gsub.expansion.context.max_glyphs)
        .min(l.joint.provenance.max_glyphs)
        .min(l.joint.gpos.marks.context.run.max_glyphs)
        .min(l.joint.gpos.attachments.max_glyphs)
        .min(l.joint.intervals.max_glyphs);
    let source=map_source_run(text,&cmap,o.allow_missing_glyphs,source_limits)?;
    let gd=face.table(*b"GDEF").map(|b|parse_gdef_with_mark_set_limits(b,domain,l.gdef,l.mark_sets)).transpose()?;
    let sl=parse_gsub_layout(sb,l.substitution_program.compile.layout)?;
    let pl=parse_gpos_layout(pb,l.positioning_program.compile.layout)?;
    let substitution_plan=build_gsub_base_plan(&sl,o.script,o.language,o.substitution_tags,l.substitution_plan)?
        .ok_or(Error::Unsupported("face shape no GSUB language"))?;
    let positioning_plan=build_gpos_base_plan(&pl,o.script,o.language,o.positioning_tags,l.positioning_plan)?
        .ok_or(Error::Unsupported("face shape no GPOS language"))?;
    if o.require_requested_features&&(!substitution_plan.unavailable_tags.is_empty()||
       !positioning_plan.unavailable_tags.is_empty()){
        return Err(Error::Unsupported("face shape requested feature unavailable"));
    }
    let gs=compile_gsub_plan_program(sb,&substitution_plan,domain,gd.as_ref(),l.substitution_program)?;
    let gp=compile_gpos_plan_program(pb,&positioning_plan,domain,gd.as_ref(),l.positioning_program)?;
    let shaped=execute_joint_shape(&gs,&gp,&source,&metrics,o.alternates,o.flow,l.joint)?;
    Ok(FaceShapeResult{source,substitution_plan,positioning_plan,
        units_per_em:metrics.head().units_per_em,shaped})
}
