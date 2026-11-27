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
    execute_joint_shape_inner(gsub,gpos,run,metrics,choices,flow,limits,None)
}
/// Independent budgets for opt-in automatic ligature mapping.
#[derive(Debug,Clone,Copy,Default)]
pub struct JointLigatureLimits{
    pub trace:LigatureTraceLimits,
    pub mapping:LigatureMappingLimits,
}
/// Simple ordered component mapping. GDEF must belong to the same face
/// as both compiled programs; use the same-face entry to enforce this.
pub fn execute_joint_shape_with_ligatures(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,gdef:&Gdef,choices:&[GsubAlternateSelection],
    flow:GposHorizontalFlow,limits:JointShapeLimits,ligatures:JointLigatureLimits,
)->Result<JointShapeResult,Error>{
    execute_joint_shape_inner(gsub,gpos,run,metrics,choices,flow,limits,Some((gdef,ligatures)))
}
fn execute_joint_shape_inner(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,choices:&[GsubAlternateSelection],
    flow:GposHorizontalFlow,limits:JointShapeLimits,ligatures:Option<(&Gdef,JointLigatureLimits)>,
)->Result<JointShapeResult,Error>{
    let sl=joint_preflight(gsub,gpos,run,metrics,limits,ligatures)?;
    let (substitution,provenance)=if let Some((_,l))=ligatures{
        execute_gsub_program_with_ligature_trace(gsub,run,choices,sl,limits.provenance,l.trace)?
    }else{execute_gsub_program_with_provenance(gsub,run,choices,sl,limits.provenance)?};
    finish_joint_shape(gpos,metrics,substitution,provenance,flow,limits,ligatures)
}
fn joint_preflight(gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,limits:JointShapeLimits,
    ligatures:Option<(&Gdef,JointLigatureLimits)>)->Result<GsubProgramExecLimits,Error>{
    let domain=u32::from(metrics.maxp().glyph_count);
    if gsub.glyph_count()!=domain||gpos.glyph_count()!=domain{
        return Err(Error::Invalid("joint shape glyph domain"));
    }
    if let Some((gd,_))=ligatures{
        if gd.glyph_count()!=domain{return Err(Error::Invalid("joint shape GDEF domain"));}
        if gd.glyph_classes().is_none(){return Err(Error::Unsupported("joint shape missing GDEF classes"));}
    }
    if ligatures.is_none()&&gpos.lookups().iter().any(|l|l.effective_kind()==5){
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
    if let Some((_,l))=ligatures{
        sl.expansion.context.max_glyphs=sl.expansion.context.max_glyphs.min(l.mapping.max_glyphs);
        if run.scalars().len()>l.mapping.max_scalars{return Err(Error::Limit("joint ligature scalars"));}
    }
    Ok(sl)
}
fn finish_joint_shape(gpos:&CompiledGposProgram<'_>,metrics:&FontMetrics<'_>,
    substitution:GsubProgramExecResult,provenance:ProvenanceLedger,flow:GposHorizontalFlow,
    limits:JointShapeLimits,ligatures:Option<(&Gdef,JointLigatureLimits)>)->Result<JointShapeResult,Error>{
    Ok(finish_joint_shape_inner(gpos,metrics,substitution,provenance,flow,limits,ligatures,None)?.0)
}
fn finish_joint_shape_inner(gpos:&CompiledGposProgram<'_>,metrics:&FontMetrics<'_>,
    substitution:GsubProgramExecResult,provenance:ProvenanceLedger,flow:GposHorizontalFlow,
    limits:JointShapeLimits,ligatures:Option<(&Gdef,JointLigatureLimits)>,
    staged:Option<JointPositionStages<'_>>,
)->Result<(JointShapeResult,Option<ShapeStageSchedule>),Error>{
    let cap=limits.max_output_glyphs.min(limits.gpos.marks.context.run.max_glyphs)
        .min(limits.gpos.attachments.max_glyphs).min(limits.intervals.max_glyphs);
    if ligatures.is_none()&&substitution.ligatures>0&&gpos.lookups().iter().any(|l|l.effective_kind()==6){
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
    let sources=if let Some((gd,l))=ligatures{
        map_gpos_ligature_sources(&provenance,gd,l.mapping)?
    }else{
        let mut sources=Vec::new();reserve(&mut sources,n)?;sources.resize(n,GposLigatureSource::Unmapped);sources
    };
    let (schedule,positioning)=if let Some(p)=staged{
        let (schedule,positioning)=execute_staged_gpos_joint(gpos,&provenance,p.masks,p.specs,
            p.limits.schedule,p.limits.max_mask_work,glyphs,&prior,&design_advances,&pixels,&sources,
            GposRunMode::DesignOnly,flow,limits.gpos,limits.intervals,ligatures.is_some())?;
        (Some(schedule),positioning)
    }else if ligatures.is_some(){
        (None,execute_gpos_mixed_joint_strict_ligatures(gpos,glyphs,&prior,&design_advances,&pixels,&sources,
            GposRunMode::DesignOnly,flow,limits.gpos,limits.intervals)?)
    }else{
        (None,execute_gpos_mixed_joint(gpos,glyphs,&prior,&design_advances,&pixels,&sources,
            GposRunMode::DesignOnly,flow,limits.gpos,limits.intervals)?)
    };
    Ok((JointShapeResult{substitution,provenance,design_advances,positioning},schedule))
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
    shape_font_face_inner(face,text,o,l,None)
}
/// Same-face opt-in mapping with mandatory GDEF glyph classes.
pub fn shape_font_face_with_ligatures(face:&FontFace<'_>,text:&str,o:FaceShapeOptions<'_>,
    l:FaceShapeLimits,ligatures:JointLigatureLimits)->Result<FaceShapeResult,Error>{
    shape_font_face_inner(face,text,o,l,Some(ligatures))
}
fn shape_font_face_inner(face:&FontFace<'_>,text:&str,o:FaceShapeOptions<'_>,l:FaceShapeLimits,
    ligatures:Option<JointLigatureLimits>)->Result<FaceShapeResult,Error>{
    let (source,substitution_plan,positioning_plan,units_per_em,shaped)=
        shape_font_face_execute(face,text,o,l,ligatures,|gs,gp,source,metrics,gd|{
            if let Some(ll)=ligatures{
                execute_joint_shape_with_ligatures(gs,gp,source,metrics,
                    gd.ok_or(Error::Unsupported("face shape missing GDEF"))?,o.alternates,o.flow,l.joint,ll)
            }else{execute_joint_shape(gs,gp,source,metrics,o.alternates,o.flow,l.joint)}
        })?;
    Ok(FaceShapeResult{source,substitution_plan,positioning_plan,units_per_em,shaped})
}
fn shape_font_face_execute<T>(
    face:&FontFace<'_>,text:&str,o:FaceShapeOptions<'_>,l:FaceShapeLimits,
    ligatures:Option<JointLigatureLimits>,
    execute:impl FnOnce(&CompiledGsubProgram<'_>,&CompiledGposProgram<'_>,&SourceRun,&FontMetrics<'_>,Option<&Gdef>)->Result<T,Error>,
)->Result<(SourceRun,GsubPlan,GposPlan,u16,T),Error>{
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
    if let Some(l)=ligatures{
        source_limits.max_glyphs=source_limits.max_glyphs.min(l.mapping.max_glyphs);
        source_limits.max_scalars=source_limits.max_scalars.min(l.mapping.max_scalars);
    }
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
    let shaped=execute(&gs,&gp,&source,&metrics,gd.as_ref())?;
    Ok((source,substitution_plan,positioning_plan,metrics.head().units_per_em,shaped))
}

/// Stage scheduling and scalar mask execution budgets, independent of the
/// post-substitution GPOS budgets in JointShapeLimits.
#[derive(Debug,Clone,Copy)]
pub struct JointStageLimits{
    pub schedule:ShapeStageLimits,
    pub max_mask_work:usize,
}
impl Default for JointStageLimits{
    fn default()->Self{Self{schedule:Default::default(),max_mask_work:4194304}}
}
#[derive(Debug)]
pub struct StagedJointShapeResult{
    pub shaped:JointShapeResult,
    pub schedule:ShapeStageSchedule,
    pub trace:Vec<ShapeStageTrace>,
    pub glyph_masks:Vec<u64>,
    pub skipped_roots:usize,
    pub blocked_inputs:usize,
    pub mask_work:usize,
}
/// Execute explicit GSUB stages in one atomic glyph/provenance buffer,
/// then GPOS once over the final glyph stream. GSUB masks remain in the
/// GSUB feature namespace; they do NOT gate GPOS lookups. The optional
/// simple ligature path shares the strict joint attachment policy.
pub fn execute_staged_joint_shape(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,choices:&[GsubAlternateSelection],
    masks:&ScalarFeatureMasks,specs:&[ShapeStageSpec],flow:GposHorizontalFlow,
    limits:JointShapeLimits,stages:JointStageLimits,
    ligatures:Option<(&Gdef,JointLigatureLimits)>,
)->Result<StagedJointShapeResult,Error>{
    Ok(execute_staged_joint_shape_inner(gsub,gpos,run,metrics,choices,masks,specs,flow,
        limits,stages,ligatures,None)?.0)
}
fn execute_staged_joint_shape_inner(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,choices:&[GsubAlternateSelection],
    masks:&ScalarFeatureMasks,specs:&[ShapeStageSpec],flow:GposHorizontalFlow,
    limits:JointShapeLimits,stages:JointStageLimits,
    ligatures:Option<(&Gdef,JointLigatureLimits)>,position:Option<JointPositionStages<'_>>,
)->Result<(StagedJointShapeResult,Option<ShapeStageSchedule>),Error>{
    let sl=joint_preflight(gsub,gpos,run,metrics,limits,ligatures)?;
    let (schedule,result,blocked_inputs,trace)=if let Some((_,l))=ligatures{
        execute_staged_gsub_with_ligature_trace(gsub,run,choices,masks,specs,stages.schedule,
            sl,limits.provenance,stages.max_mask_work,l.trace)?
    }else{
        execute_staged_gsub_traced(gsub,run,choices,masks,specs,stages.schedule,
            sl,limits.provenance,stages.max_mask_work)?
    };
    let MaskedGsubResult{execution,provenance,glyph_masks,skipped_roots,mask_work}=result;
    let (shaped,positioning_schedule)=finish_joint_shape_inner(gpos,metrics,execution,provenance,flow,limits,ligatures,position)?;
    Ok((StagedJointShapeResult{shaped,schedule,trace,glyph_masks,skipped_roots,blocked_inputs,mask_work},positioning_schedule))
}

/// Positioning stages and masks are independent of the GSUB namespace.
#[derive(Debug,Clone,Copy)]
pub struct JointPositionStages<'a>{
    pub masks:&'a GposScalarFeatureMasks,
    pub specs:&'a [ShapeStageSpec],
    pub limits:JointStageLimits,
}
#[derive(Debug)]
pub struct DualStageShapeResult{
    pub staged:StagedJointShapeResult,
    pub positioning_schedule:ShapeStageSchedule,
}
/// Execute all GSUB stages, then read final glyph metrics and project the
/// independent GPOS scalar masks through final provenance. Attachment state
/// is shared across GPOS stages. No intermediate GPOS or script reordering.
pub fn execute_dual_stage_joint_shape(
    gsub:&CompiledGsubProgram<'_>,gpos:&CompiledGposProgram<'_>,
    run:&SourceRun,metrics:&FontMetrics<'_>,choices:&[GsubAlternateSelection],
    masks:&ScalarFeatureMasks,specs:&[ShapeStageSpec],flow:GposHorizontalFlow,
    limits:JointShapeLimits,stages:JointStageLimits,
    ligatures:Option<(&Gdef,JointLigatureLimits)>,position:JointPositionStages<'_>,
)->Result<DualStageShapeResult,Error>{
    let (staged,schedule)=execute_staged_joint_shape_inner(gsub,gpos,run,metrics,choices,
        masks,specs,flow,limits,stages,ligatures,Some(position))?;
    Ok(DualStageShapeResult{staged,positioning_schedule:schedule.ok_or(Error::Invalid("joint positioning schedule"))?})
}
/// Explicit per-table feature IDs from the selected base plan. Scalar
/// overrides and work limits are separate even when numeric IDs coincide.
#[derive(Debug,Clone,Copy)]
pub struct FaceStageRequest<'a>{
    pub specs:&'a [ShapeStageSpec],
    pub overrides:&'a [FeatureMaskOverride],
    pub masks:FeatureMaskLimits,
    pub stages:JointStageLimits,
}
#[derive(Debug)]
pub struct DualStageFaceShapeResult{
    pub source:SourceRun,
    pub substitution_plan:GsubPlan,
    pub positioning_plan:GposPlan,
    pub units_per_em:u16,
    pub shaped:DualStageShapeResult,
}
/// Build both programs, both scalar mask namespaces and metrics from the
/// same immutable face. No implicit feature enabling or fallback.
pub fn shape_font_face_dual_stage(
    face:&FontFace<'_>,text:&str,o:FaceShapeOptions<'_>,l:FaceShapeLimits,
    substitution:FaceStageRequest<'_>,positioning:FaceStageRequest<'_>,
    ligatures:Option<JointLigatureLimits>,
)->Result<DualStageFaceShapeResult,Error>{
    let (source,substitution_plan,positioning_plan,units_per_em,shaped)=
        shape_font_face_execute(face,text,o,l,ligatures,|gs,gp,source,metrics,gd|{
            let sm=ScalarFeatureMasks::build(gs.validated_plan().ok_or(Error::Invalid("joint GSUB plan"))?,
                source.scalars().len(),substitution.overrides,substitution.masks)?;
            let pm=GposScalarFeatureMasks::build(gp.validated_plan().ok_or(Error::Invalid("joint GPOS plan"))?,
                source.scalars().len(),positioning.overrides,positioning.masks)?;
            let traced=match ligatures{
                Some(ll)=>Some((gd.ok_or(Error::Unsupported("face shape missing GDEF"))?,ll)),
                None=>None,
            };
            execute_dual_stage_joint_shape(gs,gp,source,metrics,o.alternates,&sm,substitution.specs,
                o.flow,l.joint,substitution.stages,traced,JointPositionStages{
                    masks:&pm,specs:positioning.specs,limits:positioning.stages})
        })?;
    Ok(DualStageFaceShapeResult{source,substitution_plan,positioning_plan,units_per_em,shaped})
}
