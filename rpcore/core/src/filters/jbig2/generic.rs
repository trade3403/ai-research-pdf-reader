//! Native T.88 generic-region header and causal arithmetic templates.
//! Context bits are assigned in reverse nominal reading order (LSB at
//! the nearest previously coded pixel); adaptive slots never move bits.
//! This stage does not reconstruct pixels or invoke QM.
use crate::{Error,Result,MAX_BYTES};
use super::bitmap::{self,Bitmap,RegionInfo};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Offset{pub x:i8,pub y:i8}
const fn at(x:i8,y:i8)->Offset{Offset{x,y}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Template{T0,T1,T2,T3}
impl Template{
    pub fn from_code(code:u8)->Result<Self>{
        match code{0=>Ok(Self::T0),1=>Ok(Self::T1),2=>Ok(Self::T2),3=>Ok(Self::T3),
            _=>Err(Error::Invalid("JBIG2 generic template"))}
    }
    pub fn bits(self)->usize{match self{Self::T0=>16,Self::T1=>13,Self::T2|Self::T3=>10}}
    pub fn context_count(self)->usize{1usize<<self.bits()}
    pub fn adaptive_count(self)->usize{if self==Self::T0{4}else{1}}
    /// T.88 figures 8–11, read in nominal reading order. Independent
    /// of actual adaptive coordinates, including overlapping slots.
    pub fn sltp_context(self)->usize{match self{Self::T0=>0x9b25,Self::T1=>0x0795,Self::T2=>0x00e5,Self::T3=>0x0195}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Neighborhood{template:Template,slots:[Offset;16]}
impl Neighborhood{
    pub fn new(template:Template,adaptive:&[Offset])->Result<Self>{
        if adaptive.len()!=template.adaptive_count(){return Err(Error::Invalid("JBIG2 adaptive pixel count"));}
        if adaptive.iter().any(|p|p.y>0||p.y==0&&p.x>=0){
            return Err(Error::Invalid("JBIG2 noncausal adaptive pixel"));
        }
        let a=adaptive[0];
        let mut slots=[at(0,0);16];
        // Figures 3–6: reverse reading order of nominal slots.
        match template{
            Template::T0=>slots.copy_from_slice(&[
                at(-1,0),at(-2,0),at(-3,0),at(-4,0),
                a,at(2,-1),at(1,-1),at(0,-1),at(-1,-1),at(-2,-1),adaptive[1],
                adaptive[2],at(1,-2),at(0,-2),at(-1,-2),adaptive[3]]),
            Template::T1=>slots[..13].copy_from_slice(&[
                at(-1,0),at(-2,0),at(-3,0),a,at(2,-1),at(1,-1),at(0,-1),at(-1,-1),at(-2,-1),
                at(2,-2),at(1,-2),at(0,-2),at(-1,-2)]),
            Template::T2=>slots[..10].copy_from_slice(&[
                at(-1,0),at(-2,0),a,at(1,-1),at(0,-1),at(-1,-1),at(-2,-1),
                at(1,-2),at(0,-2),at(-1,-2)]),
            Template::T3=>slots[..10].copy_from_slice(&[
                at(-1,0),at(-2,0),at(-3,0),at(-4,0),a,at(1,-1),at(0,-1),at(-1,-1),at(-2,-1),at(-3,-1)]),
        }
        Ok(Self{template,slots})
    }
    pub fn nominal(template:Template)->Self{
        let adaptive=[at(3,-1),at(-3,-1),at(2,-2),at(-2,-2)];
        match template{
            Template::T0=>Self::new(template,&adaptive),
            Template::T1=>Self::new(template,&adaptive[..1]),
            _=>Self::new(template,&[at(2,-1)]),
        }.expect("valid nominal JBIG2 template")
    }
    pub fn template(&self)->Template{self.template}
    pub fn slots(&self)->&[Offset]{&self.slots[..self.template.bits()]}
    /// The caller traverses in raster order; every slot is causal.
    pub fn context(&self,image:&Bitmap,x:u32,y:u32)->Result<usize>{
        if x>=image.width()||y>=image.height(){return Err(Error::Invalid("JBIG2 generic target coordinate"));}
        let mut cx=0usize;
        for (i,p) in self.slots().iter().enumerate(){
            if image.sample(i64::from(x)+i64::from(p.x),i64::from(y)+i64::from(p.y)){cx|=1usize<<i;}
        }
        Ok(cx)
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum Coding{Mmr,Arithmetic{neighborhood:Neighborhood,typical_prediction:bool}}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GenericHeader{pub region:RegionInfo,pub coding:Coding}
/// T.88 7.4.6 header syntax; entropy payload is opaque and returned
/// unchanged. Known-length segment bodies only. MMR+TPGD is outside this
/// scoped implementation. Extended/color template amendments unsupported.
pub fn parse(data:&[u8])->Result<(GenericHeader,&[u8])>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 generic segment bytes"));}
    let (region,rest)=bitmap::parse_region(data)?;
    let (&flags,rest)=rest.split_first().ok_or(Error::Invalid("JBIG2 generic flags"))?;
    if flags&0xf0!=0{return Err(Error::Unsupported("JBIG2 generic reserved flags"));}
    let template=Template::from_code((flags>>1)&3)?;
    let typical=flags&8!=0;
    if flags&1!=0{
        if template!=Template::T0||typical{return Err(Error::Unsupported("JBIG2 MMR flags"));}
        return Ok((GenericHeader{region,coding:Coding::Mmr},rest));
    }
    let count=template.adaptive_count();
    let bytes=rest.get(..count*2).ok_or(Error::Invalid("JBIG2 adaptive pixels truncated"))?;
    let mut adaptive=[at(0,0);4];
    for (i,p) in bytes.chunks_exact(2).enumerate(){adaptive[i]=at(p[0] as i8,p[1] as i8);}
    let neighborhood=Neighborhood::new(template,&adaptive[..count])?;
    Ok((GenericHeader{region,coding:Coding::Arithmetic{neighborhood,typical_prediction:typical}},&rest[count*2..]))
}
#[cfg(test)]
mod tests{
    use super::*;
    const T:[Template;4]=[Template::T0,Template::T1,Template::T2,Template::T3];
    fn body(flags:u8,at_bytes:&[u8])->Vec<u8>{
        let mut b=Vec::new();for v in [13u32,7,2,3]{b.extend(v.to_be_bytes());}
        b.push(0);b.push(flags);b.extend(at_bytes);b
    }
    #[test]fn generic_nominal_one_hot(){
        // Independent coordinate lists in high-to-low reading order
        // from T.88 figures 3–6.
        let expected:[&[(i8,i8)];4]=[
            &[(-2,-2),(-1,-2),(0,-2),(1,-2),(2,-2),(-3,-1),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(3,-1),(-4,0),(-3,0),(-2,0),(-1,0)],
            &[(-1,-2),(0,-2),(1,-2),(2,-2),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(3,-1),(-3,0),(-2,0),(-1,0)],
            &[(-1,-2),(0,-2),(1,-2),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(-2,0),(-1,0)],
            &[(-3,-1),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(-4,0),(-3,0),(-2,0),(-1,0)],
        ];
        for (t,positions) in T.into_iter().zip(expected){
            let n=Neighborhood::nominal(t);
            assert_eq!(t.context_count(),1<<positions.len());
            for (i,&(x,y)) in positions.iter().enumerate(){
                let mut b=Bitmap::new(16,6,false).unwrap();
                b.set((8+i32::from(x))as u32,(3+i32::from(y))as u32,true).unwrap();
                assert_eq!(n.context(&b,8,3).unwrap(),1<<(positions.len()-1-i));
            }
        }
    }
    #[test]fn generic_sltp_figure_vectors(){
        // Literal reading-order pixel patterns from Figures 8–11.
        let patterns=["1001101100100101","0011110010101","0011100101","0110010101"];
        for (t,p) in T.into_iter().zip(patterns){
            let n=Neighborhood::nominal(t);let mut b=Bitmap::new(16,6,false).unwrap();
            for (slot,v) in n.slots().iter().rev().zip(p.bytes()){
                b.set((8+i32::from(slot.x))as u32,(3+i32::from(slot.y))as u32,v==b'1').unwrap();
            }
            assert_eq!(n.context(&b,8,3).unwrap(),t.sltp_context());
            assert_eq!(usize::from_str_radix(p,2).unwrap(),t.sltp_context());
        }
    }
    #[test]fn generic_adaptive_slots_and_duplicates(){
        let n=Neighborhood::new(Template::T0,&[at(-1,0),at(-1,0),at(127,-1),at(-128,-128)]).unwrap();
        let mut b=Bitmap::new(140,4,false).unwrap();
        b.set(0,2,true).unwrap();b.set(128,1,true).unwrap();
        assert_eq!(n.context(&b,1,2).unwrap(),(1<<0)|(1<<4)|(1<<10)|(1<<11));
        assert_eq!(n.template().sltp_context(),0x9b25);
        for t in T{
            let n=Neighborhood::new(t,&vec![at(-1,0);t.adaptive_count()]).unwrap();
            let mut b=Bitmap::new(3,1,false).unwrap();b.set(0,0,true).unwrap();
            let want=match t{Template::T0=>0x8c11,Template::T1=>9,Template::T2=>5,Template::T3=>17};
            assert_eq!(n.context(&b,1,0).unwrap(),want);
        }
    }
    #[test]fn generic_causality_and_edges(){
        for t in T{
            for bad in [at(0,0),at(1,0),at(-1,1),at(0,127)]{
                assert!(Neighborhood::new(t,&vec![bad;t.adaptive_count()]).is_err());
            }
            assert!(Neighborhood::new(t,&[]).is_err());
            let b=Bitmap::new(3,3,true).unwrap();let n=Neighborhood::nominal(t);
            assert_eq!(n.context(&b,0,0).unwrap(),0);
            assert_eq!(n.context(&b,2,0).unwrap(),3);
            assert!(n.context(&b,3,0).is_err());assert!(n.context(&b,0,3).is_err());
        }
        assert!(Template::from_code(4).is_err());
    }
    #[test]fn generic_header_payload_offsets(){
        for (code,t) in T.into_iter().enumerate(){
            let a=if code==0{vec![3,255,253,255,2,254,254,254]}else{vec![if code==1{3}else{2},255]};
            let mut b=body((code as u8)*2|8,&a);
            for end in 0..b.len(){assert!(parse(&b[..end]).is_err());}
            b.extend([0x84,0xff,0xac]);
            let (h,p)=parse(&b).unwrap();
            assert_eq!((h.region.width,h.region.height,h.region.x,h.region.y),(13,7,2,3));
            assert_eq!(h.coding,Coding::Arithmetic{neighborhood:Neighborhood::nominal(t),typical_prediction:true});
            assert_eq!(p,[0x84,0xff,0xac]);
        }
        let b=body(1,&[0xaa,0xbb]);let (h,p)=parse(&b).unwrap();
        assert_eq!(h.coding,Coding::Mmr);assert_eq!(p,[0xaa,0xbb]);
    }
    #[test]fn generic_header_invalid_and_signed(){
        for f in [3,5,7,9,0x10,0x80,0xff]{assert!(parse(&body(f,&[255;8])).is_err());}
        assert!(parse(&body(2,&[0,0])).is_err());
        assert!(parse(&body(2,&[127,1])).is_err());
        let b=body(6,&[128,128]);let (h,_)=parse(&b).unwrap();
        match h.coding{Coding::Arithmetic{neighborhood,..}=>assert_eq!(neighborhood.slots()[4],at(-128,-128)),_=>panic!("expected arithmetic")}
    }
}
