//! GPOS base feature planning. Shared OpenType Layout selection semantics;
//! distinct plan type prevents accidental use of GSUB plans.
use crate::{Error,GposLayout,GsubPlan,GsubPlanLimits,build_gsub_base_plan};
pub type GposPlanLimits=GsubPlanLimits;
pub use crate::GsubPlannedLookup as GposPlannedLookup;
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposPlan{
    pub script:[u8;4],
    pub language:Option<[u8;4]>,
    pub required_feature:Option<u16>,
    pub features:Vec<u16>,
    pub unavailable_tags:Vec<[u8;4]>,
    pub lookups:Vec<GposPlannedLookup>,
    pub base_only_variations_present:bool,
}
impl From<GsubPlan> for GposPlan{
    fn from(p:GsubPlan)->Self{Self{script:p.script,language:p.language,required_feature:p.required_feature,features:p.features,unavailable_tags:p.unavailable_tags,lookups:p.lookups,base_only_variations_present:p.base_only_variations_present}}
}
/// Base features only. Required feature always selected, optional tags explicit.
/// Lookup indices are ascending unique, retaining contributing feature indices.
pub fn build_gpos_base_plan(layout:&GposLayout,script:[u8;4],language:Option<[u8;4]>,enabled:&[[u8;4]],l:GposPlanLimits)->Result<Option<GposPlan>,Error>{
    Ok(build_gsub_base_plan(layout,script,language,enabled,l)?.map(GposPlan::from))
}
