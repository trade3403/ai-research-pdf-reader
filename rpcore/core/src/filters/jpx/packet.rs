//! Native packet header and codeblock contribution spans for a bounded
//! baseline subset: default codeblock style, one tile-part per tile,
//! inline headers. Entropy bytes remain opaque.
use super::{codestream::inspect_codestream,geometry::{self,Band,Packet,Precinct},packet_bits::{HeaderBits,TagTree}};
use crate::{Error,Result};
use std::{collections::BTreeMap,ops::Range};
const MAX_VISITS:usize=2_000_000;
const MAX_CONTRIBUTIONS:usize=262_144;
const MAX_PASSES:u16=109;

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Contribution{
    pub band:Band,pub block:usize,pub zero_bit_planes:u8,
    pub first_pass:u16,pub passes:u16,pub lblock:u8,
    /// Absolute range into original input, including any JP2 wrapper.
    pub data:Range<usize>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PacketInfo{
    pub packet:Packet,pub present:bool,pub header:Range<usize>,
    pub body:Range<usize>,pub contributions:Vec<Contribution>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TilePackets{pub tile:u16,pub packets:Vec<PacketInfo>}
#[derive(Clone,Copy)]
struct Block{passes:u16,lblock:u8,zero:u8}
struct BandState{band:Band,width:usize,inc:TagTree,zero:TagTree,blocks:Vec<Block>}
struct PrecinctState{bands:Vec<BandState>,next_layer:u16}
impl PrecinctState{
    fn new(p:&Precinct)->Result<Self>{
        let mut bands=Vec::new();
        for b in &p.bands{
            if b.blocks.is_empty(){continue;}
            // Geometry returns the rectangular raster grid of clipped blocks.
            let width=b.blocks.iter().take_while(|q|q.y0==b.blocks[0].y0).count();
            if width==0||b.blocks.len()%width!=0{return Err(Error::Invalid("JPX block grid"));}
            let height=b.blocks.len()/width;
            bands.push(BandState{band:b.band,width,inc:TagTree::new(width,height)?,
                zero:TagTree::new(width,height)?,blocks:vec![Block{passes:0,lblock:3,zero:0};b.blocks.len()]});
        }
        Ok(Self{bands,next_layer:0})
    }
}
fn pass_count(r:&mut HeaderBits<'_>)->Result<u16>{
    if r.bit()?==0{return Ok(1);}
    if r.bit()?==0{return Ok(2);}
    let n=r.bits(2)? as u16;
    if n!=3{return Ok(n+3);}
    let n=r.bits(5)? as u16;
    if n!=31{return Ok(n+6);}
    Ok(r.bits(7)? as u16+37)
}
fn read_length(r:&mut HeaderBits<'_>,b:&mut Block,n:u16)->Result<usize>{
    while r.bit()?!=0{
        b.lblock=b.lblock.checked_add(1).ok_or(Error::Limit("JPX Lblock"))?;
        if b.lblock>32{return Err(Error::Limit("JPX Lblock"));}
    }
    let extra=(u16::BITS-1-n.leading_zeros()) as u8;
    let bits=b.lblock.checked_add(extra).ok_or(Error::Limit("JPX length bits"))?;
    if bits>32{return Err(Error::Limit("JPX length bits"));}
    usize::try_from(r.bits(bits)?).map_err(|_|Error::Limit("JPX contribution length"))
}
fn read_packet(data:&[u8],range:&Range<usize>,pos:&mut usize,packet:Packet,seq:u16,sop:bool,eph:bool,
    state:&mut PrecinctState,visits:&mut usize,remaining:&mut usize)->Result<PacketInfo>{
    if packet.layer!=state.next_layer{return Err(Error::Invalid("JPX packet layer order"));}
    let start=*pos;
    let mut p=start;
    if sop{
        let a=data.get(p..p+6).filter(|_|p+6<=range.end).ok_or(Error::Invalid("JPX truncated SOP"))?;
        if a[..4]!=[0xff,0x91,0,4]||u16::from_be_bytes([a[4],a[5]])!=seq{return Err(Error::Invalid("JPX SOP marker/sequence"));}
        p+=6;
    }
    let mut r=HeaderBits::new(data.get(p..range.end).ok_or(Error::Invalid("JPX packet bounds"))?)?;
    let present=r.bit()?!=0;
    let mut entries=Vec::new();let mut body_len=0usize;
    if present{
        for band in &mut state.bands{
            *visits=visits.checked_sub(band.blocks.len()).ok_or(Error::Limit("JPX packet block visits"))?;
            for (i,b) in band.blocks.iter_mut().enumerate(){
                let x=i%band.width;let y=i/band.width;
                let included=if b.passes==0{
                    band.inc.below(x,y,packet.layer+1,&mut r)?
                }else{r.bit()?!=0};
                if !included{continue;}
                if b.passes==0{
                    let mut found=None;
                    for threshold in 1..=64{
                        if band.zero.below(x,y,threshold,&mut r)?{found=Some((threshold-1)as u8);break;}
                    }
                    b.zero=found.ok_or(Error::Limit("JPX zero bit planes"))?;
                }
                let n=pass_count(&mut r)?;
                let end_pass=b.passes.checked_add(n).ok_or(Error::Limit("JPX coding passes"))?;
                // Default coding style uses one segment up to 109 passes.
                // Do not guess lengths for a second or alternate segment.
                if end_pass>MAX_PASSES{return Err(Error::Unsupported("JPX multiple coding segments"));}
                let length=read_length(&mut r,b,n)?;
                *remaining=remaining.checked_sub(1).ok_or(Error::Limit("JPX contributions"))?;
                let end=body_len.checked_add(length).ok_or(Error::Limit("JPX packet body length"))?;
                if end>range.end-p{return Err(Error::Invalid("JPX contribution bounds"));}
                entries.push(Contribution{band:band.band,block:i,zero_bit_planes:b.zero,
                    first_pass:b.passes,passes:n,lblock:b.lblock,data:body_len..end});
                body_len=end;b.passes=end_pass;
            }
        }
    }
    p+=r.align()?;
    if eph{
        if p+2>range.end||data.get(p..p+2)!=Some(&[0xff,0x92][..]){return Err(Error::Invalid("JPX EPH marker"));}
        p+=2;
    }
    let end=p.checked_add(body_len).ok_or(Error::Limit("JPX packet end"))?;
    if end>range.end{return Err(Error::Invalid("JPX truncated packet body"));}
    for e in &mut entries{e.data.start+=p;e.data.end+=p;}
    state.next_layer+=1;
    *pos=end;
    Ok(PacketInfo{packet,present,header:start..p,body:p..end,contributions:entries})
}
/// Inspect one tile without decoding entropy bytes. All state is private to
/// this call; failures never return partial contribution ranges.
pub fn inspect_packets(data:&[u8],tile:u16)->Result<TilePackets>{
    let c=inspect_codestream(data)?;
    if c.coding.layers>256{return Err(Error::Unsupported("JPX packet layer limit"));}
    if c.component_coding.iter().any(|s|s.codeblock_style!=0){return Err(Error::Unsupported("JPX packet codeblock style"));}
    let mut parts=c.tile_parts.iter().filter(|p|p.tile==tile);
    let part=parts.next().ok_or(Error::Invalid("JPX missing tile part"))?;
    if parts.next().is_some(){return Err(Error::Unsupported("JPX multiple tile parts"));}
    let plan=geometry::plan(&c,tile)?;
    let mut states=BTreeMap::new();let mut pos=part.payload.start;
    let mut visits=MAX_VISITS;let mut remaining=MAX_CONTRIBUTIONS;
    let mut packets=Vec::with_capacity(plan.packets.len());
    for (seq,&p) in plan.packets.iter().enumerate(){
        let key=(p.component,p.resolution,p.precinct);
        let precinct=&plan.components[usize::from(p.component)].resolutions[usize::from(p.resolution)].precincts[p.precinct];
        let state=match states.entry(key){
            std::collections::btree_map::Entry::Occupied(e)=>e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e)=>e.insert(PrecinctState::new(precinct)?),
        };
        packets.push(read_packet(data,&part.payload,&mut pos,p,seq as u16,c.coding.sop,c.coding.eph,state,&mut visits,&mut remaining)?);
    }
    if pos!=part.payload.end{return Err(Error::Invalid("JPX unconsumed tile payload"));}
    Ok(TilePackets{tile,packets})
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    // Independently generated in Pillow/OpenJPEG 2.5.4: 4x4 gray,
    // 8x8 RGB with two quality layers, 7x5 gray with four 4x4 tiles.
    const GRAY:&str="ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000004040001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000240001ff93df8098121a395f0e4ceb90f5b7a313d57e35e0075a4fffd9";
    const RGB:&str="ff4fff51002f000000000008000000080000000000000000000000080000000800000000000000000003070101070101070101ff52000c00000002000104040001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000009f0001ff93dc20120f3f5ddc201219dbfade1412191846278080a7c0300adcfc71d03efda1c01c8b93fa772afeb24b5d0693a75ffda1a055eaa33b16a2b72c5ea45ec51ffd6180d1e8a967a8dffea07fa2779fc03a1c1f383022187f037e97c7da133f00c87da0801dd461bffaef9728ef0703cfd35a8267a1e09ba4bf1b844e07c3ea05fa8167e60622c31d4f2678567fb23f231947ffd9";
    const TILED:&str="ff4fff510029000000000007000000050000000000000000000000040000000400000000000000000001070101ff52000c00000001000104040001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000230001ff93cfb414078ac18de8c1f3839f80200d01ef01d8593fff90000a0001000000280001ff93c7d40a0ac45b557fc3ea029f802c7e00400b8701fba9b8370b8bff90000a0002000000190001ff93cfb40c082f57c1f3820b97ff90000a0003000000140001ff93c7d404071180ffd9";
    #[test]fn independent_single_block(){
        let b=hex(GRAY);let p=inspect_packets(&b,0).unwrap();
        assert_eq!(p.packets.len(),1);
        let p=&p.packets[0];
        assert_eq!(p.header,118..121);assert_eq!(p.body,121..140);
        assert_eq!(p.contributions,vec![Contribution{band:Band::Ll,block:0,zero_bit_planes:1,
            first_pass:0,passes:22,lblock:3,data:121..140}]);
    }
    fn coverage(data:&[u8],tile:u16,count:usize){
        let c=inspect_codestream(data).unwrap();
        let part=c.tile_parts.iter().find(|p|p.tile==tile).unwrap();
        let p=inspect_packets(data,tile).unwrap();
        assert_eq!(p.packets.len(),count);
        let mut pos=part.payload.start;
        let mut passes=BTreeMap::new();
        for info in &p.packets{
            assert_eq!(info.header.start,pos);assert_eq!(info.header.end,info.body.start);
            let mut body=info.body.start;
            for e in &info.contributions{
                assert_eq!(e.data.start,body);body=e.data.end;
                let band=match e.band{Band::Ll=>0,Band::Hl=>1,Band::Lh=>2,Band::Hh=>3};
                let total=passes.entry((info.packet.component,info.packet.resolution,info.packet.precinct,band,e.block)).or_insert(0);
                assert_eq!(e.first_pass,*total);*total+=e.passes;
            }
            assert_eq!(body,info.body.end);pos=body;
        }
        assert_eq!(pos,part.payload.end);
    }
    #[test]fn independent_two_layer_rgb(){let b=hex(RGB); assert_eq!(&b[40..42], &[0,3]); assert_eq!(&b[51..53], &[0xff,0x52]); coverage(&b,0,12);}
    #[test]fn independent_edge_tiles(){let b=hex(TILED);for t in 0..4{coverage(&b,t,2);}}
    #[test]fn pass_count_codes(){
        // Pack independent prefix codes for all 164 pass counts.
        for n in 1u16..=164{
            let (v,len)=match n{1=>(0u32,1),2=>(2,2),3..=5=>(12+u32::from(n-3),4),6..=36=>(480+u32::from(n-6),9),_=>(65408+u32::from(n-37),16)};
            // Stuff following byte if first byte FF.
            let mut bytes=Vec::new();let mut left=len;
            let mut cap=8;
            while left>0{
                let take=left.min(cap);let x=((v>>(left-take))&((1<<take)-1))as u8;
                let b=x<<(cap-take);bytes.push(b);left-=take;cap=if b==255{7}else{8};
            }
            let mut r=HeaderBits::new(&bytes).unwrap();assert_eq!(pass_count(&mut r).unwrap(),n);
        }
    }
    #[test]fn sop_eph_and_invalid_sequence(){
        let mut b=hex(GRAY);b[49]=6; // Scod flags
        b[110..114].copy_from_slice(&44u32.to_be_bytes());
        b.splice(121..121,[0xff,0x92]);b.splice(118..118,[0xff,0x91,0,4,0,0]);
        let p=inspect_packets(&b,0).unwrap();assert_eq!(p.packets[0].header,118..129);
        assert_eq!(p.packets[0].body,129..148);
        b[123]=1;assert!(inspect_packets(&b,0).is_err());
    }
    #[test]fn truncated_or_unconsumed_body(){
        let b=hex(GRAY);
        let mut short=b.clone();short.remove(139);short[110..114].copy_from_slice(&35u32.to_be_bytes());
        assert!(inspect_packets(&short,0).is_err());
        let mut long=b.clone();long.insert(140,0);long[110..114].copy_from_slice(&37u32.to_be_bytes());
        assert!(inspect_packets(&long,0).is_err());
        let mut bad=b.clone();bad[118]=0;assert!(inspect_packets(&bad,0).is_err());
    }
    #[test]fn empty_packet_and_unsupported_modes(){
        let mut b=hex(GRAY);b.splice(118..140,[0]);b[110..114].copy_from_slice(&15u32.to_be_bytes());
        let p=inspect_packets(&b,0).unwrap();assert!(!p.packets[0].present);assert!(p.packets[0].body.is_empty());
        let mut b=hex(GRAY);b[57]=1;assert!(matches!(inspect_packets(&b,0),Err(Error::Unsupported("JPX packet codeblock style"))));
        let mut b=hex(GRAY);b[51..53].copy_from_slice(&257u16.to_be_bytes());assert!(inspect_packets(&b,0).is_err());
    }
}
