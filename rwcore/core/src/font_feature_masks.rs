//! Explicit binary feature masks in logical scalar order.
//! A dense u64 bitset supports up to 64 selected feature IDs.
//! Root scheduling integration is provided by execute_gsub_program_with_masks.
use crate::{Error,GsubPlan,ProvenanceLedger};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FeatureMaskLimits{
    pub max_scalars:usize,
    pub max_overrides:usize,
    pub max_work:usize,
}
impl Default for FeatureMaskLimits{
    fn default()->Self{Self{max_scalars:65536,max_overrides:65536,max_work:4194304}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FeatureMaskOverride{
    pub feature:u16,pub start:usize,pub end:usize,pub enabled:bool,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ScalarFeatureMasks{
    features:Vec<u16>,required:Option<u16>,masks:Vec<u64>,
}
fn bit(features:&[u16],id:u16)->Result<u64,Error>{
    let i=features.binary_search(&id).map_err(|_|Error::Invalid("feature mask unselected feature"))?;
    Ok(1u64<<i)
}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{
    *left=left.checked_sub(n).ok_or(Error::Limit("feature mask work"))?;Ok(())
}
impl ScalarFeatureMasks{
    /// All selected features initially enabled. Overrides in caller order,
    /// last wins for overlapping ranges. Required feature cannot be disabled.
    pub fn build(plan:&GsubPlan,scalar_count:usize,overrides:&[FeatureMaskOverride],l:FeatureMaskLimits)->Result<Self,Error>{
        Self::build_parts(&plan.features,plan.required_feature,scalar_count,overrides,l)
    }
    fn build_parts(features:&[u16],required:Option<u16>,scalar_count:usize,overrides:&[FeatureMaskOverride],l:FeatureMaskLimits)->Result<Self,Error>{
        if features.len()>64||scalar_count>l.max_scalars||overrides.len()>l.max_overrides{
            return Err(Error::Limit("feature mask size"));
        }
        if features.windows(2).any(|w|w[0]>=w[1]){
            return Err(Error::Invalid("feature mask feature order"));
        }
        if let Some(id)=required{bit(&features,id)?;}
        let mut left=l.max_work;
        charge(&mut left,scalar_count)?;
        for o in overrides{
            charge(&mut left,1)?;
            if o.start>o.end||o.end>scalar_count{return Err(Error::Invalid("feature mask range"));}
            bit(&features,o.feature)?;
            if !o.enabled&&Some(o.feature)==required{return Err(Error::Invalid("feature mask required feature"));}
            charge(&mut left,o.end-o.start)?;
        }
        let all=if features.len()==64{u64::MAX}else{(1u64<<features.len())-1};
        let mut masks=Vec::new();masks.try_reserve_exact(scalar_count).map_err(|_|Error::Limit("feature mask allocation"))?;
        masks.resize(scalar_count,all);
        for o in overrides{
            let b=bit(&features,o.feature)?;
            for m in &mut masks[o.start..o.end]{if o.enabled{*m|=b;}else{*m&=!b;}}
        }
        Ok(Self{features:features.to_vec(),required:required,masks})
    }
    pub fn features(&self)->&[u16]{&self.features}
    pub fn scalar_masks(&self)->&[u64]{&self.masks}
    pub fn required_feature(&self)->Option<u16>{self.required}
    pub fn enabled(&self,scalar:usize,feature:u16)->Result<bool,Error>{
        let m=self.masks.get(scalar).ok_or(Error::Invalid("feature mask scalar"))?;
        Ok(*m&bit(&self.features,feature)?!=0)
    }
    /// Derive current glyph masks from exact scalar provenance: an optional
    /// feature survives contraction only when every contributing scalar
    /// enables it. Expansion naturally inherits the same mask.
    /// This is an explicit conservative policy, not script-specific shaping.
    pub fn project(&self,ledger:&ProvenanceLedger,max_work:usize)->Result<Vec<u64>,Error>{
        if ledger.scalar_count()!=self.masks.len(){return Err(Error::Invalid("feature mask scalar domain"));}
        let mut left=max_work;
        charge(&mut left,ledger.glyphs().len())?;
        charge(&mut left,ledger.reference_count())?;
        let mut out=Vec::new();out.try_reserve_exact(ledger.glyphs().len()).map_err(|_|Error::Limit("feature mask allocation"))?;
        for g in ledger.glyphs(){
            if g.sources().is_empty(){return Err(Error::Invalid("feature mask empty sources"));}
            let mut m=u64::MAX;
            for &i in g.sources(){m&=*self.masks.get(i).ok_or(Error::Invalid("feature mask source"))?;}
            out.push(m);
        }
        Ok(out)
    }
    /// Convert the contributing features of a planned root to a bitset.
    /// Callers must associate plan and compiled program separately.
    pub fn lookup_mask(&self,features:&[u16])->Result<u64,Error>{
        if features.is_empty()||features.len()>64||features.windows(2).any(|w|w[0]>=w[1]){
            return Err(Error::Invalid("feature mask lookup features"));
        }
        let mut mask=0;
        for &id in features{mask|=bit(&self.features,id)?;}
        Ok(mask)
    }
}

/// Separate GPOS feature namespace; projection uses post-GSUB provenance.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposScalarFeatureMasks(ScalarFeatureMasks);
impl GposScalarFeatureMasks{
    pub fn build(plan:&crate::GposPlan,scalar_count:usize,overrides:&[FeatureMaskOverride],l:FeatureMaskLimits)->Result<Self,Error>{
        Ok(Self(ScalarFeatureMasks::build_parts(&plan.features,plan.required_feature,scalar_count,overrides,l)?))
    }
    pub fn features(&self)->&[u16]{self.0.features()}
    pub fn required_feature(&self)->Option<u16>{self.0.required_feature()}
    pub fn project(&self,ledger:&ProvenanceLedger,max_work:usize)->Result<Vec<u64>,Error>{self.0.project(ledger,max_work)}
    pub fn lookup_mask(&self,features:&[u16])->Result<u64,Error>{self.0.lookup_mask(features)}
}
