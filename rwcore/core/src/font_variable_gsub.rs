//! Same-face variable GSUB integration. No outline interpolation,
//! complete Unicode shaping or GPOS positioning.
use crate::{Error,FontFace,FontMetricLimits,Fvar,FvarLimits,Avar,AvarLimits,Gdef,GdefLimits,
    MarkGlyphSetLimits,GsubPlanLimits,GsubProgramLimits,FeatureVariationLimits,GsubVariationPlan,
    CompiledGsubProgram,GsubAlternateSelection,GsubProgramExecLimits,GsubProgramExecResult,
    parse_fvar,parse_avar,parse_gdef_with_mark_set_limits,normalize_variation_coordinates,
    normalize_tagged_variation_coordinates,build_gsub_variation_plan,
    compile_gsub_variation_plan_program,execute_gsub_program};

#[derive(Debug,Clone,Copy,Default)]
pub struct VariableGsubLimits{
    pub metrics:FontMetricLimits,pub fvar:FvarLimits,pub avar:AvarLimits,
    pub gdef:GdefLimits,pub mark_sets:MarkGlyphSetLimits,
    pub variations:FeatureVariationLimits,pub plan:GsubPlanLimits,
    pub program:GsubProgramLimits,pub execution:GsubProgramExecLimits,
}
/// Ordered user Fixed 16.16 values, or explicit tags with missing defaults.
#[derive(Debug,Clone,Copy)]
pub enum VariationUserCoordinates<'a>{Ordered(&'a [i32]),Tagged(&'a [([u8;4],i32)])}
/// All selection inputs are explicit. rvrn and other optional tags are
/// not implicitly enabled by this single-pass API.
#[derive(Debug,Clone,Copy)]
pub struct VariableGsubRequest<'a>{
    pub script:[u8;4],pub language:Option<[u8;4]>,
    pub enabled:&'a [[u8;4]],pub coordinates:VariationUserCoordinates<'a>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct VariableGsubResult{
    pub plan:GsubVariationPlan,pub execution:GsubProgramExecResult,
}
/// Owns parsed variation axes and GDEF while borrowing GSUB from the
/// same validated SFNT/TTC face. A compiled program borrows this object's
/// GDEF. No table checksum or cryptographic identity guarantee.
#[derive(Debug)]
pub struct VariableGsubFace<'a>{
    fvar:Fvar,avar:Option<Avar>,gdef:Option<Gdef>,gsub:&'a [u8],
    glyph_count:u32,limits:VariableGsubLimits,
}
pub fn parse_variable_gsub_face<'a>(face:&FontFace<'a>,limits:VariableGsubLimits)
    ->Result<VariableGsubFace<'a>,Error>{
    let required=|tag|face.table(tag).ok_or(Error::Invalid("variable GSUB missing table"));
    let maxp=crate::font_metrics::parse_maxp(required(*b"maxp")?,face.flavor(),limits.metrics)?;
    let glyph_count=u32::from(maxp.glyph_count);
    let fvar=parse_fvar(required(*b"fvar")?,limits.fvar)?;
    let avar=face.table(*b"avar").map(|b|parse_avar(b,fvar.axes().len(),limits.avar)).transpose()?;
    let gsub=required(*b"GSUB")?;
    crate::parse_gsub_directory(gsub,limits.program.compile.layout)?;
    let gdef=face.table(*b"GDEF").map(|b|parse_gdef_with_mark_set_limits(b,glyph_count,
        limits.gdef,limits.mark_sets)).transpose()?;
    Ok(VariableGsubFace{fvar,avar,gdef,gsub,glyph_count,limits})
}
impl VariableGsubFace<'_>{
    pub fn fvar(&self)->&Fvar{&self.fvar}
    pub fn avar(&self)->Option<&Avar>{self.avar.as_ref()}
    pub fn glyph_count(&self)->u32{self.glyph_count}
    pub fn normalize(&self,user:VariationUserCoordinates<'_>)->Result<Vec<i16>,Error>{
        match user{
            VariationUserCoordinates::Ordered(v)=>normalize_variation_coordinates(&self.fvar,self.avar.as_ref(),v),
            VariationUserCoordinates::Tagged(v)=>normalize_tagged_variation_coordinates(&self.fvar,self.avar.as_ref(),v),
        }
    }
    pub fn build_plan(&self,request:VariableGsubRequest<'_>)->Result<Option<GsubVariationPlan>,Error>{
        let coords=self.normalize(request.coordinates)?;
        build_gsub_variation_plan(self.gsub,request.script,request.language,request.enabled,
            self.fvar.axes().len(),&coords,self.limits.program.compile.layout,
            self.limits.variations,self.limits.plan)
    }
    fn compile_selected(&self,plan:&GsubVariationPlan)->Result<CompiledGsubProgram<'_>,Error>{
        compile_gsub_variation_plan_program(self.gsub,plan,self.fvar.axes().len(),&plan.coordinates,
            self.glyph_count,self.gdef.as_ref(),self.limits.program,self.limits.variations)
    }
    /// Rebuild the entire requested plan and reject stale or forged
    /// metadata, including selected feature IDs and unavailable tags.
    /// Compatible equivalent plans are allowed; no hash-based identity.
    pub fn compile_plan(&self,request:VariableGsubRequest<'_>,plan:&GsubVariationPlan)
        ->Result<CompiledGsubProgram<'_>,Error>{
        if self.build_plan(request)?.as_ref()!=Some(plan){
            return Err(Error::Invalid("variable GSUB plan mismatch"));
        }
        self.compile_selected(plan)
    }
    /// Build, compile and execute atomically on glyph IDs. None means
    /// no matching script/language; errors never return partial output.
    pub fn execute(&self,request:VariableGsubRequest<'_>,glyphs:&[u16],choices:&[GsubAlternateSelection])
        ->Result<Option<VariableGsubResult>,Error>{
        let Some(plan)=self.build_plan(request)? else{return Ok(None);};
        let program=self.compile_selected(&plan)?;
        let execution=execute_gsub_program(&program,glyphs,choices,self.limits.execution)?;
        Ok(Some(VariableGsubResult{plan,execution}))
    }
}

impl VariableGsubFace<'_>{
    /// Execute a caller-supplied nominal source run with the selected
    /// variable plan. Glyph-domain validation is not font identity proof;
    /// caller must map the source run with this same face's cmap.
    pub fn execute_with_provenance(&self,request:VariableGsubRequest<'_>,run:&crate::SourceRun,
        choices:&[GsubAlternateSelection],source_limits:crate::ProvenanceLimits)
        ->Result<Option<(VariableGsubResult,crate::ProvenanceLedger)>,Error>{
        let Some(plan)=self.build_plan(request)? else{return Ok(None);};
        let program=self.compile_selected(&plan)?;
        let (execution,ledger)=crate::execute_gsub_program_with_provenance(&program,run,choices,self.limits.execution,source_limits)?;
        Ok(Some((VariableGsubResult{plan,execution},ledger)))
    }
}

impl VariableGsubFace<'_>{
    /// Build the selected variation plan and masks together, then execute
    /// the conservative strict input policy. Overrides use selected feature
    /// IDs and scalar indices, not byte offsets. None means no language.
    /// The caller must supply nominal seeds from this same face's cmap.
    pub fn execute_with_strict_masks(&self,request:VariableGsubRequest<'_>,run:&crate::SourceRun,
        choices:&[GsubAlternateSelection],overrides:&[crate::FeatureMaskOverride],
        mask_limits:crate::FeatureMaskLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize)
        ->Result<Option<(GsubVariationPlan,crate::MaskedGsubResult,usize)>,Error>{
        let Some(plan)=self.build_plan(request)? else{return Ok(None);};
        let masks=crate::ScalarFeatureMasks::build(&plan.plan,run.scalars().len(),overrides,mask_limits)?;
        let program=self.compile_selected(&plan)?;
        let (result,blocked)=crate::execute_gsub_program_with_strict_masks(&program,run,choices,&masks,
            self.limits.execution,source_limits,max_mask_work)?;
        Ok(Some((plan,result,blocked)))
    }
}
