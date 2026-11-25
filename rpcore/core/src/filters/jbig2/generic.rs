//! Native T.88 generic-region header and causal arithmetic templates.
//! Context bits are assigned in reverse nominal reading order (LSB at
//! the nearest previously coded pixel); adaptive slots never move bits.
//! Scoped known-length arithmetic and MMR generic region reconstruction.
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

/// Maximum conservative per-region work units (neighbor reads, pixel writes
/// and optional row prediction decisions). This is not a wall-time estimate.
pub const MAX_GENERIC_WORK:u64=100_000_000;
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedGeneric{
    pub region:RegionInfo,
    pub bitmap:Bitmap,
    pub decisions:usize,
    pub copied_rows:u32,
}

/// Decode a complete arithmetic-coded generic region segment body, using
/// fresh contexts. The upper-bound work and decision budgets are checked
/// before allocation or pixel reconstruction. TPGD toggles LTP using the
/// SLTP context in the same bank; a typical first row is all zero.
/// No external skip bitmap, MMR, retained contexts or unknown-length
/// segment is supported. Explicit arithmetic termination is required.
pub fn decode_arithmetic(data:&[u8],work_limit:u64)->Result<DecodedGeneric>{
    let (header,payload)=parse(data)?;
    decode_arithmetic_body(header,payload,work_limit)
}
fn decode_arithmetic_body(header:GenericHeader,payload:&[u8],work_limit:u64)->Result<DecodedGeneric>{
    use super::qm::{Context,Decoder};
    let (neighborhood,typical_prediction)=match header.coding{
        Coding::Arithmetic{neighborhood,typical_prediction}=>(neighborhood,typical_prediction),
        Coding::Mmr=>return Err(Error::Unsupported("JBIG2 generic MMR decoder")),
    };
    let r=header.region;
    let template=neighborhood.template();
    let area=u64::from(r.width)*u64::from(r.height);
    let row_work=if typical_prediction{u64::from(r.height)}else{0};
    let work=area.checked_mul(template.bits()as u64+1).and_then(|n|n.checked_add(row_work))
        .ok_or(Error::Limit("JBIG2 generic work"))?;
    if work>work_limit.min(MAX_GENERIC_WORK){return Err(Error::Limit("JBIG2 generic work"));}
    let upper=usize::try_from(area.checked_add(row_work).ok_or(Error::Limit("JBIG2 generic decisions"))?)
        .map_err(|_|Error::Limit("JBIG2 generic decisions"))?;
    let mut decoder=Decoder::new(payload,upper)?;
    let mut contexts=Vec::new();
    contexts.try_reserve_exact(template.context_count()).map_err(|_|Error::Limit("JBIG2 generic contexts"))?;
    contexts.resize(template.context_count(),Context::default());
    let mut image=Bitmap::new(r.width,r.height,false)?;
    let mut ltp=false;
    let mut decisions=0usize;
    let mut copied_rows=0u32;
    for y in 0..r.height{
        if typical_prediction{
            let sltp=decoder.decode(&mut contexts[template.sltp_context()])?!=0;
            decisions+=1;
            ltp^=sltp;
            if ltp{
                copied_rows+=1;
                if y!=0{
                    for x in 0..r.width{
                        let bit=image.sample(i64::from(x),i64::from(y)-1);
                        image.set(x,y,bit)?;
                    }
                }
                continue;
            }
        }
        for x in 0..r.width{
            let cx=neighborhood.context(&image,x,y)?;
            let bit=decoder.decode(&mut contexts[cx])?!=0;
            decisions+=1;
            image.set(x,y,bit)?;
        }
    }
    // Marker fill is normative, not an authenticity check; unused encoded
    // bytes before the marker may occur and are not a decision-count proof.
    Ok(DecodedGeneric{region:r,bitmap:image,decisions,copied_rows})
}


#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum DecodeStats{
    Arithmetic{decisions:usize,copied_rows:u32},
    Mmr{mode_counts:[u64;9],encoded_bits:usize,eofb_present:bool},
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedRegion{
    pub region:RegionInfo,
    pub bitmap:Bitmap,
    pub stats:DecodeStats,
}
/// Dispatch a single known-length generic segment body through its native
/// coding algorithm. The header is parsed once, both paths preserve the
/// common region metadata and return canonical MSB-first black=1 bitmaps.
/// MMR uses T.88's optional EOFB rule, not the stricter standalone T.6 API.
/// The unified work cap includes the MMR packed-byte conversion; arithmetic
/// retains its existing conservative per-pixel bound. This is not a page
/// decoder and does not apply page-level combination-operator overrides.
pub fn decode_region(data:&[u8],work_limit:u64)->Result<DecodedRegion>{
    let (header,payload)=parse(data)?;
    let limit=work_limit.min(MAX_GENERIC_WORK);
    if matches!(&header.coding,Coding::Mmr){
        let r=header.region;
        let packed_len=u64::from(r.width).div_ceil(8)*u64::from(r.height);
        // from_packed initializes, copies and canonicalizes the packed rows.
        let copy_work=packed_len*3;
        let remaining=limit.checked_sub(copy_work).ok_or(Error::Limit("JBIG2 generic MMR conversion"))?;
        let decoded=crate::filters::ccitt::decode_mmr_known_length(payload,r.width,r.height,remaining)?;
        let image=Bitmap::from_packed(r.width,r.height,&decoded.packed)?;
        Ok(DecodedRegion{region:r,bitmap:image,stats:DecodeStats::Mmr{
            mode_counts:decoded.mode_counts,encoded_bits:decoded.encoded_bits,eofb_present:decoded.eofb_present,
        }})
    }else{
        let decoded=decode_arithmetic_body(header,payload,limit)?;
        Ok(DecodedRegion{region:decoded.region,bitmap:decoded.bitmap,stats:DecodeStats::Arithmetic{
            decisions:decoded.decisions,copied_rows:decoded.copied_rows,
        }})
    }
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

    fn unhex(s:&str)->Vec<u8>{
        s.as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect()
    }
    fn fixture(w:u32,h:u32,flags:u8,adaptive:&[u8],code:&str)->Vec<u8>{
        let mut data=Vec::new();
        for n in [w,h,2,3]{data.extend(n.to_be_bytes());}
        data.push(2); // XOR, common region metadata only.
        data.push(flags);data.extend(adaptive);data.extend(unhex(code));data
    }
    // Separate offline JavaScript arithmetic encoder, Annex E interval
    // selection and byteout/flush. Shared normative probability table.
    // Pixel references come from unpacked source rows, high-to-low
    // nominal coordinate lists, independently of the Rust bitmap code.
    fn check(w:u32,flags:u8,adaptive:&[u8],code:&str,packed:&str,decisions:usize,copied:u32){
        let data=fixture(w,8,flags,adaptive,code);
        let d=decode_arithmetic(&data,MAX_GENERIC_WORK).unwrap();
        assert_eq!((d.region.width,d.region.height,d.region.x,d.region.y),(w,8,2,3));
        assert_eq!(d.region.op,bitmap::ComposeOp::Xor);
        assert_eq!((d.bitmap.width(),d.bitmap.height()),(w,8));
        assert_eq!(d.bitmap.packed(),unhex(packed).as_slice());
        assert_eq!((d.decisions,d.copied_rows),(decisions,copied));
        let again=decode_arithmetic(&data,MAX_GENERIC_WORK).unwrap();
        assert_eq!(d,again);
    }
    #[test]fn generic_decode_template0_reference(){
        check(13,0,&[3,255,253,255,2,254,254,254],"c8bddaa7e0696a8f21c215feff7fffac",
            "9130e0e00000e0e091304a50aaa85b48",104,0);
    }
    #[test]fn generic_decode_template1_prediction_reference(){
        check(14,10,&[3,255],"b7536866348bffac",
            "00000000fffcfffcfffcb4a455545554",50,5);
    }
    #[test]fn generic_decode_template2_reference(){
        check(15,4,&[2,255],"483230ed786d39a11ce4ca597ca6b3ffac",
            "1332e1e00000f0e099124a5aaaaada48",120,0);
    }
    #[test]fn generic_decode_template3_prediction_reference(){
        check(16,14,&[2,255],"b70c7c882eb7ffac",
            "00000000ffffffffffff94a555555555",56,5);
    }
    #[test]fn generic_decode_adaptive_and_narrow_reference(){
        check(17,8,&[255,0,4,255,0,253,251,254],"b42e1345ba7fffac",
            "0000000000000000000000000000004b4a00aaaa80aaaa80",42,6);
        check(1,14,&[1,254],"ba2fffac","0000808080800000",10,6);
    }
    #[test]fn generic_decode_work_limits(){
        let data=fixture(13,8,0,&[3,255,253,255,2,254,254,254],"c8bddaa7e0696a8f21c215feff7fffac");
        assert!(matches!(decode_arithmetic(&data,13*8*17-1),Err(Error::Limit(_))));
        assert!(decode_arithmetic(&data,13*8*17).is_ok());
        let data=fixture(14,8,10,&[3,255],"b7536866348bffac");
        assert!(matches!(decode_arithmetic(&data,14*8*14+7),Err(Error::Limit(_))));
        assert!(decode_arithmetic(&data,14*8*14+8).is_ok());
        let huge=fixture(10_000,10_000,0,&[3,255,253,255,2,254,254,254],"ffac");
        assert!(matches!(decode_arithmetic(&huge,u64::MAX),Err(Error::Limit(_))));
    }
    #[test]fn generic_decode_malformed_and_mmr(){
        let data=fixture(13,8,0,&[3,255,253,255,2,254,254,254],"c8bddaa7e0696a8f21c215feff7fffac");
        for end in 0..data.len(){assert!(decode_arithmetic(&data[..end],MAX_GENERIC_WORK).is_err());}
        let mut trailing=data.clone();trailing.push(0);
        assert!(decode_arithmetic(&trailing,MAX_GENERIC_WORK).is_err());
        let mut bad=data.clone();let last=bad.len()-1;bad[last]=0xad;
        assert!(decode_arithmetic(&bad,MAX_GENERIC_WORK).is_err());
        let mmr=fixture(13,8,1,&[],"ffac");
        assert!(matches!(decode_arithmetic(&mmr,MAX_GENERIC_WORK),Err(Error::Unsupported(_))));
    }

    #[test]fn generic_dispatch_arithmetic_reference(){
        let body=fixture(13,8,0,&[3,255,253,255,2,254,254,254],"c8bddaa7e0696a8f21c215feff7fffac");
        let a=decode_arithmetic(&body,MAX_GENERIC_WORK).unwrap();
        let b=decode_region(&body,MAX_GENERIC_WORK).unwrap();
        assert_eq!(a.bitmap,b.bitmap);assert_eq!(a.region,b.region);
        assert_eq!(b.stats,DecodeStats::Arithmetic{decisions:104,copied_rows:0});
        assert_eq!(b.bitmap.packed(),unhex("9130e0e00000e0e091304a50aaa85b48"));
        assert!(decode_region(&body,13*8*17-1).is_err());
    }
    #[test]fn generic_dispatch_mmr_optional_marker(){
        for code in ["f0","ff","f0010010","f001001f"]{
            let data=fixture(13,4,1,&[],code);
            let r=decode_region(&data,MAX_GENERIC_WORK).unwrap();
            assert_eq!((r.region.width,r.region.height,r.region.x,r.region.y),(13,4,2,3));
            assert_eq!(r.region.op,bitmap::ComposeOp::Xor);
            assert_eq!(r.bitmap.packed(),[0;8]);
            let mut modes=[0;9];modes[5]=4;
            assert_eq!(r.stats,DecodeStats::Mmr{mode_counts:modes,encoded_bits:if code.len()==8{28}else{4},eofb_present:code.len()==8});
            assert!(decode_arithmetic(&data,MAX_GENERIC_WORK).is_err());
        }
    }
    #[test]fn generic_dispatch_mmr_libtiff_reference(){
        // Pillow 12.3.0 / libtiff 4.7.1, 23x9, PhotometricInterpretation=1,
        // one Group 4 strip, source and decoded TIFF bytes match. The
        // second fixture removes exactly the final 24-bit EOFB.
        let no="26b91d1745d11d1745d11f576169690413082b4820bf49249a097fff1082fb5fb4474924974b61584135aa5fb6124b15d826c24a95d86125";
        let yes=format!("{no}001001");
        let expected=unhex("d2525a4a9528aaaa14aaa8544a94a8525a4a924d9232266663318c");
        for (code,present) in [(no,false),(yes.as_str(),true)]{
            let data=fixture(23,9,1,&[],code);
            let r=decode_region(&data,MAX_GENERIC_WORK).unwrap();
            assert_eq!(r.bitmap.packed(),expected.as_slice());
            match r.stats{
                DecodeStats::Mmr{mode_counts,encoded_bits,eofb_present}=>{
                    assert_eq!(eofb_present,present);assert_eq!(encoded_bits,448+if present{24}else{0});
                    assert!(mode_counts[1]>0);
                }
                _=>panic!("expected MMR"),
            }
        }
    }
    #[test]fn generic_dispatch_mmr_composition(){
        let data=fixture(13,4,1,&[],"26a09f80080080");
        let r=decode_region(&data,MAX_GENERIC_WORK).unwrap();
        assert_eq!(r.bitmap.packed(),[255,248,255,248,255,248,255,248]);
        let mut dst=Bitmap::new(9,5,false).unwrap();
        let mut budget=bitmap::ComposeBudget::new(14).unwrap();
        dst.compose_region(&r.bitmap,r.region,&mut budget).unwrap();
        assert_eq!(dst.packed(),[0,0,0,0,0,0,0x3f,0x80,0x3f,0x80]);
        assert_eq!(budget.remaining(),0);
    }
    #[test]fn generic_dispatch_mmr_budget_and_errors(){
        let data=fixture(13,4,1,&[],"f0");
        // MMR area*2 + eight input bits + four mode charges + packed copy.
        let work=13*4*2+8+4*32+8*3;
        assert!(matches!(decode_region(&data,work-1),Err(Error::Limit(_))));
        assert!(decode_region(&data,work).is_ok());
        for code in ["","e0","f000","f001","f001001000"]{
            assert!(decode_region(&fixture(13,4,1,&[],code),MAX_GENERIC_WORK).is_err());
        }
        assert!(decode_region(&fixture(13,4,9,&[],"f0"),MAX_GENERIC_WORK).is_err());
        assert!(decode_region(&fixture(13,4,0,&[3,255,253,255,2,254,254,254],"f0"),MAX_GENERIC_WORK).is_err());
    }
}
