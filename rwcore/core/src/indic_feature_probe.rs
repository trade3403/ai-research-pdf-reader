//! Isolated font-evidence probes for conservative Devanagari rphf/half
//! candidates. No full Indic feature schedule or actual base/reph placement.
use crate::*;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicProbeKind{Reph,Half}
impl IndicProbeKind{
    pub fn tag(self)->[u8;4]{match self{Self::Reph=>*b"rphf",Self::Half=>*b"half"}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct IndicFeatureCandidate{
    pub kind:IndicProbeKind,pub scalar_start:usize,pub scalar_end:usize,
    pub base_candidate:usize,
}
#[derive(Debug,Clone,Copy,Default)]
pub struct IndicCandidateLimits{
    pub reorder:IndicReorderLimits,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct IndicFeatureCandidates{
    pub plan:IndicReorderPlan,pub candidates:Vec<IndicFeatureCandidate>,
}
/// Enumerate disjoint logical probe ranges. Skip terminal virama,
/// explicit join controls, broken and unsupported clusters. The last
/// admitted consonant is only a provisional base, not a font decision.
/// The additional scan is linear, bounded by the reorder scalar cap.
pub fn devanagari_feature_candidates(text:&str,p:&IndicProperties,s:&UnicodeScriptProperties,
    l:IndicCandidateLimits)->Result<IndicFeatureCandidates,Error>{
    let plan=plan_devanagari_reorder(text,p,s,l.reorder)?;
    let mut candidates=Vec::new();
    candidates.try_reserve_exact(plan.logical.scalars.len()).map_err(|_|Error::Limit("Indic candidate allocation"))?;
    for (c,r) in plan.logical.clusters.iter().zip(&plan.clusters){
        if c.kind!=IndicClusterKind::Consonant||c.terminal_virama||
            matches!(r.status,IndicReorderStatus::Unsupported|IndicReorderStatus::Broken){continue;}
        let Some(base)=r.base_candidate else{continue;};
        let a=c.scalar_start;let b=c.scalar_end;let chars=&plan.logical.scalars;
        if chars[a..b].iter().any(|x|matches!(x.character,'\u{200c}'|'\u{200d}')){continue;}
        let mut at=a;
        if let IndicRephCandidate::Prefix{ra,virama}=r.reph{
            candidates.push(IndicFeatureCandidate{kind:IndicProbeKind::Reph,scalar_start:ra,scalar_end:virama+1,base_candidate:base});
            at=virama+1;
        }
        while at<base{
            let start=at;
            if chars[at].syllabic!=IndicSyllabicCategory::Consonant{break;}
            at+=1;
            if at<base&&chars[at].syllabic==IndicSyllabicCategory::Nukta{at+=1;}
            if at>=base||chars[at].syllabic!=IndicSyllabicCategory::Virama{break;}
            at+=1;
            candidates.push(IndicFeatureCandidate{kind:IndicProbeKind::Half,scalar_start:start,scalar_end:at,base_candidate:base});
        }
    }
    Ok(IndicFeatureCandidates{plan,candidates})
}
#[derive(Debug,Clone,Copy)]
pub struct IndicProbeLimits{
    pub candidates:IndicCandidateLimits,pub masks:FeatureMaskLimits,pub stages:JointStageLimits,
    pub gsub:GsubProgramExecLimits,pub provenance:ProvenanceLimits,
    pub max_evidence_work:usize,
}
impl Default for IndicProbeLimits{
    fn default()->Self{Self{candidates:Default::default(),masks:Default::default(),stages:Default::default(),
        gsub:Default::default(),provenance:Default::default(),max_evidence_work:1048576}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum IndicFeatureEvidence{Unchanged,Changed,WholeRangeContraction}
#[derive(Debug)]
pub struct IndicProbeResult{
    pub candidate:IndicFeatureCandidate,pub evidence:IndicFeatureEvidence,
    pub masks:ScalarFeatureMasks,pub result:MaskedGsubResult,
}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit("Indic probe evidence work"))?;Ok(())
}
/// Execute precisely one candidate with only its selected tag. All other
/// selected tags (including unrelated required features) reject the probe.
/// Context may inspect neighboring input, but strict masks restrict edits.
/// Evidence is an observed net glyph change, not proof of semantic shape.
/// Layout and program must be from the same font; use the face wrapper.
pub fn probe_devanagari_feature(layout:&GsubLayout,program:&CompiledGsubProgram<'_>,run:&SourceRun,
    text:&str,p:&IndicProperties,s:&UnicodeScriptProperties,kind:IndicProbeKind,
    candidate_index:usize,l:IndicProbeLimits)->Result<IndicProbeResult,Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("Indic probe unvalidated plan"))?;
    if !matches!(&plan.script,b"dev2"|b"deva"){return Err(Error::Unsupported("Indic probe script"));}
    if plan.features.is_empty(){return Err(Error::Unsupported("Indic probe missing feature"));}
    if plan.features.len()>64{return Err(Error::Limit("Indic probe features"));}
    let candidates=devanagari_feature_candidates(text,p,s,l.candidates)?;
    let c=*candidates.candidates.get(candidate_index).ok_or(Error::Invalid("Indic probe candidate index"))?;
    if c.kind!=kind{return Err(Error::Invalid("Indic probe candidate kind"));}
    let n=run.scalars().len();
    if run.byte_len()!=text.len()||n!=candidates.plan.logical.scalars.len()||run.glyphs().len()!=n||
        run.scalars().iter().zip(&candidates.plan.logical.scalars).any(|(a,b)|
            (a.character,a.byte_start,a.byte_end)!=(b.character,b.byte_start,b.byte_end))||
        run.glyphs().iter().enumerate().any(|(i,g)|g.scalar_index!=i||g.missing){
        return Err(Error::Invalid("Indic probe source identity"));
    }
    let mut overrides=Vec::new();
    overrides.try_reserve_exact(plan.features.len()*2).map_err(|_|Error::Limit("Indic probe allocation"))?;
    for &id in &plan.features{
        if layout.features.get(id as usize).ok_or(Error::Invalid("Indic probe feature index"))?.tag!=kind.tag(){
            return Err(Error::Unsupported("Indic probe unrelated feature"));
        }
        if c.scalar_start>0{overrides.push(FeatureMaskOverride{feature:id,start:0,end:c.scalar_start,enabled:false});}
        if c.scalar_end<n{overrides.push(FeatureMaskOverride{feature:id,start:c.scalar_end,end:n,enabled:false});}
    }
    let masks=ScalarFeatureMasks::build(plan,n,&overrides,l.masks)?;
    let specs=[ShapeStageSpec{features:plan.features.clone()}];
    let (_,result,_,_)=execute_staged_gsub_traced(program,run,&[],&masks,&specs,l.stages.schedule,
        l.gsub,l.provenance,l.stages.max_mask_work)?;
    let mut left=l.max_evidence_work;
    charge(&mut left,n)?;charge(&mut left,result.provenance.glyphs().len())?;
    charge(&mut left,result.provenance.reference_count())?;
    let mut seen=Vec::new();seen.try_reserve_exact(n).map_err(|_|Error::Limit("Indic probe allocation"))?;seen.resize(n,false);
    let mut inside=0usize;let mut changed=false;
    for g in result.provenance.glyphs(){
        let src=g.sources();let first=*src.first().ok_or(Error::Invalid("Indic probe empty source"))?;
        let in_range=(c.scalar_start..c.scalar_end).contains(&first);
        for &i in src{
            if i>=n||in_range!=(c.scalar_start..c.scalar_end).contains(&i){
                return Err(Error::Invalid("Indic probe mixed source"));
            }
            if !in_range&&seen[i]{return Err(Error::Invalid("Indic probe outside expansion"));}
            seen[i]=true;
        }
        if in_range{
            inside+=1;
            if src.len()!=1||g.glyph_id!=run.glyphs()[first].glyph_id{changed=true;}
        }else if src.len()!=1||g.glyph_id!=run.glyphs()[first].glyph_id{
            return Err(Error::Invalid("Indic probe outside change"));
        }
    }
    if seen.contains(&false){return Err(Error::Invalid("Indic probe missing source"));}
    let span=c.scalar_end-c.scalar_start;
    let evidence=if inside==1&&span>1{IndicFeatureEvidence::WholeRangeContraction}
        else if changed||inside!=span{IndicFeatureEvidence::Changed}
        else{IndicFeatureEvidence::Unchanged};
    Ok(IndicProbeResult{candidate:c,evidence,masks,result})
}
/// Native same-face isolated probe with actual dev2/deva LangSys selection.
/// No fallback to a different script and no missing-glyph substitution.
pub fn probe_font_face_devanagari(face:&FontFace<'_>,text:&str,p:&IndicProperties,s:&UnicodeScriptProperties,
    script:[u8;4],language:Option<[u8;4]>,kind:IndicProbeKind,candidate_index:usize,
    face_limits:FaceShapeLimits,l:IndicProbeLimits)->Result<IndicProbeResult,Error>{
    if !matches!(&script,b"dev2"|b"deva"){return Err(Error::Unsupported("Indic probe script"));}
    let b=face.table(*b"GSUB").ok_or(Error::Unsupported("Indic probe missing GSUB"))?;
    let cmap=parse_unicode_cmap(face,face_limits.cmap)?;
    let run=map_source_run(text,&cmap,false,face_limits.source)?;
    let n=u32::from(cmap.glyph_count());
    let gd=face.table(*b"GDEF").map(|b|parse_gdef_with_mark_set_limits(b,n,face_limits.gdef,face_limits.mark_sets)).transpose()?;
    let layout=parse_gsub_layout(b,face_limits.substitution_program.compile.layout)?;
    let plan=build_gsub_base_plan(&layout,script,language,&[kind.tag()],face_limits.substitution_plan)?
        .ok_or(Error::Unsupported("Indic probe missing language"))?;
    if plan.script!=script||!plan.unavailable_tags.is_empty(){return Err(Error::Unsupported("Indic probe missing native feature"));}
    let program=compile_gsub_plan_program(b,&plan,n,gd.as_ref(),face_limits.substitution_program)?;
    probe_devanagari_feature(&layout,&program,&run,text,p,s,kind,candidate_index,l)
}
