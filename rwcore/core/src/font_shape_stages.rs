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
    if specs.len()>limits.max_stages||plan.features.len()>limits.max_features{
        return Err(Error::Limit("shape stage size"));
    }
    let mut left=limits.max_work;
    charge(&mut left,specs.len(),"shape stage work")?;
    charge(&mut left,plan.features.len(),"shape stage work")?;
    let mut owners=Vec::new();reserve(&mut owners,plan.features.len())?;owners.resize(plan.features.len(),None);
    let mut stages=Vec::new();reserve(&mut stages,specs.len())?;
    for (i,s) in specs.iter().enumerate(){
        charge(&mut left,s.features.len(),"shape stage work")?;
        if s.features.is_empty()||s.features.len()>plan.features.len()||
            s.features.windows(2).any(|w|w[0]>=w[1]){
            return Err(Error::Invalid("shape stage feature order"));
        }
        let mut features=Vec::new();reserve(&mut features,s.features.len())?;
        for &id in &s.features{
            let j=plan.features.binary_search(&id).map_err(|_|Error::Invalid("shape stage unknown feature"))?;
            if owners[j].replace(i).is_some(){return Err(Error::Invalid("shape stage duplicate feature"));}
            features.push(id);
        }
        stages.push(ShapeStage{features,lookups:Vec::new()});
    }
    if owners.iter().any(Option::is_none){return Err(Error::Invalid("shape stage missing feature"));}
    let mut root_count=0usize;let mut reference_count=0usize;
    for entry in &plan.lookups{
        charge(&mut left,1,"shape stage work")?;
        for &id in &entry.features{
            charge(&mut left,1,"shape stage work")?;
            let j=plan.features.binary_search(&id).map_err(|_|Error::Invalid("shape stage feature provenance"))?;
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
