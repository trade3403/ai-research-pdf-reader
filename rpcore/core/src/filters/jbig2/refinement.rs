//! T.88 (2000) 6.3.5.2–6.3.5.6, Figures 12–16.
//! Generic refinement context geometry only. No arithmetic decisions yet.
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
