//! Bounded structural decoder for ICC.1:2022 section 10.16 (mpet).
//! Parsing is not execution or a conformance certificate. The PDF renderer
//! deliberately does not select MPE until evaluation and intent tests exist.
use super::{sig,u16be,u32be,Signature,MAX_PROFILE,MAX_CURVE};
use crate::{Error,Result};
use std::collections::BTreeMap;
const MAX_ELEMENTS:usize=64;
const MAX_CHANNELS:usize=16;
const MAX_SEGMENTS:usize=256;

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum ElementKind{
    Matrix,
    Clut{grid:Vec<u8>},
    CurveSet{segments:Vec<usize>},
    BeginExpansion{signature:Signature},
    EndExpansion{signature:Signature},
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Element{
    pub offset:usize,pub size:usize,pub inputs:usize,pub outputs:usize,pub kind:ElementKind,
}
#[derive(Debug)]
pub struct Mpe<'a>{
    data:&'a[u8],inputs:usize,outputs:usize,elements:Vec<Element>,
}
fn channels(n:usize)->Result<usize>{
    if n==0{return Err(Error::Invalid("ICC MPE zero channels"));}
    if n>MAX_CHANNELS{return Err(Error::Limit("ICC MPE channels"));}
    Ok(n)
}
fn header(b:&[u8])->Result<(Signature,usize,usize)>{
    if b.len()<12||b[4..8]!=[0;4]{return Err(Error::Invalid("ICC MPE header"));}
    Ok((sig(b,0)?,channels(u16be(b,8)? as usize)?,channels(u16be(b,10)? as usize)?))
}
/// Return ranges in logical order. Validate physical ranges once, allowing
/// only identical shared records and zero alignment padding between records.
fn positions(b:&[u8],base:usize,n:usize,min_size:usize)->Result<Vec<(usize,usize)>>{
    let end=base+n*8;
    if end>b.len(){return Err(Error::Invalid("ICC MPE directory"));}
    let mut out=Vec::with_capacity(n);
    for i in 0..n{
        let p=base+8*i;
        let off=u32be(b,p)? as usize;let len=u32be(b,p+4)? as usize;
        let stop=off.checked_add(len).ok_or(Error::Invalid("ICC MPE range overflow"))?;
        if off<end||off%4!=0||len<min_size||stop>b.len(){return Err(Error::Invalid("ICC MPE element bounds"));}
        out.push((off,stop));
    }
    let mut physical=out.clone();physical.sort_unstable();physical.dedup();
    let mut last=end;
    for (off,stop) in physical{
        if off<last{return Err(Error::Invalid("ICC MPE overlapping elements"));}
        if off-last>3||b[last..off].iter().any(|&v|v!=0){return Err(Error::Invalid("ICC MPE padding"));}
        last=stop;
    }
    if b.len()-last>3||b[last..].iter().any(|&v|v!=0){return Err(Error::Invalid("ICC MPE padding"));}
    Ok(out)
}
fn float(b:&[u8],p:usize)->Result<f32>{
    let v=f32::from_bits(u32be(b,p)?);
    if !v.is_finite(){return Err(Error::Invalid("ICC MPE nonfinite float"));}
    Ok(v)
}
fn floats(b:&[u8],start:usize,count:usize)->Result<()>{
    let end=start.checked_add(count.checked_mul(4).ok_or(Error::Limit("ICC MPE float count"))?).ok_or(Error::Limit("ICC MPE float count"))?;
    if end>b.len(){return Err(Error::Invalid("ICC MPE float bounds"));}
    for p in (start..end).step_by(4){float(b,p)?;}
    Ok(())
}
fn curve(b:&[u8])->Result<usize>{
    if b.len()<12||&b[..4]!=b"curf"||b[4..8]!=[0;4]||b[10..12]!=[0;2]{return Err(Error::Invalid("ICC MPE segmented curve header"));}
    let n=u16be(b,8)? as usize;
    if n==0{return Err(Error::Invalid("ICC MPE zero segments"));}
    if n>MAX_SEGMENTS{return Err(Error::Limit("ICC MPE curve segments"));}
    let mut pos=12+4*(n-1);
    if pos>b.len(){return Err(Error::Invalid("ICC MPE breakpoints"));}
    let mut prev=f32::NEG_INFINITY;
    for p in (12..pos).step_by(4){
        let x=float(b,p)?;
        if x<prev{return Err(Error::Invalid("ICC MPE breakpoint order"));}
        prev=x;
    }
    for i in 0..n{
        let tail=b.get(pos..).ok_or(Error::Invalid("ICC MPE segment bounds"))?;
        if tail.len()<12||tail[4..8]!=[0;4]{return Err(Error::Invalid("ICC MPE segment header"));}
        let count=match &sig(tail,0)?{
            b"parf"=>{
                if tail[10..12]!=[0;2]{return Err(Error::Invalid("ICC MPE formula reserved"));}
                match u16be(tail,8)?{0=>4,1|2=>5,_=>return Err(Error::Unsupported("ICC MPE formula type"))}
            }
            b"samf"=>{
                if i==0||i+1==n{return Err(Error::Invalid("ICC MPE sampled end segment"));}
                let m=u32be(tail,8)? as usize;
                if m==0{return Err(Error::Invalid("ICC MPE empty sampled segment"));}
                if m>MAX_CURVE{return Err(Error::Limit("ICC MPE sampled entries"));}
                m
            }
            _=>return Err(Error::Unsupported("ICC MPE segment type")),
        };
        floats(tail,12,count)?;
        pos+=12+4*count;
    }
    if pos!=b.len(){return Err(Error::Invalid("ICC MPE curve size"));}
    Ok(n)
}
fn element(b:&[u8],off:usize,stop:usize)->Result<Element>{
    let raw=&b[off..stop];
    let (kind,inputs,outputs)=header(raw)?;
    let kind=match &kind{
        b"matf"=>{
            let count=outputs*(inputs+1);
            if raw.len()!=12+4*count{return Err(Error::Invalid("ICC MPE matrix size"));}
            floats(raw,12,count)?;
            ElementKind::Matrix
        }
        b"clut"=>{
            let grid=raw.get(12..28).ok_or(Error::Invalid("ICC MPE CLUT header"))?;
            if grid[inputs..].iter().any(|&v|v!=0)||grid[..inputs].iter().any(|&v|v<2){return Err(Error::Invalid("ICC MPE CLUT grid"));}
            let mut count=outputs;
            for &g in &grid[..inputs]{
                count=count.checked_mul(g as usize).ok_or(Error::Limit("ICC MPE CLUT count"))?;
                if count>MAX_PROFILE/4{return Err(Error::Limit("ICC MPE CLUT count"));}
            }
            if raw.len()!=28+4*count{return Err(Error::Invalid("ICC MPE CLUT size"));}
            floats(raw,28,count)?;
            ElementKind::Clut{grid:grid[..inputs].to_vec()}
        }
        b"cvst"=>{
            if inputs!=outputs{return Err(Error::Invalid("ICC MPE curve channels"));}
            let ranges=positions(raw,12,inputs,12)?;
            let mut unique=BTreeMap::new();let mut segments=Vec::with_capacity(inputs);
            for (a,z) in ranges{
                let n=if let Some(&n)=unique.get(&(a,z)){n}else{
                    let n=curve(&raw[a..z])?;unique.insert((a,z),n);n
                };
                segments.push(n);
            }
            ElementKind::CurveSet{segments}
        }
        b"bACS"|b"eACS"=>{
            if inputs!=outputs||raw.len()!=16{return Err(Error::Invalid("ICC MPE expansion element"));}
            let signature=sig(raw,12)?;
            if kind==*b"bACS"{ElementKind::BeginExpansion{signature}}else{ElementKind::EndExpansion{signature}}
        }
        _=>return Err(Error::Unsupported("ICC MPE element type")),
    };
    Ok(Element{offset:off,size:stop-off,inputs,outputs,kind})
}
impl<'a> Mpe<'a>{
    pub fn parse(data:&'a[u8])->Result<Self>{
        if data.len()>MAX_PROFILE{return Err(Error::Limit("ICC MPE bytes"));}
        let (kind,inputs,outputs)=header(data)?;
        if kind!=*b"mpet"||data.len()<16{return Err(Error::Invalid("ICC MPE tag"));}
        let n=u32be(data,12)? as usize;
        if n==0{return Err(Error::Invalid("ICC MPE empty pipeline"));}
        if n>MAX_ELEMENTS{return Err(Error::Limit("ICC MPE elements"));}
        let ranges=positions(data,16,n,12)?;
        let mut unique:BTreeMap<(usize,usize),Element>=BTreeMap::new();
        let mut elements=Vec::with_capacity(n);let mut next=inputs;
        for (a,z) in ranges{
            let e=if let Some(e)=unique.get(&(a,z)){e.clone()}else{
                let e=element(data,a,z)?;unique.insert((a,z),e.clone());e
            };
            if e.inputs!=next{return Err(Error::Invalid("ICC MPE channel chain"));}
            next=e.outputs;elements.push(e);
        }
        if next!=outputs{return Err(Error::Invalid("ICC MPE output channels"));}
        Ok(Self{data,inputs,outputs,elements})
    }
    pub fn inputs(&self)->usize{self.inputs}
    pub fn outputs(&self)->usize{self.outputs}
    pub fn elements(&self)->&[Element]{&self.elements}
    /// Payload in logical processing order, borrowing the original tag.
    pub fn element_bytes(&self,index:usize)->Option<&'a[u8]>{
        let e=self.elements.get(index)?;
        self.data.get(e.offset..e.offset+e.size)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn head(sig:&[u8;4],p:u16,q:u16)->Vec<u8>{
        let mut b=sig.to_vec();b.extend_from_slice(&[0;4]);b.extend_from_slice(&p.to_be_bytes());b.extend_from_slice(&q.to_be_bytes());b
    }
    fn put(b:&mut[u8],p:usize,v:usize){b[p..p+4].copy_from_slice(&(v as u32).to_be_bytes());}
    fn append_floats(b:&mut Vec<u8>,v:&[f32]){for &x in v{b.extend_from_slice(&x.to_bits().to_be_bytes());}}
    fn matrix(p:u16,q:u16)->Vec<u8>{
        let mut b=head(b"matf",p,q);append_floats(&mut b,&vec![0.5;q as usize*(p as usize+1)]);b
    }
    fn clut()->Vec<u8>{
        let mut b=head(b"clut",2,3);b.extend_from_slice(&[2,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);append_floats(&mut b,&[1.25;18]);b
    }
    fn formula(b:&mut Vec<u8>,kind:u16){
        b.extend_from_slice(b"parf");b.extend_from_slice(&[0;4]);b.extend_from_slice(&kind.to_be_bytes());b.extend_from_slice(&[0;2]);
        append_floats(b,&[1.,1.,0.,0.]);if kind!=0{append_floats(b,&[0.]);}
    }
    fn curves()->Vec<u8>{
        let mut c=b"curf\0\0\0\0\0\x03\0\0".to_vec();append_floats(&mut c,&[0.,1.]);
        formula(&mut c,0);c.extend_from_slice(b"samf\0\0\0\0\0\0\0\x02");append_floats(&mut c,&[0.4,1.]);formula(&mut c,2);
        let mut b=head(b"cvst",2,2);b.resize(28,0);put(&mut b,12,28);put(&mut b,16,c.len());put(&mut b,20,28);put(&mut b,24,c.len());b.extend_from_slice(&c);b
    }
    fn pipeline(p:u16,q:u16,els:&[Vec<u8>])->Vec<u8>{
        let mut b=head(b"mpet",p,q);b.extend_from_slice(&(els.len()as u32).to_be_bytes());b.resize(16+8*els.len(),0);
        for (i,e) in els.iter().enumerate(){let off=b.len();put(&mut b,16+8*i,off);put(&mut b,20+8*i,e.len());b.extend_from_slice(e);while b.len()%4!=0{b.push(0);}}
        b
    }
    #[test]fn parses_mixed_pipeline_and_shared_curves(){
        let b=pipeline(4,3,&[matrix(4,2),curves(),clut()]);
        let p=Mpe::parse(&b).unwrap();assert_eq!((p.inputs(),p.outputs()),(4,3));
        assert_eq!(p.elements()[0].kind,ElementKind::Matrix);
        assert_eq!(p.elements()[1].kind,ElementKind::CurveSet{segments:vec![3,3]});
        assert_eq!(p.elements()[2].kind,ElementKind::Clut{grid:vec![2,3]});
        assert_eq!(p.element_bytes(2).unwrap(),clut());assert!(p.element_bytes(3).is_none());
    }
    #[test]fn logical_order_and_shared_elements(){
        let m=matrix(2,2);let mut b=pipeline(2,2,&[m.clone(),m.clone()]);
        let a=u32be(&b,16).unwrap()as usize;let z=u32be(&b,24).unwrap()as usize;
        put(&mut b,16,z);put(&mut b,24,a);
        assert_eq!(Mpe::parse(&b).unwrap().elements()[0].offset,z);
        put(&mut b,16,a);b.truncate(z);let p=Mpe::parse(&b).unwrap();assert_eq!(p.elements()[0],p.elements()[1]);
    }
    #[test]fn directory_channel_and_unknown_rejection(){
        let b=pipeline(4,3,&[matrix(4,2),clut()]);
        for (pos,val) in [(12,0),(12,65),(16,0),(16,33),(20,usize::MAX),(24,36)]{
            let mut bad=b.clone();put(&mut bad,pos,val);assert!(Mpe::parse(&bad).is_err());
        }
        assert!(Mpe::parse(&pipeline(4,3,&[matrix(3,2),clut()])).is_err());
        assert!(Mpe::parse(&pipeline(4,2,&[matrix(4,2),clut()])).is_err());
        let mut e=matrix(4,3);e[..4].copy_from_slice(b"zzzz");
        assert!(matches!(Mpe::parse(&pipeline(4,3,&[e])),Err(Error::Unsupported(_))));
        for i in 0..b.len(){assert!(Mpe::parse(&b[..i]).is_err());}
    }
    #[test]fn payload_validation(){
        let mut m=matrix(4,3);m[12..16].copy_from_slice(&f32::NAN.to_bits().to_be_bytes());
        assert!(Mpe::parse(&pipeline(4,3,&[m])).is_err());
        let mut c=clut();c[12]=1;assert!(Mpe::parse(&pipeline(2,3,&[c])).is_err());
        let mut c=clut();c[14]=2;assert!(Mpe::parse(&pipeline(2,3,&[c])).is_err());
        let mut c=curves();c[40..44].copy_from_slice(&2f32.to_bits().to_be_bytes());assert!(Mpe::parse(&pipeline(2,2,&[c])).is_err());
        let mut c=curves();c[48..52].copy_from_slice(b"samf");assert!(Mpe::parse(&pipeline(2,2,&[c])).is_err());
        let mut e=head(b"bACS",2,2);e.extend_from_slice(b"test");
        let mut f=head(b"eACS",2,2);f.extend_from_slice(b"test");
        let b=pipeline(2,2,&[e,f]);assert_eq!(Mpe::parse(&b).unwrap().elements().len(),2);
    }
}
