//! Native T.88 6.7 and 7.4.4 pattern dictionary (type 16).
//! Decode a collective bitmap, then split left-to-right into equal patterns.
//! Known-length payloads only; no halftone grid, segment graph or page dispatch.
use crate::{Error,Result,MAX_BYTES};
use super::{bitmap::{self,Bitmap},generic::{Neighborhood,Template},qm};

pub const MAX_PATTERNS:u32=65_536;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Coding{Mmr,Arithmetic(Template)}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Header{
    pub width:u8,pub height:u8,pub gray_max:u32,pub count:u32,
    pub collective_width:u32,pub coding:Coding,
}
pub fn parse(data:&[u8])->Result<(Header,&[u8])>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 pattern segment bytes"));}
    let h=data.get(..7).ok_or(Error::Invalid("JBIG2 pattern header"))?;
    if h[0]&0xf8!=0{return Err(Error::Unsupported("JBIG2 pattern flags"));}
    let t=Template::from_code((h[0]>>1)&3)?;
    let coding=if h[0]&1!=0{
        if t!=Template::T0{return Err(Error::Invalid("JBIG2 pattern MMR template"));}
        Coding::Mmr
    }else{Coding::Arithmetic(t)};
    let (width,height)=(h[1],h[2]);
    if width==0||height==0{return Err(Error::Invalid("JBIG2 pattern dimensions"));}
    let gray_max=u32::from_be_bytes([h[3],h[4],h[5],h[6]]);
    let count=gray_max.checked_add(1).ok_or(Error::Limit("JBIG2 pattern count"))?;
    if count>MAX_PATTERNS{return Err(Error::Limit("JBIG2 pattern count"));}
    let collective_width=count.checked_mul(u32::from(width)).ok_or(Error::Limit("JBIG2 pattern width"))?;
    if collective_width>bitmap::MAX_SIDE||u64::from(collective_width)*u64::from(height)>bitmap::MAX_PIXELS{
        return Err(Error::Limit("JBIG2 pattern collective dimensions"));
    }
    Ok((Header{width,height,gray_max,count,collective_width,coding},&data[7..]))
}
/// T.88 Table 27: A1 is (-HDPW, 0), not a signed 8-bit on-wire
/// coordinate. Preserve the generic template's bit positions and all
/// other nominal offsets. HDPW up to 255 must not wrap to a positive x.
fn context(n:&Neighborhood,image:&Bitmap,x:u32,y:u32,width:u8)->usize{
    let first=match n.template(){Template::T0|Template::T3=>4,Template::T1=>3,Template::T2=>2};
    let mut cx=0;
    for (i,p) in n.slots().iter().enumerate(){
        let (dx,dy)=if i==first{(-i64::from(width),0)}else{(i64::from(p.x),i64::from(p.y))};
        if image.sample(i64::from(x)+dx,i64::from(y)+dy){cx|=1usize<<i;}
    }
    cx
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Limits{
    pub patterns:u32,pub pixels:u64,pub packed_bytes:usize,pub work:u64,pub decisions:usize,
}
impl Default for Limits{
    fn default()->Self{Self{patterns:MAX_PATTERNS,pixels:bitmap::MAX_PIXELS,
        packed_bytes:bitmap::MAX_PACKED_BYTES,work:bitmap::MAX_COMPOSE_WORK,decisions:16_777_216}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum EntropyStats{Mmr{encoded_bits:usize,eofb_present:bool},Arithmetic{decisions:usize}}
#[derive(Debug,PartialEq,Eq)]
pub struct PatternDictionary{
    pub header:Header,
    patterns:Vec<Bitmap>,
    pub entropy:EntropyStats,
    pub packed_bytes:usize,
    pub peak_packed_bytes:usize,
    pub work_used:u64,
}
impl PatternDictionary{
    pub fn patterns(&self)->&[Bitmap]{&self.patterns}
    pub fn pattern(&self,id:u32)->Option<&Bitmap>{self.patterns.get(id as usize)}
}
/// Work charges input scan, context initialization, native entropy
/// decoding and pixel extraction. Packed cap includes the collective
/// image and all split patterns simultaneously (also covers MMR copy).
/// Pattern slots and logical pixels have independent caps; not RSS.
pub fn decode(data:&[u8],limits:Limits)->Result<PatternDictionary>{
    use crate::filters::ccitt;
    if limits.patterns>MAX_PATTERNS||limits.pixels>bitmap::MAX_PIXELS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.work>bitmap::MAX_COMPOSE_WORK||
        limits.decisions>16_777_216{return Err(Error::Limit("JBIG2 pattern limits"));}
    let (h,payload)=parse(data)?;
    let area=u64::from(h.collective_width)*u64::from(h.height);
    if h.count>limits.patterns||area>limits.pixels{return Err(Error::Limit("JBIG2 pattern count or pixels"));}
    let collective_bytes=(h.collective_width as usize).div_ceil(8)*h.height as usize;
    let packed=(h.width as usize).div_ceil(8)*h.height as usize*h.count as usize;
    let peak=collective_bytes.checked_add(packed).ok_or(Error::Limit("JBIG2 pattern packed budget"))?;
    if peak>limits.packed_bytes{return Err(Error::Limit("JBIG2 pattern packed budget"));}
    let setup=data.len() as u64+area*2+packed as u64+u64::from(h.count);
    let left=limits.work.checked_sub(setup).ok_or(Error::Limit("JBIG2 pattern work"))?;
    let (collective,entropy,cost)=match h.coding{
        Coding::Mmr=>{
            let conversion=collective_bytes as u64*3;
            let remaining=left.checked_sub(conversion).ok_or(Error::Limit("JBIG2 pattern MMR conversion"))?;
            let m=ccitt::decode_mmr_known_length(payload,h.collective_width,u32::from(h.height),remaining)?;
            let cost=conversion+area*2+payload.len() as u64*8+32*m.mode_counts.iter().sum::<u64>();
            if cost>left{return Err(Error::Limit("JBIG2 pattern MMR work"));}
            (Bitmap::from_packed(h.collective_width,u32::from(h.height),&m.packed)?,
                EntropyStats::Mmr{encoded_bits:m.encoded_bits,eofb_present:m.eofb_present},cost)
        },
        Coding::Arithmetic(t)=>{
            let cost=area*(t.bits() as u64+1)+t.context_count() as u64;
            if cost>left{return Err(Error::Limit("JBIG2 pattern arithmetic work"));}
            let decisions=usize::try_from(area).map_err(|_|Error::Limit("JBIG2 pattern decisions"))?;
            if decisions>limits.decisions{return Err(Error::Limit("JBIG2 pattern decisions"));}
            let mut decoder=qm::Decoder::new(payload,decisions)?;
            let mut bank=Vec::new();
            bank.try_reserve_exact(t.context_count()).map_err(|_|Error::Limit("JBIG2 pattern contexts"))?;
            bank.resize(t.context_count(),qm::Context::default());
            let n=Neighborhood::nominal(t);
            let mut image=Bitmap::new(h.collective_width,u32::from(h.height),false)?;
            for y in 0..u32::from(h.height){for x in 0..h.collective_width{
                let cx=context(&n,&image,x,y,h.width);
                let bit=decoder.decode(&mut bank[cx])?!=0;
                image.set(x,y,bit)?;
            }}
            (image,EntropyStats::Arithmetic{decisions:decoder.decisions_used()},cost)
        }
    };
    let mut patterns=Vec::new();
    patterns.try_reserve_exact(h.count as usize).map_err(|_|Error::Limit("JBIG2 pattern allocation"))?;
    for i in 0..h.count{
        let mut p=Bitmap::new(u32::from(h.width),u32::from(h.height),false)?;
        let start=i*u32::from(h.width);
        for y in 0..u32::from(h.height){for x in 0..u32::from(h.width){
            p.set(x,y,collective.get(start+x,y).ok_or(Error::Invalid("JBIG2 pattern collective geometry"))?)?;
        }}
        patterns.push(p);
    }
    Ok(PatternDictionary{header:h,patterns,entropy,packed_bytes:packed,peak_packed_bytes:peak,work_used:setup+cost})
}

#[cfg(test)]
mod tests{
    use super::*;
    // Original T.88 (2000) Annex H.1 segment 5 and segment 12 data parts,
    // printed pp.129–131 and 136; Figure H.8 reference pixels transcribed
    // separately from the grid, not generated by either entropy decoder.
    const MMR:&[u8]=&[1,4,4,0,0,0,15,0x20,0xd1,0x84,0x61,0x18,0x45,0xf2,0xf9,0x7c,
        0x8f,0x11,0xc3,0x9e,0x45,0xf2,0xf9,0x7d,0x42,0x85,0x0a,0xaa,0x84,0x62,0x2f,0xee,
        0xec,0x44,0x62,0x22,0x35,0x2a,0x0a,0x83,0xb9,0xdc,0xee,0x77,0x80];
    const ARITH:&[u8]=&[6,4,4,0,0,0,15,0x90,0x71,0x6b,0x6d,0x99,0xa7,0xaa,0x49,0x7d,0xf2,0xe5,0x48,0x1f,
        0xdc,0x68,0xbc,0x6e,0x40,0xbb,0xff,0xac];
    const ROWS:[[u8;4];16]=[
        [0,0,0,0],[0,2,0,0],[0,6,0,0],[0,6,2,0],
        [0,6,6,0],[2,6,6,0],[2,14,6,0],[2,14,6,4],
        [2,14,7,4],[6,14,15,4],[6,14,15,6],[6,15,15,6],
        [14,15,15,6],[15,15,15,6],[15,15,15,7],[15,15,15,15]];
    #[test]
    fn pattern_annex_h_both_modes_exact_pixels(){
        let a=decode(MMR,Limits::default()).unwrap();
        let b=decode(ARITH,Limits::default()).unwrap();
        assert_eq!(a.patterns(),b.patterns());
        for (i,rows) in ROWS.iter().enumerate(){
            let p=a.pattern(i as u32).unwrap();
            assert_eq!((p.width(),p.height()),(4,4));
            assert_eq!(p.packed(),rows.map(|r|r<<4));
        }
        assert_eq!(a.header.count,16);assert_eq!(b.header.collective_width,64);
        assert!(matches!(a.entropy,EntropyStats::Mmr{..}));
        assert_eq!(b.entropy,EntropyStats::Arithmetic{decisions:256});
        assert_eq!((a.packed_bytes,a.peak_packed_bytes),(64,96));
        assert!(a.pattern(16).is_none());
    }
    #[test]
    fn pattern_annex_h_exact_caps_and_malformed(){
        for data in [MMR,ARITH]{
            let d=decode(data,Limits::default()).unwrap();
            let exact=Limits{patterns:16,pixels:256,packed_bytes:96,work:d.work_used,
                decisions:if data==MMR{0}else{256}};
            assert_eq!(decode(data,exact).unwrap(),d);
            for cap in [Limits{patterns:15,..exact},Limits{pixels:255,..exact},
                Limits{packed_bytes:95,..exact},Limits{work:exact.work-1,..exact}]{
                assert!(decode(data,cap).is_err());
            }
            for end in 0..data.len(){assert!(decode(&data[..end],exact).is_err(),"prefix {end}");}
            let mut trailing=data.to_vec();trailing.push(0);assert!(decode(&trailing,Limits::default()).is_err());
        }
        assert!(decode(ARITH,Limits{decisions:255,..Limits::default()}).is_err());
        for flags in [3,5,7,8,128,255]{let mut b=MMR.to_vec();b[0]=flags;assert!(parse(&b).is_err());}
        for i in [1,2]{let mut b=MMR.to_vec();b[i]=0;assert!(parse(&b).is_err());}
        let mut b=MMR.to_vec();b[3..7].fill(255);assert!(parse(&b).is_err());
    }
    #[test]
    fn pattern_context_wide_offset_all_templates(){
        for (t,bit) in [(Template::T0,4),(Template::T1,3),(Template::T2,2),(Template::T3,4)]{
            let n=Neighborhood::nominal(t);let mut b=Bitmap::new(510,2,false).unwrap();
            b.set(0,0,true).unwrap();
            assert_eq!(context(&n,&b,255,0,255),1<<bit);
            assert_eq!(context(&n,&b,254,0,255),0);
            assert_eq!(context(&n,&b,255,1,255),0);
            b.set(127,1,true).unwrap();assert_eq!(context(&n,&b,255,1,128),1<<bit);
        }
    }
    #[test]
    fn pattern_non_byte_aligned_split_and_single(){
        // MMR white reference vertical zero for each of two rows.
        for count in [1u32,3]{
            let mut b=vec![1,3,2];b.extend((count-1).to_be_bytes());b.push(0xc0);
            let d=decode(&b,Limits::default()).unwrap();
            assert_eq!(d.patterns().len(),count as usize);
            for p in d.patterns(){assert_eq!(p.packed(),[0,0]);}
            assert_eq!(d.packed_bytes,2*count as usize);
        }
    }
}
