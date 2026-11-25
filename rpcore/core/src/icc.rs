//! Bounded ICC v2/v4 binary profile inspection and RGB matrix/TRC subset.
//! ICC.1:2022 sections 7, 10 and Annex F. No runtime PDF integration yet.
use crate::{Error,Result};
use std::collections::BTreeMap;

const MAX_PROFILE:usize=16*1024*1024;
const MAX_TAGS:usize=1024;
const MAX_CURVE:usize=65536;
pub type Signature=[u8;4];

fn sig(b:&[u8],p:usize)->Result<Signature>{
    b.get(p..p+4).ok_or(Error::Invalid("ICC truncated field"))?
        .try_into().map_err(|_|Error::Invalid("ICC signature"))
}
fn u32be(b:&[u8],p:usize)->Result<u32>{Ok(u32::from_be_bytes(sig(b,p)?))}
fn u16be(b:&[u8],p:usize)->Result<u16>{
    Ok(u16::from_be_bytes(b.get(p..p+2).ok_or(Error::Invalid("ICC truncated short"))?.try_into().map_err(|_|Error::Invalid("ICC short"))?))
}
fn fixed(b:&[u8],p:usize)->Result<f64>{Ok(i32::from_be_bytes(sig(b,p)?) as f64/65536.)}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Tag{pub offset:usize,pub size:usize,pub kind:Signature}
/// Inspection of a borrowed profile, not proof of ICC conformance.
#[derive(Debug)]
pub struct Profile<'a>{
    data:&'a [u8],
    pub version_major:u8,pub class:Signature,pub color_space:Signature,pub pcs:Signature,
    pub intent:u32,pub illuminant:[f64;3],
    tags:BTreeMap<Signature,Tag>,
}
impl<'a> Profile<'a>{
    pub fn parse(bytes:&'a [u8])->Result<Self>{
        if bytes.len()>MAX_PROFILE{return Err(Error::Limit("ICC profile bytes"));}
        if bytes.len()<132{return Err(Error::Invalid("ICC profile header"));}
        let size=u32be(bytes,0)? as usize;
        if size!=bytes.len()||&bytes[36..40]!=b"acsp"{return Err(Error::Invalid("ICC size or magic"));}
        let version_major=bytes[8];
        if !matches!(version_major,2|4){return Err(Error::Unsupported("ICC version"));}
        let class=sig(bytes,12)?;let color_space=sig(bytes,16)?;let pcs=sig(bytes,20)?;
        let intent=u32be(bytes,64)?;
        if intent>3{return Err(Error::Invalid("ICC rendering intent"));}
        let illuminant=[fixed(bytes,68)?,fixed(bytes,72)?,fixed(bytes,76)?];
        let count=u32be(bytes,128)? as usize;
        if count>MAX_TAGS{return Err(Error::Limit("ICC tag count"));}
        let end=132+12*count;
        if end>size{return Err(Error::Invalid("ICC tag table"));}
        let mut tags=BTreeMap::new();let mut ranges=Vec::new();
        for i in 0..count{
            let p=132+12*i;let name=sig(bytes,p)?;
            let offset=u32be(bytes,p+4)? as usize;let len=u32be(bytes,p+8)? as usize;
            let stop=offset.checked_add(len).ok_or(Error::Invalid("ICC tag range"))?;
            if offset<end||offset%4!=0||len<8||stop>size{return Err(Error::Invalid("ICC tag bounds"));}
            let kind=sig(bytes,offset)?;
            if tags.insert(name,Tag{offset,size:len,kind}).is_some(){return Err(Error::Invalid("ICC duplicate tag"));}
            ranges.push((offset,stop));
        }
        ranges.sort_unstable();ranges.dedup();
        if ranges.windows(2).any(|w|w[0].1>w[1].0){return Err(Error::Invalid("ICC overlapping tags"));}
        Ok(Self{data:bytes,version_major,class,color_space,pcs,intent,illuminant,tags})
    }
    pub fn tags(&self)->&BTreeMap<Signature,Tag>{&self.tags}
    pub fn tag(&self,name:&Signature)->Option<&'a [u8]>{
        let t=self.tags.get(name)?;Some(&self.data[t.offset..t.offset+t.size])
    }
    fn xyz(&self,name:&Signature)->Result<[f64;3]>{
        let b=self.tag(name).ok_or(Error::Unsupported("ICC missing XYZ tag"))?;
        if b.len()!=20||sig(b,0)?!=*b"XYZ "||b[4..8]!=[0;4]{return Err(Error::Invalid("ICC XYZ tag"));}
        Ok([fixed(b,8)?,fixed(b,12)?,fixed(b,16)?])
    }
    /// Explicit relative-colorimetric matrix path to PCS XYZ D50.
    /// Refuse LUT-bearing profiles rather than silently override their LUT.
    pub fn rgb_matrix(&self)->Result<RgbMatrix>{
        if !matches!(&self.class,b"mntr"|b"scnr")||self.color_space!=*b"RGB "||self.pcs!=*b"XYZ "{
            return Err(Error::Unsupported("ICC RGB matrix profile class or space"));
        }
        if self.illuminant.iter().zip([0.9642,1.,0.8249]).any(|(a,b)|(*a-b).abs()>0.001){
            return Err(Error::Unsupported("ICC non-D50 PCS"));
        }
        if [*b"A2B0",*b"A2B1",*b"A2B2",*b"D2B0",*b"D2B1",*b"D2B2",*b"D2B3"].iter().any(|s|self.tags.contains_key(s)){
            return Err(Error::Unsupported("ICC LUT profile"));
        }
        let columns=[self.xyz(b"rXYZ")?,self.xyz(b"gXYZ")?,self.xyz(b"bXYZ")?];
        let curve=|name:&Signature|Curve::parse(self.tag(name).ok_or(Error::Unsupported("ICC missing TRC"))?);
        Ok(RgbMatrix{columns,curves:[curve(b"rTRC")?,curve(b"gTRC")?,curve(b"bTRC")?]})
    }
}
#[derive(Debug)]
enum Curve{Identity,Gamma(f64),Table(Vec<u16>)}
impl Curve{
    fn parse(b:&[u8])->Result<Self>{
        if b.len()<12||b[4..8]!=[0;4]{return Err(Error::Invalid("ICC curve header"));}
        match &sig(b,0)?{
            b"curv"=>{
                let n=u32be(b,8)? as usize;
                if n>MAX_CURVE{return Err(Error::Limit("ICC curve entries"));}
                if b.len()!=12+2*n{return Err(Error::Invalid("ICC curve size"));}
                match n{
                    0=>Ok(Self::Identity),
                    1=>{let g=f64::from(u16be(b,12)?)/256.;if g<=0.{return Err(Error::Invalid("ICC gamma"));}Ok(Self::Gamma(g))}
                    _=>{
                        let v=(0..n).map(|i|u16be(b,12+2*i)).collect::<Result<Vec<_>>>()?;
                        if v.windows(2).any(|w|w[0]>w[1]){return Err(Error::Unsupported("ICC nonmonotone TRC"));}
                        Ok(Self::Table(v))
                    }
                }
            }
            b"para"=>{
                if u16be(b,8)?!=0{return Err(Error::Unsupported("ICC parametric curve function"));}
                if b.len()!=16||u16be(b,10)?!=0{return Err(Error::Invalid("ICC parametric curve size"));}
                let g=fixed(b,12)?;
                if g<=0.{return Err(Error::Invalid("ICC gamma"));}
                Ok(Self::Gamma(g))
            }
            _=>Err(Error::Unsupported("ICC curve type")),
        }
    }
    fn eval(&self,x:f64)->f64{
        let x=x.clamp(0.,1.);
        match self{
            Self::Identity=>x,Self::Gamma(g)=>x.powf(*g),
            Self::Table(t)=>{
                let p=x*(t.len()-1)as f64;let i=p.floor()as usize;
                let f=p-i as f64;
                (f64::from(t[i])*(1.-f)+f64::from(t[(i+1).min(t.len()-1)])*f)/65535.
            }
        }
    }
}
#[derive(Debug)]
pub struct RgbMatrix{columns:[[f64;3];3],curves:[Curve;3]}
impl RgbMatrix{
    /// Return un-clipped PCS XYZ (D50), not display RGB.
    pub fn to_xyz_d50(&self,rgb:[f64;3])->Result<[f64;3]>{
        if rgb.iter().any(|v|!v.is_finite()){return Err(Error::Invalid("ICC RGB input"));}
        let v:[f64;3]=std::array::from_fn(|i|self.curves[i].eval(rgb[i]));
        Ok(std::array::from_fn(|j|(0..3).map(|i|self.columns[i][j]*v[i]).sum()))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn put(b:&mut [u8],p:usize,v:u32){b[p..p+4].copy_from_slice(&v.to_be_bytes());}
    fn xyz(v:[f64;3])->Vec<u8>{
        let mut b=b"XYZ \0\0\0\0".to_vec();
        for x in v{b.extend_from_slice(&((x*65536.).round()as i32).to_be_bytes());}b
    }
    fn profile()->Vec<u8>{
        // Original synthetic matrix with one shared gamma tag.
        let mut b=vec![0;132+6*12];b[8]=4;
        b[12..24].copy_from_slice(b"mntrRGB XYZ ");
        b[36..40].copy_from_slice(b"acsp");
        for (i,x) in [0.9642_f64,1.,0.8249].into_iter().enumerate(){put(&mut b,68+i*4,(x*65536.).round()as u32);}
        put(&mut b,128,6);
        let names=[*b"rXYZ",*b"gXYZ",*b"bXYZ",*b"rTRC",*b"gTRC",*b"bTRC"];
        for i in 0..3{
            let p=132+i*12;let off=b.len();let mut v=[0.;3];v[i]=1.;
            b[p..p+4].copy_from_slice(&names[i]);put(&mut b,p+4,off as u32);put(&mut b,p+8,20);
            b.extend(xyz(v));
        }
        let off=b.len();
        b.extend_from_slice(b"curv\0\0\0\0\0\0\0\x01\x02\0");
        for i in 3..6{
            let p=132+i*12;b[p..p+4].copy_from_slice(&names[i]);put(&mut b,p+4,off as u32);put(&mut b,p+8,14);
        }
        while b.len()%4!=0{b.push(0);}
        let size=b.len()as u32;put(&mut b,0,size);b
    }
    #[test]fn shared_tag_and_matrix_gamma(){
        let b=profile();let p=Profile::parse(&b).unwrap();
        assert_eq!(p.tags().len(),6);assert_eq!(p.tags()[b"rTRC"],p.tags()[b"gTRC"]);
        let t=p.rgb_matrix().unwrap();
        assert_eq!(t.to_xyz_d50([0.5,0.25,1.]).unwrap(),[0.25,0.0625,1.]);
        assert!(t.to_xyz_d50([f64::NAN,0.,0.]).is_err());
    }
    #[test]fn table_and_parametric_curves(){
        let t=Curve::parse(b"curv\0\0\0\0\0\0\0\x03\0\0\x80\0\xff\xff").unwrap();
        assert!((t.eval(0.25)-0.2500038).abs()<1e-6);
        assert_eq!(t.eval(1.),1.);
        let p=Curve::parse(b"para\0\0\0\0\0\0\0\0\0\x02\0\0").unwrap();
        assert_eq!(p.eval(0.5),0.25);
        assert!(Curve::parse(b"curv\0\0\0\0\0\0\0\x02\xff\xff\0\0").is_err());
    }
    #[test]fn rejects_corrupt_directory(){
        let b=profile();
        for n in 0..b.len(){assert!(Profile::parse(&b[..n]).is_err());}
        let mut x=b.clone();put(&mut x,128,1025);assert!(matches!(Profile::parse(&x),Err(Error::Limit(_))));
        let mut x=b.clone();x[144..148].copy_from_slice(b"rXYZ");assert!(Profile::parse(&x).is_err());
        let mut x=b.clone();let first=u32be(&x,136).unwrap();put(&mut x,148,first+4);assert!(Profile::parse(&x).is_err());
        let mut x=b.clone();put(&mut x,136,u32::MAX);assert!(Profile::parse(&x).is_err());
    }
    #[test]fn rejects_wrong_profile_path(){
        let mut b=profile();b[16..20].copy_from_slice(b"CMYK");
        assert!(Profile::parse(&b).unwrap().rgb_matrix().is_err());
        let mut b=profile();b[132..136].copy_from_slice(b"A2B0");
        assert!(Profile::parse(&b).unwrap().rgb_matrix().is_err());
    }
}
