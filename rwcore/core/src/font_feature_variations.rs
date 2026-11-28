//! Bounded OpenType FeatureVariations 1.0 decoding and selection.
//! Coordinates are already normalized (including any avar mapping).
//! See https://learn.microsoft.com/en-us/typography/opentype/otspec190/chapter2
use crate::{Error,GsubLimits,parse_gsub_directory};

#[derive(Debug,Clone,Copy)]
pub struct FeatureVariationLimits{
    pub max_bytes:usize,
    pub max_axes:usize,
    pub max_records:usize,
    pub max_conditions:usize,
    pub max_substitutions:usize,
    pub max_lookup_refs:usize,
}
impl Default for FeatureVariationLimits{
    fn default()->Self{Self{max_bytes:32*1024*1024,max_axes:256,
        max_records:4096,max_conditions:65536,max_substitutions:65536,max_lookup_refs:262144}}
}
/// Format 1 axis interval; values are signed F2DOT14 integers.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct VariationAxisRange{pub axis_index:u16,pub min:i16,pub max:i16}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum FeatureVariationCondition{
    AxisRange(VariationAxisRange),
    /// An unsupported condition format never matches.
    Unsupported(u16),
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct AlternateFeatureTable{
    pub feature_index:u16,
    /// Offset relative to the supplied FeatureVariations body, if present.
    /// Parameter payload is opaque; only its offset is checked.
    pub parameters:Option<usize>,
    pub lookups:Vec<u16>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FeatureVariationRecord{
    conditions:Vec<FeatureVariationCondition>,
    substitutions:Option<Vec<AlternateFeatureTable>>,
}
impl FeatureVariationRecord{
    pub fn conditions(&self)->&[FeatureVariationCondition]{&self.conditions}
    /// None means unsupported substitution version, not a valid empty table.
    pub fn substitutions(&self)->Option<&[AlternateFeatureTable]>{self.substitutions.as_deref()}
    pub fn alternate(&self,feature_index:u16)->Option<&AlternateFeatureTable>{
        let s=self.substitutions.as_ref()?;
        s.binary_search_by_key(&feature_index,|v|v.feature_index).ok().map(|i|&s[i])
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FeatureVariations{
    axis_count:usize,
    records:Vec<FeatureVariationRecord>,
}
impl FeatureVariations{
    pub fn axis_count(&self)->usize{self.axis_count}
    pub fn records(&self)->&[FeatureVariationRecord]{&self.records}
    /// Return the first matching record with a supported substitution
    /// version. Empty conditions match all; a null substitution offset
    /// is a valid empty replacement set and terminates selection.
    /// Invalid axis indices and unknown condition formats do not match.
    pub fn select(&self,coordinates:&[i16])->Result<Option<(usize,&FeatureVariationRecord)>,Error>{
        if coordinates.len()!=self.axis_count||coordinates.iter().any(|&v|!(-16384..=16384).contains(&v)){
            return Err(Error::Invalid("feature variations coordinates"));
        }
        Ok(self.records.iter().enumerate().find(|(_,r)|r.substitutions.is_some()&&
            r.conditions.iter().all(|c|match c{
                FeatureVariationCondition::AxisRange(c)=>coordinates.get(c.axis_index as usize)
                    .is_some_and(|&v|c.min<=v&&v<=c.max),
                FeatureVariationCondition::Unsupported(_)=>false,
            })))
    }
}
fn slice(b:&[u8],at:usize,n:usize)->Result<&[u8],Error>{
    let end=at.checked_add(n).ok_or(Error::Limit("feature variations offset"))?;
    b.get(at..end).ok_or(Error::Invalid("feature variations truncated"))
}
fn word(b:&[u8],at:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(slice(b,at,2)?.try_into().unwrap()))}
fn long(b:&[u8],at:usize)->Result<usize,Error>{
    usize::try_from(u32::from_be_bytes(slice(b,at,4)?.try_into().unwrap()))
        .map_err(|_|Error::Limit("feature variations offset"))
}
fn size(header:usize,n:usize,stride:usize)->Result<usize,Error>{
    n.checked_mul(stride).and_then(|v|header.checked_add(v)).ok_or(Error::Limit("feature variations size"))
}
fn offset(b:&[u8],base:usize,rel:usize,min:usize,need:usize)->Result<usize,Error>{
    if rel<min{return Err(Error::Invalid("feature variations offset into header"));}
    let at=base.checked_add(rel).ok_or(Error::Limit("feature variations offset"))?;
    slice(b,at,need)?;Ok(at)
}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit("feature variations records"))?;Ok(())
}
fn reserve<T>(out:&mut Vec<T>,n:usize)->Result<(),Error>{
    out.try_reserve_exact(n).map_err(|_|Error::Limit("feature variations allocation"))
}
fn conditions(b:&[u8],p:usize,budget:&mut usize)->Result<Vec<FeatureVariationCondition>,Error>{
    let n=word(b,p)? as usize;charge(budget,n)?;
    let header=size(2,n,4)?;slice(b,p,header)?;
    let mut out=Vec::new();reserve(&mut out,n)?;
    for i in 0..n{
        let at=offset(b,p,long(b,p+2+4*i)?,header,2)?;
        let format=word(b,at)?;
        if format!=1{out.push(FeatureVariationCondition::Unsupported(format));continue;}
        slice(b,at,8)?;
        let c=VariationAxisRange{axis_index:word(b,at+2)?,min:word(b,at+4)? as i16,max:word(b,at+6)? as i16};
        if c.min < -16384||c.max>16384||c.min>c.max{return Err(Error::Invalid("feature variations axis interval"));}
        out.push(FeatureVariationCondition::AxisRange(c));
    }
    Ok(out)
}
fn substitutions(b:&[u8],p:usize,features:usize,lookups:usize,
    substitutions_left:&mut usize,refs_left:&mut usize)->Result<Option<Vec<AlternateFeatureTable>>,Error>{
    slice(b,p,4)?;
    if word(b,p)?!=1||word(b,p+2)?!=0{return Ok(None);}
    let n=word(b,p+4)? as usize;charge(substitutions_left,n)?;
    let header=size(6,n,6)?;slice(b,p,header)?;
    let mut out:Vec<AlternateFeatureTable>=Vec::new();reserve(&mut out,n)?;
    for i in 0..n{
        let q=p+6+6*i;
        let feature_index=word(b,q)?;
        if feature_index as usize>=features||out.last().is_some_and(|r|r.feature_index>=feature_index){
            return Err(Error::Invalid("feature variations feature index or order"));
        }
        let at=offset(b,p,long(b,q+2)?,header,4)?;
        let count=word(b,at+2)? as usize;charge(refs_left,count)?;
        let length=size(4,count,2)?;slice(b,at,length)?;
        let rel=word(b,at)? as usize;
        let parameters=if rel==0{None}else{Some(offset(b,at,rel,length,1)?)};
        let mut indices=Vec::new();reserve(&mut indices,count)?;
        for j in 0..count{
            let v=word(b,at+4+2*j)?;
            if v as usize>=lookups{return Err(Error::Invalid("feature variations lookup index"));}
            indices.push(v);
        }
        out.push(AlternateFeatureTable{feature_index,parameters,lookups:indices});
    }
    Ok(Some(out))
}
/// Decode a FeatureVariations body with the caller's fvar axis count,
/// FeatureList count and LookupList count. Unknown condition formats
/// and unsupported substitution versions are retained as nonmatching
/// records, not interpreted. Malformed known tables fail closed.
/// Aliased child tables consume the cumulative budgets per reference.
pub fn parse_feature_variations(b:&[u8],axis_count:usize,feature_count:usize,
    lookup_count:usize,l:FeatureVariationLimits)->Result<FeatureVariations,Error>{
    if b.len()>l.max_bytes||axis_count>l.max_axes{return Err(Error::Limit("feature variations size"));}
    if axis_count>65535||feature_count>65535||lookup_count>65535{return Err(Error::Invalid("feature variations domain"));}
    if word(b,0)?!=1||word(b,2)?!=0{return Err(Error::Unsupported("feature variations version"));}
    let n=long(b,4)?;
    if n>l.max_records{return Err(Error::Limit("feature variations records"));}
    let header=size(8,n,8)?;slice(b,0,header)?;
    let(mut cond_left,mut sub_left,mut refs_left)=(l.max_conditions,l.max_substitutions,l.max_lookup_refs);
    let mut records=Vec::new();reserve(&mut records,n)?;
    for i in 0..n{
        let p=8+8*i;
        let c=long(b,p)?;let s=long(b,p+4)?;
        let conditions=if c==0{Vec::new()}else{conditions(b,offset(b,0,c,header,2)?,&mut cond_left)?};
        let substitutions=if s==0{Some(Vec::new())}else{
            substitutions(b,offset(b,0,s,header,4)?,feature_count,lookup_count,&mut sub_left,&mut refs_left)?
        };
        records.push(FeatureVariationRecord{conditions,substitutions});
    }
    Ok(FeatureVariations{axis_count,records})
}
/// Obtain the FeatureVariations body from an actual GSUB directory,
/// preserving the original FeatureList and LookupList index domains.
/// Does not modify base feature plans or compile alternate lookups.
pub fn parse_gsub_feature_variations(b:&[u8],axis_count:usize,layout_limits:GsubLimits,
    limits:FeatureVariationLimits)->Result<Option<FeatureVariations>,Error>{
    let d=parse_gsub_directory(b,layout_limits)?;
    d.feature_variations.map(|p|parse_feature_variations(&b[p..],axis_count,d.features.len(),d.lookups.len(),limits)).transpose()
}

/// A variation-aware single base-feature pass. The embedded plan has
/// base_only_variations_present=false because selection was evaluated;
/// use compile_gsub_variation_plan_program, not the legacy base API.
/// Indices in replaced_features refer only to selected LangSys features.
/// Alternate parameter payloads remain opaque.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubVariationPlan{
    pub plan:crate::GsubPlan,
    pub axis_count:usize,
    pub coordinates:Vec<i16>,
    pub variations_present:bool,
    pub matched_record:Option<usize>,
    pub replaced_features:Vec<u16>,
}
pub(crate) fn variations_for_layout(b:&[u8],directory:&crate::GsubDirectory,
    axis_count:usize,coordinates:&[i16],limits:FeatureVariationLimits)->Result<Option<FeatureVariations>,Error>{
    if axis_count>limits.max_axes{return Err(Error::Limit("feature variations axes"));}
    if axis_count>65535||coordinates.len()!=axis_count||
        coordinates.iter().any(|&v|!(-16384..=16384).contains(&v)){
        return Err(Error::Invalid("feature variations coordinates"));
    }
    directory.feature_variations.map(|p|parse_feature_variations(&b[p..],axis_count,
        directory.features.len(),directory.lookups.len(),limits)).transpose()
}
/// Build a dynamic plan directly from original GSUB bytes. Script and
/// language fallback, required feature membership, tag selection and
/// ordered lookup deduplication are shared with the base planner.
/// Only the first selected record supplies alternate feature tables;
/// a missing replacement keeps the original feature. No axis
/// normalization, feature values, masks or script staging here.
pub fn build_gsub_variation_plan(b:&[u8],script:[u8;4],language:Option<[u8;4]>,
    enabled:&[[u8;4]],axis_count:usize,coordinates:&[i16],layout_limits:GsubLimits,
    variation_limits:FeatureVariationLimits,plan_limits:crate::GsubPlanLimits)->Result<Option<GsubVariationPlan>,Error>{
    let layout=crate::parse_gsub_layout(b,layout_limits)?;
    let variations=variations_for_layout(b,&layout.directory,axis_count,coordinates,variation_limits)?;
    let selected=match &variations{Some(v)=>v.select(coordinates)?,None=>None};
    let record=selected.map(|(_,r)|r);
    let Some(plan)=crate::font_gsub_plan::build_gsub_plan_inner(&layout,script,language,enabled,plan_limits,record,true)?
        else{return Ok(None);};
    let replaced_features=plan.features.iter().copied().filter(|&id|
        record.and_then(|r|r.alternate(id)).is_some()).collect();
    Ok(Some(GsubVariationPlan{plan,axis_count,coordinates:coordinates.to_vec(),
        variations_present:variations.is_some(),matched_record:selected.map(|(i,_)|i),replaced_features}))
}
