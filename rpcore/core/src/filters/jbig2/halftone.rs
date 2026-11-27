//! Native T.88 (2000) 6.6, 7.4.5 and Annex C halftone region.
//! Arithmetic payloads use one QM stream and one generic probability bank
//! across MSB-first Gray-coded planes. MMR planes have mandatory EOFB.
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
    /// Byte spans of each MMR plane, including EOFB and alignment; empty for QM.
    pub mmr_plane_bytes:Vec<usize>,
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
            accumulate_gray(&h,&mut indices,plane.packed(),plane.stride())?;
        }
        decoder.decisions_used()
    };
    let (bitmap,composed_pixels)=compose(&h,dict,&indices,left)?;
    Ok(DecodedHalftone{header:h,bitmap,indices,planes,decisions,skipped_cells:skipped,
        composed_pixels,mmr_plane_bytes:Vec::new(),peak_packed_bytes:peak as usize,work_used:setup+composed_pixels})
}

fn accumulate_gray(h:&Header,indices:&mut[u16],packed:&[u8],stride:usize)->Result<()>{
    for m in 0..h.grid.height as usize{for n in 0..h.grid.width as usize{
        let i=m*h.grid.width as usize+n;
        let byte=*packed.get(m*stride+n/8).ok_or(Error::Invalid("JBIG2 halftone plane pixel"))?;
        let coded=u16::from((byte>>(7-n%8))&1);
        let previous=indices[i];indices[i]=(previous<<1)|(coded^(previous&1));
    }}
    Ok(())
}
/// Dispatch the complete halftone region payload in either coding mode.
pub fn decode(data:&[u8],dict:&PatternDictionary,limits:Limits)->Result<DecodedHalftone>{
    match parse(data)?.0.coding{
        Coding::Mmr=>decode_mmr(data,dict,limits),
        Coding::Arithmetic(_)=>decode_arithmetic(data,dict,limits),
    }
}
/// Decode concatenated mandatory-EOFB MMR planes, resetting the white
/// reference row for each. No sentinel scan or speculative decoding.
pub fn decode_mmr(data:&[u8],dict:&PatternDictionary,limits:Limits)->Result<DecodedHalftone>{
    use crate::filters::ccitt;
    if limits.grid_cells>MAX_GRID_CELLS||limits.pixels>bitmap::MAX_PIXELS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.decisions>MAX_DECISIONS||
        limits.work>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 halftone limits"));}
    let (h,body)=parse(data)?;
    if h.coding!=Coding::Mmr{return Err(Error::Unsupported("JBIG2 halftone arithmetic planes"));}
    let cells=u64::from(h.grid.width)*u64::from(h.grid.height);
    let pixels=u64::from(h.region.width)*u64::from(h.region.height);
    if cells>limits.grid_cells||pixels>limits.pixels{return Err(Error::Limit("JBIG2 halftone cells or pixels"));}
    let planes=plane_count(dict.header.count)?;
    let plane_bytes=if planes==0{0}else{u64::from(h.grid.width).div_ceil(8)*u64::from(h.grid.height)};
    let canvas=u64::from(h.region.width).div_ceil(8)*u64::from(h.region.height);
    let peak=cells*2+plane_bytes+canvas;
    if peak>limits.packed_bytes as u64{return Err(Error::Limit("JBIG2 halftone packed budget"));}
    let setup=data.len() as u64+pixels+cells*4+cells*u64::from(planes)*4;
    let mut left=limits.work.checked_sub(setup).ok_or(Error::Limit("JBIG2 halftone work"))?;
    let mut indices=Vec::new();
    indices.try_reserve_exact(cells as usize).map_err(|_|Error::Limit("JBIG2 halftone indices"))?;
    indices.resize(cells as usize,0u16);
    let mut sizes=Vec::new();
    sizes.try_reserve_exact(planes as usize).map_err(|_|Error::Limit("JBIG2 halftone plane slots"))?;
    let mut offset=0usize;
    for _ in 0..planes{
        let suffix=body.get(offset..).ok_or(Error::Invalid("JBIG2 halftone plane span"))?;
        let plane=ccitt::decode_mmr_plane_prefix(suffix,h.grid.width,h.grid.height,left)?;
        let bytes=plane.encoded_bits.div_ceil(8);
        let cost=cells*2+bytes as u64*8+32*plane.mode_counts.iter().sum::<u64>();
        left=left.checked_sub(cost).ok_or(Error::Limit("JBIG2 halftone MMR work"))?;
        offset=offset.checked_add(bytes).ok_or(Error::Limit("JBIG2 halftone plane span"))?;
        sizes.push(bytes);
        accumulate_gray(&h,&mut indices,&plane.packed,plane.stride)?;
    }
    if offset!=body.len(){return Err(Error::Invalid("JBIG2 halftone trailing plane bytes"));}
    let (bitmap,composed_pixels)=compose(&h,dict,&indices,left)?;
    Ok(DecodedHalftone{header:h,bitmap,indices,planes,decisions:0,skipped_cells:0,
        composed_pixels,mmr_plane_bytes:sizes,peak_packed_bytes:peak as usize,
        work_used:limits.work-left+composed_pixels})
}


/// Cumulative limits for pattern dictionaries and halftone regions in one
/// validated embedded page. Pixel counts for dictionaries and region canvases
/// are separate; QM decisions, work and logical packed bytes are shared.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ExecutionLimits{
    pub dictionaries:usize,pub regions:usize,pub patterns:u32,
    pub pattern_pixels:u64,pub region_pixels:u64,pub grid_cells:u64,
    pub packed_bytes:usize,pub decisions:usize,pub work:u64,
}
impl Default for ExecutionLimits{
    fn default()->Self{Self{dictionaries:super::MAX_SEGMENTS,regions:super::MAX_SEGMENTS,
        patterns:super::pattern::MAX_PATTERNS,pattern_pixels:bitmap::MAX_PIXELS,
        region_pixels:bitmap::MAX_PIXELS,grid_cells:MAX_GRID_CELLS,
        packed_bytes:bitmap::MAX_PACKED_BYTES,decisions:MAX_DECISIONS,work:bitmap::MAX_COMPOSE_WORK}}
}
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct ExecutionStats{
    pub dictionaries:usize,pub regions:usize,pub patterns:u32,
    pub pattern_pixels:u64,pub region_pixels:u64,pub grid_cells:u64,
    pub packed_bytes:usize,pub peak_packed_bytes:usize,pub decisions:usize,pub work_used:u64,
}
#[derive(Debug,PartialEq,Eq)]
pub struct HalftoneSegment{
    pub number:u32,pub kind:u8,pub page:u32,pub dictionary_number:u32,
    pub decoded:DecodedHalftone,
}
#[derive(Debug,PartialEq,Eq)]
pub struct HalftoneSet{
    dictionaries:std::collections::BTreeMap<u32,PatternDictionary>,
    pub regions:Vec<HalftoneSegment>,
    pub stats:ExecutionStats,
}
impl HalftoneSet{
    pub fn dictionary(&self,number:u32)->Option<&PatternDictionary>{self.dictionaries.get(&number)}
}
/// Execute all type 16 and 20/22/23 segments in original graph order.
/// The graph is constructed internally from the exact input streams.
/// A halftone must refer to exactly one earlier pattern dictionary.
/// All dictionaries, including unreferenced ones, are decoded; each is
/// stored once and borrowed by its regions. Other payloads are opaque.
/// This does not execute retention flags or composite the page. On error
/// no partial result escapes. Packed accounting excludes context/graph,
/// CCITT transition metadata and allocator overhead; it is not RSS.
pub fn execute_pdf_halftones(globals:&[u8],page:&[u8],limits:ExecutionLimits)->Result<HalftoneSet>{
    use super::{pattern,SegmentSource};
    if limits.dictionaries>super::MAX_SEGMENTS||limits.regions>super::MAX_SEGMENTS||
        limits.patterns>pattern::MAX_PATTERNS||limits.pattern_pixels>bitmap::MAX_PIXELS||
        limits.region_pixels>bitmap::MAX_PIXELS||limits.grid_cells>MAX_GRID_CELLS||
        limits.packed_bytes>bitmap::MAX_PACKED_BYTES||limits.decisions>MAX_DECISIONS||
        limits.work>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 halftone execution limits"));}
    let input=globals.len().checked_add(page.len()).ok_or(Error::Limit("JBIG2 halftone execution input"))?;
    if input as u64>limits.work{return Err(Error::Limit("JBIG2 halftone execution work"));}
    let graph=super::inspect_pdf_streams(globals,page)?;
    let mut counts=(0usize,0usize);
    let mut setup=input as u64+graph.graph.len() as u64*2;
    // Preflight kinds/references before entropy decoding or output allocation.
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 halftone location"))?;
        setup=setup.checked_add(node.dependencies.len() as u64).ok_or(Error::Limit("JBIG2 halftone execution work"))?;
        match seg.kind{
            16=>{
                if !node.dependencies.is_empty(){return Err(Error::Invalid("JBIG2 pattern dictionary references"));}
                counts.0+=1;
            },
            20|22|23=>{
                if node.location.source!=SegmentSource::Page||node.dependencies.len()!=1{
                    return Err(Error::Invalid("JBIG2 halftone dictionary reference count"));
                }
                let dep=graph.segment(node.dependencies[0]).ok_or(Error::Invalid("JBIG2 halftone dictionary reference"))?;
                if dep.kind!=16{return Err(Error::Invalid("JBIG2 halftone dictionary kind"));}
                counts.1+=1;
            },
            _=>{},
        }
    }
    if counts.0>limits.dictionaries||counts.1>limits.regions{return Err(Error::Limit("JBIG2 halftone segment count"));}
    if setup>limits.work{return Err(Error::Limit("JBIG2 halftone execution work"));}
    let mut out=HalftoneSet{dictionaries:std::collections::BTreeMap::new(),regions:Vec::new(),
        stats:ExecutionStats{work_used:setup,..ExecutionStats::default()}};
    out.regions.try_reserve_exact(counts.1).map_err(|_|Error::Limit("JBIG2 halftone region allocation"))?;
    for node in &graph.graph{
        let seg=graph.segment(node.location).ok_or(Error::Invalid("JBIG2 halftone location"))?;
        if !matches!(seg.kind,16|20|22|23){continue;}
        let stream=match node.location.source{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
        let data=stream.get(seg.payload.clone()).ok_or(Error::Invalid("JBIG2 halftone payload span"))?;
        let stats=&mut out.stats;
        let work=limits.work-stats.work_used;
        let packed=limits.packed_bytes-stats.packed_bytes;
        let decisions=limits.decisions-stats.decisions;
        if seg.kind==16{
            let d=pattern::decode(data,pattern::Limits{patterns:limits.patterns-stats.patterns,
                pixels:limits.pattern_pixels-stats.pattern_pixels,packed_bytes:packed,work,decisions})?;
            stats.peak_packed_bytes=stats.peak_packed_bytes.max(stats.packed_bytes+d.peak_packed_bytes);
            stats.packed_bytes+=d.packed_bytes;
            stats.patterns+=d.header.count;
            stats.pattern_pixels+=u64::from(d.header.collective_width)*u64::from(d.header.height);
            stats.work_used+=d.work_used;
            if let pattern::EntropyStats::Arithmetic{decisions}=d.entropy{stats.decisions+=decisions;}
            stats.dictionaries+=1;
            out.dictionaries.insert(seg.number,d);
        }else{
            let dep=graph.segment(node.dependencies[0]).ok_or(Error::Invalid("JBIG2 halftone dictionary reference"))?;
            let dict=out.dictionaries.get(&dep.number).ok_or(Error::Invalid("JBIG2 halftone missing dictionary"))?;
            let d=decode(data,dict,Limits{grid_cells:limits.grid_cells-stats.grid_cells,
                pixels:limits.region_pixels-stats.region_pixels,packed_bytes:packed,work,decisions})?;
            stats.peak_packed_bytes=stats.peak_packed_bytes.max(stats.packed_bytes+d.peak_packed_bytes);
            stats.packed_bytes+=d.bitmap.packed().len()+d.indices.len()*2;
            stats.region_pixels+=u64::from(d.header.region.width)*u64::from(d.header.region.height);
            stats.grid_cells+=u64::from(d.header.grid.width)*u64::from(d.header.grid.height);
            stats.decisions+=d.decisions;
            stats.work_used+=d.work_used;
            stats.regions+=1;
            out.regions.push(HalftoneSegment{number:seg.number,kind:seg.kind,page:seg.page,
                dictionary_number:dep.number,decoded:d});
        }
    }
    Ok(out)
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
    // Original Annex H.1 segments 5 and 6, printed pp.131–133.
    const MMR_DICT:&[u8]=&[1,4,4,0,0,0,15,32,209,132,97,24,69,242,249,124,143,17,195,158,69,242,249,125,66,133,10,170,132,98,47,238,236,68,98,34,53,42,10,131,185,220,238,119,128];
    const MMR_REGION:&[u8]=&[0,0,0,32,0,0,0,36,0,0,0,16,0,0,0,15,0,1,0,0,0,8,0,0,0,9,0,0,0,0,0,0,0,0,4,0,0,0,170,170,170,170,128,8,0,128,54,213,85,107,90,212,0,64,4,46,233,82,210,210,210,138,165,74,0,32,2,35,224,149,36,180,146,138,74,146,84,146,210,74,41,42,73,64,4,0,64];
    #[test]
    fn halftone_annex_h_mmr_planes_and_arithmetic_parity(){
        let dict=pattern::decode(MMR_DICT,pattern::Limits::default()).unwrap();
        let d=decode(MMR_REGION,&dict,Limits::default()).unwrap();
        let a=decode(REGION,&dict,Limits::default()).unwrap();
        assert_eq!(d.bitmap.packed(),EXPECTED);assert_eq!(d.bitmap,a.bitmap);
        assert_eq!(d.indices,a.indices);assert_eq!(d.mmr_plane_bytes,[8,9,12,20]);
        assert_eq!((d.planes,d.decisions,d.skipped_cells,d.composed_pixels),(4,0,0,1152));
        assert_eq!(d.peak_packed_bytes,297);
        let exact=Limits{grid_cells:72,pixels:1152,packed_bytes:297,decisions:0,work:d.work_used};
        assert_eq!(decode(MMR_REGION,&dict,exact).unwrap(),d);
        for cap in [Limits{grid_cells:71,..exact},Limits{pixels:1151,..exact},
            Limits{packed_bytes:296,..exact},Limits{work:exact.work-1,..exact}]{
            assert!(decode(MMR_REGION,&dict,cap).is_err());
        }
        for end in 0..MMR_REGION.len(){assert!(decode(&MMR_REGION[..end],&dict,exact).is_err(),"prefix {end}");}
        let mut extra=MMR_REGION.to_vec();extra.push(0);
        assert!(decode(&extra,&dict,Limits::default()).is_err());
        let mut bad=MMR_REGION.to_vec();bad[45]=0; // first EOFB terminal bit
        assert!(decode(&bad,&dict,Limits::default()).is_err());
        let mut padding=MMR_REGION.to_vec();padding[45]|=127;
        assert_eq!(decode(&padding,&dict,exact).unwrap(),d);
    }
    #[test]
    fn halftone_mmr_raw_planes_exact_boundaries(){
        use crate::filters::ccitt::decode_mmr_plane_prefix;
        let body=&MMR_REGION[38..];let mut offset=0;
        for (j,size) in [8usize,9,12,20].into_iter().enumerate(){
            let p=decode_mmr_plane_prefix(&body[offset..],8,9,10000).unwrap();
            assert_eq!(p.encoded_bits.div_ceil(8),size);
            assert!(p.eofb_present);
            // Independently calculate raw Gray-code planes of the H.11 grid.
            let expected:Vec<u8>=(0..9u8).map(|m|(0..8u8).fold(0,|row,n|{
                let v=m+n;row|(((v^(v>>1))>>(3-j))&1)<<(7-n)
            })).collect();
            assert_eq!(p.packed,expected);
            let cost=144+size as u64*8+32*p.mode_counts.iter().sum::<u64>();
            assert_eq!(decode_mmr_plane_prefix(&body[offset..],8,9,cost).unwrap(),p);
            assert!(decode_mmr_plane_prefix(&body[offset..],8,9,cost-1).is_err());
            assert!(decode_mmr_plane_prefix(&body[offset..offset+size-1],8,9,10000).is_err());
            offset+=size;
        }
        assert_eq!(offset,body.len());
    }
    #[test]
    fn halftone_mmr_zero_planes_and_wrong_mode(){
        let one=pattern::decode(&[1,1,1,0,0,0,0,128],pattern::Limits::default()).unwrap();
        let mut data=MMR_REGION[..38].to_vec();
        let d=decode(&data,&one,Limits{decisions:0,..Limits::default()}).unwrap();
        assert_eq!(d.planes,0);assert!(d.mmr_plane_bytes.is_empty());
        assert_eq!(d.bitmap.packed(),[0;144]);
        data.push(0);assert!(decode(&data,&one,Limits::default()).is_err());
        assert!(decode_mmr(REGION,&one,Limits::default()).is_err());
        assert!(decode_arithmetic(MMR_REGION,&one,Limits::default()).is_err());
    }

    fn graph_seg(number:u8,kind:u8,page:u8,refs:&[u8],data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,number,kind,(refs.len() as u8)<<5];
        v.extend(refs);v.push(page);v.extend((data.len() as u32).to_be_bytes());v.extend(data);v
    }
    fn graph_info(number:u8)->Vec<u8>{
        let mut info=Vec::new();for n in [64u32,56,300,300]{info.extend(n.to_be_bytes());}
        info.extend([0,0,0]);graph_seg(number,48,1,&[],&info)
    }
    fn mixed_graph()->(Vec<u8>,Vec<u8>){
        // Synthetic graph wrapper, original Annex H entropy payloads.
        let globals=graph_seg(1,16,0,&[],DICT);
        let mut page=graph_info(2);
        page.extend(graph_seg(3,20,1,&[1],REGION));
        page.extend(graph_seg(4,22,1,&[1],MMR_REGION));
        page.extend(graph_seg(5,16,1,&[],MMR_DICT));
        page.extend(graph_seg(6,23,1,&[5],REGION));
        (globals,page)
    }
    #[test]
    fn halftone_graph_shared_global_and_local_dictionaries(){
        let (g,p)=mixed_graph();let l=ExecutionLimits::default();
        let d=execute_pdf_halftones(&g,&p,l).unwrap();
        assert_eq!(d.regions.iter().map(|r|(r.number,r.kind,r.dictionary_number)).collect::<Vec<_>>(),
            [(3,20,1),(4,22,1),(6,23,5)]);
        for r in &d.regions{assert_eq!(r.decoded.bitmap.packed(),EXPECTED);}
        assert_eq!(d.dictionary(1).unwrap().patterns(),d.dictionary(5).unwrap().patterns());
        assert!(d.dictionary(2).is_none());
        let a=pattern::decode(DICT,pattern::Limits::default()).unwrap();
        let b=pattern::decode(MMR_DICT,pattern::Limits::default()).unwrap();
        assert_eq!(d.regions[0].decoded,decode(REGION,&a,Limits::default()).unwrap());
        assert_eq!(d.regions[1].decoded,decode(MMR_REGION,&a,Limits::default()).unwrap());
        assert_eq!(d.regions[2].decoded,decode(REGION,&b,Limits::default()).unwrap());
        let s=d.stats;
        assert_eq!((s.dictionaries,s.regions,s.patterns,s.pattern_pixels,s.region_pixels,s.grid_cells,s.decisions),
            (2,3,32,512,3456,216,832));
        assert_eq!(s.packed_bytes,128+3*288);assert_eq!(s.peak_packed_bytes,s.packed_bytes+9);
        assert_eq!(s.work_used,(g.len()+p.len()) as u64+6*2+3+a.work_used+b.work_used+
            d.regions.iter().map(|r|r.decoded.work_used).sum::<u64>());
        let exact=ExecutionLimits{dictionaries:2,regions:3,patterns:32,pattern_pixels:512,
            region_pixels:3456,grid_cells:216,decisions:832,packed_bytes:s.peak_packed_bytes,work:s.work_used};
        assert_eq!(execute_pdf_halftones(&g,&p,exact).unwrap(),d);
        for cap in [ExecutionLimits{dictionaries:1,..exact},ExecutionLimits{regions:2,..exact},
            ExecutionLimits{patterns:31,..exact},ExecutionLimits{pattern_pixels:511,..exact},
            ExecutionLimits{region_pixels:3455,..exact},ExecutionLimits{grid_cells:215,..exact},
            ExecutionLimits{decisions:831,..exact},ExecutionLimits{packed_bytes:s.peak_packed_bytes-1,..exact},
            ExecutionLimits{work:s.work_used-1,..exact}]{
            assert!(execute_pdf_halftones(&g,&p,cap).is_err());
        }
    }
    #[test]
    fn halftone_graph_invalid_dependencies_and_atomic_failure(){
        let g=graph_seg(1,16,0,&[],DICT);let l=ExecutionLimits::default();
        for refs in [&[][..],&[1,1][..],&[0][..]]{
            let mut p=graph_info(2);p.extend(graph_seg(3,23,1,refs,REGION));
            assert!(execute_pdf_halftones(&g,&p,l).is_err());
        }
        let wrong=graph_seg(1,53,0,&[],&[]);
        let mut p=graph_info(2);p.extend(graph_seg(3,23,1,&[1],REGION));
        assert!(execute_pdf_halftones(&wrong,&p,l).is_err());
        let mut p=graph_info(2);p.extend(graph_seg(3,16,1,&[1],DICT));
        assert!(execute_pdf_halftones(&g,&p,l).is_err());
        let mut p=graph_info(2);p.extend(graph_seg(3,23,1,&[1],REGION));
        p.extend(graph_seg(4,23,1,&[1],&MMR_REGION[..MMR_REGION.len()-1]));
        assert!(execute_pdf_halftones(&g,&p,l).is_err());
        let mut p=graph_info(2);p.extend(graph_seg(3,23,1,&[4],REGION));
        p.extend(graph_seg(4,16,1,&[],DICT));
        assert!(execute_pdf_halftones(&g,&p,l).is_err());
        let mut p=graph_info(2);p.extend(graph_seg(3,16,2,&[],DICT));
        assert!(execute_pdf_halftones(&g,&p,l).is_err());
    }
    #[test]
    fn halftone_graph_empty_and_unused_pattern(){
        let p=graph_info(2);let l=ExecutionLimits::default();
        let empty=execute_pdf_halftones(&[],&p,l).unwrap();
        assert_eq!((empty.stats.dictionaries,empty.stats.regions,empty.stats.packed_bytes),(0,0,0));
        let g=graph_seg(1,16,0,&[],DICT);
        let d=execute_pdf_halftones(&g,&p,l).unwrap();
        assert_eq!((d.stats.dictionaries,d.stats.regions,d.stats.patterns,d.stats.decisions),(1,0,16,256));
        assert_eq!((d.stats.packed_bytes,d.stats.peak_packed_bytes),(64,96));
        let bad=graph_seg(1,16,0,&[],&DICT[..DICT.len()-1]);
        assert!(execute_pdf_halftones(&bad,&p,l).is_err());
    }
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
