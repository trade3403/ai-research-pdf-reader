//! Bounded TrueType sfnt/glyf reader. No bytecode execution or system font substitution.
use crate::{graphics::{Path,Segment},text::{Matrix,Point},Error,Result};
use std::collections::{BTreeMap,BTreeSet};
pub const MAX_FONT_BYTES:usize=16*1024*1024;
const MAX_POINTS:usize=100_000;
const MAX_COMPONENTS:usize=1024;
const MAX_DEPTH:usize=16;
#[derive(Clone,Copy,Debug)]
struct Node{p:Point,on:bool}
type Contour=Vec<Node>;
#[derive(Debug)]
pub struct TrueType{bytes:Vec<u8>,glyf:(usize,usize),loca:Vec<usize>,metrics:Vec<(u16,i16)>,units:u16}
fn u16at(d:&[u8],i:usize)->Result<u16>{let a=d.get(i..i.checked_add(2).ok_or(Error::Limit("font offset"))?).ok_or(Error::Invalid("truncated font"))?;Ok(u16::from_be_bytes([a[0],a[1]]))}
fn i16at(d:&[u8],i:usize)->Result<i16>{Ok(u16at(d,i)? as i16)}
fn u32at(d:&[u8],i:usize)->Result<u32>{let a=d.get(i..i.checked_add(4).ok_or(Error::Limit("font offset"))?).ok_or(Error::Invalid("truncated font"))?;Ok(u32::from_be_bytes([a[0],a[1],a[2],a[3]]))}
struct Reader<'a>{d:&'a[u8],p:usize}
impl Reader<'_>{
    fn byte(&mut self)->Result<u8>{let v=*self.d.get(self.p).ok_or(Error::Invalid("truncated glyph"))?;self.p+=1;Ok(v)}
    fn u16(&mut self)->Result<u16>{let v=u16at(self.d,self.p)?;self.p+=2;Ok(v)}
    fn i16(&mut self)->Result<i16>{Ok(self.u16()? as i16)}
    fn skip(&mut self,n:usize)->Result<()>{self.p=self.p.checked_add(n).ok_or(Error::Limit("glyph offset"))?;if self.p>self.d.len(){return Err(Error::Invalid("truncated glyph instructions"));}Ok(())}
    fn f2(&mut self)->Result<f64>{Ok(f64::from(self.i16()?)/16384.)}
}
#[derive(Default)]
struct Budget{points:usize,components:usize}
impl TrueType{
    pub fn parse(bytes:Vec<u8>)->Result<Self>{
        if bytes.len()>MAX_FONT_BYTES{return Err(Error::Limit("TrueType font bytes"));}
        if u32at(&bytes,0)?!=0x00010000{return Err(Error::Unsupported("not TrueType glyf sfnt"));}
        let n=usize::from(u16at(&bytes,4)?);
        if n==0||n>256{return Err(Error::Limit("font tables"));}
        let end=12+n*16;
        if end>bytes.len(){return Err(Error::Invalid("font directory"));}
        let mut tables=BTreeMap::<[u8;4],(usize,usize)>::new();
        for i in 0..n{
            let p=12+i*16;let tag=bytes[p..p+4].try_into().unwrap();
            let offset=u32at(&bytes,p+8)? as usize;let length=u32at(&bytes,p+12)? as usize;
            let last=offset.checked_add(length).ok_or(Error::Limit("font table range"))?;
            if offset<end||last>bytes.len()||tables.insert(tag,(offset,last)).is_some(){return Err(Error::Invalid("font table range"));}
        }
        let mut ranges:Vec<_>=tables.values().copied().filter(|(a,b)|a<b).collect();ranges.sort_unstable();
        if ranges.windows(2).any(|p|p[0].1>p[1].0){return Err(Error::Invalid("overlapping font tables"));}
        let table=|tag:&[u8;4]|->Result<&[u8]>{let &(a,b)=tables.get(tag).ok_or(Error::Invalid("missing font table"))?;Ok(&bytes[a..b])};
        let head=table(b"head")?;let maxp=table(b"maxp")?;let hhea=table(b"hhea")?;
        if head.len()<54||u32at(head,12)?!=0x5f0f3cf5||u32at(maxp,0)?!=0x00010000||hhea.len()<36{return Err(Error::Invalid("TrueType header"));}
        let units=u16at(head,18)?;if !(16..=16384).contains(&units){return Err(Error::Invalid("unitsPerEm"));}
        let glyphs=usize::from(u16at(maxp,4)?);if glyphs==0{return Err(Error::Invalid("font glyph count"));}
        let raw=table(b"loca")?;let format=i16at(head,50)?;
        let glyf=*tables.get(b"glyf").ok_or(Error::Invalid("glyf table"))?;
        let mut loca=Vec::with_capacity(glyphs+1);
        for i in 0..=glyphs{
            let off=match format{0=>usize::from(u16at(raw,i*2)?)*2,1=>u32at(raw,i*4)? as usize,_=>return Err(Error::Invalid("loca format"))};
            if off>glyf.1-glyf.0||loca.last().is_some_and(|&prev|prev>off){return Err(Error::Invalid("loca ordering"));}
            loca.push(off);
        }
        let nmetrics=usize::from(u16at(hhea,34)?);if nmetrics==0||nmetrics>glyphs{return Err(Error::Invalid("hhea metrics count"));}
        let hmtx=table(b"hmtx")?;let mut metrics=Vec::with_capacity(glyphs);
        for i in 0..glyphs{
            let (advance,lsb)=if i<nmetrics{(u16at(hmtx,i*4)?,i16at(hmtx,i*4+2)?)}else{(u16at(hmtx,(nmetrics-1)*4)?,i16at(hmtx,nmetrics*4+(i-nmetrics)*2)?)};
            metrics.push((advance,lsb));
        }
        Ok(Self{bytes,glyf,loca,metrics,units})
    }
    pub fn units_per_em(&self)->u16{self.units}
    pub fn glyph_count(&self)->usize{self.metrics.len()}
    pub fn horizontal_metrics(&self,gid:u16)->Result<(u16,i16)>{self.metrics.get(gid as usize).copied().ok_or(Error::Invalid("glyph ID"))}
    pub fn outline(&self,gid:u16)->Result<Path>{
        let contours=self.contours(gid,&mut BTreeSet::new(),&mut Budget::default())?;
        let mut out=Vec::new();
        for c in contours{emit(&c,&mut out)?;if out.len()>MAX_POINTS*2{return Err(Error::Limit("glyph path"));}}
        Ok(out)
    }
    fn contours(&self,gid:u16,active:&mut BTreeSet<u16>,budget:&mut Budget)->Result<Vec<Contour>>{
        if active.len()>=MAX_DEPTH||budget.components>=MAX_COMPONENTS{return Err(Error::Limit("compound glyph depth/components"));}
        if !active.insert(gid){return Err(Error::Invalid("compound glyph cycle"));}
        budget.components+=1;
        let result=self.read_glyph(gid,active,budget);active.remove(&gid);result
    }
    fn read_glyph(&self,gid:u16,active:&mut BTreeSet<u16>,budget:&mut Budget)->Result<Vec<Contour>>{
        let i=usize::from(gid);let &a=self.loca.get(i).ok_or(Error::Invalid("glyph ID"))?;
        let &b=self.loca.get(i+1).ok_or(Error::Invalid("glyph ID"))?;
        if a==b{return Ok(Vec::new());}
        let data=&self.bytes[self.glyf.0+a..self.glyf.0+b];
        let n=i16at(data,0)?;if data.len()<10{return Err(Error::Invalid("glyph header"));}
        let mut r=Reader{d:data,p:10};
        if n>=0{return simple(n as usize,&mut r,budget);}
        if n != -1{return Err(Error::Invalid("compound contour count"));}
        let mut out=Vec::new();
        loop{
            let f=r.u16()?;let g=r.u16()?;
            if f&2==0{return Err(Error::Unsupported("compound point attachment"));}
            let (x,y)=if f&1!=0{(f64::from(r.i16()?),f64::from(r.i16()?))}else{(f64::from(r.byte()? as i8),f64::from(r.byte()? as i8))};
            let mut m=Matrix::IDENTITY;
            if (f&8!=0) as u8+(f&64!=0) as u8+(f&128!=0) as u8>1{return Err(Error::Invalid("compound transform flags"));}
            if f&8!=0{let k=r.f2()?;m.0[0]=k;m.0[3]=k;}
            if f&64!=0{m.0[0]=r.f2()?;m.0[3]=r.f2()?;}
            if f&128!=0{m.0[0]=r.f2()?;m.0[1]=r.f2()?;m.0[2]=r.f2()?;m.0[3]=r.f2()?;}
            if f&0x1800==0x1800{return Err(Error::Invalid("compound offset flags"));}
            let mut offset=if f&0x0800!=0{m.point([x,y])?}else{[x,y]};
            if f&4!=0{offset=offset.map(f64::round);}
            m.0[4]=offset[0];m.0[5]=offset[1];
            let mut child=self.contours(g,active,budget)?;
            for c in &mut child{for node in c{node.p=m.point(node.p)?;}}
            out.extend(child);
            if f&32==0{if f&256!=0{let len=usize::from(r.u16()?);r.skip(len)?;}break;}
            if f&256!=0{return Err(Error::Invalid("compound instructions flag"));}
        }
        Ok(out)
    }
}
fn simple(n:usize,r:&mut Reader<'_>,budget:&mut Budget)->Result<Vec<Contour>>{
    if n==0{return Ok(Vec::new());}
    let mut ends=Vec::with_capacity(n);
    for _ in 0..n{let e=usize::from(r.u16()?);if ends.last().is_some_and(|&prev|prev>=e){return Err(Error::Invalid("glyph contour endpoints"));}ends.push(e);}
    let count=ends[n-1]+1;budget.points=budget.points.checked_add(count).ok_or(Error::Limit("glyph points"))?;
    if budget.points>MAX_POINTS{return Err(Error::Limit("glyph points"));}
    let instr=usize::from(r.u16()?);r.skip(instr)?;
    let mut flags=Vec::with_capacity(count);
    while flags.len()<count{
        let f=r.byte()?;let repeat=if f&8!=0{usize::from(r.byte()?)+1}else{1};
        if repeat>count-flags.len(){return Err(Error::Invalid("glyph flag repeat"));}
        flags.resize(flags.len()+repeat,f);
    }
    fn axis(r:&mut Reader<'_>,flags:&[u8],short:u8,same:u8)->Result<Vec<f64>>{
        let mut values=Vec::with_capacity(flags.len());let mut coord=0i32;
        for &f in flags{
            let d=if f&short!=0{let v=i32::from(r.byte()?);if f&same!=0{v}else{-v}}else if f&same!=0{0}else{i32::from(r.i16()?)};
            coord=coord.checked_add(d).ok_or(Error::Limit("glyph coordinates"))?;
            if !(-32768..=32767).contains(&coord){return Err(Error::Invalid("glyph coordinate range"));}
            values.push(f64::from(coord));
        }Ok(values)
    }
    let xs=axis(r,&flags,2,16)?;let ys=axis(r,&flags,4,32)?;
    let mut out=Vec::with_capacity(n);let mut start=0;
    for end in ends{let mut c=Vec::with_capacity(end-start+1);for i in start..=end{c.push(Node{p:[xs[i],ys[i]],on:flags[i]&1!=0});}out.push(c);start=end+1;}
    Ok(out)
}
fn mid(a:Point,b:Point)->Point{[(a[0]+b[0])*0.5,(a[1]+b[1])*0.5]}
fn quad(a:Point,b:Point,c:Point)->Segment{
    let x=[a[0]+(b[0]-a[0])*2./3.,a[1]+(b[1]-a[1])*2./3.];
    let y=[c[0]+(b[0]-c[0])*2./3.,c[1]+(b[1]-c[1])*2./3.];Segment::Cubic(x,y,c)
}
fn emit(c:&[Node],out:&mut Path)->Result<()>{
    let first=*c.first().ok_or(Error::Invalid("empty contour"))?;let last=*c.last().unwrap();
    let (start,begin,end)=if first.on{(first.p,1,c.len())}else if last.on{(last.p,0,c.len()-1)}else{(mid(last.p,first.p),0,c.len())};
    out.push(Segment::Move(start));let mut at=start;let mut control=None;
    for node in &c[begin..end]{
        if node.on{
            if let Some(p)=control.take(){out.push(quad(at,p,node.p));}else{out.push(Segment::Line(node.p));}at=node.p;
        }else if let Some(p)=control.replace(node.p){let m=mid(p,node.p);out.push(quad(at,p,m));at=m;}
    }
    if let Some(p)=control{out.push(quad(at,p,start));}
    out.push(Segment::Close);Ok(())
}
