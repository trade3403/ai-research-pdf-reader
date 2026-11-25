//! PDF ICCBased source color transforms and compatible device Alternate fallback.
use crate::{Document,Object,Error,Result,icc::ProfilePath};
pub(crate) fn icc_components(doc:&Document<'_>,a:&[Object])->Result<usize>{
    Ok(icc_stream(doc,a)?.1)
}
fn icc_stream(doc:&Document<'_>,a:&[Object])->Result<(Object,usize)>{
    if a.len()!=2{return Err(Error::Invalid("ICCBased array"));}
    let o=doc.dereference(&a[1])?;
    let Object::Stream{dict,..}=&o else{return Err(Error::Invalid("ICCBased profile stream"));};
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
    Ok((o,n))
}

// Native calibrated color to an sRGB display target. Bradford adaptation
// and the standard sRGB transfer function; see W3C CSS Color 4 conversion
// and ISO 32000 CIE-based color spaces.
const BRADFORD:[[f64;3];3]=[
    [0.8951,0.2664,-0.1614],[-0.7502,1.7135,0.0367],[0.0389,-0.0685,1.0296]
];
const BRADFORD_INV:[[f64;3];3]=[
    [0.9869929,-0.1470543,0.1599627],[0.4323053,0.5183603,0.0492912],[-0.0085287,0.0400428,0.9684867]
];
const XYZ_RGB:[[f64;3];3]=[
    [3.2404542,-1.5371385,-0.4985314],[-0.9692660,1.8760108,0.0415560],[0.0556434,-0.2040259,1.0572252]
];
const D65:[f64;3]=[0.95047,1.,1.08883];
fn mv(m:[[f64;3];3],v:[f64;3])->[f64;3]{
    m.map(|r|r[0]*v[0]+r[1]*v[1]+r[2]*v[2])
}
fn srgb(x:f64)->f64{
    let x=x.clamp(0.,1.);
    if x<=0.0031308{12.92*x}else{1.055*x.powf(1./2.4)-0.055}
}

const MAX_ICC_BYTES:usize=16*1024*1024;
pub(crate) const ICC_PAGE_BYTES:usize=32*1024*1024;
pub(crate) const ICC_PAGE_PROFILES:usize=16;
#[derive(Debug)]
pub(crate) enum IccTransform{
    Rgb(crate::icc::RgbMatrix),
    Gray(crate::icc::GrayTrc),
    Cmyk(crate::icc::OwnedCmykLut),
    Mab(crate::icc::CmykMab),
    Mpe{transform:crate::icc::MpeTransform<'static>,components:usize},
    AbsoluteOutput{inner:Box<IccTransform>,scale:[f64;3]},
}
impl IccTransform{
    pub(crate) fn components(&self)->usize{match self{Self::Gray(_)=>1,Self::Rgb(_)=>3,Self::Cmyk(_)|Self::Mab(_)=>4,Self::Mpe{components,..}=>*components,Self::AbsoluteOutput{inner,..}=>inner.components()}}
    pub(crate) fn work(&self)->usize{
        match self{Self::Mpe{transform,..}=>transform.work().max(32),Self::AbsoluteOutput{inner,..}=>inner.work()+3,_=>if self.components()==4{128}else{32}}
    }
    fn xyz(&self,v:[f64;4])->Result<[f64;3]>{
        Ok(match self{
            Self::Rgb(m)=>m.to_xyz_d50([v[0],v[1],v[2]])?,
            Self::Gray(t)=>t.to_xyz_d50(v[0])?,
            Self::Cmyk(lut)=>lut.to_xyz_d50(v)?,
            Self::Mab(lut)=>lut.to_xyz_d50(v)?,
            Self::Mpe{transform,components}=>transform.to_xyz_d50(&v[..*components])?,
            Self::AbsoluteOutput{inner,scale}=>{let xyz=inner.xyz(v)?;std::array::from_fn(|i|xyz[i]*scale[i])},
        })
    }
    pub(crate) fn rgb(&self,v:[f64;4])->Result<[f64;3]>{
        let xyz=self.xyz(v)?;
        let src=mv(BRADFORD,[0.9642,1.,0.8249]);let dst=mv(BRADFORD,D65);
        let cone=mv(BRADFORD,xyz);
        let adapted=mv(BRADFORD_INV,std::array::from_fn(|i|cone[i]*dst[i]/src[i]));
        Ok(mv(XYZ_RGB,adapted).map(srgb))
    }
}
/// Default relative-colorimetric source path. For legacy CMYK choose A2B1
/// if present, otherwise A2B0. Full PDF rendering-intent switching is deferred.
/// Invalid or unsupported profile payloads use the documented device alternate;
/// a recognized profile with wrong component count or a resource limit is fatal.
pub(crate) fn load_icc(doc:&Document<'_>,a:&[Object],remaining:usize,intent:crate::icc::RenderingIntent)
    ->Result<(usize,Option<IccTransform>,usize,ProfilePath)>{
    let (o,n)=icc_stream(doc,a)?;
    let Object::Stream{data,..}=&o else{unreachable!()};
    let encoded=data.len();
    if encoded>MAX_ICC_BYTES||encoded>remaining{return Err(Error::Limit("ICC encoded bytes"));}
    let bytes=match crate::filters::decode(&o,MAX_ICC_BYTES.min(remaining-encoded)){
        Ok(b)=>b,Err(Error::Limit(s))=>return Err(Error::Limit(s)),Err(Error::Invalid(s)|Error::Unsupported(s))=>return Ok((n,None,encoded,ProfilePath::Alternate{reason:s})),
    };
    let charged=encoded+bytes.len();
    let p=match crate::icc::Profile::parse(&bytes){
        Ok(p)=>p,Err(Error::Limit(s))=>return Err(Error::Limit(s)),Err(Error::Invalid(s)|Error::Unsupported(s))=>return Ok((n,None,charged,ProfilePath::Alternate{reason:s})),
    };
    if matches!(&p.color_space,b"GRAY"|b"RGB "|b"CMYK"){
        let actual=match &p.color_space{b"GRAY"=>1,b"RGB "=>3,_=>4};
        if n!=actual{return Err(Error::Invalid("ICCBased profile N mismatch"));}
    }
    // Exact MPE intent takes precedence; native D2B3 is already absolute.
    use crate::icc::{MpeDirection,RenderingIntent};
    let tag=MpeDirection::DeviceToPcs.tag(intent);
    if p.tag(&tag).is_some(){
        match p.mpe_transform(MpeDirection::DeviceToPcs,intent){
            Ok(transform)=>{
                // Bound per-operation vector work as well as per-pixel
                // raster work. Charge owned payloads conservatively.
                if transform.work()>4096{return Err(Error::Limit("PDF MPE evaluation work"));}
                let charged=charged.checked_add(bytes.len()).ok_or(Error::Limit("ICC owned MPE bytes"))?;
                if charged>remaining{return Err(Error::Limit("ICC owned MPE bytes"));}
                let transform=transform.into_owned();
                return Ok((n,Some(IccTransform::Mpe{transform,components:n}),charged,ProfilePath::Mpe{tag}));
            }
            Err(Error::Unsupported(_))=>{},
            Err(Error::Limit(s))=>return Err(Error::Limit(s)),
            Err(Error::Invalid(s))=>return Ok((n,None,charged,ProfilePath::Alternate{reason:s})),
        }
    }
    // Legacy candidates: exact intent first, then A2B0 for missing or
    // unsupported LUTs. Invalid payloads remain explicit alternate decisions.
    let preferred=match intent{RenderingIntent::Perceptual=>b"A2B0",RenderingIntent::Saturation=>b"A2B2",_=>b"A2B1"};
    let prepare=|key:&crate::icc::Signature|->Result<IccTransform>{
        match n{
            1=>p.gray_trc().map(IccTransform::Gray),
            3=>p.rgb_matrix().map(IccTransform::Rgb),
            4 if p.tag(key).is_some_and(|b|b.starts_with(b"mAB "))=>p.cmyk_mab(key).map(IccTransform::Mab),
            4=>p.cmyk_a2b(key).and_then(|lut|{lut.to_xyz_d50([0.;4])?;Ok(IccTransform::Cmyk(lut.to_owned_lut()))}),
            _=>Err(Error::Unsupported("ICC transform channels")),
        }
    };
    let mut key=preferred;
    let mut transform=prepare(key);
    if n==4&&key!=b"A2B0"&&matches!(&transform,Err(Error::Unsupported(_))){
        key=b"A2B0";transform=prepare(key);
    }
    match transform{
        Ok(t)=>{
            let path=match &t{
                IccTransform::Gray(_)=>ProfilePath::GrayTrc,
                IccTransform::Rgb(_)=>ProfilePath::RgbMatrix,
                IccTransform::Cmyk(_)=>ProfilePath::CmykLut{tag:*key},
                IccTransform::Mab(_)=>ProfilePath::CmykMab{tag:*key},
                _=>return Err(Error::Invalid("ICC unexpected legacy transform")),
            };
            let t=if intent==RenderingIntent::AbsoluteColorimetric&&n==4{
                match p.absolute_output_scale(){
                    Ok(scale)=>IccTransform::AbsoluteOutput{inner:Box::new(t),scale},
                    Err(Error::Limit(s))=>return Err(Error::Limit(s)),
                    Err(Error::Invalid(s)|Error::Unsupported(s))=>return Ok((n,None,charged,ProfilePath::Alternate{reason:s})),
                }
            }else{t};
            Ok((n,Some(t),charged,path))
        }
        Err(Error::Limit(s))=>Err(Error::Limit(s)),
        Err(Error::Invalid(s)|Error::Unsupported(s))=>Ok((n,None,charged,ProfilePath::Alternate{reason:s})),
    }
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub(crate) enum CalKind{
    Gray{gamma:f64},
    Rgb{gamma:[f64;3],matrix:[f64;9]},
    Lab{range:[f64;4]},
}
#[derive(Clone,Copy,Debug,PartialEq)]
pub(crate) struct Calibrated{
    white:[f64;3],adapt:[f64;3],kind:CalKind,
}
fn num(o:Object)->Result<f64>{
    if let Object::Number(v)=o{if v.is_finite()&&v.abs()<=1e6{return Ok(v);}}
    Err(Error::Invalid("calibrated color number"))
}
fn array<const N:usize>(doc:&Document<'_>,o:&Object)->Result<[f64;N]>{
    let Object::Array(v)=doc.dereference(o)? else{return Err(Error::Invalid("calibrated color array"));};
    if v.len()!=N{return Err(Error::Invalid("calibrated color array length"));}
    let mut out=[0.;N];
    for (i,x) in v.iter().enumerate(){out[i]=num(doc.dereference(x)?)?;}
    Ok(out)
}
impl Calibrated{
    pub(crate) fn parse(doc:&Document<'_>,a:&[Object])->Result<Self>{
        if a.len()!=2{return Err(Error::Invalid("calibrated color space array"));}
        let Object::Name(name)=&a[0] else{return Err(Error::Invalid("calibrated color space name"));};
        let d=crate::dict(doc.dereference(&a[1])?)?;
        let white=array::<3>(doc,d.get(b"WhitePoint".as_slice()).ok_or(Error::Invalid("calibrated WhitePoint"))?)?;
        if white[0]<=0.||white[1]!=1.||white[2]<=0.{return Err(Error::Invalid("calibrated WhitePoint"));}
        if let Some(v)=d.get(b"BlackPoint".as_slice()){
            let b=array::<3>(doc,v)?;
            if b.iter().any(|x|*x<0.){return Err(Error::Invalid("calibrated BlackPoint"));}
            if b!=[0.;3]{return Err(Error::Unsupported("calibrated nonzero BlackPoint"));}
        }
        let src=mv(BRADFORD,white);let dst=mv(BRADFORD,D65);
        if src.iter().any(|x|*x<=1e-6){return Err(Error::Unsupported("calibrated white adaptation"));}
        let adapt=std::array::from_fn(|i|dst[i]/src[i]);
        let kind=match name.as_slice(){
            b"CalGray"=>{
                let gamma=d.get(b"Gamma".as_slice()).map(|v|doc.dereference(v).and_then(num)).transpose()?.unwrap_or(1.);
                if gamma<=0.{return Err(Error::Invalid("CalGray Gamma"));}
                CalKind::Gray{gamma}
            }
            b"CalRGB"=>{
                let gamma=d.get(b"Gamma".as_slice()).map(|v|array::<3>(doc,v)).transpose()?.unwrap_or([1.;3]);
                if gamma.iter().any(|x|*x<=0.){return Err(Error::Invalid("CalRGB Gamma"));}
                let matrix=d.get(b"Matrix".as_slice()).map(|v|array::<9>(doc,v)).transpose()?.unwrap_or([1.,0.,0.,0.,1.,0.,0.,0.,1.]);
                CalKind::Rgb{gamma,matrix}
            }
            b"Lab"=>{
                let range=d.get(b"Range".as_slice()).map(|v|array::<4>(doc,v)).transpose()?.unwrap_or([-100.,100.,-100.,100.]);
                if range[0]>range[1]||range[2]>range[3]{return Err(Error::Invalid("Lab Range"));}
                CalKind::Lab{range}
            }
            _=>return Err(Error::Unsupported("calibrated color space")),
        };
        Ok(Self{white,adapt,kind})
    }
    pub(crate) fn components(self)->usize{if matches!(self.kind,CalKind::Gray{..}){1}else{3}}
    pub(crate) fn rgb(self,v:[f64;3])->[f64;3]{
        let xyz=match self.kind{
            CalKind::Gray{gamma}=>self.white.map(|w|w*v[0].clamp(0.,1.).powf(gamma)),
            CalKind::Rgb{gamma,matrix:m}=>{
                let [a,b,c]=std::array::from_fn(|i|v[i].clamp(0.,1.).powf(gamma[i]));
                // PDF stores the matrix by columns, not rows.
                [m[0]*a+m[3]*b+m[6]*c,m[1]*a+m[4]*b+m[7]*c,m[2]*a+m[5]*b+m[8]*c]
            }
            CalKind::Lab{range:r}=>{
                let l=(v[0].clamp(0.,100.)+16.)/116.;
                let a=v[1].clamp(r[0],r[1]);let b=v[2].clamp(r[2],r[3]);
                let f=|t:f64|if t>=6./29.{t*t*t}else{(108./841.)*(t-4./29.)};
                [self.white[0]*f(l+a/500.),self.white[1]*f(l),self.white[2]*f(l-b/200.)]
            }
        };
        let cone=mv(BRADFORD,xyz);
        let adapted=mv(BRADFORD_INV,std::array::from_fn(|i|cone[i]*self.adapt[i]));
        mv(XYZ_RGB,adapted).map(srgb)
    }
}
