//! Bounded avar 1.0 axis segment maps. Input and intermediate values
//! are normalized signed Fixed 16.16; map endpoints use F2DOT14.
//! https://learn.microsoft.com/en-us/typography/opentype/spec/avar
use crate::{Error,Fvar,normalized_fixed_to_f2dot14};

#[derive(Debug,Clone,Copy)]
pub struct AvarLimits{
    pub max_bytes:usize,pub max_axes:usize,pub max_pairs:usize,
}
impl Default for AvarLimits{
    fn default()->Self{Self{max_bytes:4*1024*1024,max_axes:256,max_pairs:262144}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct AvarPair{pub from:i16,pub to:i16}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct AvarSegmentMap{
    pairs:Vec<AvarPair>,
    active:bool,
}
impl AvarSegmentMap{
    pub fn pairs(&self)->&[AvarPair]{&self.pairs}
    /// Empty maps or maps lacking any of the three essential identity
    /// anchors do not modify coordinates. Malformed order is rejected
    /// by the parser rather than silently repairing ambiguous records.
    pub fn is_active(&self)->bool{self.active}
    pub fn map_fixed(&self,x:i32)->Result<i32,Error>{
        if !(-65536..=65536).contains(&x){return Err(Error::Invalid("avar normalized coordinate"));}
        if !self.active{return Ok(x);}
        let i=self.pairs.partition_point(|p|i32::from(p.from)*4<x);
        let hi=self.pairs.get(i).ok_or(Error::Invalid("avar upper anchor"))?;
        let hx=i64::from(hi.from)*4;let hy=i64::from(hi.to)*4;
        if i64::from(x)==hx{return Ok(hy as i32);}
        let lo=self.pairs.get(i.checked_sub(1).ok_or(Error::Invalid("avar lower anchor"))?)
            .ok_or(Error::Invalid("avar lower anchor"))?;
        let lx=i64::from(lo.from)*4;let ly=i64::from(lo.to)*4;
        // Fixed 16.16 ratio, then Fixed multiplication. Both operands
        // are nonnegative by validated ordering. Round each operation
        // to nearest (ties upward), as an explicit deterministic policy.
        let den=hx-lx;
        let ratio=((i64::from(x)-lx)*65536+den/2)/den;
        let delta=(ratio*(hy-ly)+32768)/65536;
        Ok((ly+delta).clamp(-65536,65536) as i32)
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Avar{maps:Vec<AvarSegmentMap>}
impl Avar{
    pub fn maps(&self)->&[AvarSegmentMap]{&self.maps}
    pub fn map_fixed(&self,coordinates:&[i32])->Result<Vec<i32>,Error>{
        if coordinates.len()!=self.maps.len(){return Err(Error::Invalid("avar coordinate count"));}
        let mut out=Vec::new();reserve(&mut out,self.maps.len())?;
        for (m,&v) in self.maps.iter().zip(coordinates){out.push(m.map_fixed(v)?);}
        Ok(out)
    }
}
fn reserve<T>(v:&mut Vec<T>,n:usize)->Result<(),Error>{
    v.try_reserve_exact(n).map_err(|_|Error::Limit("avar allocation"))
}
fn slice(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("avar offset"))?).ok_or(Error::Invalid("avar truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(slice(b,p,2)?.try_into().unwrap()))}
/// Parse avar 1.0 in fvar axis order. The axis count must agree with
/// the independently parsed fvar. Empty and missing-anchor maps are
/// identity; invalid coordinate range/order, reserved fields and
/// unexpected trailing bytes fail closed. Version 2 is unsupported.
pub fn parse_avar(b:&[u8],axis_count:usize,l:AvarLimits)->Result<Avar,Error>{
    if b.len()>l.max_bytes||axis_count>l.max_axes{return Err(Error::Limit("avar size"));}
    slice(b,0,8)?;
    if word(b,0)?!=1||word(b,2)?!=0{return Err(Error::Unsupported("avar version"));}
    if word(b,4)?!=0{return Err(Error::Invalid("avar reserved"));}
    let count=word(b,6)? as usize;
    if count!=axis_count{return Err(Error::Invalid("avar axis count"));}
    if count==0{return Err(Error::Unsupported("avar zero axes"));}
    let mut maps=Vec::new();reserve(&mut maps,count)?;
    let mut at=8;let mut budget=l.max_pairs;
    for _ in 0..count{
        let n=word(b,at)? as usize;at+=2;
        budget=budget.checked_sub(n).ok_or(Error::Limit("avar pairs"))?;
        let bytes=n.checked_mul(4).ok_or(Error::Limit("avar pair extent"))?;
        slice(b,at,bytes)?;
        let mut pairs:Vec<AvarPair>=Vec::new();reserve(&mut pairs,n)?;
        for j in 0..n{
            let p=at+4*j;
            let pair=AvarPair{from:word(b,p)? as i16,to:word(b,p+2)? as i16};
            if !(-16384..=16384).contains(&pair.from)||!(-16384..=16384).contains(&pair.to)||
                pairs.last().is_some_and(|last|last.from>=pair.from||last.to>pair.to){
                return Err(Error::Invalid("avar pair range or order"));
            }
            pairs.push(pair);
        }
        at+=bytes;
        let active=[-16384,0,16384].iter().all(|&a|pairs.iter().any(|p|p.from==a&&p.to==a));
        maps.push(AvarSegmentMap{pairs,active});
    }
    if at!=b.len(){return Err(Error::Invalid("avar trailing bytes"));}
    Ok(Avar{maps})
}
/// Complete fvar -> optional avar 1.0 -> final F2DOT14 normalization.
/// An absent avar is identity. Avar axis order is defined by the
/// associated fvar; equal counts alone do not establish font identity.
/// Callers must obtain both tables from the same font face.
pub fn normalize_variation_coordinates(fvar:&Fvar,avar:Option<&Avar>,user:&[i32])->Result<Vec<i16>,Error>{
    let fixed=fvar.normalize_fixed(user)?;
    final_coordinates(avar,&fixed)
}
/// Tag-based variant; unspecified axes use the fvar defaults.
pub fn normalize_tagged_variation_coordinates(fvar:&Fvar,avar:Option<&Avar>,
    user:&[([u8;4],i32)])->Result<Vec<i16>,Error>{
    let fixed=fvar.normalize_tagged_fixed(user)?;
    final_coordinates(avar,&fixed)
}
fn final_coordinates(avar:Option<&Avar>,fixed:&[i32])->Result<Vec<i16>,Error>{
    let mapped=avar.map(|a|a.map_fixed(fixed)).transpose()?;
    let values=mapped.as_deref().unwrap_or(fixed);
    let mut out=Vec::new();reserve(&mut out,values.len())?;
    for &v in values{out.push(normalized_fixed_to_f2dot14(v)?);}
    Ok(out)
}
