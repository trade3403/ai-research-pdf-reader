//! Scoped native reversible JPEG 2000 grayscale and RGB pixel reconstruction.
//! Multiple tiles and quality layers, unsigned 8-bit, optional reversible MCT, baseline MQ style.
//! This does not activate the PDF JPXDecode rendering path.
use super::{Container,codestream::{inspect_codestream,Codestream,QuantMode,Wavelet},geometry::{self,Band,Rect},packet,ebcot,wavelet53::{self,Detail53}};
use crate::{Error,Result};
use std::collections::BTreeMap;

const MAX_PIXELS:usize=4_194_304;
const MAX_WORK:usize=MAX_PIXELS*128;
#[derive(Debug,PartialEq,Eq)]
pub struct GrayImage{pub width:usize,pub height:usize,pub pixels:Vec<u8>}
#[derive(Debug,PartialEq,Eq)]
pub struct RgbImage{pub width:usize,pub height:usize,pub pixels:Vec<u8>}
struct Plane{bounds:Rect,values:Vec<i32>}
struct Segment{band:Band,zero:u8,passes:u16,bytes:Vec<u8>}
fn band_index(b:Band)->usize{match b{Band::Ll=>0,Band::Hl=>1,Band::Lh=>2,Band::Hh=>3}}

/// Reconstruct baseline grayscale raw codestreams. Quantization is
/// reversible (unit steps). Missing contributions represent zero blocks.
/// Even all quality layers are not proof of lossless encoding; this API
/// returns the represented samples, including zero-filled omitted bits.
pub fn decode_gray53(data:&[u8])->Result<GrayImage>{
    decode_layers(data,None)
}
/// Decode the first `layers` quality layers, validating packet structure for
/// the full stream. Omitted coding passes are zero-filled, not interpolated.
pub fn decode_gray53_layers(data:&[u8],layers:u16)->Result<GrayImage>{
    decode_layers(data,Some(layers))
}
fn decode_layers(data:&[u8],requested:Option<u16>)->Result<GrayImage>{
    let (width,height,pixels)=decode_components(data,requested,1)?;
    Ok(GrayImage{width,height,pixels})
}
/// Decode three co-sited unsigned 8-bit components as interleaved RGB.
/// Raw codestream component order is assumed RGB when MCT is absent.
pub fn decode_rgb53(data:&[u8])->Result<RgbImage>{
    let (width,height,pixels)=decode_components(data,None,3)?;
    Ok(RgbImage{width,height,pixels})
}
fn sample8(v:i64)->u8{(v+128).clamp(0,255)as u8}
/// Inverse reversible component transform, before unsigned level shift.
fn inverse_rct(y:i32,cb:i32,cr:i32)->[i64;3]{
    let (y,cb,cr)=(i64::from(y),i64::from(cb),i64::from(cr));
    let g=y-(cb+cr).div_euclid(4);
    [g+cr,g,g+cb]
}
fn decode_components(data:&[u8],requested:Option<u16>,channels:usize)->Result<(usize,usize,Vec<u8>)>{
    let c=inspect_codestream(data)?;
    let h=&c.geometry;
    let layers=requested.unwrap_or(c.coding.layers);
    if layers==0||layers>c.coding.layers{return Err(Error::Invalid("JPX gray53 quality layers"));}
    if c.coding.layers>256{return Err(Error::Unsupported("JPX gray53 layer limit"));}
    if h.container!=Container::RawCodestream||h.components.len()!=channels||
        channels==1&&c.coding.mct{
        return Err(Error::Unsupported("JPX gray53 container/components/MCT"));
    }
    for ((g,s),q) in h.components.iter().zip(&c.component_coding).zip(&c.component_quantization){
    if g.precision!=8||g.signed||g.x_sampling!=1||g.y_sampling!=1||
        s.wavelet!=Wavelet::Reversible53||s.codeblock_style!=0||q.mode!=QuantMode::None{
        return Err(Error::Unsupported("JPX gray53 coding parameters"));
    }
    }
    let (w,ht)=(h.width as usize,h.height as usize);
    let area=w.checked_mul(ht).ok_or(Error::Limit("JPX gray53 pixels"))?;
    if area==0||area>MAX_PIXELS||w>1_048_576||ht>1_048_576{return Err(Error::Limit("JPX gray53 pixels"));}
    let mut pixels=vec![0;area.checked_mul(channels).ok_or(Error::Limit("JPX output samples"))?];
    let mut work=0usize;
    let mut geometry_work=0usize;
    let mut covered=0usize;
    let tile_count=u64::from(h.tiles_x)*u64::from(h.tiles_y);
    if tile_count>4096{return Err(Error::Limit("JPX gray53 tile count"));}
    for i in 0..tile_count{
        let plan=geometry::plan(&c,i as u16)?;
        geometry_work=geometry_work.checked_add(plan.codeblock_count)
            .and_then(|n|n.checked_add(plan.packets.len())).ok_or(Error::Limit("JPX gray53 geometry work"))?;
        if geometry_work>2_000_000{return Err(Error::Limit("JPX gray53 geometry work"));}
        let b=plan.components[0].bounds;
        if b.x0<i64::from(h.x_origin)||b.y0<i64::from(h.y_origin)||
            b.x1>i64::from(h.x_origin)+w as i64||b.y1>i64::from(h.y_origin)+ht as i64{
            return Err(Error::Invalid("JPX gray53 tile placement"));
        }
        let tw=b.width()as usize;let th=b.height()as usize;
        let n=tw.checked_mul(th).ok_or(Error::Limit("JPX gray53 tile area"))?;
        covered=covered.checked_add(n).ok_or(Error::Limit("JPX gray53 coverage"))?;
        if n==0||covered>area{return Err(Error::Invalid("JPX gray53 tile coverage"));}
        let packets=packet::inspect_planned_packets(data,&c,&plan)?;
        let mut samples=Vec::with_capacity(channels);
        for component in 0..channels{
            if plan.components[component].bounds!=b{return Err(Error::Unsupported("JPX component grid mismatch"));}
            samples.push(decode_tile(data,&c,&plan,&packets,&mut work,layers,component)?);
        }
        let ox=(b.x0-i64::from(h.x_origin))as usize;
        let oy=(b.y0-i64::from(h.y_origin))as usize;
        for y in 0..th{
            for x in 0..tw{
                let src=y*tw+x;
                let dst=((oy+y)*w+ox+x)*channels;
                if channels==1{pixels[dst]=sample8(i64::from(samples[0][src]));}
                else{
                    let rgb=if c.coding.mct{inverse_rct(samples[0][src],samples[1][src],samples[2][src])}
                        else{[i64::from(samples[0][src]),i64::from(samples[1][src]),i64::from(samples[2][src])]};
                    for k in 0..3{pixels[dst+k]=sample8(rgb[k]);}
                }
            }
        }
    }
    if covered!=area{return Err(Error::Invalid("JPX gray53 tile coverage"));}
    Ok((w,ht,pixels))
}

fn decode_tile(data:&[u8],c:&Codestream,plan:&geometry::TilePlan,packets:&packet::TilePackets,work:&mut usize,layers:u16,component:usize)->Result<Vec<i32>>{
    let t=&plan.components[component];let q=&c.component_quantization[component];
    let (w,ht)=(t.bounds.width()as usize,t.bounds.height()as usize);
    let area=w.checked_mul(ht).ok_or(Error::Limit("JPX gray53 tile area"))?;
    let mut planes:Vec<Vec<Plane>>=Vec::new();
    let mut total=0usize;
    for res in &t.resolutions{
        let mut bands=Vec::new();
        for b in &res.subbands{
            let n=(b.bounds.width() as usize).checked_mul(b.bounds.height() as usize).ok_or(Error::Limit("JPX gray53 band area"))?;
            total=total.checked_add(n).ok_or(Error::Limit("JPX gray53 band area"))?;
            if total>area{return Err(Error::Invalid("JPX gray53 band area"));}
            bands.push(Plane{bounds:b.bounds,values:vec![0;n]});
        }
        planes.push(bands);
    }
    if total!=area{return Err(Error::Invalid("JPX gray53 band partition"));}
    // Default coding style has one continuous MQ segment per codeblock
    // (<=109 passes). Layer boundaries do not reset arithmetic or context state.
    let mut segments:BTreeMap<(usize,usize,usize,usize),Segment>=BTreeMap::new();
    for p in &packets.packets{
        if p.packet.layer>=layers||usize::from(p.packet.component)!=component{continue;}
        for e in &p.contributions{
            let key=(usize::from(p.packet.resolution),p.packet.precinct,band_index(e.band),e.block);
            let seg=segments.entry(key).or_insert_with(||Segment{band:e.band,zero:e.zero_bit_planes,passes:0,bytes:Vec::new()});
            if seg.passes!=e.first_pass||seg.zero!=e.zero_bit_planes||seg.band!=e.band{
                return Err(Error::Invalid("JPX gray53 contribution sequence"));
            }
            seg.passes=seg.passes.checked_add(e.passes).ok_or(Error::Limit("JPX gray53 passes"))?;
            if seg.passes>109{return Err(Error::Unsupported("JPX gray53 multiple segments"));}
            seg.bytes.extend_from_slice(&data[e.data.clone()]);
        }
    }
    for ((ri,pi,_,block_index),seg) in segments{
            let precinct=&t.resolutions[ri].precincts[pi];
            let band=precinct.bands.iter().find(|b|b.band==seg.band).ok_or(Error::Invalid("JPX gray53 band"))?;
            let block=*band.blocks.get(block_index).ok_or(Error::Invalid("JPX gray53 block"))?;
            let bi=band_index(seg.band);
            let qi=if ri==0{0}else{3*(ri-1)+bi};
            let step=q.steps.get(qi).ok_or(Error::Invalid("JPX gray53 quantization step"))?;
            let plane=i32::from(q.guard_bits)+i32::from(step.exponent)-2-i32::from(seg.zero);
            if !(0..=30).contains(&plane){return Err(Error::Invalid("JPX gray53 highest bitplane"));}
            let (bw,bh)=(block.width() as usize,block.height() as usize);
            let charge=bw.checked_mul(bh).and_then(|n|n.checked_mul(usize::from(seg.passes))).ok_or(Error::Limit("JPX gray53 work"))?;
            *work=work.checked_add(charge).ok_or(Error::Limit("JPX gray53 work"))?;
            if *work>MAX_WORK{return Err(Error::Limit("JPX gray53 work"));}
            let coeff=ebcot::decode_codeblock(&seg.bytes,bw,bh,seg.band,plane as u8,seg.passes,0)?.signed_coefficients();
            let target=&mut planes[ri][if ri==0{0}else{bi-1}];
            let b=target.bounds;
            if block.x0<b.x0||block.y0<b.y0||block.x1>b.x1||block.y1>b.y1{
                return Err(Error::Invalid("JPX gray53 block placement"));
            }
            let stride=b.width() as usize;
            let ox=(block.x0-b.x0)as usize;let oy=(block.y0-b.y0)as usize;
            for y in 0..bh{
                let start=(oy+y)*stride+ox;
                target.values[start..start+bw].copy_from_slice(&coeff[y*bw..(y+1)*bw]);
            }
    }
    let details:Vec<_>=planes.iter().skip(1).map(|r|Detail53{hl:&r[0].values,lh:&r[1].values,hh:&r[2].values}).collect();
    wavelet53::inverse53_levels(w,ht,t.bounds.x0 as u32,t.bounds.y0 as u32,&planes[0][0].values,&details)
}

/// Experimental irreversible grayscale component reconstruction.
/// Samples are row-major, centered, unrounded and unclipped; this is not a
/// PDF image rendering entry point. No color transform or JP2 interpretation.
#[derive(Debug,PartialEq)]
pub struct Gray97Samples{pub width:usize,pub height:usize,pub samples:Vec<f64>}

pub fn decode_gray97_centered(data:&[u8])->Result<Gray97Samples>{
    let c=inspect_codestream(data)?;
    let h=&c.geometry;
    if h.container!=Container::RawCodestream||h.components.len()!=1||c.coding.mct||
        c.coding.layers>256{
        return Err(Error::Unsupported("JPX gray97 container/components/tiles/layers"));
    }
    let g=&h.components[0];let style=&c.component_coding[0];let q=&c.component_quantization[0];
    if g.precision!=8||g.signed||g.x_sampling!=1||g.y_sampling!=1||
        style.wavelet!=Wavelet::Irreversible97||style.codeblock_style!=0||q.mode==QuantMode::None{
        return Err(Error::Unsupported("JPX gray97 coding parameters"));
    }
    let (w,ht)=(h.width as usize,h.height as usize);
    let area=w.checked_mul(ht).ok_or(Error::Limit("JPX gray97 area"))?;
    if area==0||area>MAX_PIXELS||w>1_048_576||ht>1_048_576{
        return Err(Error::Limit("JPX gray97 area"));
    }
    let tiles=u64::from(h.tiles_x)*u64::from(h.tiles_y);
    if tiles>4096{return Err(Error::Limit("JPX gray97 tile count"));}
    if c.tile_parts.len()!=tiles as usize||c.tile_parts.iter().any(|p|p.index!=0||p.declared_parts>1){
        return Err(Error::Unsupported("JPX gray97 multiple tile parts"));
    }
    let mut samples=vec![0.;area];
    let mut covered=0usize;
    let mut work=0usize;
    let mut geometry_work=0usize;
    let mut segment_bytes=0usize;
    for i in 0..tiles{
        let plan=geometry::plan(&c,i as u16)?;
        geometry_work=geometry_work.checked_add(plan.codeblock_count)
            .and_then(|n|n.checked_add(plan.packets.len())).ok_or(Error::Limit("JPX gray97 geometry work"))?;
        if geometry_work>2_000_000{return Err(Error::Limit("JPX gray97 geometry work"));}
        let b=plan.components[0].bounds;
        if b.x0<i64::from(h.x_origin)||b.y0<i64::from(h.y_origin)||
            b.x1>i64::from(h.x_origin)+w as i64||b.y1>i64::from(h.y_origin)+ht as i64{
            return Err(Error::Invalid("JPX gray97 tile placement"));
        }
        let (tw,th)=(b.width()as usize,b.height()as usize);
        let n=tw.checked_mul(th).ok_or(Error::Limit("JPX gray97 tile area"))?;
        covered=covered.checked_add(n).ok_or(Error::Limit("JPX gray97 coverage"))?;
        if n==0||covered>area{return Err(Error::Invalid("JPX gray97 tile coverage"));}
        let packets=packet::inspect_planned_packets(data,&c,&plan)?;
        let tile=decode_tile97(data,&c,&plan,&packets,&mut work,&mut segment_bytes)?;
        if tile.len()!=n{return Err(Error::Invalid("JPX gray97 tile samples"));}
        let ox=(b.x0-i64::from(h.x_origin))as usize;
        let oy=(b.y0-i64::from(h.y_origin))as usize;
        for y in 0..th{
            let dst=(oy+y)*w+ox;
            samples[dst..dst+tw].copy_from_slice(&tile[y*tw..(y+1)*tw]);
        }
    }
    if covered!=area{return Err(Error::Invalid("JPX gray97 coverage"));}
    Ok(Gray97Samples{width:w,height:ht,samples})
}

fn decode_tile97(data:&[u8],c:&Codestream,plan:&geometry::TilePlan,packets:&packet::TilePackets,
    work:&mut usize,segment_bytes:&mut usize)->Result<Vec<f64>>{
    use super::{codestream::inverse_quantization as iq,wavelet97::{self,Detail97}};
    let t=&plan.components[0];
    let g=&c.geometry.components[0];
    let style=&c.component_coding[0];
    let q=&c.component_quantization[0];
    let (w,ht)=(t.bounds.width()as usize,t.bounds.height()as usize);
    let area=w.checked_mul(ht).ok_or(Error::Limit("JPX gray97 tile area"))?;
    struct FloatPlane{bounds:Rect,values:Vec<f64>,delta:f64}
    let mut planes:Vec<Vec<FloatPlane>>=Vec::new();
    let mut total=0usize;
    for (ri,res) in t.resolutions.iter().enumerate(){
        let mut bands=Vec::new();
        for b in &res.subbands{
            let n=(b.bounds.width()as usize).checked_mul(b.bounds.height()as usize)
                .ok_or(Error::Limit("JPX gray97 band area"))?;
            total=total.checked_add(n).ok_or(Error::Limit("JPX gray97 band area"))?;
            if total>area{return Err(Error::Invalid("JPX gray97 band partition"));}
            // Part 1 gain convention matches the native inverse97 lifting
            // normalization; independent pixel parity is a separate gate.
            let delta=iq::step_size(q,style.wavelet,g.precision,style.levels,ri as u8,b.band)?;
            bands.push(FloatPlane{bounds:b.bounds,values:vec![0.;n],delta});
        }
        planes.push(bands);
    }
    if total!=area{return Err(Error::Invalid("JPX gray97 band partition"));}
    // Same baseline continuous MQ segment contract as gray53. Contributions
    // across all quality layers are assembled before decoding each block.
    let mut segments:BTreeMap<(usize,usize,usize,usize),Segment>=BTreeMap::new();
    for p in &packets.packets{
        for e in &p.contributions{
            let key=(usize::from(p.packet.resolution),p.packet.precinct,band_index(e.band),e.block);
            let seg=segments.entry(key).or_insert_with(||Segment{band:e.band,zero:e.zero_bit_planes,passes:0,bytes:Vec::new()});
            if seg.passes!=e.first_pass||seg.zero!=e.zero_bit_planes||seg.band!=e.band{
                return Err(Error::Invalid("JPX gray97 contribution sequence"));
            }
            seg.passes=seg.passes.checked_add(e.passes).ok_or(Error::Limit("JPX gray97 passes"))?;
            if seg.passes>109{return Err(Error::Unsupported("JPX gray97 multiple segments"));}
            let bytes=data.get(e.data.clone()).ok_or(Error::Invalid("JPX gray97 contribution range"))?;
            *segment_bytes=segment_bytes.checked_add(bytes.len()).ok_or(Error::Limit("JPX gray97 segment bytes"))?;
            if *segment_bytes>crate::MAX_BYTES{return Err(Error::Limit("JPX gray97 segment bytes"));}
            seg.bytes.extend_from_slice(bytes);
        }
    }
    for ((ri,pi,bi,block_index),seg) in segments{
        let precinct=&t.resolutions[ri].precincts[pi];
        let band=precinct.bands.iter().find(|b|b.band==seg.band).ok_or(Error::Invalid("JPX gray97 band"))?;
        let block=*band.blocks.get(block_index).ok_or(Error::Invalid("JPX gray97 block"))?;
        let plane=iq::highest_bitplane(q,style.levels,ri as u8,seg.band,seg.zero)?;
        let (bw,bh)=(block.width()as usize,block.height()as usize);
        let charge=bw.checked_mul(bh).and_then(|n|n.checked_mul(usize::from(seg.passes)))
            .ok_or(Error::Limit("JPX gray97 work"))?;
        *work=work.checked_add(charge).ok_or(Error::Limit("JPX gray97 work"))?;
        if *work>MAX_WORK{return Err(Error::Limit("JPX gray97 work"));}
        let target=&mut planes[ri][if ri==0{0}else{bi-1}];
        let b=target.bounds;
        if block.x0<b.x0||block.y0<b.y0||block.x1>b.x1||block.y1>b.y1{
            return Err(Error::Invalid("JPX gray97 block placement"));
        }
        let values=ebcot::decode_codeblock(&seg.bytes,bw,bh,seg.band,plane,seg.passes,0)?
            .reconstruct97(target.delta)?;
        let stride=b.width()as usize;
        let ox=(block.x0-b.x0)as usize;let oy=(block.y0-b.y0)as usize;
        for y in 0..bh{
            let start=(oy+y)*stride+ox;
            target.values[start..start+bw].copy_from_slice(&values[y*bw..(y+1)*bw]);
        }
    }
    let details:Vec<_>=planes.iter().skip(1).map(|r|Detail97{hl:&r[0].values,lh:&r[1].values,hh:&r[2].values}).collect();
    wavelet97::inverse97_levels(w,ht,t.bounds.x0 as u32,t.bounds.y0 as u32,&planes[0][0].values,&details)
}

#[cfg(test)]
mod tests{
    use super::*;
    // Generated using Pillow/OpenJPEG 2.5.4 with no_jp2=True,
    // irreversible=False, codeblock_size=(4,4); reference decode roundtrip
    // checked against original input before embedding.
    const FLAT:&str="ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000001e0001ff93df806818aa09fcb2996738ca69cbe485ffd9";
    const ONE:&str="ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000002d0001ff93c3e7080b051374c7da0d0fa8123f00500c4d9e8d14ef0f3b351b0b55c6a1f2ffd9";
    const MULTI:&str="ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000c00000001000200000001ff5c000a4040484850484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000600001ff93c7d40e059ed913a7ba5fc3ea059f80447e00e00d930db53f04bd9d31b3784d2f0fa5ad3feeff7fc7da159fc033f80247e00c00275a16db54454395cb1284b8bae2fc961bafe4a4050c0bff7f0c3bba897267ffd9";
    const ZERO:&str="ff4fff510029000000000005000000050000000000000000000000050000000500000000000000000001070101ff52000c00000001000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000100001ff938080ffd9";
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    const PIXELS:[u8;16]=[128,129,127,130,125,140,116,128,255,0,128,129,130,131,132,133];
    #[test]fn gray53_flat(){let a=decode_gray53(&hex(FLAT)).unwrap();assert_eq!((a.width,a.height),(4,4));assert_eq!(a.pixels,PIXELS);}
    #[test]fn gray53_one_level(){let a=decode_gray53(&hex(ONE)).unwrap();assert_eq!((a.width,a.height),(4,4));assert_eq!(a.pixels,PIXELS);}
    #[test]fn gray53_multilevel_multiblock(){let a=decode_gray53(&hex(MULTI)).unwrap();assert_eq!((a.width,a.height),(9,7));assert_eq!(a.pixels,(0..63).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());}
    #[test]fn gray53_empty_contributions(){let a=decode_gray53(&hex(ZERO)).unwrap();assert_eq!((a.width,a.height),(5,5));assert_eq!(a.pixels,vec![128;25]);}
    #[test]fn gray53_reject_unsupported_and_truncated(){
        let mut a=hex(FLAT);a[53]=1;assert!(decode_gray53(&a).is_err());
        let mut a=hex(FLAT);a[52]=2;assert!(decode_gray53(&a).is_err());
        let mut a=hex(FLAT);a[42]=0x87;assert!(decode_gray53(&a).is_err());
        let a=hex(ONE);assert!(decode_gray53(&a[..a.len()-1]).is_err());
        assert!(decode_gray53(&[]).is_err());
    }

    // Independent Pillow/OpenJPEG 2.5.4 lossless fixtures; roundtrip checked.
    const EDGE_TILES:&str="ff4fff510029000000000007000000050000000000000000000000040000000400000000000000000001070101ff52000c00000001000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000210001ff93cfb4140917828437c3ea038074200d02110b37ff90000a00010000001b0001ff93cfb4140cd4db86c1a01d080b37ff90000a0002000000190001ff93cfb40c07c24fc3ea020b92ff90000a0003000000180001ff93cfb40c02b91fcfc00404ffd9";
    #[test]fn gray53_edge_tiles(){
        let a=decode_gray53(&hex(EDGE_TILES)).unwrap();
        assert_eq!((a.width,a.height),(7,5));
        assert_eq!(a.pixels,(0..35).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const ODD_TILES:&str="ff4fff510029000000000009000000070000000000000000000000050000000300000000000000000001070101ff52000c00000001000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000290001ff93cfb41c06dda479c0df97c7da073f00387da040108be70d83e40b8bff90000a0001000000300001ff93e5f501fed0300b9d6f0982afc7da0b8fda057d4030fb40800e2bf8446f097b040b8bff90000a00020000002a0001ff93c1f2030cc63fc3ea029f80447e0060093f146360091f4eb17f06b6bfff90000a00030000002f0001ff93e3f5015f3820057f097fc3ea03cfe007fc00e3f00309237f034c2705aea90e737fff90000a00040000001b0001ff93cfb4140bf6c71e1fcfc008079fff90000a00050000001e0001ff93e7ed02bea020031f087bcfc00c0b4d66ffd9";
    #[test]fn gray53_odd_tiles(){
        let a=decode_gray53(&hex(ODD_TILES)).unwrap();
        assert_eq!((a.width,a.height),(9,7));
        assert_eq!(a.pixels,(0..63).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const OFFSET_TILES:&str="ff4fff510029000000000008000000070000000100000001000000040000000400000000000000000001070101ff52000c00000001000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000001e0001ff93cfb408075fc3ea0380742009f68f099fff90000a00010000001e0001ff93cfb40c0b5ffac3ea028074200b9209a1ff90000a00020000001c0001ff93c7d4060845bfc3ea040008241be7ff90000a0003000000270001ff93cfb4140db0a9078fdf98243ea028fc008010874bd30c5f0c5fffd9";
    #[test]fn gray53_offset_tiles(){
        let a=decode_gray53(&hex(OFFSET_TILES)).unwrap();
        assert_eq!((a.width,a.height),(7,6));
        assert_eq!(a.pixels,(0..42).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const FLAT_TILES:&str="ff4fff510029000000000005000000030000000000000000000000020000000200000000000000000001070101ff52000c00000001000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000160001ff93cfb414099e0f4c4fff90000a0001000000170001ff93cfb4180c139a3cad7fff90000a0002000000140001ff93c7d40602dc5fff90000a0003000000140001ff93c7d4060b525fff90000a0004000000140001ff93cfb40c020af7ff90000a0005000000130001ff93cfb4080893ffd9";
    #[test]fn gray53_flat_tiles(){
        let a=decode_gray53(&hex(FLAT_TILES)).unwrap();
        assert_eq!((a.width,a.height),(5,3));
        assert_eq!(a.pixels,(0..15).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    #[test]fn gray53_incomplete_tile_rejected(){
        let mut a=hex(EDGE_TILES);
        let c=inspect_codestream(&a).unwrap();
        let last=c.tile_parts.last().unwrap().span.clone();
        a.drain(last);
        assert!(decode_gray53(&a).is_err());
    }

    // Pillow/OpenJPEG 2.5.4: reversible, 4x4 codeblocks, quality_mode=rates,
    // layers [8,1], [16,4,1], [8,1]; locally verified full roundtrip.
    const LAYERS_TWO:&str="ff4fff510029000000000010000000100000000000000000000000100000001000000000000000000001070101ff52000c00000002000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000001190001ff9380b3c3e4f0e00c95a7097833b10ac5610e0564fe40623008e7ed12fda25fb447f68880107413c0cfd858fbb88a26f9203343366a9f12917bec1f1551fe6f7ea896e2f88a32095f0ae43c39ec1d725fe2255f58140c182dc70a88dff2a3a15f9df168fb69ab0866f37fe5f6867f00fbed0bfe023f6847de15f6847f00ecbf008bf009bf00afe6140eca8c3e1ea52a6011abada724a87bf27d7d691076f8f0a606eead096b18d696b429bc14628e22e3379b3e65253e1a1298a229c753bd7fbd0edae9cf8c5a4f42a64830009dfb5c40bfa25041fc62dea96f1a3a91db25e1cee3ced413ddde0c188a21e63434bedf09706b82743cec165d118576e42faee5f8172f21a6c8742ade9fd3ec7fffd9";
    #[test]fn gray53_layers_two(){
        let a=decode_gray53(&hex(LAYERS_TWO)).unwrap();
        assert_eq!((a.width,a.height),(16,16));
        assert_eq!(a.pixels,(0..256).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const LAYERS_THREE:&str="ff4fff5100290000000000110000000d0000000000000000000000110000000d00000000000000000001070101ff52000c00000003000100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000001070001ff9380b9e3f1fc5c4c401428ab09409e5c13a115684241d5140b08f2312b085180e32885e0510005603632a78d47ff32f3fb447f688ffb416fa832fa82ff34140c1aa6f9227eee372890a68137ce89c26f03b7473c4c670852642432675ef63b477f01025fb6a402b096425c2b667de74c637f0b70446e3c478d68af4f4d0b2a19c04ffd80ff5a19fb427f684fda0df7037d403f701fdc07fc00b1fc01ff402bf802ff0030b0f239c8b1b1d1000ab274de1d5f94e468304f02fa6e1a920191654509c16d7c8d06a871df3c80461d549fbf96edcc3e67bf23d1243105870c2f00022701ef9ba502b171899f0e35a574f70ef133095f0c1c11dd977fffd9";
    #[test]fn gray53_layers_three(){
        let a=decode_gray53(&hex(LAYERS_THREE)).unwrap();
        assert_eq!((a.width,a.height),(17,13));
        assert_eq!(a.pixels,(0..221).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const LAYERS_TILED:&str="ff4fff51002900000000000d0000000b0000000000000000000000070000000600000000000000000001070101ff52000c00000002000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000004e0001ff93e7e027e1be17861054ee7c79dddf52d41811c49f8b790d3f98d52508e0ecfce27e80ff202fd60859af419d07af18973fd52509b99a6057b9bd46713fa4bfa272ff90000a0001000000500001ff93f3f804f87f84f0bc20105685dd1135ac244c9b7d148b0372b9505c0bedfce0be815f581cf9c1fd409f5808f54f326169a4d5b7b8b1f0a7e59cc704017fec29a2716aff90000a0002000000470001ff93ebe016e13f106be0140c208ee1ea0174ea150c374039669f0f9631728afce17ea03fa03fce1088342b617f680bf906542e461bfec2538c99bfff90000a00030000004b0001ff93f3705e13f058be1bf18308d41185b0c309b2088e0be847e357c2037c66fce07ea05fa807ec02fa047c80404f26b39fd13def12b2752d5eaec97b092f3fffd9";
    #[test]fn gray53_layers_tiled(){
        let a=decode_gray53(&hex(LAYERS_TILED)).unwrap();
        assert_eq!((a.width,a.height),(13,11));
        assert_eq!(a.pixels,(0..143).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    #[test]fn gray53_layers_split_and_prefix(){
        let b=hex(LAYERS_THREE);
        let p=packet::inspect_packets(&b,0).unwrap();
        assert!(p.packets.iter().flat_map(|p|&p.contributions).any(|e|e.first_pass>0));
        let full=decode_gray53(&b).unwrap();
        assert_eq!(decode_gray53_layers(&b,3).unwrap(),full);
        let preview=decode_gray53_layers(&b,1).unwrap();
        assert_eq!((preview.width,preview.height),(full.width,full.height));
        assert_ne!(preview.pixels,full.pixels);
    }
    #[test]fn gray53_layers_invalid_limits(){
        let b=hex(LAYERS_TWO);
        assert!(decode_gray53_layers(&b,0).is_err());
        assert!(decode_gray53_layers(&b,3).is_err());
        assert!(decode_gray53_layers(&b[..b.len()-1],1).is_err());
    }

    // Independent Pillow/OpenJPEG 2.5.4 lossless reference codestreams.
    const RGB_DIRECT:&str="ff4fff51002f000000000004000000040000000000000000000000040000000400000000000000000003070101070101070101ff52000c00000001000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000004d0001ff93cfb448103d80112ff5a0dee849878f6f1929805e7fcfb448104510db0fe5f29ed599e42c39c48b92353fcfb4480ac5c1747600ac519296da6faec6ac70036fffd9";
    #[test]fn rgb53_direct(){
        let a=decode_rgb53(&hex(RGB_DIRECT)).unwrap();
        assert_eq!((a.width,a.height),(4,4));
        assert_eq!(a.pixels,(0..48).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const RGB_RCT:&str="ff4fff51002f000000000005000000030000000000000000000000050000000300000000000000000003070101070101070101ff52000c00000001010100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000650001ff93c7d40c11d49f1f045fcfb41c0d232f96d2967fcfb41c09887894956c44cfc0161f501c3ed030050f0cb0bf03ea99015a9fcfc0127e0070fb40800cb357bf0d83e40b8bcfc0167e0090fb408009b9e10ad70ba01c1c093fffd9";
    #[test]fn rgb53_rct(){
        let a=decode_rgb53(&hex(RGB_RCT)).unwrap();
        assert_eq!((a.width,a.height),(5,3));
        assert_eq!(a.pixels,(0..45).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    const RGB_TILED:&str="ff4fff51002f000000000009000000070000000000000000000000050000000400000000000000000003070101070101070101ff52000c00000002010100000001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000007e0001ff9380dc181254dade100d907c09c4400424af080c2d93a4c8a440089e0c50c7d40e0e186a0eb5ff7ffda0a0d1154a7f7ffd6080c6f98f1bfd8090fa81e1f682008c74ff7f019af4b416c22f0c1bd00fcfc013ee028fc00c0d7aeec53a41101e7ffde05f78247e00607f331113921f09d27fff90000a00010000007e0001ff93808080cf068fbc0c0e30200f7ec692201084c893f2030f640b08dfe1f9c17e7060043108be7fe3f501fea02003477f0469e5f501fed03005771f05aabffd60785f383fd40700b8cbdf0a7b7f0a863fc7da0597da05fc00bf201010870c5f0c5fbffde0593ea02fe005004f4f079f0beaff90000a0002000000690001ff93c44012728080cf803006744fc880108bcfbc0e7053f2020a922b0d650a8dfd21008b6c341fcfb41c118a0174385d0fcfb41c07932e5ed3987ffd40507ce0a0fa808044bf0967093ffde031f681c7e004eb0ce83f0b8bc1f680c057ff90000a0003000000770001ff938080d3c1000a15ce0c0b790acfbc0d3f20200894c705b3cc3004bb5de13e10bf38300b3f05937fe5f501fed03005947f0a73f7fed03fa8040c23910ffda0783f382be40307d4040ccb3702bf070237c18bea01fda030000704fdc0389f381fda031f80109f070805b3ffd9";
    #[test]fn rgb53_tiled(){
        let a=decode_rgb53(&hex(RGB_TILED)).unwrap();
        assert_eq!((a.width,a.height),(9,7));
        assert_eq!(a.pixels,(0..189).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
    }
    #[test]fn rgb53_rct_signed_floor(){
        for r in [-128i32,-1,0,1,127]{for g in [-128i32,-1,0,1,127]{for b in [-128i32,-1,0,1,127]{
            let y=(r+2*g+b).div_euclid(4);
            assert_eq!(inverse_rct(y,b-g,r-g),[i64::from(r),i64::from(g),i64::from(b)]);
        }}}
        assert_eq!(sample8(i64::from(i32::MAX)),255);
        assert_eq!(sample8(i64::from(i32::MIN)),0);
    }
    #[test]fn rgb53_reject_wrong_components_and_truncation(){
        assert!(decode_rgb53(&hex(FLAT)).is_err());
        assert!(decode_gray53(&hex(RGB_DIRECT)).is_err());
        let b=hex(RGB_RCT);assert!(decode_rgb53(&b[..b.len()-1]).is_err());
    }
    // Independent reference fixtures generated using Pillow 12.3.0 / OpenJPEG 2.5.4.
    // Reproduction: Image.frombytes(mode, (w,h), pixels).save(BytesIO(),
    // format="JPEG2000", no_jp2=True, irreversible=False, num_resolutions=2,
    // codeblock_size=(4,4), precinct_size=(8,8), quality_layers=[4,1],
    // progression=order, mct=0 or 1, comment="RPCore f5d").
    // All seven full reference decodes were compared to their source bytes.
    fn closure_gray(s:&str){
        let b=hex(s);
        let a=decode_gray53(&b).unwrap();
        assert_eq!((a.width,a.height),(9,7));
        assert_eq!(a.pixels,(0..63).map(|i|((i*37+23)%256)as u8).collect::<Vec<_>>());
        assert_eq!(decode_gray53_layers(&b,2).unwrap(),a);
    }
    fn closure_rgb(s:&str){
        let a=decode_rgb53(&hex(s)).unwrap();
        assert_eq!((a.width,a.height),(7,5));
        assert_eq!(a.pixels,(0..105).map(|i|((i*53+17)%256)as u8).collect::<Vec<_>>());
    }
    #[test]fn closure_gray_lrcp(){closure_gray("ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000e010000020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000600001ff93c940104937afb5c8801194c493860000275a161284b8bae2fccf80200c0bfd8180ed0d74dc9d2d5e6d70199664fd8060c9fd5bfd80df68347e00c0db54454395cb961bafe4a4050c3bba897267fd4040ff7fffd9");}
    #[test]fn closure_gray_rlcp(){closure_gray("ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000e010100020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000600001ff93c940104937afb5c8801194fd8180ed0d74dc9d2d5e6d70199664fd8060c9fd5bc493860000275a161284b8bae2fccf80200c0bfd80df68347e00c0db54454395cb961bafe4a4050c3bba897267fd4040ff7fffd9");}
    #[test]fn closure_gray_rpcl(){closure_gray("ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000e010200020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000600001ff93c940104937afb5fd8180ed0d74dc9d2d5e6d70199664c8801194fd8060c9fd5bc493860000275a161284b8bae2fcfd80df68347e00c0db54454395cb961bafe4a4050c3bba897267cf80200c0bfd4040ff7fffd9");}
    #[test]fn closure_gray_pcrl(){closure_gray("ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000e010300020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000600001ff93c940104937afb5fd8180ed0d74dc9d2d5e6d70199664c493860000275a161284b8bae2fcfd80df68347e00c0db54454395cb961bafe4a4050c3bba897267c8801194fd8060c9fd5bcf80200c0bfd4040ff7fffd9");}
    #[test]fn closure_gray_cprl(){closure_gray("ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000e010400020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000600001ff93c940104937afb5fd8180ed0d74dc9d2d5e6d70199664c493860000275a161284b8bae2fcfd80df68347e00c0db54454395cb961bafe4a4050c3bba897267c8801194fd8060c9fd5bcf80200c0bfd4040ff7fffd9");}
    #[test]fn closure_rgb_pcrl_rct(){closure_rgb("ff4fff51002f000000000007000000050000000000000000000000070000000500000000000000000003070101070101070101ff52000e010300020101000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a0000000000a00001ff9380c7d418097fdecf3e9af63bc87dbbcb911006ddc7da111f6847de08105fce18ec4fa7c419117c446101d267ff4a1ad480cfb4380bae292edc5444ed1c8b170a42a3c8d31007aa230baa8042fde0d3f008fc4040d57361560a7f02e409a1dda8bd5f0980cfb4301989ab48fa53b64d7ae13887c8e61cc30fd7241153f40ddad3fde0bf7027e204912d84707f6b49147f8edfffd9");}
    #[test]fn closure_rgb_cprl_direct(){closure_rgb("ff4fff51002f000000000007000000050000000000000000000000070000000500000000000000000003070101070101070101ff52000e010400020001000000012233ff5c00074040484850ff64000e00015250436f726520663564ff90000a00000000009e0001ff9380cfb438057f174464fd2188fed56258b47f93840e5bc82fcfc0227e011f800816ede22e70c7db7f16a29adb05e3533f0f80cfb4340b743798d7916b034808ec220f91f781800a16dfcfc0227e011000105fce1eb405ab80188197ab631ee48f80cfb43015c631364fd08ab775941399cc44e10006530a290da7f304fdc0b3f006fc00402733662d7f04ec931bd6f43fffd9");}
    #[test]fn closure_reject_unsupported_coding(){
        let a=hex(FLAT);
        let cod=a.windows(2).position(|w|w==[0xff,0x52]).unwrap();
        let siz=a.windows(2).position(|w|w==[0xff,0x51]).unwrap();
        for (at,value) in [(cod+12,1u8),(cod+13,0),(siz+40,0x87),(siz+40,15),(siz+41,2)]{
            let mut b=a.clone();b[at]=value;
            assert!(decode_gray53(&b).is_err(),"unsupported field offset {}",at);
        }
    }

    // Independently encoded and decoded this turn with Pillow/OpenJPEG 2.5.4,
    // no_jp2=True, irreversible=True, codeblock_size=(4,4),
    // num_resolutions=1 or 2, default one layer. Source (i*37+23)%256,
    // except ZERO97 constant 128. References are decoder output, not source.
    const FLAT97:&str="ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000000000000ff5c0005424000ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000240001ff93cfb44c117bb2617fbedccc4de18580bdcbecf21b9c6fffd9";
    const ZERO97:&str="ff4fff510029000000000005000000050000000000000000000000050000000500000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000100001ff938080ffd9";
    const ONE97:&str="ff4fff510029000000000007000000050000000000000000000000070000000500000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000003d0001ff93cfc038094388b57844d6fb4519baa27849c3ed0987da0f07d40a1a78c7f17057ad0f5f185ac8b335bb7f14c35ff37fffd9";
    const ODD97:&str="ff4fff510029000000000005000000030000000000000000000000050000000300000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000002e0001ff93cfc0200a652bc06fa492efc3ed058fc0121f6818045690589b03351d8b03007fffd9";
    fn compare_gray97(data:&str,w:usize,h:usize,want:&[u8]){
        let a=decode_gray97_centered(&hex(data)).unwrap();
        assert_eq!((a.width,a.height),(w,h));
        assert_eq!(a.samples.len(),want.len());
        for (i,(&v,&expected)) in a.samples.iter().zip(want).enumerate(){
            assert!(v.is_finite());
            // Compare final u8 samples; allow only one level for the
            // independent reference's float32 and tie rounding differences.
            let actual=(v+128.).round().clamp(0.,255.)as u8;
            assert!(actual.abs_diff(expected)<=1,"sample {i}: centered {v}, actual {actual}, reference {expected}");
        }
    }
    #[test]fn gray97_flat_reference(){
        compare_gray97(FLAT97,4,4,&[22,60,96,134,172,208,246,26,62,100,138,174,212,248,28,66]);
    }
    #[test]fn gray97_zero_reference(){
        let a=decode_gray97_centered(&hex(ZERO97)).unwrap();
        assert_eq!((a.width,a.height),(5,5));assert_eq!(a.samples,vec![0.;25]);
    }
    #[test]fn gray97_one_resolution_reference(){
        compare_gray97(ONE97,7,5,&[23,60,97,134,171,208,245,25,63,100,137,174,211,248,29,66,103,140,177,214,251,32,69,106,143,180,217,254,34,72,109,145,183,220,1]);
    }
    #[test]fn gray97_odd_reference(){
        compare_gray97(ODD97,5,3,&[23,60,97,134,171,208,245,26,63,100,137,174,211,248,29]);
    }
    #[test]fn gray97_rejects_unsupported(){
        assert!(decode_gray97_centered(&hex(FLAT)).is_err());
        let a=hex(ONE97);
        assert!(decode_gray97_centered(&a[..a.len()-1]).is_err());
        assert!(decode_gray97_centered(&[]).is_err());
        let mut a=hex(ONE97);a[42]=0x87;assert!(decode_gray97_centered(&a).is_err());
        let mut a=hex(ONE97);a[42]=15;assert!(decode_gray97_centered(&a).is_err());
    }

    // Independent Pillow/OpenJPEG 2.5.4 generated and decoded locally this turn.
    // Source (i*37+23)%256, irreversible, 4x4 codeblocks, raw J2K.
    const GRAY97_TILES:&str="ff4fff510029000000000009000000070000000000000000000000050000000400000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000340001ff93cfc020076634527310376fc3ed058fc0221f6828105a85e1c90fdcde4538d9f8bf0c014c240bff90000a0001000000360001ff93e41f085fc00c0bb707102fc3ed05c9f681ff18243ed0500e3036cc470567070c4b5fdf10917525bfff90000a00020000002e0001ff93cfc020142480413eedb9e1c3ed058fc0121f68180bcd8d51440dd3e7cb0bbd77ff90000a0003000000310001ff93e7f0039f501805a17202797fc3ed05c21f083fb40a0fa80c0f477d0c5f08041f0c20bfffd9";
    #[test]fn gray97_tiles_reference(){
        let c=inspect_codestream(&hex(GRAY97_TILES)).unwrap();
        assert_eq!((c.tile_parts.len(),c.coding.style.levels,c.coding.layers),(4,1,1));
        compare_gray97(GRAY97_TILES,9,7,&[23,60,97,134,171,208,245,25,63,100,137,174,211,248,28,66,103,140,177,214,251,32,69,106,143,180,217,254,35,72,109,147,183,220,1,38,75,112,149,186,223,4,41,78,114,152,189,227,7,44,81,118,155,192,229,10,47,84,121,158,195,232,13]);
    }
    const GRAY97_LEVELS:&str="ff4fff510029000000000009000000070000000000000000000000090000000700000000000000000001070101ff52000c00000001000200000000ff5c0011425f5250055005504757d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000740001ff93c1f684000a768d7dd3e1a47fc0f9c1c7e0111f802804e68804bd94efea2340550fc7b92861c3ed12c7f010fe00d0fb434000275a1941ada7d54fe3fee75f6c3b19d2df1284bb19744e62172a36332e5fe79c430c0bb36eff7f0c1bf6addc188c1c00a6dfaf4fffd9";
    #[test]fn gray97_levels_reference(){
        let c=inspect_codestream(&hex(GRAY97_LEVELS)).unwrap();
        assert_eq!((c.tile_parts.len(),c.coding.style.levels,c.coding.layers),(1,2,1));
        compare_gray97(GRAY97_LEVELS,9,7,&[23,60,97,134,171,208,245,26,64,100,137,174,211,248,29,66,103,140,176,214,251,31,69,106,143,180,217,254,35,72,109,146,183,220,1,38,75,112,149,186,223,4,41,78,116,153,189,226,7,44,81,118,155,193,229,11,47,84,122,158,195,233,13]);
    }
    const GRAY97_LAYERS:&str="ff4fff51002900000000000c0000000900000000000000000000000c0000000900000000000000000001070101ff52000c00000003000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000c30001ff93e786f84016e43146e6140f51ee0ab1c7f0601431e37df7511718b67ce13e0c15fa0d1062d1f0a143201a6693a7bc1180189bfd619f5837d6057da0608782d9d6f9e56fe8543797df4dae8908539fdb42063107df689fd60b7da0b7da07f7077da0ff5a27f6030014641928f233adad0f09feff0e2b97dca05f5574df82dcfd05aef83a5f0b90cfb978fa5d06cace0c65aeb987f17fef84985b7c85c714309f7781cccbaa5999678407bd84fa961d63663edb789f97ffd9";
    #[test]fn gray97_layers_reference(){
        let c=inspect_codestream(&hex(GRAY97_LAYERS)).unwrap();
        assert_eq!((c.tile_parts.len(),c.coding.style.levels,c.coding.layers),(1,1,3));
        compare_gray97(GRAY97_LAYERS,12,9,&[23,60,96,134,171,208,245,26,62,100,137,173,212,248,29,66,103,140,177,214,251,32,69,106,143,180,217,254,35,72,109,146,183,220,1,38,75,112,149,186,223,4,41,78,115,152,189,226,7,44,80,118,155,192,229,10,47,84,121,158,194,233,13,50,87,124,161,198,235,16,53,90,127,164,201,239,19,55,93,130,167,204,241,22,59,96,133,170,207,244,25,62,99,136,173,209,247,27,65,102,139,175,213,250,31,68,105,142]);
    }
    const GRAY97_OFFSET:&str="ff4fff510029000000000008000000070000000100000001000000050000000400000000000000000001070101ff52000c00000001000100000000ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000001e0001ff93cfc00c076b5fc1f501401d08075f09a1ff90000a00010000001e0001ff93cfc00802cfc1f501c01d0809d77f099fff90000a00020000001d0001ff93c7da0a0918538617c1f50180095c73ff90000a0003000000260001ff93cfc00c0456dfcfcc160fa8061f681810847b1705070c4db9ffd9";
    #[test]fn gray97_offset_reference(){
        let c=inspect_codestream(&hex(GRAY97_OFFSET)).unwrap();
        assert_eq!((c.tile_parts.len(),c.coding.style.levels,c.coding.layers),(4,1,1));
        compare_gray97(GRAY97_OFFSET,7,6,&[23,60,97,134,171,208,246,26,63,100,137,174,211,249,29,66,103,140,177,214,251,32,69,106,143,180,217,254,35,72,109,146,183,220,1,38,75,112,149,186,223,5]);
    }
    #[test]fn gray97_missing_tile_rejected(){
        let mut a=hex(GRAY97_TILES);
        let c=inspect_codestream(&a).unwrap();
        a.drain(c.tile_parts.last().unwrap().span.clone());
        assert!(decode_gray97_centered(&a).is_err());
    }
}
