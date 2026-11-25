//! Bounded Part 1 main-header coding and quantization inspection and
//! tile-part spans. No packet parsing or entropy decoding. Tile-local
//! parameter overrides and extended packet-header markers are not supported.
use super::{inspect,u16be,u32be,Header};
use crate::{Error,Result};
use std::ops::Range;
const MAX_MARKERS:usize=131072;
const MAX_PARTS:usize=65536;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Progression{Lrcp,Rlcp,Rpcl,Pcrl,Cprl}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Wavelet{Irreversible97,Reversible53}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CodingStyle{
    pub levels:u8,pub codeblock_width:u16,pub codeblock_height:u16,
    pub codeblock_style:u8,pub wavelet:Wavelet,
    /// Precinct log2 width,height for each resolution, low to high.
    pub precincts:Vec<(u8,u8)>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CodingDefault{
    pub progression:Progression,pub layers:u16,pub mct:bool,pub sop:bool,pub eph:bool,
    pub style:CodingStyle,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum QuantMode{None,ScalarDerived,ScalarExpounded}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Step{pub exponent:u8,pub mantissa:u16}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Quantization{pub mode:QuantMode,pub guard_bits:u8,pub steps:Vec<Step>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TilePart{
    pub tile:u16,pub index:u8,pub declared_parts:u8,
    /// All ranges are absolute offsets into the original input (including JP2).
    pub span:Range<usize>,pub payload:Range<usize>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Codestream{
    pub geometry:Header,pub coding:CodingDefault,pub quantization:Quantization,
    pub component_coding:Vec<CodingStyle>,pub component_quantization:Vec<Quantization>,
    pub tile_parts:Vec<TilePart>,
}
fn style(p:&[u8],precinct:bool)->Result<CodingStyle>{
    if p.len()<5{return Err(Error::Invalid("JPX coding style length"));}
    let levels=p[0];let w=p[1];let h=p[2];
    if levels>32||w>8||h>8||w+h>8{return Err(Error::Invalid("JPX decomposition/codeblock size"));}
    if p[3]&!0x3f!=0{return Err(Error::Unsupported("JPX codeblock style extension"));}
    let wavelet=match p[4]{0=>Wavelet::Irreversible97,1=>Wavelet::Reversible53,_=>return Err(Error::Unsupported("JPX wavelet"))};
    let n=usize::from(levels)+1;
    if p.len()!=5+if precinct{n}else{0}{return Err(Error::Invalid("JPX precinct length"));}
    let precincts=if precinct{
        let mut v=Vec::with_capacity(n);
        for (i,&x) in p[5..].iter().enumerate(){
            let (a,b)=(x&15,x>>4);
            if i!=0&&(a==0||b==0){return Err(Error::Invalid("JPX precinct exponent"));}
            v.push((a,b));
        }v
    }else{vec![(15,15);n]};
    Ok(CodingStyle{levels,codeblock_width:1u16<<(w+2),codeblock_height:1u16<<(h+2),codeblock_style:p[3],wavelet,precincts})
}
fn cod(p:&[u8])->Result<CodingDefault>{
    if p.len()<10{return Err(Error::Invalid("JPX COD length"));}
    if p[0]&!7!=0{return Err(Error::Unsupported("JPX COD flags"));}
    let progression=match p[1]{0=>Progression::Lrcp,1=>Progression::Rlcp,2=>Progression::Rpcl,3=>Progression::Pcrl,4=>Progression::Cprl,_=>return Err(Error::Invalid("JPX progression order"))};
    let layers=u16be(p,2)?;
    if layers==0||p[4]>1{return Err(Error::Invalid("JPX layers/MCT"));}
    Ok(CodingDefault{progression,layers,mct:p[4]==1,sop:p[0]&2!=0,eph:p[0]&4!=0,style:style(&p[5..],p[0]&1!=0)?})
}
fn quant(p:&[u8])->Result<Quantization>{
    let &s=p.first().ok_or(Error::Invalid("JPX quantization length"))?;
    let mode=match s&31{0=>QuantMode::None,1=>QuantMode::ScalarDerived,2=>QuantMode::ScalarExpounded,_=>return Err(Error::Unsupported("JPX quantization mode"))};
    let body=&p[1..];
    let mut steps=Vec::new();
    match mode{
        QuantMode::None=>{
            if body.is_empty()||body.len()>97{return Err(Error::Invalid("JPX quantization step count"));}
            for &x in body{
                if x&7!=0{return Err(Error::Invalid("JPX reversible step reserved bits"));}
                steps.push(Step{exponent:x>>3,mantissa:0});
            }
        }
        _=>{
            if body.is_empty()||body.len()%2!=0||body.len()>194||mode==QuantMode::ScalarDerived&&body.len()!=2{return Err(Error::Invalid("JPX quantization step count"));}
            for x in body.chunks_exact(2){
                let v=u16::from_be_bytes([x[0],x[1]]);
                steps.push(Step{exponent:(v>>11)as u8,mantissa:v&2047});
            }
        }
    }
    Ok(Quantization{mode,guard_bits:s>>5,steps})
}
fn check_steps(q:&Quantization,s:&CodingStyle,exact:bool)->Result<()>{
    let n=if q.mode==QuantMode::ScalarDerived{1}else{1+3*usize::from(s.levels)};
    if q.steps.len()<n||exact&&q.steps.len()!=n{return Err(Error::Invalid("JPX quantization/subband count"));}
    // Reversible 5/3 must use exponent-only quantization in supported subset.
    if s.wavelet==Wavelet::Reversible53&&q.mode!=QuantMode::None{return Err(Error::Unsupported("JPX reversible scalar quantization"));}
    Ok(())
}
fn marker(b:&[u8],p:usize,budget:&mut usize)->Result<u16>{
    *budget=budget.checked_sub(1).ok_or(Error::Limit("JPX marker count"))?;
    let m=u16be(b,p)?;
    if m&0xff00!=0xff00||m&255==0||m&255==255{return Err(Error::Invalid("JPX marker"));}
    Ok(m)
}
fn segment<'a>(b:&'a[u8],p:&mut usize)->Result<&'a[u8]>{
    let l=usize::from(u16be(b,*p+2)?);
    if l<2{return Err(Error::Invalid("JPX marker length"));}
    let end=*p+2+l;
    let payload=b.get(*p+4..end).ok_or(Error::Invalid("JPX truncated marker segment"))?;
    *p=end;Ok(payload)
}
/// Inspect main COD/COC/QCD/QCC and tile-part boundaries. Reject unsupported
/// markers rather than guessing their effect. Packet data is opaque.
pub fn inspect_codestream(data:&[u8])->Result<Codestream>{
    let geometry=inspect(data)?;
    let base=geometry.codestream_offset;
    let b=&data[base..base+geometry.codestream_length];
    let count=geometry.components.len(); // <=16, one-byte component index
    let mut p=4+usize::from(u16be(b,4)?);
    let mut budget=MAX_MARKERS;let mut coding=None;let mut quantization=None;
    let mut coc=vec![None;count];let mut qcc=vec![None;count];
    loop{
        let m=marker(b,p,&mut budget)?;
        if m==0xff90{break;}
        match m{
            0xff52=>{
                if coding.is_some(){return Err(Error::Invalid("JPX duplicate COD"));}
                coding=Some(cod(segment(b,&mut p)?)?);
            }
            0xff53=>{
                let s=segment(b,&mut p)?;
                if s.len()<7{return Err(Error::Invalid("JPX COC length"));}
                let i=usize::from(s[0]);
                if i>=count||coc[i].is_some()||s[1]&!1!=0{return Err(Error::Invalid("JPX COC component/flags"));}
                coc[i]=Some(style(&s[2..],s[1]&1!=0)?);
            }
            0xff5c=>{
                if quantization.is_some(){return Err(Error::Invalid("JPX duplicate QCD"));}
                quantization=Some(quant(segment(b,&mut p)?)?);
            }
            0xff5d=>{
                let s=segment(b,&mut p)?;
                let i=usize::from(*s.first().ok_or(Error::Invalid("JPX QCC length"))?);
                if i>=count||qcc[i].is_some(){return Err(Error::Invalid("JPX QCC component"));}
                qcc[i]=Some(quant(&s[1..])?);
            }
            0xff64=>{if segment(b,&mut p)?.len()<2{return Err(Error::Invalid("JPX COM length"));}}
            0xffd9|0xff93|0xff4f|0xff51=>return Err(Error::Invalid("JPX main header order")),
            _=>return Err(Error::Unsupported("JPX main header marker")),
        }
    }
    let coding=coding.ok_or(Error::Invalid("JPX missing COD"))?;
    let quantization=quantization.ok_or(Error::Invalid("JPX missing QCD"))?;
    if coding.mct&&count<3{return Err(Error::Invalid("JPX MCT components"));}
    check_steps(&quantization,&coding.style,true)?;
    let mut component_coding=Vec::with_capacity(count);
    let mut component_quantization=Vec::with_capacity(count);
    for (s,q) in coc.into_iter().zip(qcc){
        let s=s.unwrap_or_else(||coding.style.clone());
        let explicit=q.is_some();
        let q=q.unwrap_or_else(||quantization.clone());
        check_steps(&q,&s,explicit)?;
        component_coding.push(s);component_quantization.push(q);
    }
    let tiles=(u64::from(geometry.tiles_x)*u64::from(geometry.tiles_y))as usize;
    let mut next=vec![0u16;tiles];let mut declared=vec![0u8;tiles];
    let mut tile_parts=Vec::new();
    loop{
        let start=p;
        let m=marker(b,p,&mut budget)?;
        if m==0xffd9{
            if p+2!=b.len(){return Err(Error::Invalid("JPX trailing codestream bytes"));}
            break;
        }
        if m!=0xff90{return Err(Error::Invalid("JPX expected SOT/EOC"));}
        if tile_parts.len()>=MAX_PARTS{return Err(Error::Limit("JPX tile parts"));}
        let s=segment(b,&mut p)?;
        if s.len()!=8{return Err(Error::Invalid("JPX SOT length"));}
        let tile=u16be(s,0)?;let i=usize::from(tile);let len=u32be(s,2)? as usize;
        let index=s[6];let total=s[7];
        if i>=tiles||u16::from(index)!=next[i]||total!=0&&index>=total{return Err(Error::Invalid("JPX tile part sequence"));}
        if declared[i]!=0&&(total!=0&&declared[i]!=total||u16::from(index)>=u16::from(declared[i])){return Err(Error::Invalid("JPX tile part count"));}
        if total!=0{declared[i]=total;}
        next[i]+=1;
        let end=if len==0{
            // Psot=0 is supported only for the final tile-part, with final EOC.
            if !b.ends_with(&[0xff,0xd9]){return Err(Error::Invalid("JPX final EOC"));}
            b.len()-2
        }else{
            start.checked_add(len).ok_or(Error::Limit("JPX tile part length"))?
        };
        if end<p+2||end>b.len().saturating_sub(2){return Err(Error::Invalid("JPX tile part boundary"));}
        loop{
            if p+2>end{return Err(Error::Invalid("JPX missing SOD"));}
            let m=marker(b,p,&mut budget)?;
            if m==0xff93{p+=2;break;}
            if m!=0xff64{return Err(Error::Unsupported("JPX tile header marker"));}
            if segment(&b[..end],&mut p)?.len()<2{return Err(Error::Invalid("JPX tile COM length"));}
        }
        tile_parts.push(TilePart{tile,index,declared_parts:total,span:base+start..base+end,payload:base+p..base+end});
        p=end;
    }
    if next.iter().zip(declared).any(|(&n,d)|n==0||d!=0&&n!=u16::from(d)){
        return Err(Error::Invalid("JPX incomplete tile parts"));
    }
    Ok(Codestream{geometry,coding,quantization,component_coding,component_quantization,tile_parts})
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    // Pillow/OpenJPEG 2.5.4, 12x9, reversible four tiles and irreversible RGB.
    const GRAY:&str="ff4fff51002900000000000c000000090000000000000000000000080000000800000000000000000001070101ff52000c00000001000104040001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000002c0001ff93df8080120a4b899e9b0e1790c691ddf4cded37c01140f901802217037e97ff90000a0001000000240001ff93cfb424111df46a1ccbe3ba53c01140f9010017cf036eff90000a0002000000190001ff93c7d408065eaa47c010800eff90000a0003000000180001ff93c3e7060828bfc010800affd9";
    const RGB:&str="ff4fff51002f00000000000c0000000900000000000000000000000c0000000900000000000000000003070101070101070101ff52000e010100010101010100003344ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000001020001ff93cfc0841289bf64d7b33e30291626d80a861e346fca9e9dc125a2833a40269493e4f3fd9fcfc0843ab81599e1e5a571c7d7f78e5ba8a7debf6b807648db9a1117b1fedb391268c83fcfc0780168f484c2b86f7abc55d96f99d41b3b3668a10a6912db7a36167616e7e7c1f50543ed1583ea063140259cdb742a75272b0385db2d968875dcbdde1aea8d63d73a0163bdc77f042d61e78bd4c0f9c647e02d07d4120bb4093f2244a9d583d077bf014fcb841bca7f10b2573e0b15f205a6696c78c8087f0c2216a6623668ff6fc0f9c443ed1281f386014370cdeac16ec1015ceb6999ce6f9cdd1918704a6a3eb3590f01694c82dca7ffd9";
    #[test]fn independent_reversible_tiles(){
        let b=hex(GRAY);let h=inspect_codestream(&b).unwrap();
        assert_eq!((h.coding.style.wavelet,h.coding.style.levels),(Wavelet::Reversible53,1));
        assert_eq!((h.coding.style.codeblock_width,h.coding.style.codeblock_height),(64,64));
        assert_eq!(h.coding.progression,Progression::Lrcp);
        assert_eq!(h.quantization.mode,QuantMode::None);
        assert_eq!(h.quantization.steps.iter().map(|s|s.exponent).collect::<Vec<_>>(),vec![8,9,9,10]);
        let spans:Vec<_>=h.tile_parts.iter().map(|t|(t.tile,t.span.clone(),t.payload.clone())).collect();
        assert_eq!(spans,vec![(0,107..151,121..151),(1,151..187,165..187),(2,187..212,201..212),(3,212..236,226..236)]);
    }
    #[test]fn independent_irreversible_precincts(){
        let b=hex(RGB);let h=inspect_codestream(&b).unwrap();
        assert_eq!(h.coding.progression,Progression::Rlcp);assert!(h.coding.mct);
        assert_eq!(h.coding.style.wavelet,Wavelet::Irreversible97);
        assert_eq!(h.coding.style.precincts,vec![(3,3),(4,4)]);
        assert_eq!(h.coding.style.codeblock_width,8);
        assert_eq!(h.quantization.mode,QuantMode::ScalarExpounded);
        assert_eq!(h.quantization.steps[0],Step{exponent:9,mantissa:36});
        assert_eq!(h.tile_parts[0].span,119..377);assert_eq!(h.tile_parts[0].payload,133..377);
    }
    #[test]fn component_override_and_derived_quantization(){
        let mut b=hex(RGB);
        // COC comp 1: zero levels, 8x8, 9/7. QCC comp 1: scalar derived.
        let add=hex("ff53000901000001010000ff5d000601414824");
        b.splice(119..119,add);
        let h=inspect_codestream(&b).unwrap();
        assert_eq!(h.component_coding[1].levels,0);
        assert_eq!(h.component_coding[0].levels,1);
        assert_eq!(h.component_quantization[1].mode,QuantMode::ScalarDerived);
        assert_eq!(h.component_quantization[1].steps,vec![Step{exponent:9,mantissa:36}]);
    }
    #[test]fn final_zero_psot_and_sequence_checks(){
        let b=hex(GRAY);let mut z=b.clone();z[218..222].fill(0);
        assert_eq!(inspect_codestream(&z).unwrap().tile_parts[3].span,212..236);
        for (off,v) in [(113,0xff),(157,0xff),(161,2),(162,2),(216,0xff)]{
            let mut bad=b.clone();bad[off]=v;
            assert!(inspect_codestream(&bad).is_err(),"offset {off}");
        }
        let mut bad=b.clone();bad[113..117].copy_from_slice(&13u32.to_be_bytes());assert!(inspect_codestream(&bad).is_err());
        let mut bad=b.clone();bad.truncate(212);bad.extend([0xff,0xd9]);assert!(inspect_codestream(&bad).is_err());
    }
    #[test]fn reject_missing_duplicate_unsupported_markers(){
        let b=hex(GRAY);
        let mut bad=b.clone();bad.drain(45..59);assert!(inspect_codestream(&bad).is_err());
        let mut bad=b.clone();bad.splice(107..107,b[45..59].iter().copied());assert!(inspect_codestream(&bad).is_err());
        let mut bad=b.clone();bad[46]=0x5f;assert!(matches!(inspect_codestream(&bad),Err(Error::Unsupported("JPX main header marker"))));
        let mut bad=b.clone();bad[120]=0x52;assert!(inspect_codestream(&bad).is_err());
        let mut bad=b.clone();bad[58]=2;assert!(inspect_codestream(&bad).is_err());
        let mut bad=b.clone();bad[63]=0x42;assert!(inspect_codestream(&bad).is_err());
    }
    #[test]fn malformed_parameter_lengths_and_limits(){
        assert!(style(&[33,1,1,0,1],false).is_err());
        assert!(style(&[1,5,4,0,1],false).is_err());
        assert!(style(&[1,1,1,0,1,0x33],true).is_err());
        assert!(quant(&[0x41,0x48]).is_err());
        assert!(quant(&[0x40,0x49]).is_err());
        let b=hex(RGB);
        for n in [51,54,66,80,119,130,133,376,378]{assert!(inspect_codestream(&b[..n]).is_err(),"prefix {n}");}
    }
}
