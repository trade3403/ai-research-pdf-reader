//! Decode image samples and masks without delegating to an external image library.
use super::{Budget,MAX_PIXELS,MAX_SIDE,rgb,Color};
use crate::{Document,Object,Error,Result,filters::jpeg::{self,Transform}};
use std::{collections::BTreeMap,sync::Arc};
use crate::color::{IccTransform,ICC_PAGE_BYTES,ICC_PAGE_PROFILES};
type Dict=BTreeMap<Vec<u8>,Object>;
pub(super) struct Image{pub w:usize,pub h:usize,pub rgb:Vec<u8>,pub alpha:Option<Vec<u8>>,pub stencil:bool,pub interpolate:bool}
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
fn channel(v:u16,max:u16,p:[f64;2])->u8{((p[0]+f64::from(v)/f64::from(max)*(p[1]-p[0])).clamp(0.,1.)*255.).round() as u8}
fn device(o:&Object)->Result<usize>{
    match o{
        Object::Name(n) if n==b"DeviceGray"=>Ok(1),
        Object::Name(n) if n==b"DeviceRGB"=>Ok(3),
        Object::Name(n) if n==b"DeviceCMYK"=>Ok(4),
        _=>Err(Error::Unsupported("image color space")),
    }
}
enum Space{Device(usize),Icc(Arc<IccTransform>),Indexed{base:usize,high:usize,palette:Vec<u8>}}
/// Cache even alternate decisions so uncached image objects cannot repeatedly
/// inflate/parse the same profile. Scope is a single raster pass.
#[derive(Default)]
pub(super) struct IccCache{entries:BTreeMap<(u32,u16),(usize,Option<Arc<IccTransform>>)>,bytes:usize,pub(super) diagnostics:Vec<crate::icc::ProfileDiagnostic>}
impl IccCache{
    fn space(&mut self,doc:&Document<'_>,a:&[Object],budget:&mut Budget)->Result<Space>{
        let id=match a.get(1){Some(Object::Ref(id,gen)) if a.len()==2=>(*id,*gen),_=>return Err(Error::Invalid("ICCBased profile reference"))};
        if let Some((n,t))=self.entries.get(&id){
            return Ok(match t{Some(t)=>Space::Icc(Arc::clone(t)),None=>Space::Device(*n)});
        }
        if self.entries.len()>=ICC_PAGE_PROFILES{return Err(Error::Limit("ICC image profiles"));}
        let (n,t,used,path)=crate::color::load_icc(doc,a,ICC_PAGE_BYTES-self.bytes)?;
        self.bytes+=used;
        budget.charge(used)?;
        let t=t.map(Arc::new);
        let space=match &t{Some(t)=>Space::Icc(Arc::clone(t)),None=>Space::Device(n)};
        self.entries.insert(id,(n,t));
        self.diagnostics.push(crate::icc::ProfileDiagnostic{object_id:id,source:crate::icc::ProfileUse::Image,components:n,path});
        Ok(space)
    }
}
fn space(doc:&Document<'_>,d:&Dict,cache:&mut IccCache,budget:&mut Budget)->Result<Space>{
    let o=doc.dereference(d.get(b"ColorSpace".as_slice()).ok_or(Error::Invalid("image color space"))?)?;
    if let Object::Array(a)=&o{
        if matches!(a.first(),Some(Object::Name(n)) if n==b"ICCBased"){return cache.space(doc,a,budget);}
        if a.len()!=4||!matches!(&a[0],Object::Name(n) if n==b"Indexed"||n==b"I"){return Err(Error::Unsupported("image color space array"));}
        let base=device(&doc.dereference(&a[1])?)?;
        let high=match a[2]{Object::Number(v) if v>=0.&&v<=255.&&v.fract()==0.=>v as usize,_=>return Err(Error::Invalid("Indexed hival"))};
        let n=(high+1)*base;
        let lookup=doc.dereference(&a[3])?;
        let palette=match &lookup{Object::String(s)=>s.clone(),Object::Stream{..}=>crate::filters::decode(&lookup,n)?,_=>return Err(Error::Invalid("Indexed lookup"))};
        if palette.len()!=n{return Err(Error::Invalid("Indexed palette length"));}
        Ok(Space::Indexed{base,high,palette})
    }else{Ok(Space::Device(device(&o)?))}
}
fn bits(d:&Dict)->Result<usize>{
    let b=integer(d,b"BitsPerComponent",16)?;
    if !matches!(b,1|2|4|8|16){return Err(Error::Unsupported("image sample depth"));}Ok(b)
}
fn max_sample(b:usize)->u16{if b==16{65535}else{((1u32<<b)-1) as u16}}
fn unpack(data:&[u8],w:usize,h:usize,n:usize,b:usize)->Vec<u16>{
    let row=(w*n*b).div_ceil(8);let mut out=Vec::with_capacity(w*h*n);
    for y in 0..h{let d=&data[y*row..(y+1)*row];for x in 0..w*n{
        let bit=x*b;
        let value=if b==16{u16::from_be_bytes([d[bit/8],d[bit/8+1]])}else{u16::from((d[bit/8]>>(8-b-bit%8))&max_sample(b) as u8)};
        out.push(value);
    }}out
}
fn rgb_components(c:&[u8])->[u8;3]{
    match c.len(){1=>[c[0];3],3=>[c[0],c[1],c[2]],_=>rgb(Color::Cmyk([0,1,2,3].map(|i|f64::from(c[i])/255.)))}
}
fn sample_data(o:&Object,w:usize,h:usize,n:usize,b:usize,budget:&mut Budget)->Result<Vec<u16>>{
    let Object::Stream{dict,data} = o else{return Err(Error::Invalid("image stream"));};
    let expected=(w*n*b).div_ceil(8)*h;budget.charge(w*h*n)?;
    // Only terminal DCT supported. Filter arrays with DCT are explicitly rejected
    // by the ordinary decoder rather than losing JPEG metadata.
    let dct=matches!(dict.get(b"Filter".as_slice()),Some(Object::Name(n)) if n==b"DCTDecode"||n==b"DCT");
    let out=if dct{
        if b!=8{return Err(Error::Unsupported("JPEG sample depth"));}
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
    if out.len()!=expected{return Err(Error::Invalid("image sample count"));}Ok(unpack(&out,w,h,n,b))
}
fn resample_alpha(data:&[u8],sw:usize,sh:usize,w:usize,h:usize,budget:&mut Budget)->Result<Vec<u8>>{
    budget.charge(w*h)?;let mut a=Vec::with_capacity(w*h);
    for y in 0..h{for x in 0..w{a.push(data[((2*y+1)*sh/(2*h))*sw+(2*x+1)*sw/(2*w)]);}}Ok(a)
}
fn mask_image(doc:&Document<'_>,o:&Object,w:usize,h:usize,budget:&mut Budget,hard:bool)->Result<Vec<u8>>{
    let o=doc.dereference(o)?;let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("mask stream"));};
    for k in [b"Mask".as_slice(),b"SMask",b"Matte",b"SMaskInData"]{if dict.contains_key(k){return Err(Error::Unsupported("nested or matte mask"));}}
    if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Image")||flag(dict,b"Interpolate")?||flag(dict,b"ImageMask")?!=hard{return Err(Error::Unsupported("mask layout"));}
    let(sw,sh)=dimensions(dict)?;
    let b=if hard&&!dict.contains_key(b"BitsPerComponent".as_slice()){1}else{bits(dict)?};
    if hard{
        if b!=1||dict.contains_key(b"ColorSpace".as_slice()){return Err(Error::Invalid("explicit mask dictionary"));}
    }else{
        // A soft-mask sample is opacity, not a display color. Keep the
        // existing one-component ICC alternate policy, never apply RGB/TRCs.
        let cs=doc.dereference(dict.get(b"ColorSpace".as_slice()).ok_or(Error::Invalid("image color space"))?)?;
        let n=match &cs{
            Object::Array(a) if matches!(a.first(),Some(Object::Name(n)) if n==b"ICCBased")=>crate::color::icc_components(doc,a)?,
            _=>device(&cs)?,
        };
        if n!=1{return Err(Error::Unsupported("SMask color space"));}
    }
    let p=pairs(dict,1)?[0];
    if hard&&p!=[0.,1.]&&p!=[1.,0.]{return Err(Error::Invalid("mask Decode"));}
    let raw=sample_data(&o,sw,sh,1,b,budget)?;
    let a:Vec<u8>=raw.into_iter().map(|v|{let c=channel(v,max_sample(b),p);if hard{255-c}else{c}}).collect();
    if (sw,sh)==(w,h){Ok(a)}else{resample_alpha(&a,sw,sh,w,h,budget)}
}
pub(super) fn load(doc:&Document<'_>,id:(u32,u16),budget:&mut Budget,cache:&mut IccCache)->Result<Image>{
    let o=doc.resolve(id.0,id.1)?;let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("image stream"));};
    if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Image"){return Err(Error::Invalid("image subtype"));}
    let interpolate=flag(dict,b"Interpolate")?;
    if dict.contains_key(b"SMaskInData".as_slice()){return Err(Error::Unsupported("image interpolation or embedded alpha"));}
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
        return Ok(Image{w,h,rgb:Vec::new(),alpha:Some(alpha),stencil:true,interpolate});
    }
    let b=bits(dict)?;let max=max_sample(b);let cs=space(doc,dict,cache,budget)?;
    let n=match &cs{Space::Device(n)=>*n,Space::Icc(t)=>t.components(),Space::Indexed{..}=>1};
    if matches!(&cs,Space::Indexed{..})&&(b==16||matches!(dict.get(b"Filter".as_slice()),Some(Object::Name(n)) if n==b"DCTDecode"||n==b"DCT")){return Err(Error::Unsupported("Indexed sample format"));}
    let data=sample_data(&o,w,h,n,b,budget)?;
    let mut p=pairs(dict,n)?;
    if matches!(&cs,Space::Indexed{..})&&!dict.contains_key(b"Decode".as_slice()){p[0]=[0.,f64::from(max)];}
    let alpha=if let Some(m)=dict.get(b"SMask".as_slice()){
        if dict.contains_key(b"Mask".as_slice()){return Err(Error::Unsupported("combined Mask and SMask"));}
        if matches!(m,Object::Name(n) if n==b"None"){None}else{Some(mask_image(doc,m,w,h,budget,false)?)}
    }else if let Some(m)=dict.get(b"Mask".as_slice()){
        match doc.dereference(m)?{
            Object::Array(a)=>{
                if a.len()!=2*n{return Err(Error::Invalid("color key Mask"));}
                let mut ranges=Vec::new();
                for p in a.chunks_exact(2){
                    let val=|o:&Object|->Result<u16>{match o{Object::Number(n) if *n>=0.&&*n<=f64::from(max)&&n.fract()==0.=>Ok(*n as u16),_=>Err(Error::Invalid("color key sample"))}};
                    let(l,h)=(val(&p[0])?,val(&p[1])?);if l>h{return Err(Error::Invalid("color key range"));}ranges.push((l,h));
                }
                Some(data.chunks_exact(n).map(|s|if s.iter().zip(&ranges).all(|(&v,&(l,h))|v>=l&&v<=h){0}else{255}).collect())
            }
            Object::Stream{..}=>Some(mask_image(doc,m,w,h,budget,true)?),
            _=>return Err(Error::Invalid("image Mask")),
        }
    }else{None};
    // Charge before evaluating potentially millions of LUTs. The CMYK path
    // uses 16 interpolation corners; the RGB path is three TRCs and a matrix.
    if let Space::Icc(t)=&cs{
        budget.stage="raster work (image ICC)";
        budget.charge((w*h).checked_mul(t.work()).ok_or(Error::Limit("raster work (image ICC)"))?)?;
    }
    let mut out=Vec::with_capacity(w*h*3);
    for v in data.chunks_exact(n){
        let color=match &cs{
            Space::Device(n)=>{let mut c=[0;4];for i in 0..*n{c[i]=channel(v[i],max,p[i]);}rgb_components(&c[..*n])}
            Space::Icc(t)=>{
                // Decode at original sample precision, clamp in the profile's
                // supported default Range, and quantize only after conversion.
                let mut c=[0.;4];
                for i in 0..n{c[i]=(p[i][0]+f64::from(v[i])/f64::from(max)*(p[i][1]-p[i][0])).clamp(0.,1.);}
                t.rgb(c)?.map(|x|(x.clamp(0.,1.)*255.).round() as u8)
            }
            Space::Indexed{base,high,palette}=>{
                let i=(p[0][0]+f64::from(v[0])/f64::from(max)*(p[0][1]-p[0][0])).round().clamp(0.,*high as f64)as usize;
                rgb_components(&palette[i*base..(i+1)*base])
            }
        };out.extend(color);
    }
    Ok(Image{w,h,rgb:out,alpha,stencil:false,interpolate})
}
impl Image{
    pub(super) fn sample(&self,u:f64,v:f64,stencil:[u8;3])->([u8;3],f64){
        let pixel=|x:usize,y:usize|{
            let p=y*self.w+x;let color=if self.stencil{stencil}else{[self.rgb[p*3],self.rgb[p*3+1],self.rgb[p*3+2]]};
            (color,self.alpha.as_ref().map_or(1.,|a|f64::from(a[p])/255.))
        };
        if !self.interpolate{return pixel(((u*self.w as f64).floor()as usize).min(self.w-1),((v*self.h as f64).floor()as usize).min(self.h-1));}
        let x=(u*self.w as f64-0.5).clamp(0.,(self.w-1)as f64);
        let y=(v*self.h as f64-0.5).clamp(0.,(self.h-1)as f64);
        let(x0,y0)=(x.floor()as usize,y.floor()as usize);let(x1,y1)=((x0+1).min(self.w-1),(y0+1).min(self.h-1));
        let(dx,dy)=(x-x0 as f64,y-y0 as f64);let(mut a,mut c)=(0.,[0.;3]);
        // Premultiplied interpolation avoids color bleeding from transparent texels.
        for (x,y,w) in [(x0,y0,(1.-dx)*(1.-dy)),(x1,y0,dx*(1.-dy)),(x0,y1,(1.-dx)*dy),(x1,y1,dx*dy)]{
            let(p,alpha)=pixel(x,y);a+=w*alpha;for i in 0..3{c[i]+=w*alpha*f64::from(p[i]);}
        }
        (if a>0.{c.map(|v|(v/a).round().clamp(0.,255.)as u8)}else{[0;3]},a.clamp(0.,1.))
    }
}
