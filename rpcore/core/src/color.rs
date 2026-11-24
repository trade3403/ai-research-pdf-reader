//! PDF ICCBased fallback to a compatible device Alternate.
//! ICC transforms themselves are not implemented.
use crate::{Document,Object,Error,Result};
pub(crate) fn icc_components(doc:&Document<'_>,a:&[Object])->Result<usize>{
    if a.len()!=2{return Err(Error::Invalid("ICCBased array"));}
    let o=doc.dereference(&a[1])?;
    let Object::Stream{dict,..}=o else{return Err(Error::Invalid("ICCBased profile stream"));};
    let n=match dict.get(b"N".as_slice()).map(|x|doc.dereference(x)).transpose()?{
        Some(Object::Number(1.))=>1,Some(Object::Number(3.))=>3,Some(Object::Number(4.))=>4,
        _=>return Err(Error::Invalid("ICCBased N")),
    };
    if let Some(v)=dict.get(b"Range".as_slice()){
        let Object::Array(range)=doc.dereference(v)? else{return Err(Error::Invalid("ICCBased Range"));};
        if range.len()!=2*n||range.chunks_exact(2).any(|p|p!=[Object::Number(0.),Object::Number(1.)]){
            return Err(Error::Unsupported("ICCBased nondefault Range"));
        }
    }
    if let Some(v)=dict.get(b"Alternate".as_slice()){
        let a=doc.dereference(v)?;
        let want=match n{1=>b"DeviceGray".as_slice(),3=>b"DeviceRGB".as_slice(),_=>b"DeviceCMYK".as_slice()};
        if !matches!(a,Object::Name(name) if name==want){return Err(Error::Unsupported("ICCBased nondevice or mismatched Alternate"));}
    }
    // PDF defines DeviceGray/RGB/CMYK defaults for N=1/3/4 respectively
    // when the reader cannot process the embedded ICC profile.
    Ok(n)
}
