//! Decode image samples and masks without delegating to an external image library.
use super::{Budget,MAX_PIXELS,MAX_SIDE,rgb,Color};
use crate::{Document,Object,Error,Result,filters::jpeg::{self,Transform}};
use std::collections::BTreeMap;
type Dict=BTreeMap<Vec<u8>,Object>;
pub(super) struct Image{pub w:usize,pub h:usize,pub rgb:Vec<u8>,pub alpha:Option<Vec<u8>>,pub stencil:bool}
fn integer(d:&Dict,k:&[u8],max:usize)->Result<usize>{
    match d.get(k){Some(Object::Number(n)) if *n>=1.&&*n<=max as f64&&n.fract()==0.=>Ok(*n as usize),_=>Err(Error::Invalid("image integer"))}
}
fn dimensions(d:&Dict)->Result<(usize,usize)>{
    let(w,h)=(integer(d,b"Width",MAX_SIDE)?,integer(d,b"Height",MAX_SIDE)?);
    if w*h>MAX_PIXELS{return Err(Error::Limit("image pixels"));}Ok((w,h))
}
fn flag(d:&Dict,k:&[u8])->Result<bool>{match d.get(k){None|Some(Object::Bool(false))=>Ok(false),Some(Object::Bool(true))=>Ok(true),_=>Err(Error::Invalid("image flag"))}}
fn pairs(d:&Dict,n:usize)->Result<Vec<[f64;2]>>{
    match d.get(b"Decode".as_slice()){
        None=>Ok(vec![[0.,1.];n]),
        Some(Object::Array(a)) if a.len()==n*2=>{
            let mut out=Vec::new();for p in a.chunks_exact(2){let mut v=[0.;2];for i in 0..2{if let Object::Number(x)=p[i]{if x.is_finite()&&x.abs()<=1e6{v[i]=x;continue;}}return Err(Error::Invalid("image Decode"));}out.push(v);}Ok(out)
        }
        _=>Err(Error::Invalid("image Decode array")),
    }
}
fn channel(v:u8,p:[f64;2])->u8{((p[0]+f64::from(v)/255.*(p[1]-p[0])).clamp(0.,1.)*255.).round() as u8}
fn channels(d:&Dict)->Result<usize>{
    match d.get(b"ColorSpace".as_slice()){
        Some(Object::Name(n)) if n==b"DeviceGray"=>Ok(1),
        Some(Object::Name(n)) if n==b"DeviceRGB"=>Ok(3),
        Some(Object::Name(n)) if n==b"DeviceCMYK"=>Ok(4),
        _=>Err(Error::Unsupported("image color space")),
    }
}
fn sample_data(o:&Object,w:usize,h:usize,n:usize,budget:&mut Budget)->Result<Vec<u8>>{
    let Object::Stream{dict,data} = o else{return Err(Error::Invalid("image stream"));};
    let expected=w*h*n;budget.charge(expected)?;
    // Only terminal DCT supported. Filter arrays with DCT are explicitly rejected
    // by the ordinary decoder rather than losing JPEG metadata.
    let dct=matches!(dict.get(b"Filter".as_slice()),Some(Object::Name(n)) if n==b"DCTDecode"||n==b"DCT");
    let out=if dct{
        let mode=match dict.get(b"DecodeParms".as_slice()){
            None|Some(Object::Null)=>Transform::Auto,
            Some(Object::Dict(d))=>{
                if d.keys().any(|k|k!=b"ColorTransform"){return Err(Error::Unsupported("DCT DecodeParms"));}
                match d.get(b"ColorTransform".as_slice()){None=>Transform::Auto,Some(Object::Number(v)) if *v==0.=>Transform::None,Some(Object::Number(v)) if *v==1.=>Transform::YCbCr,_=>return Err(Error::Unsupported("DCT ColorTransform"))}
            }
            _=>return Err(Error::Unsupported("DCT DecodeParms")),
        };
        // Charge a conservative per-pixel work estimate to the page budget.
        budget.charge(w*h*16)?;
        let j=jpeg::decode(data,expected,mode)?;
        if (j.width,j.height,j.channels)!=(w,h,n){return Err(Error::Invalid("JPEG/PDF dimensions or components mismatch"));}
        j.data
    }else{crate::filters::decode(o,expected)?};
    if out.len()!=expected{return Err(Error::Invalid("image sample count"));}Ok(out)
}
fn mask_image(doc:&Document<'_>,o:&Object,w:usize,h:usize,budget:&mut Budget)->Result<Vec<u8>>{
    let o=doc.dereference(o)?;let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("SMask stream"));};
    for k in [b"Mask".as_slice(),b"SMask",b"Matte",b"SMaskInData"]{if dict.contains_key(k){return Err(Error::Unsupported("nested or matte SMask"));}}
    if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Image")||flag(dict,b"ImageMask")?||flag(dict,b"Interpolate")?||dimensions(dict)?!=(w,h)||channels(dict)?!=1||integer(dict,b"BitsPerComponent",16)?!=8{return Err(Error::Unsupported("SMask layout"));}
    let p=pairs(dict,1)?[0];Ok(sample_data(&o,w,h,1,budget)?.into_iter().map(|v|channel(v,p)).collect())
}
pub(super) fn load(doc:&Document<'_>,id:(u32,u16),budget:&mut Budget)->Result<Image>{
    let o=doc.resolve(id.0,id.1)?;let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("image stream"));};
    if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Image"){return Err(Error::Invalid("image subtype"));}
    if flag(dict,b"Interpolate")?||dict.contains_key(b"SMaskInData".as_slice()){return Err(Error::Unsupported("image interpolation or embedded alpha"));}
    let(w,h)=dimensions(dict)?;
    let stencil=flag(dict,b"ImageMask")?;
    if stencil{
        if dict.contains_key(b"ColorSpace".as_slice())||dict.contains_key(b"Mask".as_slice())||dict.contains_key(b"SMask".as_slice()){return Err(Error::Invalid("stencil dictionary"));}
        if dict.contains_key(b"BitsPerComponent".as_slice())&&integer(dict,b"BitsPerComponent",16)?!=1{return Err(Error::Invalid("stencil bits"));}
        let pair=pairs(dict,1)?[0];
        if pair!=[0.,1.]&&pair!=[1.,0.]{return Err(Error::Invalid("stencil Decode"));}
        let row=w.div_ceil(8);budget.charge(w*h)?;
        let data=crate::filters::decode(&o,row*h)?;if data.len()!=row*h{return Err(Error::Invalid("stencil sample count"));}
        let mut alpha=Vec::with_capacity(w*h);
        for y in 0..h{for x in 0..w{let b=(data[y*row+x/8]>>(7-x%8))&1;let paint=if pair[0]==0.{b==0}else{b==1};alpha.push(if paint{255}else{0});}}
        return Ok(Image{w,h,rgb:Vec::new(),alpha:Some(alpha),stencil:true});
    }
    if integer(dict,b"BitsPerComponent",16)?!=8{return Err(Error::Unsupported("image sample depth"));}
    let n=channels(dict)?;let data=sample_data(&o,w,h,n,budget)?;let p=pairs(dict,n)?;
    let alpha=if let Some(m)=dict.get(b"SMask".as_slice()){
        if dict.contains_key(b"Mask".as_slice()){return Err(Error::Unsupported("combined Mask and SMask"));}
        if matches!(m,Object::Name(n) if n==b"None"){None}else{Some(mask_image(doc,m,w,h,budget)?)}
    }else if let Some(m)=dict.get(b"Mask".as_slice()){
        let Object::Array(a)=doc.dereference(m)? else{return Err(Error::Unsupported("explicit mask stream"));};
        if a.len()!=2*n{return Err(Error::Invalid("color key Mask"));}
        let mut ranges=Vec::new();
        for p in a.chunks_exact(2){
            let val=|o:&Object|->Result<u8>{match o{Object::Number(n) if *n>=0.&&*n<=255.&&n.fract()==0.=>Ok(*n as u8),_=>Err(Error::Invalid("color key sample"))}};
            let(l,h)=(val(&p[0])?,val(&p[1])?);if l>h{return Err(Error::Invalid("color key range"));}ranges.push((l,h));
        }
        // JPEG is lossy; color-key samples are defined on decoded component data.
        Some(data.chunks_exact(n).map(|s|if s.iter().zip(&ranges).all(|(&v,&(l,h))|v>=l&&v<=h){0}else{255}).collect())
    }else{None};
    let mut out=Vec::with_capacity(w*h*3);
    for v in data.chunks_exact(n){
        let mut c=[0;4];for i in 0..n{c[i]=channel(v[i],p[i]);}
        let color=match n{1=>[c[0];3],3=>[c[0],c[1],c[2]],_=>rgb(Color::Cmyk(c.map(|x|f64::from(x)/255.)))};
        out.extend(color);
    }
    Ok(Image{w,h,rgb:out,alpha,stencil:false})
}
