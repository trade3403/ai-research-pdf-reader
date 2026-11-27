//! Explicit, bounded stage scheduling from validated feature provenance.
//! A feature belongs to exactly one stage; a shared lookup runs once per
//! contributing stage, in ascending original LookupList order.
//! No implicit script-specific ordering or execution in this module.
use crate::{CompiledGsubProgram,Error,GsubPlannedLookup};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ShapeStageLimits{
    pub max_stages:usize,pub max_features:usize,pub max_roots:usize,
    pub max_refs:usize,pub max_work:usize,
}
impl Default for ShapeStageLimits{
    fn default()->Self{Self{max_stages:64,max_features:4096,max_roots:65536,max_refs:65536,max_work:4194304}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ShapeStageSpec{
    pub features:Vec<u16>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ShapeStage{
    features:Vec<u16>,lookups:Vec<GsubPlannedLookup>,
}
impl ShapeStage{
    pub fn features(&self)->&[u16]{&self.features}
    pub fn lookups(&self)->&[GsubPlannedLookup]{&self.lookups}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ShapeStageSchedule{
    stages:Vec<ShapeStage>,root_count:usize,reference_count:usize,work:usize,
}
impl ShapeStageSchedule{
    pub fn stages(&self)->&[ShapeStage]{&self.stages}
    pub fn root_count(&self)->usize{self.root_count}
    pub fn reference_count(&self)->usize{self.reference_count}
    pub fn work(&self)->usize{self.work}
}
fn charge(left:&mut usize,n:usize,label:&'static str)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit(label))?;Ok(())
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("shape stage allocation"))
}
/// The caller explicitly partitions every selected feature (including
/// required features) into nonempty stages. Feature order inside each
/// stage is canonical; stage order is caller-defined. Empty plans accept
/// an empty schedule. No feature silently disappears or repeats.
pub fn build_shape_stage_schedule(program:&CompiledGsubProgram<'_>,specs:&[ShapeStageSpec],
    limits:ShapeStageLimits)->Result<ShapeStageSchedule,Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("shape stage unvalidated plan"))?;
    build_schedule(&plan.features,&plan.lookups,specs,limits)
}
/// GPOS has its own feature IDs, even when lookup/stage record shapes coincide.
pub fn build_gpos_stage_schedule(program:&crate::CompiledGposProgram<'_>,specs:&[ShapeStageSpec],
    limits:ShapeStageLimits)->Result<ShapeStageSchedule,Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("gpos stage unvalidated plan"))?;
    build_schedule(&plan.features,&plan.lookups,specs,limits)
}
fn build_schedule(plan_features:&[u16],lookups:&[GsubPlannedLookup],specs:&[ShapeStageSpec],
    limits:ShapeStageLimits)->Result<ShapeStageSchedule,Error>{
    if specs.len()>limits.max_stages||plan_features.len()>limits.max_features{
        return Err(Error::Limit("shape stage size"));
    }
    let mut left=limits.max_work;
    charge(&mut left,specs.len(),"shape stage work")?;
    charge(&mut left,plan_features.len(),"shape stage work")?;
    let mut owners=Vec::new();reserve(&mut owners,plan_features.len())?;owners.resize(plan_features.len(),None);
    let mut stages=Vec::new();reserve(&mut stages,specs.len())?;
    for (i,s) in specs.iter().enumerate(){
        charge(&mut left,s.features.len(),"shape stage work")?;
        if s.features.is_empty()||s.features.len()>plan_features.len()||
            s.features.windows(2).any(|w|w[0]>=w[1]){
            return Err(Error::Invalid("shape stage feature order"));
        }
        let mut features=Vec::new();reserve(&mut features,s.features.len())?;
        for &id in &s.features{
            let j=plan_features.binary_search(&id).map_err(|_|Error::Invalid("shape stage unknown feature"))?;
            if owners[j].replace(i).is_some(){return Err(Error::Invalid("shape stage duplicate feature"));}
            features.push(id);
        }
        stages.push(ShapeStage{features,lookups:Vec::new()});
    }
    if owners.iter().any(Option::is_none){return Err(Error::Invalid("shape stage missing feature"));}
    let mut root_count=0usize;let mut reference_count=0usize;
    for entry in lookups{
        charge(&mut left,1,"shape stage work")?;
        for &id in &entry.features{
            charge(&mut left,1,"shape stage work")?;
            let j=plan_features.binary_search(&id).map_err(|_|Error::Invalid("shape stage feature provenance"))?;
            let owner=owners[j].ok_or(Error::Invalid("shape stage missing owner"))?;
            reference_count=reference_count.checked_add(1).ok_or(Error::Limit("shape stage refs"))?;
            if reference_count>limits.max_refs{return Err(Error::Limit("shape stage refs"));}
            let roots=&mut stages[owner].lookups;
            if roots.last().map(|r|r.index)!=Some(entry.index){
                root_count=root_count.checked_add(1).ok_or(Error::Limit("shape stage roots"))?;
                if root_count>limits.max_roots{return Err(Error::Limit("shape stage roots"));}
                roots.try_reserve(1).map_err(|_|Error::Limit("shape stage allocation"))?;
                roots.push(GsubPlannedLookup{index:entry.index,features:Vec::new()});
            }
            let refs=&mut roots.last_mut().ok_or(Error::Invalid("shape stage root"))?.features;
            refs.try_reserve(1).map_err(|_|Error::Limit("shape stage allocation"))?;
            refs.push(id);
        }
    }
    Ok(ShapeStageSchedule{stages,root_count,reference_count,work:limits.max_work-left})
}

/// Explicit tag groups in caller order. Missing tags are harmless, but
/// duplicate tags are rejected. All records sharing a selected tag
/// enter the same stage. Unlisted selected features enter a final stage.
/// The required feature is not moved implicitly.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ShapeTagStage{pub tags:Vec<[u8;4]>}
/// Layout must describe the same GSUB FeatureList as the validated plan.
/// This helper only assigns stages, never changes feature selection.
pub fn shape_stage_specs_from_tags(layout:&crate::GsubLayout,program:&CompiledGsubProgram<'_>,
    groups:&[ShapeTagStage],limits:ShapeStageLimits)->Result<Vec<ShapeStageSpec>,Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("shape tag unvalidated plan"))?;
    if groups.len()>limits.max_stages||plan.features.len()>limits.max_features{
        return Err(Error::Limit("shape tag size"));
    }
    let mut left=limits.max_work;
    charge(&mut left,groups.len(),"shape tag work")?;
    let mut tags=Vec::new();
    for (i,group) in groups.iter().enumerate(){
        if group.tags.is_empty(){return Err(Error::Invalid("shape tag empty group"));}
        charge(&mut left,group.tags.len(),"shape tag work")?;
        tags.try_reserve(group.tags.len()).map_err(|_|Error::Limit("shape tag allocation"))?;
        for &tag in &group.tags{tags.push((tag,i));}
    }
    // Charge a conservative comparison bound before sorting.
    let factor=(usize::BITS-tags.len().leading_zeros()) as usize;
    charge(&mut left,tags.len().checked_mul(factor+1).ok_or(Error::Limit("shape tag work"))?,"shape tag work")?;
    tags.sort_unstable();
    if tags.windows(2).any(|w|w[0].0==w[1].0){return Err(Error::Invalid("shape tag duplicate"));}
    let count=groups.len().checked_add(1).ok_or(Error::Limit("shape tag size"))?;
    let mut out=Vec::new();reserve(&mut out,count)?;
    out.resize_with(count,||ShapeStageSpec{features:Vec::new()});
    for &id in &plan.features{
        charge(&mut left,factor+1,"shape tag work")?;
        let tag=layout.features.get(id as usize).ok_or(Error::Invalid("shape tag feature index"))?.tag;
        let at=tags.binary_search_by_key(&tag,|x|x.0).ok().map_or(groups.len(),|j|tags[j].1);
        out[at].features.try_reserve(1).map_err(|_|Error::Limit("shape tag allocation"))?;
        out[at].features.push(id);
    }
    out.retain(|s|!s.features.is_empty());
    if out.len()>limits.max_stages{return Err(Error::Limit("shape tag stages"));}
    Ok(out)
}

 /// Explicit experimental ordering profiles. These are scheduling
 /// templates, not full script shapers. No joining analysis, syllable
 /// segmentation, reordering, implicit feature enabling or bidi.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ShapeScriptProfile{Latin,Arabic,IndicV2}
impl ShapeScriptProfile{
    pub fn for_script(script:[u8;4])->Result<Self,Error>{
        match &script{
            b"latn"=>Ok(Self::Latin),
            b"arab"=>Ok(Self::Arabic),
            b"dev2"|b"bng2"|b"gjr2"|b"gur2"|b"knd2"|b"mlm2"|b"ory2"|b"tml2"|b"tel2"=>Ok(Self::IndicV2),
            _=>Err(Error::Unsupported("shape script profile")),
        }
    }
    /// Static ordering of explicitly selected tags only. The caller
    /// must supply script-specific scalar masks and preprocessing.
    pub fn groups(self)->Vec<ShapeTagStage>{
        let groups:&[&[[u8;4]]]=match self{
            Self::Latin=>&[&[*b"rvrn"],&[*b"ccmp",*b"locl"],&[*b"rlig"],&[*b"liga",*b"clig",*b"calt"]],
            Self::Arabic=>&[&[*b"rvrn"],&[*b"ccmp",*b"locl"],&[*b"isol",*b"fina",*b"fin2",*b"fin3",*b"medi",*b"med2",*b"init"],&[*b"rlig"],&[*b"calt",*b"liga",*b"clig"]],
            Self::IndicV2=>&[&[*b"rvrn"],&[*b"locl",*b"ccmp"],&[*b"nukt"],&[*b"akhn"],&[*b"rphf"],&[*b"rkrf"],&[*b"pref"],&[*b"blwf"],&[*b"half"],&[*b"pstf"],&[*b"vatu"],&[*b"cjct"],&[*b"init",*b"pres",*b"abvs",*b"blws",*b"psts",*b"haln"]],
        };
        groups.iter().map(|tags|ShapeTagStage{tags:tags.to_vec()}).collect()
    }
}
