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

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubLangSys{pub required_feature:Option<u16>,pub features:Vec<u16>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubScript{pub tag:[u8;4],pub default_language:Option<GsubLangSys>,pub languages:Vec<([u8;4],GsubLangSys)>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubFeature{pub tag:[u8;4],pub parameters:Option<usize>,pub lookups:Vec<u16>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubLookup{pub kind:u16,pub flags:u16,pub subtables:Vec<usize>,pub mark_filtering_set:Option<u16>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubLayout{pub directory:GsubDirectory,pub scripts:Vec<GsubScript>,pub features:Vec<GsubFeature>,pub lookups:Vec<GsubLookup>}
fn charge(budget:&mut usize,n:usize)->Result<(),Error>{
    *budget=budget.checked_sub(n).ok_or(Error::Limit("gsub child records"))?;Ok(())
}
fn indices(b:&[u8],p:usize,n:usize,bound:usize,budget:&mut usize)->Result<Vec<u16>,Error>{
    charge(budget,n)?;slice(b,p,n*2)?;
    let mut out=Vec::with_capacity(n);
    for i in 0..n{let v=word(b,p+2*i)?;if usize::from(v)>=bound{return Err(Error::Invalid("gsub cross index"));}out.push(v);}
    Ok(out)
}
fn language(b:&[u8],p:usize,features:usize,budget:&mut usize)->Result<GsubLangSys,Error>{
    slice(b,p,6)?;
    if word(b,p)?!=0{return Err(Error::Invalid("gsub reserved lookup order"));}
    let r=word(b,p+2)?;
    let required_feature=if r==0xffff{None}else{
        if usize::from(r)>=features{return Err(Error::Invalid("gsub required feature index"));}Some(r)
    };
    let n=usize::from(word(b,p+4)?);
    Ok(GsubLangSys{required_feature,features:indices(b,p+6,n,features,budget)?})
}
/// Decode child headers and cross-indices. Offsets refer to the input GSUB
/// slice. Substitution subtables, feature parameters, GDEF mark-set indices
/// and variation bodies remain opaque, not validated for execution.
pub fn parse_gsub_layout(b:&[u8],limits:GsubLimits)->Result<GsubLayout,Error>{
    let directory=parse_gsub_directory(b,limits)?;
    let mut budget=limits.max_records;
    charge(&mut budget,directory.scripts.len()+directory.features.len()+directory.lookups.len())?;
    let mut scripts=Vec::new();
    for r in &directory.scripts{
        let p=r.offset;let n=usize::from(word(b,p+2)?);
        charge(&mut budget,n)?;
        let size=4+6*n;slice(b,p,size)?;
        let rel=usize::from(word(b,p)?);
        let default_language=if rel==0{None}else{
            charge(&mut budget,1)?;
            Some(language(b,offset(b,p,rel,size,6)?,directory.features.len(),&mut budget)?)
        };
        let mut languages:Vec<([u8;4],GsubLangSys)>=Vec::with_capacity(n);
        for i in 0..n{
            let q=p+4+6*i;let tag:[u8;4]=slice(b,q,4)?.try_into().unwrap();
            if languages.last().is_some_and(|r|r.0>=tag){return Err(Error::Invalid("gsub language tag order"));}
            let at=offset(b,p,usize::from(word(b,q+4)?),size,6)?;
            languages.push((tag,language(b,at,directory.features.len(),&mut budget)?));
        }
        scripts.push(GsubScript{tag:r.tag,default_language,languages});
    }
    let mut features=Vec::new();
    for r in &directory.features{
        let p=r.offset;let n=usize::from(word(b,p+2)?);
        let refs=indices(b,p+4,n,directory.lookups.len(),&mut budget)?;
        let rel=usize::from(word(b,p)?);
        let parameters=if rel==0{None}else{Some(offset(b,p,rel,4+2*n,1)?)};
        features.push(GsubFeature{tag:r.tag,parameters,lookups:refs});
    }
    let mut lookups=Vec::new();
    for &p in &directory.lookups{
        let kind=word(b,p)?;let flags=word(b,p+2)?;let n=usize::from(word(b,p+4)?);
        if !(1..=8).contains(&kind)||flags&0x00e0!=0{return Err(Error::Invalid("gsub lookup kind or flags"));}
        charge(&mut budget,n)?;
        let size=6+2*n+if flags&0x10!=0{2}else{0};slice(b,p,size)?;
        let mut subtables=Vec::with_capacity(n);
        for i in 0..n{subtables.push(offset(b,p,usize::from(word(b,p+6+2*i)?),size,2)?);}
        let mark_filtering_set=if flags&0x10!=0{Some(word(b,p+6+2*n)?)}else{None};
        lookups.push(GsubLookup{kind,flags,subtables,mark_filtering_set});
    }
    Ok(GsubLayout{directory,scripts,features,lookups})
}
