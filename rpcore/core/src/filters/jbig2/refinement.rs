//! T.88 (2000) 6.3.5.2–6.3.5.6, Figures 12–16.
//! Generic refinement geometry, bounded QM reconstruction and scoped segment decoding.
use crate::{Error,Result};
use super::{bitmap::Bitmap,generic::Offset};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Template{T0,T1}
impl Template{
    pub fn from_code(code:u8)->Result<Self>{
        match code{0=>Ok(Self::T0),1=>Ok(Self::T1),_=>Err(Error::Invalid("JBIG2 refinement template"))}
    }
    pub fn bits(self)->usize{match self{Self::T0=>13,Self::T1=>10}}
    pub fn context_count(self)->usize{1usize<<self.bits()}
    pub fn adaptive_count(self)->usize{match self{Self::T0=>2,Self::T1=>0}}
    // Figure 14/15: all slots zero except the reference centre.
    // Reading order, target before reference, first slot is MSB.
    pub fn sltp_context(self)->usize{match self{Self::T0=>16,Self::T1=>8}}
}
const fn at(x:i8,y:i8)->Offset{Offset{x,y}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Plane{Target,Reference}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Slot{pub plane:Plane,pub offset:Offset}
const fn target(x:i8,y:i8)->Slot{Slot{plane:Plane::Target,offset:at(x,y)}}
const fn reference(x:i8,y:i8)->Slot{Slot{plane:Plane::Reference,offset:at(x,y)}}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Neighborhood{template:Template,slots:[Slot;13]}
impl Neighborhood{
    pub fn new(template:Template,adaptive:&[Offset])->Result<Self>{
        if adaptive.len()!=template.adaptive_count(){
            return Err(Error::Invalid("JBIG2 refinement adaptive count"));
        }
        let mut slots=[target(-1,0);13];
        match template{
            Template::T0=>{
                let a=adaptive[0];let b=adaptive[1];
                if a.y>0||a.y==0&&a.x>=0{
                    return Err(Error::Invalid("JBIG2 refinement noncausal target AT"));
                }
                // The reference AT can sample any signed byte offset.
                // AT positions may coincide with other slots; keep both bits.
                slots.copy_from_slice(&[
                    target(a.x,a.y),target(0,-1),target(1,-1),target(-1,0),
                    reference(b.x,b.y),reference(0,-1),reference(1,-1),
                    reference(-1,0),reference(0,0),reference(1,0),
                    reference(-1,1),reference(0,1),reference(1,1)]);
            }
            Template::T1=>slots[..10].copy_from_slice(&[
                target(-1,-1),target(0,-1),target(1,-1),target(-1,0),
                reference(0,-1),reference(-1,0),reference(0,0),
                reference(1,0),reference(0,1),reference(1,1)]),
        }
        Ok(Self{template,slots})
    }
    pub fn nominal(template:Template)->Self{
        match template{
            Template::T0=>Self::new(template,&[at(-1,-1),at(-1,-1)]),
            Template::T1=>Self::new(template,&[]),
        }.expect("valid nominal refinement template")
    }
    pub fn template(&self)->Template{self.template}
    /// In stable nominal reading order, regardless of relocated AT pixels.
    pub fn slots(&self)->&[Slot]{&self.slots[..self.template.bits()]}
    /// Reference centre is (x-dx,y-dy), never (x+dx,y+dy).
    /// Both planes are zero outside their own independent bounds.
    /// Caller must traverse target pixels in raster order.
    pub fn context(&self,dst:&Bitmap,src:&Bitmap,x:u32,y:u32,dx:i64,dy:i64)->Result<usize>{
        if x>=dst.width()||y>=dst.height(){return Err(Error::Invalid("JBIG2 refinement target coordinate"));}
        let (x,y)=(i128::from(x),i128::from(y));
        let (rx,ry)=(x-i128::from(dx),y-i128::from(dy));
        let mut cx=0usize;
        for s in self.slots(){
            let (image,bx,by)=match s.plane{Plane::Target=>(dst,x,y),Plane::Reference=>(src,rx,ry)};
            let bit=sample(image,bx+i128::from(s.offset.x),by+i128::from(s.offset.y));
            cx=(cx<<1)|usize::from(bit);
        }
        Ok(cx)
    }
}
fn sample(image:&Bitmap,x:i128,y:i128)->bool{
    let (Ok(x),Ok(y))=(u32::try_from(x),u32::try_from(y))else{return false;};
    image.get(x,y).unwrap_or(false)
}
/// Figure 16: prediction is possible iff all nine reference pixels agree.
/// Uses the complete fixed 3x3 neighborhood, independent of template and AT.
/// Caller still needs the decoded row LTP state; None means explicit decode.
pub fn typical_pixel(src:&Bitmap,x:u32,y:u32,dx:i64,dy:i64)->Option<bool>{
    let rx=i128::from(x)-i128::from(dx);
    let ry=i128::from(y)-i128::from(dy);
    let centre=sample(src,rx,ry);
    for oy in -1..=1{
        for ox in -1..=1{
            if sample(src,rx+ox,ry+oy)!=centre{return None;}
        }
    }
    Some(centre)
}

#[cfg(test)]
mod tests{
    use super::*;
    // Independent explicit coordinate lists transcribed from Figures 12/13.
    const T0:[(bool,i8,i8);13]=[
        (false,-1,-1),(false,0,-1),(false,1,-1),(false,-1,0),
        (true,-1,-1),(true,0,-1),(true,1,-1),
        (true,-1,0),(true,0,0),(true,1,0),
        (true,-1,1),(true,0,1),(true,1,1)];
    const T1:[(bool,i8,i8);10]=[
        (false,-1,-1),(false,0,-1),(false,1,-1),(false,-1,0),
        (true,0,-1),(true,-1,0),(true,0,0),(true,1,0),(true,0,1),(true,1,1)];
    #[test]
    fn refinement_all_nominal_contexts(){
        // Exhaust every bit pattern, checking separate reference and
        // target planes, using expected context as the fixture generator.
        for (t,slots) in [(Template::T0,&T0[..]),(Template::T1,&T1[..])]{
            let n=Neighborhood::nominal(t);
            assert_eq!(n.slots().len(),slots.len());
            for cx in 0..t.context_count(){
                let mut dst=Bitmap::new(3,3,false).unwrap();
                let mut src=Bitmap::new(3,3,false).unwrap();
                for (i,&(is_ref,ox,oy)) in slots.iter().enumerate(){
                    let b=if is_ref{&mut src}else{&mut dst};
                    b.set((1i16+i16::from(ox)) as u32,(1i16+i16::from(oy)) as u32,
                        cx&(1<<(slots.len()-1-i))!=0).unwrap();
                }
                assert_eq!(n.context(&dst,&src,1,1,0,0).unwrap(),cx);
            }
        }
    }
    #[test]
    fn refinement_reference_translation_and_planes(){
        let n=Neighborhood::nominal(Template::T0);
        let mut dst=Bitmap::new(6,5,false).unwrap();
        let mut src=Bitmap::new(4,3,false).unwrap();
        dst.set(2,2,true).unwrap(); // target left of (3,2)
        src.set(1,1,true).unwrap(); // reference centre (3-2,2-1)
        assert_eq!(n.context(&dst,&src,3,2,2,1).unwrap(),512|16);
        assert_eq!(n.context(&dst,&src,3,2,-1,-1).unwrap(),512);
        assert_eq!(n.context(&dst,&src,0,0,-1,-1).unwrap(),16);
        assert!(n.context(&dst,&src,6,0,0,0).is_err());
        assert!(n.context(&dst,&src,0,5,0,0).is_err());
    }
    #[test]
    fn refinement_adaptive_slot_identity(){
        let n=Neighborhood::new(Template::T0,&[at(-1,0),at(0,0)]).unwrap();
        let mut dst=Bitmap::new(3,3,false).unwrap();
        let mut src=Bitmap::new(3,3,false).unwrap();
        dst.set(0,1,true).unwrap();src.set(1,1,true).unwrap();
        assert_eq!(n.context(&dst,&src,1,1,0,0).unwrap(),4096|512|256|16);
        let n=Neighborhood::new(Template::T0,&[at(127,-128),at(-128,127)]).unwrap();
        assert_eq!(n.slots()[0].offset,at(127,-128));
        assert_eq!(n.slots()[4].offset,at(-128,127));
    }
    #[test]
    fn refinement_invalid_templates_and_causality(){
        assert!(Template::from_code(2).is_err());
        assert!(Template::from_code(255).is_err());
        assert_eq!(Template::from_code(0).unwrap(),Template::T0);
        assert_eq!(Template::from_code(1).unwrap(),Template::T1);
        for a in [at(0,0),at(1,0),at(-128,1),at(127,127)]{
            assert!(Neighborhood::new(Template::T0,&[a,at(0,0)]).is_err());
        }
        assert!(Neighborhood::new(Template::T0,&[]).is_err());
        assert!(Neighborhood::new(Template::T1,&[at(-1,-1)]).is_err());
    }
    #[test]
    fn refinement_sltp_fixed_indices(){
        let dst=Bitmap::new(3,3,false).unwrap();
        let mut src=Bitmap::new(3,3,false).unwrap();src.set(1,1,true).unwrap();
        for (t,cx) in [(Template::T0,16),(Template::T1,8)]{
            assert_eq!(t.sltp_context(),cx);
            assert_eq!(Neighborhood::nominal(t).context(&dst,&src,1,1,0,0).unwrap(),cx);
        }
        let n=Neighborhood::new(Template::T0,&[at(-1,0),at(0,0)]).unwrap();
        assert_eq!(n.template().sltp_context(),16);
        assert_ne!(n.context(&dst,&src,1,1,0,0).unwrap(),16);
    }
    #[test]
    fn refinement_typical_all_reference_patterns(){
        // Exhaust all 512 possible 3x3 reference bit patterns.
        for mask in 0u16..512{
            let mut src=Bitmap::new(3,3,false).unwrap();
            for i in 0..9{src.set(i%3,i/3,mask&(1<<i)!=0).unwrap();}
            let want=match mask{0=>Some(false),511=>Some(true),_=>None};
            assert_eq!(typical_pixel(&src,1,1,0,0),want);
            assert_eq!(typical_pixel(&src,3,2,2,1),want);
        }
    }
    #[test]
    fn refinement_edges_and_extreme_displacements(){
        let dst=Bitmap::new(2,2,true).unwrap();
        let src=Bitmap::new(3,3,true).unwrap();
        assert_eq!(typical_pixel(&src,1,1,0,0),Some(true));
        assert_eq!(typical_pixel(&src,0,0,0,0),None);
        assert_eq!(typical_pixel(&src,10,10,0,0),Some(false));
        for dx in [i64::MIN,i64::MAX]{
            for dy in [i64::MIN,i64::MAX]{
                assert_eq!(typical_pixel(&src,u32::MAX,u32::MAX,dx,dy),Some(false));
                let n=Neighborhood::nominal(Template::T0);
                assert_eq!(n.context(&dst,&src,0,0,dx,dy).unwrap(),0);
                assert_eq!(n.context(&dst,&src,1,1,dx,dy).unwrap(),4096|2048|512);
            }
        }
    }
}


/// T.88 6.3.5.6: GR statistics are independent of GB and integer banks.
/// A caller may reuse this bank and a QM stream across multiple bitmaps.
/// Each bitmap resets LTP; moving AT slots does not change their identities.
use super::{bitmap, qm::{Context,Decoder}};
pub const MAX_REFINEMENT_WORK:u64=200_000_000;
pub const MAX_REFINEMENT_DECISIONS:usize=16_777_216;

#[derive(Debug,PartialEq,Eq)]
pub struct ContextBank{template:Template,entries:Vec<Context>}
impl ContextBank{
    pub fn new(template:Template)->Result<Self>{
        let mut entries=Vec::new();
        entries.try_reserve_exact(template.context_count()).map_err(|_|Error::Limit("JBIG2 GR context allocation"))?;
        entries.resize(template.context_count(),Context::default());
        Ok(Self{template,entries})
    }
    pub fn template(&self)->Template{self.template}
    pub fn entries(&self)->&[Context]{&self.entries}
    pub fn reset(&mut self){self.entries.fill(Context::default());}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RefinementOptions{
    pub width:u32,pub height:u32,pub dx:i64,pub dy:i64,pub typical_prediction:bool,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RefinementStats{
    pub explicit_pixels:u64,pub implicit_pixels:u64,pub typical_rows:u32,
    pub decisions:usize,pub work_bound:u64,
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedRefinement{pub bitmap:Bitmap,pub stats:RefinementStats}

/// Conservative upper bound: every pixel could require full context sampling,
/// a write and (with TPGR) ten reference samples; each row may decode SLTP.
/// This is an algorithmic work unit, not elapsed time or RSS.
pub fn work_bound(template:Template,opt:RefinementOptions)->Result<u64>{
    let area=u64::from(opt.width)*u64::from(opt.height);
    if opt.width==0||opt.height==0{return Err(Error::Invalid("JBIG2 refinement dimensions"));}
    if opt.width>bitmap::MAX_SIDE||opt.height>bitmap::MAX_SIDE||area>bitmap::MAX_PIXELS{
        return Err(Error::Limit("JBIG2 refinement dimensions"));
    }
    let bytes=u64::from(opt.width).div_ceil(8)*u64::from(opt.height);
    if bytes>bitmap::MAX_PACKED_BYTES as u64{return Err(Error::Limit("JBIG2 refinement packed bytes"));}
    let per=template.bits()as u64+1+if opt.typical_prediction{10}else{0};
    area.checked_mul(per).and_then(|v|v.checked_add(if opt.typical_prediction{u64::from(opt.height)}else{0}))
        .ok_or(Error::Limit("JBIG2 refinement work"))
}

/// Decode one bitmap on a shared arithmetic stream. Dimension/template/work
/// checks precede mutation; errors during decoding may advance bank and stream.
/// Implicit pixels do not consume decisions or update GR statistics. The SLTP
/// bit shares the GR bank with explicit pixels. No row copying.
pub fn decode_bitmap(src:&Bitmap,neighborhood:&Neighborhood,opt:RefinementOptions,
    bank:&mut ContextBank,decoder:&mut Decoder<'_>,work_limit:u64)->Result<DecodedRefinement>{
    let template=neighborhood.template();
    if bank.template!=template{return Err(Error::Invalid("JBIG2 GR context template"));}
    let work=work_bound(template,opt)?;
    if work>work_limit.min(MAX_REFINEMENT_WORK){return Err(Error::Limit("JBIG2 refinement work"));}
    let mut image=Bitmap::new(opt.width,opt.height,false)?;
    let mut stats=RefinementStats{explicit_pixels:0,implicit_pixels:0,typical_rows:0,decisions:0,work_bound:work};
    let mut ltp=false;
    for y in 0..opt.height{
        if opt.typical_prediction{
            let sltp=decoder.decode(&mut bank.entries[template.sltp_context()])?!=0;
            stats.decisions+=1;
            ltp^=sltp;
            if ltp{stats.typical_rows+=1;}
        }
        for x in 0..opt.width{
            let predicted=if ltp{typical_pixel(src,x,y,opt.dx,opt.dy)}else{None};
            let bit=if let Some(v)=predicted{
                stats.implicit_pixels+=1;v
            }else{
                let cx=neighborhood.context(&image,src,x,y,opt.dx,opt.dy)?;
                let v=decoder.decode(&mut bank.entries[cx])?!=0;
                stats.explicit_pixels+=1;
                stats.decisions+=1;v
            };
            image.set(x,y,bit)?;
        }
    }
    Ok(DecodedRefinement{bitmap:image,stats})
}

/// Fresh standalone entropy payload (explicit final FF AC marker required).
/// The shared-stream entry above does not create or finish a QM decoder.
/// Neither entry validates segment headers, reference selection or integrity.
pub fn decode_arithmetic(data:&[u8],src:&Bitmap,neighborhood:&Neighborhood,opt:RefinementOptions,
    work_limit:u64,decision_limit:usize)->Result<DecodedRefinement>{
    let work=work_bound(neighborhood.template(),opt)?;
    // Include input scanning and fresh context initialization in this wrapper.
    let overhead=(data.len() as u64).checked_add(neighborhood.template().context_count()as u64)
        .ok_or(Error::Limit("JBIG2 refinement work"))?;
    let remaining=work_limit.min(MAX_REFINEMENT_WORK).checked_sub(overhead)
        .ok_or(Error::Limit("JBIG2 refinement work"))?;
    if work>remaining||decision_limit>MAX_REFINEMENT_DECISIONS{return Err(Error::Limit("JBIG2 refinement budget"));}
    let mut decoder=Decoder::new(data,decision_limit)?;
    let mut bank=ContextBank::new(neighborhood.template())?;
    let mut result=decode_bitmap(src,neighborhood,opt,&mut bank,&mut decoder,remaining)?;
    result.stats.work_bound+=overhead;
    Ok(result)
}

#[cfg(test)]
mod decode_tests{
    use super::*;
    // Offline JS Annex E interval encoder, independent of Rust decode
    // equations, verified with a JS reference reader. Original synthetic
    // target/source pixels and row SLTP sequences, no production dependency.
    const REF:[u8;6]=[0,124,124,124,0,0];
    const A:[u8;7]=[192,34,140,221,147,255,172];
    const B:[u8;7]=[56,108,187,186,43,255,172];
    const C:[u8;7]=[62,159,39,101,31,255,172];
    const SHARED:[u8;12]=[192,34,140,221,148,194,215,177,162,223,255,172];
    fn src()->Bitmap{Bitmap::from_packed(7,6,&REF).unwrap()}
    fn opts(width:u32,dx:i64,dy:i64,tp:bool)->RefinementOptions{
        RefinementOptions{width,height:6,dx,dy,typical_prediction:tp}
    }
    fn relocated()->Neighborhood{Neighborhood::new(Template::T0,&[at(-1,0),at(0,0)]).unwrap()}
    fn check(d:&DecodedRefinement,pixels:&[u8],explicit:u64,implicit:u64,rows:u32,decisions:usize){
        assert_eq!(d.bitmap.packed(),pixels);
        assert_eq!((d.stats.explicit_pixels,d.stats.implicit_pixels,d.stats.typical_rows,d.stats.decisions),(explicit,implicit,rows,decisions));
    }
    #[test]
    fn refinement_decode_t0_explicit(){
        let d=decode_arithmetic(&A,&src(),&Neighborhood::nominal(Template::T0),opts(7,0,0,false),MAX_REFINEMENT_WORK,42).unwrap();
        check(&d,&[170,56,56,198,170,84],42,0,0,42);
        assert_eq!((d.bitmap.width(),d.bitmap.height()),(7,6));
    }
    #[test]
    fn refinement_decode_t1_shifted_typical(){
        // SLTP 001110 -> LTP 001011.
        let d=decode_arithmetic(&B,&src(),&Neighborhood::nominal(Template::T1),opts(8,1,-1,true),MAX_REFINEMENT_WORK,37).unwrap();
        check(&d,&[170,56,42,85,0,0],31,17,3,37);
    }
    #[test]
    fn refinement_decode_t0_relocated_typical(){
        // SLTP 010101, LTP 011001: mix implicit black, implicit white
        // and explicitly decoded pixels even on typical rows.
        let d=decode_arithmetic(&C,&src(),&relocated(),opts(7,0,0,true),MAX_REFINEMENT_WORK,38).unwrap();
        check(&d,&[170,170,254,170,84,0],32,10,3,38);
    }
    #[test]
    fn refinement_decode_shared_stream_and_bank(){
        let s=src();let n=Neighborhood::nominal(Template::T0);
        let mut bank=ContextBank::new(Template::T0).unwrap();
        let mut qm=Decoder::new(&SHARED,80).unwrap();
        let a=decode_bitmap(&s,&n,opts(7,0,0,false),&mut bank,&mut qm,MAX_REFINEMENT_WORK).unwrap();
        check(&a,&[170,56,56,198,170,84],42,0,0,42);
        assert_eq!(bank.entries().iter().filter(|&&v|v!=Context::default()).count(),36);
        let c=decode_bitmap(&s,&relocated(),opts(7,0,0,true),&mut bank,&mut qm,MAX_REFINEMENT_WORK).unwrap();
        check(&c,&[170,170,254,170,84,0],32,10,3,38);
        assert_eq!(bank.entries().iter().filter(|&&v|v!=Context::default()).count(),56);
        assert!(qm.decode(&mut Context::default()).is_err());
        bank.reset();assert!(bank.entries().iter().all(|&v|v==Context::default()));
    }
    #[test]
    fn refinement_decode_exact_budgets(){
        let s=src();let n=Neighborhood::nominal(Template::T1);let o=opts(8,1,-1,true);
        let d=decode_arithmetic(&B,&s,&n,o,MAX_REFINEMENT_WORK,37).unwrap();
        assert_eq!(d.stats.work_bound,48*21+6+1024+7);
        assert_eq!(decode_arithmetic(&B,&s,&n,o,d.stats.work_bound,37).unwrap(),d);
        assert!(decode_arithmetic(&B,&s,&n,o,d.stats.work_bound-1,37).is_err());
        assert!(decode_arithmetic(&B,&s,&n,o,d.stats.work_bound,36).is_err());
        assert!(decode_arithmetic(&B,&s,&n,o,d.stats.work_bound,MAX_REFINEMENT_DECISIONS+1).is_err());
    }
    #[test]
    fn refinement_decode_marker_and_preflight(){
        let s=src();let n=Neighborhood::nominal(Template::T0);let o=opts(7,0,0,false);
        for i in 0..A.len(){assert!(decode_arithmetic(&A[..i],&s,&n,o,MAX_REFINEMENT_WORK,42).is_err());}
        let mut bad=A.to_vec();bad.push(0);
        assert!(decode_arithmetic(&bad,&s,&n,o,MAX_REFINEMENT_WORK,42).is_err());
        let mut bank=ContextBank::new(Template::T1).unwrap();let mut qm=Decoder::new(&A,42).unwrap();
        assert!(decode_bitmap(&s,&n,o,&mut bank,&mut qm,MAX_REFINEMENT_WORK).is_err());
        let mut bank=ContextBank::new(Template::T0).unwrap();
        for opt in [RefinementOptions{width:0,..o},RefinementOptions{height:0,..o},
            RefinementOptions{width:u32::MAX,..o},RefinementOptions{width:bitmap::MAX_SIDE,height:bitmap::MAX_SIDE,..o}]{
            assert!(decode_bitmap(&s,&n,opt,&mut bank,&mut qm,MAX_REFINEMENT_WORK).is_err());
        }
        assert!(decode_bitmap(&s,&n,o,&mut bank,&mut qm,0).is_err());
        assert!(bank.entries().iter().all(|&v|v==Context::default()));
        check(&decode_bitmap(&s,&n,o,&mut bank,&mut qm,MAX_REFINEMENT_WORK).unwrap(),&[170,56,56,198,170,84],42,0,0,42);
    }
    #[test]
    fn refinement_decode_prediction_only(){
        // A fresh zero reference far away, FF AC yields first SLTP=1
        // with initial context. Single typical row needs no pixel decisions.
        let s=Bitmap::new(1,1,false).unwrap();let n=Neighborhood::nominal(Template::T1);
        let o=RefinementOptions{width:9,height:1,dx:i64::MAX,dy:i64::MIN,typical_prediction:true};
        // Explicit independent short encoding for SLTP=1.
        let mut bank=ContextBank::new(Template::T1).unwrap();
        let mut qm=Decoder::new(&[255,172],1).unwrap();
        let d=decode_bitmap(&s,&n,o,&mut bank,&mut qm,MAX_REFINEMENT_WORK).unwrap();
        check(&d,&[0,0],0,9,1,1);
        assert_eq!(bank.entries().iter().filter(|&&v|v!=Context::default()).count(),1);
    }
}


/// Known-length generic refinement region segments (T.88 7.4.7, Table 35).
/// Header parsing leaves entropy opaque; segment-level displacement is zero.
use super::{Segment,bitmap::{RegionInfo,ComposeOp}};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RegionKind{Intermediate,Immediate,ImmediateLossless}
impl RegionKind{
    pub fn from_code(code:u8)->Result<Self>{
        match code{40=>Ok(Self::Intermediate),42=>Ok(Self::Immediate),43=>Ok(Self::ImmediateLossless),
            _=>Err(Error::Invalid("JBIG2 refinement segment kind"))}
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct RegionHeader{
    pub region:RegionInfo,
    pub neighborhood:Neighborhood,
    pub typical_prediction:bool,
}
pub fn parse_region(data:&[u8])->Result<(RegionHeader,&[u8])>{
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JBIG2 refinement segment bytes"));}
    let (region,rest)=bitmap::parse_region(data)?;
    let (&flags,rest)=rest.split_first().ok_or(Error::Invalid("JBIG2 refinement flags"))?;
    if flags&0xfc!=0{return Err(Error::Invalid("JBIG2 refinement reserved flags"));}
    let template=Template::from_code(flags&1)?;
    let n=template.adaptive_count()*2;
    let bytes=rest.get(..n).ok_or(Error::Invalid("JBIG2 refinement AT truncated"))?;
    let mut at_pixels=[at(0,0);2];
    for (i,p) in bytes.chunks_exact(2).enumerate(){at_pixels[i]=at(p[0] as i8,p[1] as i8);}
    let neighborhood=Neighborhood::new(template,&at_pixels[..n/2])?;
    Ok((RegionHeader{region,neighborhood,typical_prediction:flags&2!=0},&rest[n..]))
}
/// Borrowed live auxiliary buffer or current page. The caller must own
/// provenance/liveness and must not supply a previously consumed auxiliary
/// buffer. This scoped API does not execute or mutate the page graph.
pub enum RegionReference<'a>{
    Auxiliary{segment:&'a Segment,region:RegionInfo,bitmap:&'a Bitmap},
    Page{number:u32,bitmap:&'a Bitmap},
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedRegion{
    pub kind:RegionKind,
    pub region:RegionInfo,
    pub reference_segment:Option<u32>,
    pub bitmap:Bitmap,
    pub stats:RefinementStats,
}
/// Decode a known-length segment from its inspected byte span and an explicit
/// reference. No page painting, retention handling or cross-segment state.
/// For page references, this subset requires the rectangle to be fully
/// inside the current page; clipped/striped page refinement is not implemented.
/// Auxiliary buffers must be same-page, with identical region metadata.
pub fn decode_segment(segment:&Segment,stream:&[u8],reference:RegionReference<'_>,
    work_limit:u64,decision_limit:usize)->Result<DecodedRegion>{
    let kind=RegionKind::from_code(segment.kind)?;
    if segment.page==0{return Err(Error::Unsupported("JBIG2 refinement page zero"));}
    let data=stream.get(segment.payload.clone()).ok_or(Error::Invalid("JBIG2 refinement payload span"))?;
    let (header,payload)=parse_region(data)?;
    let r=header.region;
    let opt=RefinementOptions{width:r.width,height:r.height,dx:0,dy:0,typical_prediction:header.typical_prediction};
    let limit=work_limit.min(MAX_REFINEMENT_WORK);
    let header_work=(data.len()-payload.len())as u64;
    let mut remaining=limit.checked_sub(header_work).ok_or(Error::Limit("JBIG2 refinement header work"))?;
    let mut reference_segment=None;
    let crop;
    let src=match reference{
        RegionReference::Auxiliary{segment:dep,region,bitmap}=>{
            if segment.referred.as_slice()!=[dep.number]||dep.number>=segment.number||
                !matches!(dep.kind,4|20|36|40)||dep.page!=segment.page{
                return Err(Error::Invalid("JBIG2 refinement auxiliary reference"));
            }
            if r!=region||(bitmap.width(),bitmap.height())!=(r.width,r.height){
                return Err(Error::Invalid("JBIG2 refinement auxiliary region mismatch"));
            }
            reference_segment=Some(dep.number);
            bitmap
        }
        RegionReference::Page{number,bitmap}=>{
            if kind==RegionKind::Intermediate||!segment.referred.is_empty()||number!=segment.page||r.op!=ComposeOp::Replace{
                return Err(Error::Invalid("JBIG2 refinement page reference"));
            }
            if u64::from(r.x)+u64::from(r.width)>u64::from(bitmap.width())||
                u64::from(r.y)+u64::from(r.height)>u64::from(bitmap.height()){
                return Err(Error::Unsupported("JBIG2 refinement page crop outside bounds"));
            }
            // Bound crop + target allocations before any allocation. Each
            // bitmap is already individually bounded by parse_region.
            let bytes=u64::from(r.width).div_ceil(8)*u64::from(r.height);
            if bytes*2>bitmap::MAX_PACKED_BYTES as u64{return Err(Error::Limit("JBIG2 refinement crop and target bytes"));}
            let crop_work=u64::from(r.width)*u64::from(r.height)*2+bytes;
            remaining=remaining.checked_sub(crop_work).ok_or(Error::Limit("JBIG2 refinement crop work"))?;
            let decode_work=work_bound(header.neighborhood.template(),opt)?
                .checked_add(payload.len()as u64+header.neighborhood.template().context_count()as u64)
                .ok_or(Error::Limit("JBIG2 refinement total work"))?;
            if decode_work>remaining||decision_limit>MAX_REFINEMENT_DECISIONS{return Err(Error::Limit("JBIG2 refinement budget"));}
            let mut image=Bitmap::new(r.width,r.height,false)?;
            for y in 0..r.height{for x in 0..r.width{
                let bit=bitmap.get(r.x+x,r.y+y).ok_or(Error::Invalid("JBIG2 refinement crop coordinate"))?;
                image.set(x,y,bit)?;
            }}
            crop=image;
            &crop
        }
    };
    let mut d=decode_arithmetic(payload,src,&header.neighborhood,opt,remaining,decision_limit)?;
    d.stats.work_bound+=limit-remaining;
    Ok(DecodedRegion{kind,region:r,reference_segment,bitmap:d.bitmap,stats:d.stats})
}

#[cfg(test)]
mod segment_tests{
    use super::*;
    // Prior independently encoded and cross-platform verified g2 vectors.
    const A:&[u8]=&[192,34,140,221,147,255,172];
    const B:&[u8]=&[56,108,187,186,43,255,172];
    const C:&[u8]=&[62,159,39,101,31,255,172];
    fn body(w:u32,h:u32,x:u32,y:u32,op:u8,flags:u8,at:&[u8],entropy:&[u8])->Vec<u8>{
        let mut b=Vec::new();for n in [w,h,x,y]{b.extend(n.to_be_bytes());}
        b.extend([op,flags]);b.extend(at);b.extend(entropy);b
    }
    fn seg(number:u32,kind:u8,page:u32,refs:&[u32],len:usize)->Segment{
        Segment{number,kind,deferred_non_retain:false,retention:vec![],referred:refs.to_vec(),page,header:0..0,payload:0..len}
    }
    fn src()->Bitmap{Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap()}
    #[test]
    fn refinement_segment_headers(){
        for flags in 0..4{
            let at:&[u8]=if flags&1==0{&[255,255,128,127]}else{&[]};
            let b=body(7,6,12,20,2,flags,at,A);
            let (h,p)=parse_region(&b).unwrap();
            assert_eq!((h.region.width,h.region.height,h.region.x,h.region.y,h.region.op),(7,6,12,20,ComposeOp::Xor));
            assert_eq!(h.neighborhood.template(),if flags&1==0{Template::T0}else{Template::T1});
            assert_eq!(h.typical_prediction,flags&2!=0);
            assert_eq!(p,A);
            if flags&1==0{assert_eq!(h.neighborhood.slots()[4].offset,super::at(-128,127));}
            for i in 0..b.len()-A.len(){assert!(parse_region(&b[..i]).is_err());}
        }
    }
    #[test]
    fn refinement_segment_all_kinds_and_reference(){
        let b=body(7,6,2,3,0,0,&[255;4],A);
        let r=parse_region(&b).unwrap().0.region;
        let source=src();
        for kind in [40,42,43]{
            let s=seg(9,kind,1,&[3],b.len());
            for dep_kind in [4,20,36,40]{
                let dep=seg(3,dep_kind,1,&[],0);
                let d=decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region:r,bitmap:&source},MAX_REFINEMENT_WORK,42).unwrap();
                assert_eq!(d.kind,RegionKind::from_code(kind).unwrap());
                assert_eq!(d.region,r);assert_eq!(d.reference_segment,Some(3));
                assert_eq!(d.bitmap.packed(),[170,56,56,198,170,84]);
                assert_eq!(d.stats.decisions,42);
            }
        }
    }
    #[test]
    fn refinement_segment_t1_zero_displacement(){
        // Move g2's shifted source into local region coordinates. The old
        // row 0 and all samples outside the new region are zero.
        let source=Bitmap::from_packed(8,6,&[62,62,62,0,0,0]).unwrap();
        let b=body(8,6,5,9,1,3,&[],B);
        let r=parse_region(&b).unwrap().0.region;
        let dep=seg(1,36,1,&[],0);let s=seg(2,40,1,&[1],b.len());
        let d=decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region:r,bitmap:&source},MAX_REFINEMENT_WORK,37).unwrap();
        assert_eq!(d.bitmap.packed(),[170,56,42,85,0,0]);
        assert_eq!((d.stats.implicit_pixels,d.stats.explicit_pixels,d.stats.decisions),(17,31,37));
    }
    #[test]
    fn refinement_segment_page_crop_replace(){
        let b=body(7,6,3,2,4,2,&[255,0,0,0],C);
        let s=seg(8,43,2,&[],b.len());
        // Outside-crop pixels are deliberately black: GR must see only the
        // cropped reference, not neighboring page pixels.
        let mut page=Bitmap::new(13,11,true).unwrap();let source=src();
        for y in 0..6{for x in 0..7{page.set(x+3,y+2,source.get(x,y).unwrap()).unwrap();}}
        let original=page.packed().to_vec();
        let d=decode_segment(&s,&b,RegionReference::Page{number:2,bitmap:&page},MAX_REFINEMENT_WORK,38).unwrap();
        assert_eq!(d.bitmap.packed(),[170,170,254,170,84,0]);
        assert_eq!(d.reference_segment,None);assert_eq!(d.stats.implicit_pixels,10);
        assert_eq!(page.packed(),original);
        let d2=decode_segment(&s,&b,RegionReference::Page{number:2,bitmap:&page},d.stats.work_bound,38).unwrap();
        assert_eq!(d,d2);
        assert!(decode_segment(&s,&b,RegionReference::Page{number:2,bitmap:&page},d.stats.work_bound-1,38).is_err());
    }
    #[test]
    fn refinement_segment_invalid_auxiliary(){
        let b=body(7,6,2,3,0,0,&[255;4],A);let r=parse_region(&b).unwrap().0.region;
        let source=src();
        let s=seg(9,40,1,&[3],b.len());
        for dep in [seg(2,36,1,&[],0),seg(3,38,1,&[],0),seg(3,0,1,&[],0),seg(3,36,2,&[],0)]{
            assert!(decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region:r,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
        let dep=seg(3,36,1,&[],0);
        for region in [RegionInfo{x:3,..r},RegionInfo{y:4,..r},RegionInfo{width:8,..r},RegionInfo{op:ComposeOp::And,..r}]{
            assert!(decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
        let wrong=Bitmap::new(8,6,false).unwrap();
        assert!(decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region:r,bitmap:&wrong},MAX_REFINEMENT_WORK,42).is_err());
        let s=seg(9,40,1,&[3,4],b.len());
        assert!(decode_segment(&s,&b,RegionReference::Auxiliary{segment:&dep,region:r,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
    }
    #[test]
    fn refinement_segment_invalid_page_reference(){
        let source=src();
        for (kind,op,x,refs,number) in [(40,4,0,&[][..],1),(42,0,0,&[][..],1),
            (42,4,1,&[][..],1),(42,4,u32::MAX,&[][..],1),(43,4,0,&[2][..],1),(43,4,0,&[][..],2)]{
            let b=body(7,6,x,0,op,0,&[255;4],A);let s=seg(9,kind,1,refs,b.len());
            assert!(decode_segment(&s,&b,RegionReference::Page{number,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
    }
    #[test]
    fn refinement_segment_malformed_and_spans(){
        let source=src();let b=body(7,6,0,0,4,0,&[255;4],A);
        for i in 0..b.len(){
            let s=seg(9,42,1,&[],i);
            assert!(decode_segment(&s,&b,RegionReference::Page{number:1,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
        for (index,value) in [(17,4),(18,0),(19,0),(16,7)]{
            let mut bad=b.clone();bad[index]=value;
            if index==18{bad[19]=0;}
            let s=seg(9,42,1,&[],bad.len());
            assert!(decode_segment(&s,&bad,RegionReference::Page{number:1,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
        for (kind,page,len) in [(38,1,b.len()),(42,0,b.len()),(42,1,usize::MAX)]{
            let s=seg(9,kind,page,&[],len);
            assert!(decode_segment(&s,&b,RegionReference::Page{number:1,bitmap:&source},MAX_REFINEMENT_WORK,42).is_err());
        }
    }
}
