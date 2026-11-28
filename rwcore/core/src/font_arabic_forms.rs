//! Basic Arabic four-form joining mask bridge into the validated GSUB plan.
//! No implicit feature selection, Syriac forms or complete script shaping.
use crate::*;
#[derive(Debug,Clone,Copy,Default)]
pub struct ArabicFormLimits{
    pub joining:JoiningAnalysisLimits,
    pub masks:FeatureMaskLimits,
    pub stages:JointStageLimits,
}
#[derive(Debug)]
pub struct ArabicFormPlan{
    pub joining:Vec<JoiningScalar>,
    pub masks:ScalarFeatureMasks,
    pub specs:Vec<ShapeStageSpec>,
}
fn is_form(tag:[u8;4])->bool{matches!(&tag,b"isol"|b"init"|b"medi"|b"fina")}
/// The layout must be the original layout associated with the compiled
/// program. The same-face entry point below establishes that contract.
/// Every selected four-form feature is enabled only for its own scalar
/// form; ordinary features retain their default masks. Missing optional
/// forms are not synthesized. Required forms incompatible with the input
/// are rejected by the existing required-mask policy.
pub fn build_arabic_form_plan(layout:&GsubLayout,program:&CompiledGsubProgram<'_>,
    run:&SourceRun,text:&str,data:&UnicodeJoiningTypes,l:ArabicFormLimits)->Result<ArabicFormPlan,Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("Arabic form unvalidated plan"))?;
    if plan.script!=*b"arab"{return Err(Error::Unsupported("Arabic form script"));}
    let n=run.scalars().len();
    if n>l.masks.max_scalars||plan.features.len()>64{return Err(Error::Limit("Arabic form size"));}
    if text.len()!=run.byte_len(){return Err(Error::Invalid("Arabic form source bytes"));}
    let joining=analyze_arabic_joining(text,data,l.joining)?;
    if joining.len()!=n||joining.iter().zip(run.scalars()).any(|(a,b)|
        (a.character,a.byte_start,a.byte_end)!=(b.character,b.byte_start,b.byte_end)){
        return Err(Error::Invalid("Arabic form source identity"));
    }
    let mut overrides=Vec::new();let mut left=l.masks.max_work;
    for &id in &plan.features{
        let f=layout.features.get(id as usize).ok_or(Error::Invalid("Arabic form feature index"))?;
        if matches!(&f.tag,b"fin2"|b"fin3"|b"med2"){return Err(Error::Unsupported("Arabic special joining form"));}
        if !is_form(f.tag){continue;}
        left=left.checked_sub(n).ok_or(Error::Limit("Arabic form mask work"))?;
        let mut at=0;
        while at<n{
            if joining[at].form.feature()==Some(f.tag){at+=1;continue;}
            let start=at;at+=1;
            while at<n&&joining[at].form.feature()!=Some(f.tag){at+=1;}
            if overrides.len()>=l.masks.max_overrides{return Err(Error::Limit("Arabic form overrides"));}
            overrides.try_reserve(1).map_err(|_|Error::Limit("Arabic form allocation"))?;
            overrides.push(FeatureMaskOverride{feature:id,start,end:at,enabled:false});
        }
    }
    // The generation and mask construction share one budget.
    let masks=ScalarFeatureMasks::build(plan,n,&overrides,FeatureMaskLimits{max_work:left,..l.masks})?;
    let specs=shape_stage_specs_from_tags(layout,program,&ShapeScriptProfile::Arabic.groups(),l.stages.schedule)?;
    Ok(ArabicFormPlan{joining,masks,specs})
}
