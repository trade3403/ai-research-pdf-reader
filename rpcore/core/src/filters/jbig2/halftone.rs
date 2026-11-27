//! Native T.88 (2000) 6.6, 7.4.5 and Annex C halftone region.
//! Arithmetic payloads use one QM stream and one generic probability bank
//! across MSB-first Gray-coded planes. MMR multi-plane framing is deferred.
use crate::{Error,Result,MAX_BYTES};
use super::{bitmap::{self,Bitmap,ComposeBudget,ComposeOp,RegionInfo},generic::{Neighborhood,Template},pattern::PatternDictionary,qm};

pub const MAX_GRID_CELLS:u64=8_388_608;
pub const MAX_DECISIONS:usize=16_777_216;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Coding{Mmr,Arithmetic(Template)}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Grid{
    pub width:u32,pub height:u32,pub x:i32,pub y:i32,pub rx:u16,pub ry:u16,
}
impl Grid{
    /// 24.8 fixed-point origin and 8.8 unsigned vector. Floor for negative
    /// coordinates, using widened signed arithmetic before shifting.
    pub fn position(&self,n:u32,m:u32)->Result<(i64,i64)>{
        if n>=self.width||m>=self.height{return Err(Error::Invalid("JBIG2 halftone grid coordinate"));}
        let (n,m,rx,ry)=(i64::from(n),i64::from(m),i64::from(self.rx),i64::from(self.ry));
        Ok(((i64::from(self.x)+m*ry+n*rx)>>8,(i64::from(self.y)+m*rx-n*ry)>>8))
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Header{
    pub region:RegionInfo,pub grid:Grid,pub coding:Coding,
    pub enable_skip:bool,pub default_black:bool,pub op:ComposeOp,
}
pub fn parse(data:&[u8])->Result<(Header,&[u8])>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 halftone input"));}
    let (region,rest)=bitmap::parse_region(data)?;
    let p=rest.get(..21).ok_or(Error::Invalid("JBIG2 halftone header"))?;
    let flags=p[0];let t=Template::from_code((flags>>1)&3)?;
    let enable_skip=flags&8!=0;
    let coding=if flags&1!=0{
        if t!=Template::T0||enable_skip{return Err(Error::Invalid("JBIG2 halftone MMR flags"));}
        Coding::Mmr
    }else{Coding::Arithmetic(t)};
    let op=ComposeOp::from_code((flags>>4)&7)?;
    let word=|i:usize|u32::from_be_bytes([p[i],p[i+1],p[i+2],p[i+3]]);
    let grid=Grid{width:word(1),height:word(5),x:word(9) as i32,y:word(13) as i32,
        rx:u16::from_be_bytes([p[17],p[18]]),ry:u16::from_be_bytes([p[19],p[20]])};
    if grid.width==0||grid.height==0{return Err(Error::Invalid("JBIG2 halftone grid dimensions"));}
    if grid.width>bitmap::MAX_SIDE||grid.height>bitmap::MAX_SIDE||
        u64::from(grid.width)*u64::from(grid.height)>MAX_GRID_CELLS{
        return Err(Error::Limit("JBIG2 halftone grid dimensions"));
    }
    Ok((Header{region,grid,coding,enable_skip,default_black:flags&128!=0,op},&rest[21..]))
}
fn outside(h:&Header,dict:&PatternDictionary,x:i64,y:i64)->bool{
    x+i64::from(dict.header.width)<=0||y+i64::from(dict.header.height)<=0||
        x>=i64::from(h.region.width)||y>=i64::from(h.region.height)
}
fn plane_count(count:u32)->Result<u8>{
    if count==0||count>super::pattern::MAX_PATTERNS{return Err(Error::Invalid("JBIG2 halftone pattern count"));}
    Ok((32-(count-1).leading_zeros()) as u8)
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Limits{
    pub grid_cells:u64,pub pixels:u64,pub packed_bytes:usize,pub decisions:usize,pub work:u64,
}
impl Default for Limits{
    fn default()->Self{Self{grid_cells:MAX_GRID_CELLS,pixels:bitmap::MAX_PIXELS,
        packed_bytes:bitmap::MAX_PACKED_BYTES,decisions:MAX_DECISIONS,work:bitmap::MAX_COMPOSE_WORK}}
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedHalftone{
    pub header:Header,pub bitmap:Bitmap,pub indices:Vec<u16>,pub planes:u8,
    pub decisions:usize,pub skipped_cells:u64,pub composed_pixels:u64,
    /// Logical index bytes + current plane + output; excludes borrowed dictionary
    /// and context metadata. Conservative simultaneous bitmap/index bound.
    pub peak_packed_bytes:usize,pub work_used:u64,
}
/// Compose in row-major grid order; clip patterns without rotating their pixels.
/// Validate every index, including wholly clipped cells; skipped entropy is
/// reconstructed as zero. All output is local and discarded on error.
fn compose(h:&Header,dict:&PatternDictionary,values:&[u16],work:u64)->Result<(Bitmap,u64)>{
    let mut output=Bitmap::new(h.region.width,h.region.height,h.default_black)?;
    let mut budget=ComposeBudget::new(work)?;
    for m in 0..h.grid.height{for n in 0..h.grid.width{
        let i=m as usize*h.grid.width as usize+n as usize;
        let value=*values.get(i).ok_or(Error::Invalid("JBIG2 halftone index array"))?;
        let p=dict.pattern(u32::from(value)).ok_or(Error::Invalid("JBIG2 halftone pattern index"))?;
        let (x,y)=h.grid.position(n,m)?;
        output.compose(p,x,y,h.op,&mut budget)?;
    }}
    Ok((output,work-budget.remaining()))
}
/// Decode a complete arithmetic halftone payload with a caller-provided
/// decoded pattern dictionary. Contexts persist across planes; the plane
/// bitmap is reset each time, and Gray decoding occurs only after all coded
/// pixels in that plane have been decoded (never modifies its contexts).
/// Work is a conservative logical bound, not a wall-clock or RSS guarantee.
pub fn decode_arithmetic(data:&[u8],dict:&PatternDictionary,limits:Limits)->Result<DecodedHalftone>{
    if limits.grid_cells>MAX_GRID_CELLS||limits.pixels>bitmap::MAX_PIXELS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.decisions>MAX_DECISIONS||
        limits.work>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 halftone limits"));}
    let (h,body)=parse(data)?;
    let t=match h.coding{Coding::Arithmetic(t)=>t,Coding::Mmr=>return Err(Error::Unsupported("JBIG2 halftone MMR planes"))};
    let cells=u64::from(h.grid.width)*u64::from(h.grid.height);
    let pixels=u64::from(h.region.width)*u64::from(h.region.height);
    if cells>limits.grid_cells||pixels>limits.pixels{return Err(Error::Limit("JBIG2 halftone cells or pixels"));}
    let planes=plane_count(dict.header.count)?;
    let index_bytes=cells*2;
    let plane_bytes=if planes==0{0}else{u64::from(h.grid.width).div_ceil(8)*u64::from(h.grid.height)};
    let canvas=u64::from(h.region.width).div_ceil(8)*u64::from(h.region.height);
    let peak=index_bytes+plane_bytes+canvas;
    if peak>limits.packed_bytes as u64{return Err(Error::Limit("JBIG2 halftone packed budget"));}
    // Charge full plane traversal even when every cell is skipped.
    // Include index initialization, grid/skip tests, Gray reconstruction,
    // output initialization, input scan and active context bank.
    let setup=data.len() as u64+pixels+cells*4+
        cells*u64::from(planes)*(t.bits() as u64+8)+
        if planes==0{0}else{t.context_count() as u64};
    let left=limits.work.checked_sub(setup).ok_or(Error::Limit("JBIG2 halftone work"))?;
    let mut indices=Vec::new();
    indices.try_reserve_exact(cells as usize).map_err(|_|Error::Limit("JBIG2 halftone indices"))?;
    indices.resize(cells as usize,0u16);
    let mut skipped=0u64;
    if h.enable_skip{
        for m in 0..h.grid.height{for n in 0..h.grid.width{
            let (x,y)=h.grid.position(n,m)?;
            if outside(&h,dict,x,y){skipped+=1;}
        }}
    }
    let decisions=if planes==0{
        // A one-pattern dictionary carries no Gray planes.
        if !body.is_empty(){return Err(Error::Invalid("JBIG2 halftone zero-plane bytes"));}
        0
    }else{
        let required=(cells-skipped)*u64::from(planes);
        if required>limits.decisions as u64{return Err(Error::Limit("JBIG2 halftone decisions"));}
        let mut decoder=qm::Decoder::new(body,required as usize)?;
        let mut bank=Vec::new();bank.try_reserve_exact(t.context_count())
            .map_err(|_|Error::Limit("JBIG2 halftone contexts"))?;
        bank.resize(t.context_count(),qm::Context::default());
        let neighborhood=Neighborhood::nominal(t);
        for _ in 0..planes{
            let mut plane=Bitmap::new(h.grid.width,h.grid.height,false)?;
            for m in 0..h.grid.height{for n in 0..h.grid.width{
                if h.enable_skip{
                    let (x,y)=h.grid.position(n,m)?;
                    if outside(&h,dict,x,y){continue;}
                }
                let cx=neighborhood.context(&plane,n,m)?;
                plane.set(n,m,decoder.decode(&mut bank[cx])?!=0)?;
            }}
            for m in 0..h.grid.height{for n in 0..h.grid.width{
                let i=m as usize*h.grid.width as usize+n as usize;
                let coded=u16::from(plane.get(n,m).ok_or(Error::Invalid("JBIG2 halftone plane pixel"))?);
                let previous=indices[i];
                indices[i]=(previous<<1)|(coded^(previous&1));
            }}
        }
        decoder.decisions_used()
    };
    let (bitmap,composed_pixels)=compose(&h,dict,&indices,left)?;
    Ok(DecodedHalftone{header:h,bitmap,indices,planes,decisions,skipped_cells:skipped,
        composed_pixels,peak_packed_bytes:peak as usize,work_used:setup+composed_pixels})
}

#[cfg(test)]
mod tests{
    use super::*;
    use super::super::pattern;
    // Original Annex H.1 segments 12 and 13 data parts, printed pp.136–137.
    // Independent expected indices from the table on p.133; packed pixels
    // constructed offline from Figure H.8 pattern rows and that index table.
    const DICT:&[u8]=&[6,4,4,0,0,0,15,144,113,107,109,153,167,170,73,125,242,229,72,31,220,104,188,110,64,187,255,172];
    const REGION:&[u8]=&[0,0,0,32,0,0,0,36,0,0,0,16,0,0,0,15,0,2,0,0,0,8,0,0,0,9,0,0,0,0,0,0,0,0,4,0,0,0,135,203,130,30,102,164,20,235,60,74,21,250,204,214,243,177,111,76,237,191,167,191,255,172];
    const EXPECTED:&[u8]=&[
        0,0,2,34,
        2,102,102,238,
        0,2,102,102,
        0,0,0,4,
        0,0,34,34,
        38,102,110,238,
        0,38,102,103,
        0,0,0,68,
        0,2,34,38,
        102,102,238,238,
        2,102,102,127,
        0,0,4,68,
        0,34,34,102,
        102,110,238,238,
        38,102,103,255,
        0,0,68,70,
        2,34,38,102,
        102,238,238,239,
        102,102,127,255,
        0,4,68,102,
        34,34,102,110,
        110,238,238,255,
        102,103,255,255,
        0,68,70,102,
        34,38,102,239,
        238,238,239,255,
        102,127,255,255,
        4,68,102,102,
        34,102,110,255,
        238,238,255,255,
        103,255,255,255,
        68,70,102,103,
        38,102,239,255,
        238,239,255,255,
        127,255,255,255,
        68,102,102,127,
    ];
    #[test]
    fn halftone_annex_h_arithmetic_shared_planes(){
        let dict=pattern::decode(DICT,pattern::Limits::default()).unwrap();
        let d=decode_arithmetic(REGION,&dict,Limits::default()).unwrap();
        assert_eq!((d.header.region.x,d.header.region.y,d.header.region.width,d.header.region.height),(16,15,32,36));
        assert_eq!(d.header.grid,Grid{width:8,height:9,x:0,y:0,rx:1024,ry:0});
        assert_eq!(d.header.coding,Coding::Arithmetic(Template::T1));
        assert_eq!((d.planes,d.decisions,d.skipped_cells,d.composed_pixels),(4,288,0,1152));
        assert_eq!(d.indices,(0..9u16).flat_map(|m|(0..8u16).map(move |n|m+n)).collect::<Vec<_>>());
        assert_eq!(d.bitmap.packed(),EXPECTED);
        assert_eq!(d.peak_packed_bytes,297);
    }
    #[test]
    fn halftone_exact_caps_and_truncation(){
        let dict=pattern::decode(DICT,pattern::Limits::default()).unwrap();
        let d=decode_arithmetic(REGION,&dict,Limits::default()).unwrap();
        let exact=Limits{grid_cells:72,pixels:1152,packed_bytes:297,decisions:288,work:d.work_used};
        assert_eq!(decode_arithmetic(REGION,&dict,exact).unwrap(),d);
        for cap in [Limits{grid_cells:71,..exact},Limits{pixels:1151,..exact},Limits{packed_bytes:296,..exact},
            Limits{decisions:287,..exact},Limits{work:exact.work-1,..exact}]{
            assert!(decode_arithmetic(REGION,&dict,cap).is_err());
        }
        for n in 0..REGION.len(){assert!(decode_arithmetic(&REGION[..n],&dict,exact).is_err(),"prefix {n}");}
        let mut extra=REGION.to_vec();extra.push(0);assert!(decode_arithmetic(&extra,&dict,Limits::default()).is_err());
        for flags in [3,5,7,9,0x50,0x60,0x70]{let mut b=REGION.to_vec();b[17]=flags;assert!(parse(&b).is_err());}
        let mut b=REGION.to_vec();b[18..22].fill(255);assert!(parse(&b).is_err());
    }
    #[test]
    fn halftone_rotated_fixed_point_and_clipped_composition(){
        let grid=Grid{width:3,height:2,x:-1,y:-257,rx:384,ry:128};
        assert_eq!(grid.position(0,0).unwrap(),(-1,-2));
        assert_eq!(grid.position(1,0).unwrap(),(1,-2));
        assert_eq!(grid.position(2,0).unwrap(),(2,-3));
        assert_eq!(grid.position(0,1).unwrap(),(0,0));
        assert_eq!(grid.position(2,1).unwrap(),(3,-1));
        assert!(grid.position(3,0).is_err());
        let dict=pattern::decode(DICT,pattern::Limits::default()).unwrap();
        let h=Header{region:RegionInfo{width:5,height:3,x:0,y:0,op:ComposeOp::Or},grid,
            coding:Coding::Arithmetic(Template::T1),enable_skip:false,default_black:false,op:ComposeOp::Xor};
        // Independent boolean-grid result: six solid 4x4 patterns, clipped.
        let (b,work)=compose(&h,&dict,&[15;6],100).unwrap();
        assert_eq!(b.packed(),[48,8,144]);
        assert_eq!(work,47);
        assert!(compose(&h,&dict,&[16;6],100).is_err());
        assert!(compose(&h,&dict,&[15;6],work-1).is_err());
    }
    #[test]
    fn halftone_skip_all_and_single_pattern(){
        let dict=pattern::decode(DICT,pattern::Limits::default()).unwrap();
        let mut data=REGION[..38].to_vec();data[17]=0x8a;
        data[26..30].copy_from_slice(&(-65536i32).to_be_bytes());
        data.extend([255,172]);
        let d=decode_arithmetic(&data,&dict,Limits{decisions:0,..Limits::default()}).unwrap();
        assert_eq!((d.decisions,d.skipped_cells,d.composed_pixels),(0,72,0));
        assert_eq!(d.indices,[0;72]);assert_eq!(d.bitmap.packed(),[255;144]);
        let one=pattern::decode(&[1,1,1,0,0,0,0,128],pattern::Limits::default()).unwrap();
        let mut data=REGION[..38].to_vec();data[17]=0;
        let d=decode_arithmetic(&data,&one,Limits{decisions:0,..Limits::default()}).unwrap();
        assert_eq!((d.planes,d.decisions),(0,0));assert_eq!(d.indices,[0;72]);
        assert_eq!(d.bitmap.packed(),[0;144]);data.push(0);
        assert!(decode_arithmetic(&data,&one,Limits::default()).is_err());
    }
}
