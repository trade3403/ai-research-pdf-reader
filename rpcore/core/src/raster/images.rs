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
pub(super) struct IccCache{entries:BTreeMap<((u32,u16),crate::icc::RenderingIntent),(usize,Option<Arc<IccTransform>>)>,bytes:usize,pub(super) diagnostics:Vec<crate::icc::ProfileDiagnostic>,pub(super) degree:crate::icc::AdaptationDegree}
impl IccCache{
    fn space(&mut self,doc:&Document<'_>,a:&[Object],budget:&mut Budget,intent:crate::icc::RenderingIntent)->Result<Space>{
        let id=match a.get(1){Some(Object::Ref(id,gen)) if a.len()==2=>(*id,*gen),_=>return Err(Error::Invalid("ICCBased profile reference"))};
        if let Some((n,t))=self.entries.get(&(id,intent)){
            return Ok(match t{Some(t)=>Space::Icc(Arc::clone(t)),None=>Space::Device(*n)});
        }
        if self.entries.len()>=ICC_PAGE_PROFILES{return Err(Error::Limit("ICC image profiles"));}
        let (n,t,used,path)=crate::color::load_icc(doc,a,ICC_PAGE_BYTES-self.bytes,intent,self.degree)?;
        self.bytes+=used;
        budget.charge(used)?;
        let t=t.map(Arc::new);
        let space=match &t{Some(t)=>Space::Icc(Arc::clone(t)),None=>Space::Device(n)};
        self.entries.insert((id,intent),(n,t));
        self.diagnostics.push(crate::icc::ProfileDiagnostic{object_id:id,source:crate::icc::ProfileUse::Image,components:n,path});
        Ok(space)
    }
}
fn space(doc:&Document<'_>,d:&Dict,cache:&mut IccCache,budget:&mut Budget,intent:crate::icc::RenderingIntent)->Result<Space>{
    let o=doc.dereference(d.get(b"ColorSpace".as_slice()).ok_or(Error::Invalid("image color space"))?)?;
    if let Object::Array(a)=&o{
        if matches!(a.first(),Some(Object::Name(n)) if n==b"ICCBased"){return cache.space(doc,a,budget,intent);}
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

fn is_jpx(d:&Dict)->bool{
    match d.get(b"Filter".as_slice()){
        Some(Object::Name(n))=>n==b"JPXDecode",
        Some(Object::Array(a))=>a.iter().any(|v|matches!(v,Object::Name(n) if n==b"JPXDecode")),
        _=>false,
    }
}

/// Extract the terminal JPX payload without extending the generic stream
/// decoder to accept image codecs. Every prefix is charged and bounded.
fn jpx_payload<'a>(dict:&Dict,data:&'a [u8],budget:&mut Budget)->Result<std::borrow::Cow<'a,[u8]>>{
    use std::borrow::Cow;
    let filters:Vec<&[u8]>=match dict.get(b"Filter".as_slice()){
        Some(Object::Name(n))=>vec![n.as_slice()],
        Some(Object::Array(a)) if !a.is_empty()&&a.len()<=8=>a.iter().map(|v|
            match v{Object::Name(n)=>Ok(n.as_slice()),_=>Err(Error::Invalid("JPX filter name"))}
        ).collect::<Result<Vec<_>>>()?,
        _=>return Err(Error::Unsupported("JPX filter structure")),
    };
    if filters.last()!=Some(&b"JPXDecode".as_slice())||
        filters.iter().filter(|&&n|n==b"JPXDecode").count()!=1{
        return Err(Error::Unsupported("JPX must be unique terminal filter"));
    }
    let params:Vec<Option<&Object>>=match dict.get(b"DecodeParms".as_slice()){
        None|Some(Object::Null)=>vec![None;filters.len()],
        Some(v@Object::Dict(_)) if filters.len()==1=>vec![Some(v)],
        Some(Object::Array(a)) if a.len()==filters.len()=>a.iter().map(Some).collect(),
        _=>return Err(Error::Unsupported("JPX DecodeParms structure")),
    };
    match params.last().copied().flatten(){
        None|Some(Object::Null)=>{},
        Some(Object::Dict(p)) if p.is_empty()=>{},
        _=>return Err(Error::Unsupported("JPX DecodeParms")),
    }
    for (name,param) in filters[..filters.len()-1].iter().zip(&params){
        if !matches!(*name,b"ASCIIHexDecode"|b"AHx"|b"ASCII85Decode"|b"A85"|
            b"FlateDecode"|b"Fl"|b"RunLengthDecode"|b"RL"){
            return Err(Error::Unsupported("JPX prefix filter"));
        }
        // Prefixes unwrap compressed bytes, not image samples. No predictors.
        if !matches!(param,None|Some(Object::Null)) &&
            !matches!(param,Some(Object::Dict(p)) if p.is_empty()&&matches!(*name,b"FlateDecode"|b"Fl")){
            return Err(Error::Unsupported("JPX prefix DecodeParms"));
        }
    }
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JPX input bytes"));}
    budget.charge(data.len())?;
    let mut payload=Cow::Borrowed(data);
    for (name,param) in filters[..filters.len()-1].iter().zip(&params){
        // The cap is applied to each individual prefix decoder, not merely
        // the last prefix. Charge both encoded and decoded work cumulatively.
        budget.charge(payload.len())?;
        let cap=crate::MAX_BYTES.min(super::MAX_WORK.saturating_sub(budget.work));
        let mut d=Dict::new();
        d.insert(b"Filter".to_vec(),Object::Name(name.to_vec()));
        if let Some(p)=param{d.insert(b"DecodeParms".to_vec(),(*p).clone());}
        let stream=Object::Stream{dict:d,data:payload.into_owned()};
        let decoded=crate::filters::decode(&stream,cap)?;
        budget.charge(decoded.len())?;
        payload=Cow::Owned(decoded);
    }
    Ok(payload)
}

/// Image-only baseline JPX path with bounded lossless prefix filters.
/// Arbitrary content streams, masks and non-device color remain excluded.
fn load_jpx(doc:&Document<'_>,dict:&Dict,data:&[u8],w:usize,h:usize,
    interpolate:bool,budget:&mut Budget)->Result<Image>{
    use crate::filters::jpx::{self,BaselineColor,Container};
    if flag(dict,b"ImageMask")?||dict.contains_key(b"Mask".as_slice())||
        !matches!(dict.get(b"SMask".as_slice()),None|Some(Object::Name(_)) )
    {
        return Err(Error::Unsupported("JPX image mask"));
    }
    if let Some(Object::Name(n))=dict.get(b"SMask".as_slice()){
        if n!=b"None"{return Err(Error::Unsupported("JPX external soft mask"));}
    }
    match dict.get(b"SMaskInData".as_slice()){
        None=>{},
        Some(Object::Number(v)) if *v==0.=>{},
        _=>return Err(Error::Unsupported("JPX embedded alpha")),
    }
    if dict.contains_key(b"BitsPerComponent".as_slice())&&bits(dict)?!=8{
        return Err(Error::Unsupported("JPX PDF sample depth"));
    }
    // Charge the compressed input and a conservative per-sample estimate
    // before expensive decoding; codec-local geometry/MQ limits also apply.
    let payload=jpx_payload(dict,data,budget)?;
    let header=jpx::inspect(&payload)?;
    if (header.width as usize,header.height as usize)!=(w,h){
        return Err(Error::Invalid("JPX/PDF dimensions mismatch"));
    }
    let n=header.components.len();
    if !matches!(n,1|3){return Err(Error::Unsupported("JPX PDF components"));}
    if let Some(cs)=dict.get(b"ColorSpace".as_slice()){
        // PDF color space can override the codestream's color space. This
        // scoped path supports only the matching device space, not conversion.
        if device(&doc.dereference(cs)?)?!=n{
            return Err(Error::Unsupported("JPX PDF color space mismatch"));
        }
    }else if header.container==Container::RawCodestream{
        return Err(Error::Unsupported("raw JPX requires PDF color space"));
    }
    let charge=w.checked_mul(h).and_then(|v|v.checked_mul(n))
        .and_then(|v|v.checked_mul(128)).ok_or(Error::Limit("JPX page decode work"))?;
    budget.charge(charge)?;
    let image=jpx::decode_baseline(&payload)?;
    if (image.width,image.height)!=(w,h)||
        image.color!=(if n==1{BaselineColor::Gray}else{BaselineColor::Srgb})||
        image.pixels.len()!=w*h*n{
        return Err(Error::Invalid("JPX/PDF decoded samples mismatch"));
    }
    // For non-stencil JPX images the PDF Decode entry is ignored.
    let rgb=if n==1{
        let mut rgb=Vec::with_capacity(w*h*3);
        for v in image.pixels{rgb.extend([v;3]);}rgb
    }else{image.pixels};
    Ok(Image{w,h,rgb,alpha:None,stencil:false,interpolate})
}


fn is_jbig2(d:&Dict)->bool{
    match d.get(b"Filter".as_slice()){
        Some(Object::Name(n))=>n==b"JBIG2Decode",
        Some(Object::Array(a))=>a.iter().any(|v|matches!(v,Object::Name(n) if n==b"JBIG2Decode")),
        _=>false,
    }
}

/// Parse a bounded, direct filter list. Terminal JBIG2 is allowed only
/// in the image stream; the globals stream must use byte wrappers only.
fn jbig2_filter_names(dict:&Dict,allow_empty:bool)->Result<(Vec<&[u8]>,bool)>{
    match dict.get(b"Filter".as_slice()){
        None|Some(Object::Null) if allow_empty=>Ok((Vec::new(),false)),
        Some(Object::Name(n))=>Ok((vec![n.as_slice()],false)),
        Some(Object::Array(a)) if !a.is_empty()&&a.len()<=8=>{
            let names=a.iter().map(|v|match v{
                Object::Name(n)=>Ok(n.as_slice()),
                _=>Err(Error::Invalid("JBIG2 filter name")),
            }).collect::<Result<Vec<_>>>()?;
            Ok((names,true))
        },
        _=>Err(Error::Unsupported("JBIG2 filter structure")),
    }
}
/// Resolve the parameter container and each parameter entry independently.
/// No parameter shifting, implicit predictors or misaligned arrays.
fn jbig2_parameters(doc:&Document<'_>,dict:&Dict,count:usize,array:bool)->Result<Vec<Object>>{
    let params=match dict.get(b"DecodeParms".as_slice()){
        None=>Object::Null,
        Some(o)=>doc.dereference(o)?,
    };
    match params{
        Object::Null=>Ok(vec![Object::Null;count]),
        Object::Dict(_) if !array&&count==1=>Ok(vec![params]),
        Object::Array(a) if array&&a.len()==count=>{
            a.iter().map(|o|doc.dereference(o)).collect()
        },
        _=>Err(Error::Unsupported("JBIG2 DecodeParms structure")),
    }
}
fn jbig2_byte_wrappers<'a>(data:&'a [u8],filters:&[&[u8]],params:&[Object],
    final_cap:usize,budget:&mut Budget)->Result<std::borrow::Cow<'a,[u8]>>{
    use std::borrow::Cow;
    if filters.len()!=params.len(){return Err(Error::Invalid("JBIG2 prefix parameter count"));}
    for (&name,param) in filters.iter().zip(params){
        if !matches!(name,b"ASCIIHexDecode"|b"AHx"|b"ASCII85Decode"|b"A85"|
            b"FlateDecode"|b"Fl"|b"RunLengthDecode"|b"RL"){
            return Err(Error::Unsupported("JBIG2 byte wrapper filter"));
        }
        // These filters carry encoded JBIG2 bytes, not image rows.
        if !matches!(param,Object::Null)&&!matches!(param,Object::Dict(p) if p.is_empty()){
            return Err(Error::Unsupported("JBIG2 byte wrapper DecodeParms"));
        }
    }
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JBIG2 wrapped input bytes"));}
    budget.charge(data.len())?;
    let mut payload=Cow::Borrowed(data);
    for (i,&name) in filters.iter().enumerate(){
        budget.charge(payload.len())?;
        let cap=crate::MAX_BYTES.min(super::MAX_WORK.saturating_sub(budget.work));
        let cap=if i+1==filters.len(){cap.min(final_cap)}else{cap};
        let mut d=Dict::new();
        d.insert(b"Filter".to_vec(),Object::Name(name.to_vec()));
        // Empty dictionaries are normalized to absent parameters.
        let stream=Object::Stream{dict:d,data:payload.into_owned()};
        let decoded=crate::filters::decode(&stream,cap)?;
        budget.charge(decoded.len())?;
        payload=Cow::Owned(decoded);
    }
    if payload.len()>final_cap{return Err(Error::Limit("JBIG2 wrapped output bytes"));}
    Ok(payload)
}
/// PDF image-only JBIG2 path. T.88 black=1 is converted to PDF
/// sample zero before applying Decode; row padding is never a pixel.
/// The unique terminal JBIG2 filter and the independent globals stream
/// may each be wrapped in bounded native lossless byte filters.
fn load_jbig2(doc:&Document<'_>,dict:&Dict,data:&[u8],w:usize,h:usize,
    interpolate:bool,budget:&mut Budget)->Result<Image>{
    use crate::filters::jbig2::page;
    let (filters,array)=jbig2_filter_names(dict,false)?;
    if filters.last()!=Some(&b"JBIG2Decode".as_slice())||
        filters.iter().filter(|&&n|n==b"JBIG2Decode").count()!=1{
        return Err(Error::Unsupported("JBIG2 must be unique terminal filter"));
    }
    let params=jbig2_parameters(doc,dict,filters.len(),array)?;
    let prefix=filters.len()-1;
    let payload=jbig2_byte_wrappers(data,&filters[..prefix],&params[..prefix],crate::MAX_BYTES,budget)?;
    let globals=match &params[prefix]{
        Object::Null=>Vec::new(),
        Object::Dict(p)=>{
            if p.keys().any(|k|k!=b"JBIG2Globals"){return Err(Error::Unsupported("JBIG2 DecodeParms key"));}
            match p.get(b"JBIG2Globals".as_slice()){
                None=>Vec::new(),
                Some(o@Object::Ref(..))=>{
                    let Object::Stream{dict,data}=doc.dereference(o)? else{return Err(Error::Invalid("JBIG2Globals stream"));};
                    let (names,array)=jbig2_filter_names(&dict,true)?;
                    let params=jbig2_parameters(doc,&dict,names.len(),array)?;
                    let cap=crate::MAX_BYTES.checked_sub(payload.len()).ok_or(Error::Limit("JBIG2 combined bytes"))?;
                    jbig2_byte_wrappers(&data,&names,&params,cap,budget)?.into_owned()
                }
                _=>return Err(Error::Invalid("JBIG2Globals indirect stream")),
            }
        }
        _=>return Err(Error::Invalid("JBIG2 DecodeParms dictionary")),
    };
    let stencil=flag(dict,b"ImageMask")?;
    if dict.contains_key(b"Mask".as_slice())||dict.contains_key(b"SMaskInData".as_slice())||
        !matches!(dict.get(b"SMask".as_slice()),None|Some(Object::Name(_))){
        return Err(Error::Unsupported("JBIG2 external mask"));
    }
    if let Some(Object::Name(n))=dict.get(b"SMask".as_slice()){
        if stencil||n!=b"None"{return Err(Error::Unsupported("JBIG2 soft mask"));}
    }
    if stencil{
        if dict.contains_key(b"ColorSpace".as_slice())||
            (dict.contains_key(b"BitsPerComponent".as_slice())&&bits(dict)?!=1){
            return Err(Error::Invalid("JBIG2 stencil dictionary"));
        }
    }else{
        if bits(dict)?!=1||device(&doc.dereference(dict.get(b"ColorSpace".as_slice())
            .ok_or(Error::Invalid("JBIG2 color space"))?)?)?!=1{
            return Err(Error::Unsupported("JBIG2 PDF samples"));
        }
    }
    let pair=pairs(dict,1)?[0];
    if stencil&&pair!=[0.,1.]&&pair!=[1.,0.]{return Err(Error::Invalid("JBIG2 stencil Decode"));}
    // Reserve conversion work and constrain the native executor by remaining
    // raster work. The executor itself charges input, graph and decode work.
    budget.charge(w*h)?;
    let mut limits=page::Limits::default();
    limits.page_pixels=(w*h) as u64;
    limits.work=limits.work.min(super::MAX_WORK.saturating_sub(budget.work) as u64);
    let decoded=page::decode_pdf_image_compatible(&globals,&payload,limits)?;
    budget.charge(usize::try_from(decoded.stats.work_used).map_err(|_|Error::Limit("JBIG2 raster work"))?)?;
    let bitmap=decoded.bitmap;
    if (bitmap.width() as usize,bitmap.height() as usize)!=(w,h){
        return Err(Error::Invalid("JBIG2/PDF dimensions mismatch"));
    }
    let mut rgb=Vec::with_capacity(if stencil{0}else{w*h*3});
    let mut alpha=if stencil{Some(Vec::with_capacity(w*h))}else{None};
    for row in bitmap.packed().chunks_exact(bitmap.stride()){
        for x in 0..w{
            let sample=if row[x/8]&(0x80>>(x%8))!=0{0}else{1};
            let v=channel(sample,1,pair);
            if let Some(a)=&mut alpha{a.push(255-v);}else{rgb.extend([v;3]);}
        }
    }
    Ok(Image{w,h,rgb,alpha,stencil,interpolate})
}

pub(super) fn load(doc:&Document<'_>,id:(u32,u16),budget:&mut Budget,cache:&mut IccCache,intent:crate::icc::RenderingIntent)->Result<Image>{
    let o=doc.resolve(id.0,id.1)?;let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("image stream"));};
    if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Image"){return Err(Error::Invalid("image subtype"));}
    let interpolate=flag(dict,b"Interpolate")?;
    let(w,h)=dimensions(dict)?;
    if is_jbig2(dict){
        let Object::Stream{data,..}=&o else{return Err(Error::Invalid("JBIG2 image stream"));};
        return load_jbig2(doc,dict,data,w,h,interpolate,budget);
    }
    if is_jpx(dict){
        let Object::Stream{data,..}=&o else{return Err(Error::Invalid("JPX image stream"));};
        return load_jpx(doc,dict,data,w,h,interpolate,budget);
    }
    if dict.contains_key(b"SMaskInData".as_slice()){return Err(Error::Unsupported("image interpolation or embedded alpha"));}
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
    let b=bits(dict)?;let max=max_sample(b);let cs=space(doc,dict,cache,budget,intent)?;
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
    // Caller guarantees nearest, non-stencil, no alpha. Preserve the exact
    // coordinate conversion and edge clamp used by the general sampler.
    #[inline]
    pub(super) fn opaque_nearest(&self,u:f64,v:f64)->[u8;3]{
        let x=((u*self.w as f64).floor()as usize).min(self.w-1);
        let y=((v*self.h as f64).floor()as usize).min(self.h-1);
        let p=(y*self.w+x)*3;
        [self.rgb[p],self.rgb[p+1],self.rgb[p+2]]
    }
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

#[cfg(test)]
mod jpx_pdf_tests{
    use super::*;
    // Existing independent OpenJPEG fixtures from B11.2h2b/h3, reused here.
    const GRAY:&str="ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000000000000ff5c0005424000ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000240001ff93cfb44c117bb2617fbedccc4de18580bdcbecf21b9c6fffd9";
    const ZERO:&str="ff4fff510029000000000005000000050000000000000000000000050000000500000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000100001ff938080ffd9";
    const RGB:&str="ff4fff51002f000000000005000000030000000000000000000000050000000300000000000000000003070101070101070101ff52000c00000001000000000000ff5c0005424000ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000560001ff93e7ed0efda0800650755682a4deb51fad11e62edd01d8be8fe7ed0efda0a005a7ecf01b757433609a96865e7f002b779b1fe7ed0efda0600ef58e3ef470c6917e13d6fbbc5f065fb3ffd9";
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn box_bytes(kind:&[u8;4],p:&[u8])->Vec<u8>{
        let mut b=((p.len()+8)as u32).to_be_bytes().to_vec();b.extend(kind);b.extend(p);b
    }
    fn jp2_zero()->Vec<u8>{
        let mut b=hex("0000000c6a5020200d0a870a00000014667479706a703220000000006a703220");
        let h=hex("000000166968647200000005000000050001070700000000000f636f6c7201000000000011");
        b.extend(box_bytes(b"jp2h",&h));b.extend(box_bytes(b"jp2c",&hex(ZERO)));b
    }
    fn pdf(data:&[u8],w:usize,h:usize,fields:&str)->Vec<u8>{
        let contents=format!("q {w} 0 0 {h} 0 0 cm /Im Do Q");
        pdf_layout(data,w,h,w,h,fields,&contents)
    }
    fn pdf_layout(data:&[u8],iw:usize,ih:usize,w:usize,h:usize,fields:&str,contents:&str)->Vec<u8>{
        pdf_layout_globals(data,iw,ih,w,h,fields,contents,None)
    }
    fn pdf_layout_globals(data:&[u8],iw:usize,ih:usize,w:usize,h:usize,fields:&str,contents:&str,globals:Option<&[u8]>)->Vec<u8>{
        pdf_layout_wrapped_globals(data,iw,ih,w,h,fields,contents,globals,"",None)
    }
    fn pdf_layout_wrapped_globals(data:&[u8],iw:usize,ih:usize,w:usize,h:usize,fields:&str,contents:&str,
        globals:Option<&[u8]>,globals_fields:&str,extra:Option<&str>)->Vec<u8>{
        let mut image=format!("<< /Type /XObject /Subtype /Image /Width {iw} /Height {ih} {fields} /Length {} >>\nstream\n",data.len()).into_bytes();
        image.extend(data);image.extend(b"\nendstream");
        let mut objects=vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>").into_bytes(),
            format!("<< /Length {} >>\nstream\n{contents}\nendstream",contents.len()).into_bytes(),
            image
        ];
        if let Some(g)=globals{
            let mut stream=format!("<< {globals_fields} /Length {} >>\nstream\n",g.len()).into_bytes();
            stream.extend(g);stream.extend(b"\nendstream");objects.push(stream);
        }
        if let Some(extra)=extra{objects.push(extra.as_bytes().to_vec());}
        let size=objects.len()+1;
        let mut b=b"%PDF-1.5\n".to_vec();let mut offsets=Vec::new();
        for (i,o) in objects.iter().enumerate(){
            offsets.push(b.len());b.extend(format!("{} 0 obj\n",i+1).as_bytes());b.extend(o);b.extend(b"\nendobj\n");
        }
        let x=b.len();b.extend(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for off in offsets{b.extend(format!("{off:010} 00000 n \n").as_bytes());}
        b.extend(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").as_bytes());b
    }
    fn render(data:&[u8],w:usize,h:usize,fields:&str)->Result<super::super::Bitmap>{
        let bytes=pdf(data,w,h,fields);
        let doc=Document::open(&bytes)?;let pages=doc.pages()?;
        doc.render_page(&pages[0],super::super::Options::default(),4096)
    }

    fn jbig2_segment(n:u8,kind:u8,body:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,n,kind,0,1];
        v.extend((body.len() as u32).to_be_bytes());v.extend(body);v
    }
    fn jbig2_fixture()->Vec<u8>{
        let mut info=Vec::new();for n in [9u32,2,0,0]{info.extend(n.to_be_bytes());}
        info.extend([0x44,0,0]); // black page, operator override
        let mut p=jbig2_segment(1,48,&info);
        let mut region=Vec::new();for n in [5u32,1,0,0]{region.extend(n.to_be_bytes());}
        region.extend([4,1,0x80]); // white MMR row, replace five pixels
        p.extend(jbig2_segment(2,39,&region));p
    }
    #[test]fn pdf_jbig2_gray_polarity_and_padding(){
        let data=jbig2_fixture();
        let want=[255u8,255,255,255,255,0,0,0,0,0,0,0,0,0,0,0,0,0];
        for (filter,reverse) in [("/Filter /JBIG2Decode",false),
            ("/Filter [/JBIG2Decode] /DecodeParms [<< >>]",false),
            ("/Filter /JBIG2Decode /Decode [1 0]",true)]{
            let fields=format!("{filter} /ColorSpace /DeviceGray /BitsPerComponent 1");
            let a=render(&data,9,2,&fields).unwrap();
            assert_eq!((a.width,a.height),(9,2));
            for (p,e) in a.rgba.chunks_exact(4).zip(want){
                let e=if reverse{255-e}else{e};assert_eq!(p,[e,e,e,255]);
            }
        }
    }
    #[test]fn pdf_jbig2_stencil_polarity(){
        let data=jbig2_fixture();
        let bytes=pdf_layout(&data,9,2,9,2,"/Filter /JBIG2Decode /ImageMask true",
            "q 1 0 0 rg 9 0 0 2 0 0 cm /Im Do Q");
        let doc=Document::open(&bytes).unwrap();
        let a=doc.render_page(&doc.pages().unwrap()[0],super::super::Options::default(),4096).unwrap();
        for (i,p) in a.rgba.chunks_exact(4).enumerate(){
            assert_eq!(p,if i<5{[255,255,255,255]}else{[255,0,0,255]});
        }
    }
    // Original T.88 Annex H page 1 and globals, already independently
    // pixel-validated in page::annex_h_full_page_tests.
    const GLOBAL1:&[u8]=&[
        0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x18,0x00,0x01,0x00,0x00,0x00,
        0x01,0x00,0x00,0x00,0x01,0xe9,0xcb,0xf4,0x00,0x26,0xaf,0x04,0xbf,0xf0,0x78,0x2f,
        0xe0,0x00,0x40,
    ];
    const PAGE1:&[u8]=&[
        0x00,0x00,0x00,0x01,0x30,0x00,0x01,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x40,0x00,
        0x00,0x00,0x38,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,
        0x00,0x02,0x00,0x01,0x01,0x00,0x00,0x00,0x1c,0x00,0x01,0x00,0x00,0x00,0x02,0x00,
        0x00,0x00,0x02,0xe5,0xcd,0xf8,0x00,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,
        0x10,0x79,0xf0,0x00,0x80,0x00,0x00,0x00,0x03,0x07,0x42,0x00,0x02,0x01,0x00,0x00,
        0x00,0x31,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,
        0x00,0x01,0x00,0x0c,0x09,0x00,0x10,0x00,0x00,0x00,0x05,0x01,0x10,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0c,0x40,0x07,0x08,
        0x70,0x41,0xd0,0x00,0x00,0x00,0x04,0x27,0x00,0x01,0x00,0x00,0x00,0x2c,0x00,0x00,
        0x00,0x36,0x00,0x00,0x00,0x2c,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x0b,0x00,0x01,
        0x26,0xa0,0x71,0xce,0xa7,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
        0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xf8,0xf0,0x00,0x00,0x00,0x05,0x10,0x01,
        0x01,0x00,0x00,0x00,0x2d,0x01,0x04,0x04,0x00,0x00,0x00,0x0f,0x20,0xd1,0x84,0x61,
        0x18,0x45,0xf2,0xf9,0x7c,0x8f,0x11,0xc3,0x9e,0x45,0xf2,0xf9,0x7d,0x42,0x85,0x0a,
        0xaa,0x84,0x62,0x2f,0xee,0xec,0x44,0x62,0x22,0x35,0x2a,0x0a,0x83,0xb9,0xdc,0xee,
        0x77,0x80,0x00,0x00,0x00,0x06,0x17,0x20,0x05,0x01,0x00,0x00,0x00,0x57,0x00,0x00,
        0x00,0x20,0x00,0x00,0x00,0x24,0x00,0x00,0x00,0x10,0x00,0x00,0x00,0x0f,0x00,0x01,
        0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x09,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x04,0x00,0x00,0x00,0xaa,0xaa,0xaa,0xaa,0x80,0x08,0x00,0x80,0x36,0xd5,0x55,0x6b,
        0x5a,0xd4,0x00,0x40,0x04,0x2e,0xe9,0x52,0xd2,0xd2,0xd2,0x8a,0xa5,0x4a,0x00,0x20,
        0x02,0x23,0xe0,0x95,0x24,0xb4,0x92,0x8a,0x4a,0x92,0x54,0x92,0xd2,0x4a,0x29,0x2a,
        0x49,0x40,0x04,0x00,0x40,0x00,0x00,0x00,0x07,0x31,0x00,0x01,0x00,0x00,0x00,0x00,
    ];
    #[test]fn pdf_jbig2_indirect_globals(){
        let want=crate::filters::jbig2::page::decode_pdf_page(GLOBAL1,PAGE1,Default::default()).unwrap();
        let fields="/Filter /JBIG2Decode /ColorSpace /DeviceGray /BitsPerComponent 1 /DecodeParms << /JBIG2Globals 6 0 R >>";
        let bytes=pdf_layout_globals(PAGE1,64,56,64,56,fields,"q 64 0 0 56 0 0 cm /Im Do Q",Some(GLOBAL1));
        let doc=Document::open(&bytes).unwrap();
        let a=doc.render_page(&doc.pages().unwrap()[0],super::super::Options::default(),16384).unwrap();
        assert_eq!((a.width,a.height),(64,56));
        for (i,p) in a.rgba.chunks_exact(4).enumerate(){
            let e=if want.bitmap.get((i%64) as u32,(i/64) as u32).unwrap(){0}else{255};
            assert_eq!(p,[e,e,e,255]);
        }
        assert!(render(PAGE1,64,56,"/Filter /JBIG2Decode /ColorSpace /DeviceGray /BitsPerComponent 1").is_err());
    }

    fn ascii_hex_wrapper(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();for b in data{out.extend(format!("{b:02x}").as_bytes());}out.push(b'>');out
    }
    fn rl_wrapper(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();for chunk in data.chunks(128){out.push((chunk.len()-1) as u8);out.extend(chunk);}
        out.push(128);out
    }
    fn stored_zlib_wrapper(data:&[u8])->Vec<u8>{
        assert!(data.len()<=65535);
        let n=data.len() as u16;
        let mut out=vec![0x78,0x01,0x01];out.extend(n.to_le_bytes());out.extend((!n).to_le_bytes());out.extend(data);
        let (mut a,mut b)=(1u32,0u32);
        for &v in data{a=(a+u32::from(v))%65521;b=(b+a)%65521;}
        out.extend(((b<<16)|a).to_be_bytes());out
    }
    fn ascii85_wrapper(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();
        for chunk in data.chunks(4){
            let mut word=[0u8;4];word[..chunk.len()].copy_from_slice(chunk);
            let mut v=u32::from_be_bytes(word);let mut digits=[0u8;5];
            for x in digits.iter_mut().rev(){*x=(v%85) as u8+33;v/=85;}
            out.extend(&digits[..chunk.len()+1]);
        }
        out.extend(b"~>");out
    }
    #[test]fn pdf_jbig2_wrapped_terminal_filters(){
        let data=jbig2_fixture();let metadata="/ColorSpace /DeviceGray /BitsPerComponent 1";
        let base=render(&data,9,2,&format!("/Filter /JBIG2Decode {metadata}")).unwrap();
        let variants=[
            (ascii_hex_wrapper(&data),"/Filter [/AHx /JBIG2Decode] /DecodeParms [null <<>>]"),
            (ascii85_wrapper(&data),"/Filter [/ASCII85Decode /JBIG2Decode]"),
            (stored_zlib_wrapper(&data),"/Filter [/Fl /JBIG2Decode] /DecodeParms [<<>> null]"),
            (rl_wrapper(&data),"/Filter [/RunLengthDecode /JBIG2Decode]"),
            (ascii_hex_wrapper(&rl_wrapper(&stored_zlib_wrapper(&data))),"/Filter [/AHx /RL /FlateDecode /JBIG2Decode] /DecodeParms [null null null null]"),
        ];
        for (wrapped,fields) in variants{
            let image=render(&wrapped,9,2,&format!("{fields} {metadata}")).unwrap();
            assert_eq!(image.rgba,base.rgba);
        }
    }
    #[test]fn pdf_jbig2_compressed_globals_and_indirect_parameters(){
        let fields="/Filter [/AHx /JBIG2Decode] /ColorSpace /DeviceGray /BitsPerComponent 1 /DecodeParms 7 0 R";
        let g=ascii85_wrapper(&stored_zlib_wrapper(GLOBAL1));
        let p=ascii_hex_wrapper(PAGE1);
        let bytes=pdf_layout_wrapped_globals(&p,64,56,64,56,fields,"q 64 0 0 56 0 0 cm /Im Do Q",
            Some(&g),"/Filter [/A85 /FlateDecode] /DecodeParms [null <<>>]",Some("[null << /JBIG2Globals 6 0 R >>]"));
        let doc=Document::open(&bytes).unwrap();
        let actual=doc.render_page(&doc.pages().unwrap()[0],super::super::Options::default(),16384).unwrap();
        let want=crate::filters::jbig2::page::decode_pdf_page(GLOBAL1,PAGE1,Default::default()).unwrap();
        for (i,p) in actual.rgba.chunks_exact(4).enumerate(){
            let e=if want.bitmap.get((i%64) as u32,(i/64) as u32).unwrap(){0}else{255};assert_eq!(p,[e,e,e,255]);
        }
    }
    #[test]fn pdf_jbig2_wrapped_metadata_and_budget_rejection(){
        let data=jbig2_fixture();let wrapped=ascii_hex_wrapper(&data);
        let meta="/ColorSpace /DeviceGray /BitsPerComponent 1";
        for f in ["/Filter [/AHx /JBIG2Decode] /DecodeParms [null]",
            "/Filter [/AHx /JBIG2Decode] /DecodeParms [<< /Predictor 2 >> null]",
            "/Filter [/AHx /JBIG2Decode] /DecodeParms [null << /Predictor 1 >>]",
            "/Filter [/JBIG2Decode /JBIG2Decode]","/Filter [/JBIG2Decode /AHx]",
            "/Filter [/DCTDecode /JBIG2Decode]","/Filter [/AHx /JBIG2Decode] /DecodeParms <<>>"]{
            assert!(render(&wrapped,9,2,&format!("{f} {meta}")).is_err(),"{f}");
        }
        assert!(render(&wrapped[..wrapped.len()-1],9,2,&format!("/Filter [/AHx /JBIG2Decode] {meta}")).is_err());
        let mut budget=Budget{work:0,stage:"JBIG2 wrapper test"};
        assert!(jbig2_byte_wrappers(&wrapped,&[b"AHx"],&[Object::Null],data.len()-1,&mut budget).is_err());
        let mut budget=Budget{work:super::super::MAX_WORK-wrapped.len()*2-data.len()+1,stage:"JBIG2 wrapper test"};
        assert!(jbig2_byte_wrappers(&wrapped,&[b"AHx"],&[Object::Null],data.len(),&mut budget).is_err());
        let f="/Filter /JBIG2Decode /ColorSpace /DeviceGray /BitsPerComponent 1 /DecodeParms << /JBIG2Globals 6 0 R >>";
        for gf in ["/Filter /JBIG2Decode","/Filter /FlateDecode /DecodeParms << /Predictor 2 >>","/Filter /AHx /DecodeParms [null]"]{
            let bytes=pdf_layout_wrapped_globals(PAGE1,64,56,64,56,f,"q 64 0 0 56 0 0 cm /Im Do Q",Some(GLOBAL1),gf,None);
            let doc=Document::open(&bytes).unwrap();
            assert!(doc.render_page(&doc.pages().unwrap()[0],super::super::Options::default(),16384).is_err());
        }
    }
    #[test]fn pdf_jbig2_invalid_metadata_and_budget(){
        let data=jbig2_fixture();
        let f="/Filter /JBIG2Decode /ColorSpace /DeviceGray /BitsPerComponent 1";
        for extra in ["/DecodeParms << /JBIG2Globals 6 0 R >>",
            "/DecodeParms << /JBIG2Globals 5 0 R >>","/DecodeParms << /JBIG2Globals 42 >>",
            "/DecodeParms << /Other 1 >>","/DecodeParms [null]","/Mask [0 1]","/SMaskInData 1"]{
            assert!(render(&data,9,2,&format!("{f} {extra}")).is_err(),"{extra}");
        }
        assert!(render(&data,8,2,f).is_err());
        assert!(render(&data,18,1,f).is_err());
        assert!(render(&data,9,2,"/Filter [/JBIG2Decode /AHx] /ColorSpace /DeviceGray /BitsPerComponent 1").is_err());
        assert!(render(&data,9,2,"/Filter /JBIG2Decode /ColorSpace /DeviceGray /BitsPerComponent 8").is_err());
        assert!(render(&data[..data.len()-1],9,2,f).is_err());
        let bytes=pdf(&data,9,2,f);let doc=Document::open(&bytes).unwrap();
        assert!(crate::filters::decode(&doc.resolve(5,0).unwrap(),64).is_err());
        let mut budget=Budget{work:super::super::MAX_WORK-1,stage:"JBIG2 test budget"};
        assert!(load(&doc,(5,0),&mut budget,&mut IccCache::default(),crate::icc::RenderingIntent::RelativeColorimetric).is_err());
    }

    #[test]fn pdf_jpx_raw_gray(){
        let a=render(&hex(GRAY),4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [1 0]").unwrap();
        assert_eq!((a.width,a.height),(4,4));
        let want=[22u8,60,96,134,172,208,246,26,62,100,138,174,212,248,28,66];
        assert_eq!(a.rgba.len(),64);
        for (p,e) in a.rgba.chunks_exact(4).zip(want){
            assert_eq!(p[0],p[1]);assert_eq!(p[1],p[2]);assert_eq!(p[3],255);
            assert!(p[0].abs_diff(e)<=1);
        }
    }
    #[test]fn pdf_jpx_container_implicit_metadata(){
        let a=render(&jp2_zero(),5,5,"/Filter /JPXDecode /SMaskInData 0").unwrap();
        assert_eq!((a.width,a.height),(5,5));assert_eq!(a.rgba.len(),100);
        assert!(a.rgba.chunks_exact(4).all(|p|p==[128,128,128,255]));
    }
    #[test]fn pdf_jpx_raw_rgb(){
        let a=render(&hex(RGB),5,3,"/Filter /JPXDecode /ColorSpace /DeviceRGB").unwrap();
        let want=hex("163c6086acd0f61a3e648aaed4f81c42668cb2d6fc20446a90b4dafe22486c92b8dc00264a7096bae004284e72");
        assert_eq!((a.width,a.height),(5,3));assert_eq!(a.rgba.len(),60);
        for (p,e) in a.rgba.chunks_exact(4).zip(want.chunks_exact(3)){
            assert_eq!(p[3],255);assert!(p[..3].iter().zip(e).all(|(&x,&y)|x.abs_diff(y)<=1));
        }
    }
    #[test]fn pdf_jpx_rejects_dictionary(){
        let data=hex(GRAY);
        for f in [
            "/Filter /JPXDecode",
            "/Filter /JPXDecode /ColorSpace /DeviceRGB",
            "/Filter /JPXDecode /ColorSpace /DeviceGray /BitsPerComponent 16",
            "/Filter /JPXDecode /ColorSpace /DeviceGray /SMaskInData 1",
            "/Filter /JPXDecode /ColorSpace /DeviceGray /Mask [0 0]",
            "/Filter /JPXDecode /ColorSpace /DeviceGray /ImageMask true",
            "/Filter /JPXDecode /ColorSpace /DeviceGray /DecodeParms << /ColorTransform 1 >>",
            "/Filter [/JPXDecode /ASCIIHexDecode] /ColorSpace /DeviceGray /BitsPerComponent 8",
        ]{assert!(render(&data,4,4,f).is_err(),"{f}");}
        assert!(render(&data,5,4,"/Filter /JPXDecode /ColorSpace /DeviceGray").is_err());
        assert!(render(&data[..data.len()-1],4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray").is_err());
    }
    #[test]fn pdf_jpx_budget_and_content_stream_isolation(){
        let data=hex(GRAY);
        let bytes=pdf(&data,4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray");
        let doc=Document::open(&bytes).unwrap();let stream=doc.resolve(5,0).unwrap();
        assert!(crate::filters::decode(&stream,64).is_err());
        let mut budget=Budget{work:super::super::MAX_WORK-1,stage:"JPX test budget"};
        assert!(load(&doc,(5,0),&mut budget,&mut IccCache::default(),crate::icc::RenderingIntent::RelativeColorimetric).is_err());
    }

    fn ascii_hex_wrap(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();
        for b in data{out.extend(format!("{b:02x}").bytes());}
        out.push(b'>');out
    }
    fn run_wrap(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();
        for chunk in data.chunks(128){out.push((chunk.len()-1)as u8);out.extend(chunk);}
        out.push(128);out
    }
    fn ascii85_wrap(data:&[u8])->Vec<u8>{
        let mut out=Vec::new();
        for chunk in data.chunks(4){
            let mut word=[0u8;4];word[..chunk.len()].copy_from_slice(chunk);
            let mut n=u32::from_be_bytes(word);
            let mut group=[0u8;5];
            for i in (0..5).rev(){group[i]=(n%85)as u8+33;n/=85;}
            out.extend(&group[..chunk.len()+1]);
        }
        out.extend(b"~>");out
    }
    fn stored_flate(data:&[u8])->Vec<u8>{
        assert!(data.len()<=65535);
        let n=data.len()as u16;
        let mut out=vec![0x78,0x01,1];
        out.extend(n.to_le_bytes());out.extend((!n).to_le_bytes());out.extend(data);
        let (mut a,mut b)=(1u32,0u32);
        for &v in data{a=(a+u32::from(v))%65521;b=(b+a)%65521;}
        out.extend(((b<<16)|a).to_be_bytes());out
    }
    #[test]fn pdf_jpx_single_filter_array(){
        let data=hex(GRAY);
        let a=render(&data,4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray").unwrap();
        for fields in ["/Filter [/JPXDecode] /ColorSpace /DeviceGray",
            "/Filter [/JPXDecode] /ColorSpace /DeviceGray /DecodeParms [null]",
            "/Filter [/JPXDecode] /ColorSpace /DeviceGray /DecodeParms [<< >>]"]{
            assert_eq!(render(&data,4,4,fields).unwrap(),a);
        }
    }
    #[test]fn pdf_jpx_prefix_ascii(){
        let data=hex(GRAY);
        let a=render(&data,4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray").unwrap();
        for (wrapped,fields) in [
            (ascii_hex_wrap(&data),"/Filter [/ASCIIHexDecode /JPXDecode] /ColorSpace /DeviceGray /DecodeParms [null null]"),
            (ascii85_wrap(&data),"/Filter [/A85 /JPXDecode] /ColorSpace /DeviceGray")]{
            assert_eq!(render(&wrapped,4,4,fields).unwrap(),a);
        }
    }
    #[test]fn pdf_jpx_prefix_flate_runlength(){
        let data=hex(RGB);
        let a=render(&data,5,3,"/Filter /JPXDecode /ColorSpace /DeviceRGB").unwrap();
        for (wrapped,fields) in [
            (stored_flate(&data),"/Filter [/FlateDecode /JPXDecode] /DecodeParms [<< >> null] /ColorSpace /DeviceRGB"),
            (run_wrap(&data),"/Filter [/RL /JPXDecode] /ColorSpace /DeviceRGB")]{
            assert_eq!(render(&wrapped,5,3,fields).unwrap(),a);
        }
    }
    #[test]fn pdf_jpx_multiple_prefixes_and_jp2(){
        let data=jp2_zero();
        let wrapped=ascii85_wrap(&stored_flate(&run_wrap(&data)));
        let a=render(&data,5,5,"/Filter /JPXDecode").unwrap();
        assert_eq!(render(&wrapped,5,5,"/Filter [/ASCII85Decode /Fl /RunLengthDecode /JPXDecode] /DecodeParms [null null null << >>]").unwrap(),a);
    }
    #[test]fn pdf_jpx_prefix_validation(){
        let data=hex(GRAY);
        for fields in [
            "/Filter [/JPXDecode /AHx] /ColorSpace /DeviceGray",
            "/Filter [/JPXDecode /JPXDecode] /ColorSpace /DeviceGray",
            "/Filter [/DCTDecode /JPXDecode] /ColorSpace /DeviceGray",
            "/Filter [/AHx 7 /JPXDecode] /ColorSpace /DeviceGray",
            "/Filter [/AHx /JPXDecode] /DecodeParms [null] /ColorSpace /DeviceGray",
            "/Filter [/Fl /JPXDecode] /DecodeParms [<< /Predictor 12 >> null] /ColorSpace /DeviceGray",
            "/Filter [/AHx /JPXDecode] /DecodeParms [null << /ColorTransform 1 >>] /ColorSpace /DeviceGray",
            "/Filter [/AHx /AHx /AHx /AHx /AHx /AHx /AHx /AHx /JPXDecode] /ColorSpace /DeviceGray"
        ]{assert!(render(&data,4,4,fields).is_err(),"{fields}");}
        assert!(render(b"123",4,4,"/Filter [/AHx /JPXDecode] /ColorSpace /DeviceGray").is_err());
    }
    #[test]fn pdf_jpx_prefix_budget_and_isolation(){
        let data=ascii_hex_wrap(&hex(GRAY));
        let bytes=pdf(&data,4,4,"/Filter [/AHx /JPXDecode] /ColorSpace /DeviceGray");
        let doc=Document::open(&bytes).unwrap();let stream=doc.resolve(5,0).unwrap();
        assert!(crate::filters::decode(&stream,4096).is_err());
        let Object::Stream{dict,data}=&stream else{panic!("stream");};
        let mut budget=Budget{work:super::super::MAX_WORK-data.len()*2-1,stage:"JPX prefix budget"};
        assert!(matches!(jpx_payload(dict,data,&mut budget),Err(Error::Limit(_))));
    }

    // B11.2i3a: independent layout reference pixels from the historical
    // OpenJPEG-decoded 4x4 gray signal. Exact integer source-index geometry
    // for nearest-neighbor/rotation/clip, rational 2x2 subpixel averaging
    // for bilinear interpolation. Poppler 25.06.0 independent PDF raster
    // reference is used separately for uniform image interiors below.
    const GRAY_LAYOUT_REFERENCE:[u8;16]=[22,60,96,134,172,208,246,26,62,100,138,174,212,248,28,66];
    fn layout(data:&[u8],w:usize,h:usize,fields:&str,contents:&str)->Result<super::super::Bitmap>{
        let bytes=pdf_layout(data,4,4,w,h,fields,contents);
        let doc=Document::open(&bytes)?;let pages=doc.pages()?;
        doc.render_page(&pages[0],super::super::Options::default(),4096)
    }
    fn gray_layout_compare(a:&super::super::Bitmap,w:usize,h:usize,want:&[u8]){
        assert_eq!((a.width,a.height),(w,h));
        assert_eq!(a.rgba.len(),4*w*h);assert_eq!(want.len(),w*h);
        for (i,(p,&e)) in a.rgba.chunks_exact(4).zip(want).enumerate(){
            assert_eq!(p[0],p[1]);assert_eq!(p[1],p[2]);assert_eq!(p[3],255);
            assert!(p[0].abs_diff(e)<=1,"pixel {i}: actual {}, expected {e}",p[0]);
        }
    }
    #[test]fn pdf_jpx_layout_nearest_scale(){
        let a=layout(&hex(GRAY),8,8,"/Filter /JPXDecode /ColorSpace /DeviceGray /Interpolate false",
            "q 8 0 0 8 0 0 cm /Im Do Q").unwrap();
        let want:Vec<u8>=(0..64).map(|i|GRAY_LAYOUT_REFERENCE[(i/8/2)*4+(i%8)/2]).collect();
        gray_layout_compare(&a,8,8,&want);
    }
    #[test]fn pdf_jpx_layout_rotate(){
        let a=layout(&hex(GRAY),4,4,"/Filter /JPXDecode /ColorSpace /DeviceGray",
            "q 0 4 -4 0 4 0 cm /Im Do Q").unwrap();
        gray_layout_compare(&a,4,4,&hex("861aae4260f68a1c3cd064f816ac3ed4"));
    }
    #[test]fn pdf_jpx_layout_clip_repeat(){
        let a=layout(&hex(GRAY),8,4,"/Filter /JPXDecode /ColorSpace /DeviceGray",
            "q 0 0 2 4 re W n 4 0 0 4 0 0 cm /Im Do Q q 4 0 0 4 4 0 cm /Im Do Q").unwrap();
        let want:Vec<u8>=(0..32).map(|i|match i%8{
            0|1=>GRAY_LAYOUT_REFERENCE[i/8*4+i%8],
            2|3=>255,
            _=>GRAY_LAYOUT_REFERENCE[i/8*4+i%8-4],
        }).collect();
        gray_layout_compare(&a,8,4,&want);
    }
    #[test]fn pdf_jpx_layout_bilinear(){
        let a=layout(&hex(GRAY),8,8,"/Filter /JPXDecode /ColorSpace /DeviceGray /Interpolate true",
            "q 8 0 0 8 0 0 cm /Im Do Q").unwrap();
        // Python Fraction reference calculated independently from exact
        // eighth-coordinate bilinear weights and four subcenter samples.
        gray_layout_compare(&a,8,8,&hex("16203346586a7d863c45586a7c7f726c8790a2b4c7aa5c36919aacbfd2b466405a6376899c9e908a646d808275788a94afb8caac5e41545ed4def0c254263942"));
    }
    #[test]fn pdf_jpx_layout_poppler_uniform(){
        // New independent Pillow 12.3.0/OpenJPEG 2.5.4 4x4 constant JP2,
        // actual Poppler 25.06.0 pdftoppm -r 72 RGB reference generated in
        // B11.2i3a from the exact same PDF layout and dictionary.
        let data=hex("0000000c6a5020200d0a870a00000014667479706a703220000000006a7032200000002d6a703268000000166968647200000004000000040001070700000000000f636f6c7201000000000011000000896a703263ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000100001ff938080ffd9");
        let a=layout(&data,8,4,"/Filter /JPXDecode /ColorSpace /DeviceGray /BitsPerComponent 8",
            "q 1 0 2 4 re W n 4 0 0 4 0 0 cm /Im Do Q q 0 4 -4 0 8 0 cm /Im Do Q").unwrap();
        let external=hex("ff8080ef80808080ff8080ef80808080ff8080ef80808080ff8080ef80808080");
        assert_eq!((a.width,a.height),(8,4));assert_eq!(a.rgba.len(),128);
        for (i,p) in a.rgba.chunks_exact(4).enumerate(){
            // Poppler antialiases the clip boundary into column 3 (239);
            // RPCore's 2x2 coverage gives exact white there. Do not hide
            // this policy difference by weakening the interior threshold.
            let want=if i%8==3{255}else{external[i]};
            assert_eq!(p,[want,want,want,255],"pixel {i}");
        }
    }
}

#[cfg(test)]
#[test]
fn opaque_nearest_matches_general_sampler(){
    for w in 1..=9usize{for h in 1..=7usize{
        let img=Image{w,h,rgb:(0..w*h*3).map(|n|(n*73+19) as u8).collect(),alpha:None,stencil:false,interpolate:false};
        for x in 0..=64{for y in 0..=64{
            let u=x as f64/64.;let v=y as f64/64.;
            assert_eq!((img.opaque_nearest(u,v),1.),img.sample(u,v,[7,9,11]));
        }}
    }}
}
