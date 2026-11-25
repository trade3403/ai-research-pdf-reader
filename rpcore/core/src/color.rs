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

// Native calibrated color to an sRGB display target. Bradford adaptation
// and the standard sRGB transfer function; see W3C CSS Color 4 conversion
// and ISO 32000 CIE-based color spaces. No ICC profile execution.
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
