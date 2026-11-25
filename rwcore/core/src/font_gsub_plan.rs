//! GSUB base feature selection and ordered lookup planning.
//! No script-specific shaping stages, feature variations, or lookup execution.
use crate::{Error,GsubLayout,GsubLangSys};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Clone,Copy)]
pub struct GsubPlanLimits{
    pub max_scan_records:usize,
    pub max_requested_tags:usize,
    pub max_features:usize,
    pub max_lookup_refs:usize,
    pub max_lookups:usize,
}
impl Default for GsubPlanLimits{
    fn default()->Self{Self{max_scan_records:65536,max_requested_tags:1024,max_features:4096,max_lookup_refs:65536,max_lookups:16384}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubPlannedLookup{
    pub index:u16,
    /// Sorted unique contributing feature indices.
    pub features:Vec<u16>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubPlan{
    pub script:[u8;4],
    /// None means the selected script's DefaultLangSys.
    pub language:Option<[u8;4]>,
    pub required_feature:Option<u16>,
    /// Required and selected optional feature indices, sorted and unique.
    pub features:Vec<u16>,
    /// Requested tags unavailable in the selected LangSys, sorted and unique.
    pub unavailable_tags:Vec<[u8;4]>,
    /// Ascending unique LookupList indices, with feature provenance.
    pub lookups:Vec<GsubPlannedLookup>,
    /// True when only the base feature tables were used despite a present
    /// FeatureVariations table. No variation-condition evaluation here.
    pub base_only_variations_present:bool,
}
fn charge(left:&mut usize,n:usize,what:&'static str)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit(what))?;Ok(())
}
fn select_language<'a>(script:&'a crate::GsubScript,requested:Option<[u8;4]>,scan:&mut usize)
    ->Result<Option<(&'a GsubLangSys,Option<[u8;4]>)>,Error>{
    charge(scan,script.languages.len(),"gsub plan scan")?;
    // Public structs can be caller-built: reject duplicate tags rather than
    // silently picking one. Sorted order is not assumed by this API.
    let mut tags=BTreeSet::new();
    let mut selected=None;
    for (tag,lang) in &script.languages{
        if !tags.insert(*tag){return Err(Error::Invalid("gsub plan duplicate language"));}
        if Some(*tag)==requested{selected=Some((lang,Some(*tag)));}
    }
    Ok(selected.or_else(||script.default_language.as_ref().map(|l|(l,None))))
}
/// Select exact script or DFLT if the requested script is absent; never
/// fall back to an arbitrary script or language. Exact language or selected
/// script's default; None if no eligible script/language exists.
/// Optional feature tags are explicitly enabled by the caller; the required
/// feature is always enabled. All matching feature records in the selected
/// LangSys are included (tags may repeat). Shared lookup indices execute once
/// in LookupList order for this single base-feature pass. Feature-specific
/// values, masks, script-dependent stages and FeatureVariations are separate.
pub fn build_gsub_base_plan(layout:&GsubLayout,script:[u8;4],language:Option<[u8;4]>,
    enabled:&[[u8;4]],l:GsubPlanLimits)->Result<Option<GsubPlan>,Error>{
    if enabled.len()>l.max_requested_tags{return Err(Error::Limit("gsub plan requested tags"));}
    let mut scan=l.max_scan_records;
    charge(&mut scan,layout.scripts.len(),"gsub plan scan")?;
    let mut tags=BTreeSet::new();
    let mut exact=None;
    let mut fallback=None;
    for s in &layout.scripts{
        if !tags.insert(s.tag){return Err(Error::Invalid("gsub plan duplicate script"));}
        if s.tag==script{exact=Some(s);}
        if s.tag==*b"DFLT"{fallback=Some(s);}
    }
    let Some(s)=exact.or(fallback)else{return Ok(None);};
    let Some((lang,actual_language))=select_language(s,language,&mut scan)? else{return Ok(None);};
    charge(&mut scan,lang.features.len(),"gsub plan scan")?;
    charge(&mut scan,usize::from(lang.required_feature.is_some()),"gsub plan scan")?;
    let requested:BTreeSet<_>=enabled.iter().copied().collect();
    let mut available=BTreeSet::new();
    let mut features=BTreeSet::new();
    for id in lang.required_feature.into_iter().chain(lang.features.iter().copied()){
        let feature=layout.features.get(usize::from(id)).ok_or(Error::Invalid("gsub plan feature index"))?;
        available.insert(feature.tag);
        if Some(id)==lang.required_feature||requested.contains(&feature.tag){
            features.insert(id);
            if features.len()>l.max_features{return Err(Error::Limit("gsub plan features"));}
        }
    }
    let mut refs=l.max_lookup_refs;
    let mut ordered:BTreeMap<u16,BTreeSet<u16>>=BTreeMap::new();
    for &id in &features{
        let feature=&layout.features[usize::from(id)];
        charge(&mut refs,feature.lookups.len(),"gsub plan lookup refs")?;
        for &index in &feature.lookups{
            if usize::from(index)>=layout.lookups.len(){return Err(Error::Invalid("gsub plan lookup index"));}
            ordered.entry(index).or_default().insert(id);
            if ordered.len()>l.max_lookups{return Err(Error::Limit("gsub plan lookups"));}
        }
    }
    Ok(Some(GsubPlan{
        script:s.tag,language:actual_language,required_feature:lang.required_feature,
        features:features.into_iter().collect(),
        unavailable_tags:requested.difference(&available).copied().collect(),
        lookups:ordered.into_iter().map(|(index,features)|GsubPlannedLookup{index,features:features.into_iter().collect()}).collect(),
        base_only_variations_present:layout.directory.feature_variations.is_some(),
    }))
}
