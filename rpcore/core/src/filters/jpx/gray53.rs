//! Scoped native reversible JPEG 2000 grayscale pixel reconstruction.
//! Multiple tiles, one layer, unsigned 8-bit, no MCT, baseline MQ style.
//! This does not activate the PDF JPXDecode rendering path.
use super::{Container,codestream::{inspect_codestream,Codestream,QuantMode,Wavelet},geometry::{self,Band,Rect},packet,ebcot,wavelet53::{self,Detail53}};
use crate::{Error,Result};

const MAX_PIXELS:usize=4_194_304;
const MAX_WORK:usize=MAX_PIXELS*128;
#[derive(Debug,PartialEq,Eq)]
pub struct GrayImage{pub width:usize,pub height:usize,pub pixels:Vec<u8>}
struct Plane{bounds:Rect,values:Vec<i32>}
fn band_index(b:Band)->usize{match b{Band::Ll=>0,Band::Hl=>1,Band::Lh=>2,Band::Hh=>3}}

/// Reconstruct baseline grayscale raw codestreams. Quantization is
/// reversible (unit steps). Missing contributions represent zero blocks.
/// A single quality layer is not proof of lossless encoding; this API
/// returns the represented samples, including zero-filled omitted bits.
pub fn decode_gray53(data:&[u8])->Result<GrayImage>{
    let c=inspect_codestream(data)?;
    let h=&c.geometry;
    if h.container!=Container::RawCodestream||h.components.len()!=1||
        c.coding.layers!=1||c.coding.mct{
        return Err(Error::Unsupported("JPX gray53 container/components/layers"));
    }
    let g=&h.components[0];let s=&c.component_coding[0];let q=&c.component_quantization[0];
    if g.precision!=8||g.signed||g.x_sampling!=1||g.y_sampling!=1||
        s.wavelet!=Wavelet::Reversible53||s.codeblock_style!=0||q.mode!=QuantMode::None{
        return Err(Error::Unsupported("JPX gray53 coding parameters"));
    }
    let (w,ht)=(h.width as usize,h.height as usize);
    let area=w.checked_mul(ht).ok_or(Error::Limit("JPX gray53 pixels"))?;
    if area==0||area>MAX_PIXELS||w>1_048_576||ht>1_048_576{return Err(Error::Limit("JPX gray53 pixels"));}
    let mut pixels=vec![0;area];
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
        let samples=decode_tile(data,&c,&plan,&mut work)?;
        let ox=(b.x0-i64::from(h.x_origin))as usize;
        let oy=(b.y0-i64::from(h.y_origin))as usize;
        for y in 0..th{
            for x in 0..tw{pixels[(oy+y)*w+ox+x]=(i64::from(samples[y*tw+x])+128).clamp(0,255)as u8;}
        }
    }
    if covered!=area{return Err(Error::Invalid("JPX gray53 tile coverage"));}
    Ok(GrayImage{width:w,height:ht,pixels})
}

fn decode_tile(data:&[u8],c:&Codestream,plan:&geometry::TilePlan,work:&mut usize)->Result<Vec<i32>>{
    let t=&plan.components[0];let q=&c.component_quantization[0];
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
    let packets=packet::inspect_planned_packets(data,c,plan)?;
    for p in &packets.packets{
        let ri=usize::from(p.packet.resolution);
        let precinct=&t.resolutions[ri].precincts[p.packet.precinct];
        for e in &p.contributions{
            if e.first_pass!=0{return Err(Error::Invalid("JPX gray53 noninitial contribution"));}
            let band=precinct.bands.iter().find(|b|b.band==e.band).ok_or(Error::Invalid("JPX gray53 band"))?;
            let block=*band.blocks.get(e.block).ok_or(Error::Invalid("JPX gray53 block"))?;
            let bi=band_index(e.band);
            let qi=if ri==0{0}else{3*(ri-1)+bi};
            let step=q.steps.get(qi).ok_or(Error::Invalid("JPX gray53 quantization step"))?;
            let plane=i32::from(q.guard_bits)+i32::from(step.exponent)-2-i32::from(e.zero_bit_planes);
            if !(0..=30).contains(&plane){return Err(Error::Invalid("JPX gray53 highest bitplane"));}
            let (bw,bh)=(block.width() as usize,block.height() as usize);
            let charge=bw.checked_mul(bh).and_then(|n|n.checked_mul(usize::from(e.passes))).ok_or(Error::Limit("JPX gray53 work"))?;
            *work=work.checked_add(charge).ok_or(Error::Limit("JPX gray53 work"))?;
            if *work>MAX_WORK{return Err(Error::Limit("JPX gray53 work"));}
            let coeff=ebcot::decode_codeblock(&data[e.data.clone()],bw,bh,e.band,plane as u8,e.passes,0)?.signed_coefficients();
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
    }
    let details:Vec<_>=planes.iter().skip(1).map(|r|Detail53{hl:&r[0].values,lh:&r[1].values,hh:&r[2].values}).collect();
    wavelet53::inverse53_levels(w,ht,t.bounds.x0 as u32,t.bounds.y0 as u32,&planes[0][0].values,&details)
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
}
