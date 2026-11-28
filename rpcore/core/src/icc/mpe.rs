//! Bounded structural decoder for ICC.1:2022 section 10.16 (mpet).
//! Structural validation and separately prepared native floating-point execution.
//! The PDF renderer deliberately does not select MPE until intent tests exist.
use super::{sig,u16be,u32be,Signature,MAX_PROFILE,MAX_CURVE};
use crate::{Error,Result};
use std::{borrow::Cow,collections::BTreeMap};
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


/// Prepared, borrowed pipeline. Shared element and curve data are compiled
/// once; large CLUT/sample arrays remain in the original validated tag.
#[derive(Debug)]
pub struct Program<'a>{
    inputs:usize,outputs:usize,stages:Vec<std::sync::Arc<Stage<'a>>>,work:usize,
}
#[derive(Debug)]
enum Stage<'a>{
    Matrix{inputs:usize,outputs:usize,coeff:Vec<f64>,offset:Vec<f64>},
    Clut{grid:Vec<u8>,strides:Vec<usize>,outputs:usize,data:Cow<'a,[u8]>},
    Curves(Vec<std::sync::Arc<Segmented<'a>>>),
    Identity,
}
#[derive(Debug)]
struct Segmented<'a>{breaks:Vec<f64>,segments:Vec<Segment<'a>>}
#[derive(Debug)]
enum Segment<'a>{
    Formula{kind:u16,p:[f64;5]},
    Sampled{lo:f64,hi:f64,first:f64,data:Cow<'a,[u8]>},
}
fn checked_float(v:f64)->Result<f64>{
    if !v.is_finite()||v.abs()>f32::MAX as f64{return Err(Error::Invalid("ICC MPE evaluation range"));}
    Ok(v)
}
impl Segment<'_>{
    fn eval(&self,x:f64)->Result<f64>{
        let v=match self{
            Self::Formula{kind,p}=>{
                let [a,b,c,d,e]=*p;
                match kind{
                    0=>(b*x+c).powf(a)+d,
                    1=>b*(c*x.powf(a)+d).log10()+e,
                    2=>a*b.powf(c*x+d)+e,
                    _=>return Err(Error::Unsupported("ICC MPE formula type")),
                }
            }
            Self::Sampled{lo,hi,first,data}=>{
                let n=data.len()/4;
                let pos=((x-lo)/(hi-lo)).clamp(0.,1.)*n as f64;
                let i=(pos.floor()as usize).min(n-1);
                let f=pos-i as f64;
                let a=if i==0{*first}else{float(data,4*(i-1))? as f64};
                let b=float(data,4*i)? as f64;
                a*(1.-f)+b*f
            }
        };
        checked_float(v)
    }
}
impl<'a> Segmented<'a>{
    fn prepare(raw:&'a[u8])->Result<Self>{
        // Called only on payloads already validated by Mpe::parse.
        let n=u16be(raw,8)? as usize;
        let breaks=(0..n-1).map(|i|float(raw,12+4*i).map(f64::from)).collect::<Result<Vec<_>>>()?;
        let mut pos=12+4*(n-1);
        let mut segments:Vec<Segment<'a>>=Vec::with_capacity(n);
        for i in 0..n{
            let tail=&raw[pos..];
            let seg=if &tail[..4]==b"parf"{
                let kind=u16be(tail,8)?;
                let count=if kind==0{4}else{5};
                let mut p=[0.;5];for j in 0..count{p[j]=float(tail,12+4*j)? as f64;}
                pos+=12+4*count;
                Segment::Formula{kind,p}
            }else{
                let count=u32be(tail,8)? as usize;
                let (lo,hi)=(breaks[i-1],breaks[i]);
                if hi<=lo{return Err(Error::Unsupported("ICC MPE zero-width sampled segment"));}
                let first=segments[i-1].eval(lo)?;
                let data=&tail[12..12+4*count];
                pos+=12+4*count;
                Segment::Sampled{lo,hi,first,data:Cow::Borrowed(data)}
            };
            segments.push(seg);
        }
        Ok(Self{breaks,segments})
    }
    fn eval(&self,x:f64)->Result<f64>{
        // The earlier segment owns an exact breakpoint.
        self.segments[self.breaks.partition_point(|&b|b<x)].eval(x)
    }
}
impl<'a> Mpe<'a>{
    pub fn prepare(&self)->Result<Program<'a>>{
        use std::sync::Arc;
        let mut unique=BTreeMap::<(usize,usize),Arc<Stage<'a>>>::new();
        let mut stages=Vec::with_capacity(self.elements.len());let mut work=0usize;
        for (i,e) in self.elements.iter().enumerate(){
            let key=(e.offset,e.size);
            // Work counts logical executions, even when payloads are shared.
            let cost=match &e.kind{
                ElementKind::Clut{..}=>(1usize<<e.inputs)*e.outputs*(e.inputs+1),
                ElementKind::Matrix=>e.inputs*e.outputs,
                ElementKind::CurveSet{..}=>e.inputs*32,
                _=>e.inputs,
            };
            work=work.checked_add(cost).ok_or(Error::Limit("ICC MPE evaluation work"))?;
            if work>1_000_000{return Err(Error::Limit("ICC MPE evaluation work"));}
            if let Some(stage)=unique.get(&key){stages.push(Arc::clone(stage));continue;}
            let raw=self.element_bytes(i).ok_or(Error::Invalid("ICC MPE element bounds"))?;
            let stage=match &e.kind{
                ElementKind::Matrix=>{
                    let n=e.inputs*e.outputs;
                    let coeff=(0..n).map(|j|float(raw,12+4*j).map(f64::from)).collect::<Result<Vec<_>>>()?;
                    let offset=(0..e.outputs).map(|j|float(raw,12+4*(n+j)).map(f64::from)).collect::<Result<Vec<_>>>()?;
                    Stage::Matrix{inputs:e.inputs,outputs:e.outputs,coeff,offset}
                }
                ElementKind::Clut{grid}=>{
                    let mut strides=vec![1;grid.len()];let mut s=1;
                    for j in (0..grid.len()).rev(){strides[j]=s;s*=grid[j] as usize;}
                    Stage::Clut{grid:grid.clone(),strides,outputs:e.outputs,data:Cow::Borrowed(&raw[28..])}
                }
                ElementKind::CurveSet{..}=>{
                    let mut seen=BTreeMap::<(usize,usize),Arc<Segmented<'a>>>::new();
                    let mut curves=Vec::with_capacity(e.inputs);
                    for (a,z) in positions(raw,12,e.inputs,12)?{
                        let c=if let Some(c)=seen.get(&(a,z)){Arc::clone(c)}else{
                            let c=Arc::new(Segmented::prepare(&raw[a..z])?);
                            seen.insert((a,z),Arc::clone(&c));c
                        };
                        curves.push(c);
                    }
                    Stage::Curves(curves)
                }
                ElementKind::BeginExpansion{..}|ElementKind::EndExpansion{..}=>Stage::Identity,
            };
            let stage=Arc::new(stage);unique.insert(key,Arc::clone(&stage));stages.push(stage);
        }
        Ok(Program{inputs:self.inputs,outputs:self.outputs,stages,work})
    }
}
impl Program<'_>{
    pub fn work(&self)->usize{self.work}
    /// Own all borrowed payloads once per shared stage and curve. No
    /// self-referential structure, leaked allocation or unsafe code.
    pub fn into_owned(self)->Program<'static>{
        use std::sync::Arc;
        let mut stages=Vec::with_capacity(self.stages.len());
        let mut shared:BTreeMap<_,Arc<Stage<'static>>>=BTreeMap::new();
        for stage in self.stages{
            let key=Arc::as_ptr(&stage);
            if let Some(s)=shared.get(&key){stages.push(Arc::clone(s));continue;}
            let owned=match stage.as_ref(){
                Stage::Matrix{inputs,outputs,coeff,offset}=>Stage::Matrix{inputs:*inputs,outputs:*outputs,coeff:coeff.clone(),offset:offset.clone()},
                Stage::Clut{grid,strides,outputs,data}=>Stage::Clut{grid:grid.clone(),strides:strides.clone(),outputs:*outputs,data:Cow::Owned(data.to_vec())},
                Stage::Curves(curves)=>{
                    let mut cache:BTreeMap<_,Arc<Segmented<'static>>>=BTreeMap::new();
                    let mut out=Vec::with_capacity(curves.len());
                    for curve in curves{
                        let k=Arc::as_ptr(curve);
                        let c=if let Some(c)=cache.get(&k){Arc::clone(c)}else{
                            let segments=curve.segments.iter().map(|s|match s{
                                Segment::Formula{kind,p}=>Segment::Formula{kind:*kind,p:*p},
                                Segment::Sampled{lo,hi,first,data}=>Segment::Sampled{lo:*lo,hi:*hi,first:*first,data:Cow::Owned(data.to_vec())},
                            }).collect();
                            let c=Arc::new(Segmented{breaks:curve.breaks.clone(),segments});
                            cache.insert(k,Arc::clone(&c));c
                        };
                        out.push(c);
                    }
                    Stage::Curves(out)
                }
                Stage::Identity=>Stage::Identity,
            };
            let owned=Arc::new(owned);
            shared.insert(key,Arc::clone(&owned));stages.push(owned);
        }
        Program{inputs:self.inputs,outputs:self.outputs,stages,work:self.work}
    }

    /// Raw MPE channel values, NOT normalized Lab, XYZ decoding or display RGB.
    /// Only CLUT inputs clip. Float32 payloads, f64 arithmetic, checked
    /// float32-representable stage outputs; no per-sample heap allocation.
    pub fn evaluate_into(&self,input:&[f64],output:&mut[f64])->Result<()>{
        if input.len()!=self.inputs||output.len()!=self.outputs{return Err(Error::Invalid("ICC MPE evaluation channels"));}
        let mut a=[0.;MAX_CHANNELS];let mut b=[0.;MAX_CHANNELS];let mut n=self.inputs;
        for (dst,&v) in a.iter_mut().zip(input){*dst=checked_float(v)?;}
        for stage in &self.stages{
            let m=match stage.as_ref(){
                Stage::Matrix{inputs,outputs,coeff,offset}=>{
                    for j in 0..*outputs{
                        b[j]=checked_float(offset[j]+(0..*inputs).map(|i|coeff[j*inputs+i]*a[i]).sum::<f64>())?;
                    }
                    *outputs
                }
                Stage::Curves(curves)=>{
                    for (i,c) in curves.iter().enumerate(){b[i]=c.eval(a[i])?;}curves.len()
                }
                Stage::Clut{grid,strides,outputs,data}=>{
                    let mut f=[0.;MAX_CHANNELS];let mut base=0;
                    for i in 0..grid.len(){
                        let p=a[i].clamp(0.,1.)*f64::from(grid[i]-1);
                        let low=(p.floor()as usize).min(grid[i]as usize-2);
                        base+=low*strides[i];f[i]=p-low as f64;
                    }
                    b[..*outputs].fill(0.);
                    for c in 0..(1usize<<grid.len()){
                        let mut w=1.;let mut index=base;
                        for i in 0..grid.len(){
                            if c&(1<<i)!=0{index+=strides[i];w*=f[i];}else{w*=1.-f[i];}
                            if w==0.{break;}
                        }
                        if w!=0.{for j in 0..*outputs{b[j]+=w*f64::from(float(data,4*(index*outputs+j))?);}}
                    }
                    for v in &mut b[..*outputs]{*v=checked_float(*v)?;}*outputs
                }
                Stage::Identity=>{b[..n].copy_from_slice(&a[..n]);n}
            };
            std::mem::swap(&mut a,&mut b);n=m;
        }
        output.copy_from_slice(&a[..n]);Ok(())
    }
    pub fn evaluate(&self,input:&[f64])->Result<Vec<f64>>{
        let mut out=vec![0.;self.outputs];self.evaluate_into(input,&mut out)?;Ok(out)
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
    #[test]fn execution_matrix_clut_and_unclipped_output(){
        let mut m=head(b"matf",2,2);append_floats(&mut m,&[2.,0.,0.,1.,-0.5,0.]);
        let mut c=head(b"clut",2,3);c.extend_from_slice(&[2,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
        for i in 0..2{for j in 0..3{append_floats(&mut c,&[i as f32,j as f32/2.,-1.+i as f32+j as f32]);}}
        let raw=pipeline(2,3,&[m,c]);let parsed=Mpe::parse(&raw).unwrap();let p=parsed.prepare().unwrap();
        assert_eq!(p.evaluate(&[0.5,0.25]).unwrap(),vec![0.5,0.25,0.]);
        assert_eq!(p.evaluate(&[2.,2.]).unwrap(),vec![1.,1.,2.]);
        assert!(p.evaluate(&[f64::NAN,0.]).is_err());
        assert!(p.evaluate(&[0.]).is_err());
        assert!(p.evaluate_into(&[0.,0.],&mut [0.;2]).is_err());
    }
    #[test]fn segmented_execution_implicit_first_point_and_formulas(){
        let b=curves();let raw=pipeline(2,2,&[b]);let m=Mpe::parse(&raw).unwrap();let p=m.prepare().unwrap();
        let y=p.evaluate(&[-0.5,0.25]).unwrap();assert_eq!(y[0],-0.5);assert!((y[1]-0.2).abs()<1e-7);
        assert_eq!(p.evaluate(&[0.,1.]).unwrap(),vec![0.,1.]);
        let log=Segment::Formula{kind:1,p:[1.,2.,1.,1.,0.5]};
        assert!((log.eval(9.).unwrap()-2.5).abs()<1e-12);assert!(log.eval(-2.).is_err());
        let exp=Segment::Formula{kind:2,p:[2.,3.,1.,0.,-1.]};
        assert_eq!(exp.eval(2.).unwrap(),17.);
        let bad=Segment::Formula{kind:0,p:[0.5,1.,0.,0.,0.]};
        assert!(bad.eval(-1.).is_err());
        assert!(exp.eval(1000.).is_err());
    }
    #[test]fn owned_program_outlives_profile_and_matches_borrowed(){
        let (owned,expected)={
            let raw=pipeline(4,3,&[matrix(4,2),curves(),clut()]);
            let parsed=Mpe::parse(&raw).unwrap();
            let program=parsed.prepare().unwrap();
            let expected=program.evaluate(&[0.1,0.2,0.3,0.4]).unwrap();
            (program.into_owned(),expected)
        };
        assert_eq!(owned.evaluate(&[0.1,0.2,0.3,0.4]).unwrap(),expected);
        assert!(owned.work()>0);
    }
    #[test]fn shared_matrix_and_expansion_pass_through(){
        let mut m=head(b"matf",1,1);append_floats(&mut m,&[2.,1.]);
        let mut e=head(b"bACS",1,1);e.extend_from_slice(b"test");
        let raw=pipeline(1,1,&[m.clone(),e,m]);let p=Mpe::parse(&raw).unwrap().prepare().unwrap();
        assert_eq!(p.evaluate(&[2.]).unwrap(),vec![11.]);
    }
}
