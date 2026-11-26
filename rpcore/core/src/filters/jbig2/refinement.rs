//! T.88 (2000) 6.3.5.2–6.3.5.6, Figures 12–16.
//! Generic refinement geometry and bounded native QM pixel reconstruction.
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
