//! Bounded GSUB 1.0/1.1 top-level directory. No lookup execution.
use crate::Error;
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubRecord{pub tag:[u8;4],pub offset:usize}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubDirectory{
    pub minor_version:u16,pub scripts:Vec<GsubRecord>,pub features:Vec<GsubRecord>,
    pub lookups:Vec<usize>,pub feature_variations:Option<usize>,
}
#[derive(Debug,Clone,Copy)]
pub struct GsubLimits{pub max_records:usize,pub max_bytes:usize}
impl Default for GsubLimits{fn default()->Self{Self{max_records:16384,max_bytes:32*1024*1024}}}
fn slice(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("gsub offset"))?).ok_or(Error::Invalid("gsub truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(slice(b,p,2)?.try_into().unwrap()))}
fn offset(b:&[u8],base:usize,rel:usize,min:usize,need:usize)->Result<usize,Error>{
    if rel<min{return Err(Error::Invalid("gsub offset into directory"));}
    let p=base.checked_add(rel).ok_or(Error::Limit("gsub offset"))?;
    slice(b,p,need)?;Ok(p)
}
fn count(b:&[u8],base:usize,stride:usize,budget:&mut usize)->Result<(usize,usize),Error>{
    let n=usize::from(word(b,base)?);
    *budget=budget.checked_sub(n).ok_or(Error::Limit("gsub directory records"))?;
    let size=2+n*stride;slice(b,base,size)?;Ok((n,size))
}
fn tagged(b:&[u8],base:usize,budget:&mut usize,unique:bool)->Result<Vec<GsubRecord>,Error>{
    let (n,size)=count(b,base,6,budget)?;
    let mut out:Vec<GsubRecord>=Vec::with_capacity(n);
    for i in 0..n{
        let p=base+2+i*6;
        let tag:[u8;4]=slice(b,p,4)?.try_into().unwrap();
        if out.last().is_some_and(|r|r.tag>tag||(unique&&r.tag==tag)){return Err(Error::Invalid("gsub tag order"));}
        let at=offset(b,base,usize::from(word(b,p+4)?),size,4)?;
        out.push(GsubRecord{tag,offset:at});
    }
    Ok(out)
}
/// Top-level offsets are absolute in the supplied GSUB slice. Child table
/// contents are not validated here. Feature tags may repeat per OpenType.
pub fn parse_gsub_directory(b:&[u8],limits:GsubLimits)->Result<GsubDirectory,Error>{
    if b.len()>limits.max_bytes{return Err(Error::Limit("gsub bytes"));}
    let major=word(b,0)?;let minor=word(b,2)?;
    if major!=1||minor>1{return Err(Error::Unsupported("gsub version"));}
    let header=if minor==0{10}else{14};slice(b,0,header)?;
    let s=offset(b,0,usize::from(word(b,4)?),header,2)?;
    let f=offset(b,0,usize::from(word(b,6)?),header,2)?;
    let l=offset(b,0,usize::from(word(b,8)?),header,2)?;
    let mut budget=limits.max_records;
    let scripts=tagged(b,s,&mut budget,true)?;
    let features=tagged(b,f,&mut budget,false)?;
    let (n,size)=count(b,l,2,&mut budget)?;
    let mut lookups=Vec::with_capacity(n);
    for i in 0..n{lookups.push(offset(b,l,usize::from(word(b,l+2+i*2)?),size,6)?);}
    let feature_variations=if minor==1{
        let rel=usize::try_from(u32::from_be_bytes(slice(b,10,4)?.try_into().unwrap())).map_err(|_|Error::Limit("gsub variations offset"))?;
        if rel==0{None}else{Some(offset(b,0,rel,header,8)?)}
    }else{None};
    Ok(GsubDirectory{minor_version:minor,scripts,features,lookups,feature_variations})
}
