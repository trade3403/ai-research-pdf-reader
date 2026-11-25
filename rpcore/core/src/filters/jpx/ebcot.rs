//! JPEG 2000 Tier-1 context modelling for the baseline MQ coding style.
//! Baseline MQ codeblock passes and signed integer coefficient reconstruction.
use super::{geometry::Band,mq::{Context,Decoder}};
use crate::{Error,Result};

pub const ZERO_CONTEXTS:usize=9;
pub const SIGN_CONTEXT_BASE:usize=9;
pub const MAG_CONTEXT_BASE:usize=14;
pub const AGG_CONTEXT:usize=17;
pub const UNI_CONTEXT:usize=18;
pub const CONTEXT_COUNT:usize=19;

/// Counts of significant horizontal, vertical and diagonal neighbours.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Neighbours{pub h:u8,pub v:u8,pub d:u8}
pub fn zero_context(band:Band,n:Neighbours)->Result<usize>{
    let Neighbours{mut h,mut v,d}=n;
    if h>2||v>2||d>4{return Err(Error::Invalid("JPX EBCOT neighbour count"));}
    if band==Band::Hl{std::mem::swap(&mut h,&mut v);}
    if band==Band::Hh{
        let hv=h+v;
        return Ok(match d{
            0=>match hv{0=>0,1=>1,_=>2},
            1=>match hv{0=>3,1=>4,_=>5},
            2=>if hv==0{6}else{7},
            _=>8,
        });
    }
    Ok(match h{
        0=>match v{0=>match d{0=>0,1=>1,_=>2},1=>3,_=>4},
        1=>if v!=0{7}else if d==0{5}else{6},
        _=>8,
    })
}
/// Signed contributions are the clipped sums of two significant neighbours
/// on each axis: positive sample +1, negative sample -1.
pub fn sign_context(h:i8,v:i8)->Result<(usize,u8)>{
    if !(-1..=1).contains(&h)||!(-1..=1).contains(&v){
        return Err(Error::Invalid("JPX EBCOT sign contribution"));
    }
    let prediction=u8::from(h<0||(h==0&&v<0));
    let (h,v)=if h<0{(-h,-v)}else{(h,v)};
    let cx=if h==0{if v==0{0}else{1}}else{match v{-1=>2,0=>3,_=>4}};
    Ok((SIGN_CONTEXT_BASE+cx,prediction))
}
pub fn magnitude_context(refined:bool,significant_neighbour:bool)->usize{
    MAG_CONTEXT_BASE+if refined{2}else{usize::from(significant_neighbour)}
}
/// Baseline Tier-1 reset: zero context 0 starts at state 4,
/// aggregation at state 3, uniform at state 46; all others state 0.
pub struct Contexts{contexts:[Context;CONTEXT_COUNT]}
impl Contexts{
    pub fn new()->Result<Self>{
        let mut contexts=[Context::new(0,0)?;CONTEXT_COUNT];
        contexts[0]=Context::new(4,0)?;
        contexts[AGG_CONTEXT]=Context::new(3,0)?;
        contexts[UNI_CONTEXT]=Context::new(46,0)?;
        Ok(Self{contexts})
    }
    pub fn get(&self,index:usize)->Result<Context>{
        self.contexts.get(index).copied().ok_or(Error::Invalid("JPX EBCOT context index"))
    }
    pub fn decode(&mut self,mq:&mut Decoder<'_>,index:usize)->Result<u8>{
        mq.decode(self.contexts.get_mut(index).ok_or(Error::Invalid("JPX EBCOT context index"))?)
    }
}

/// Bounded raster storage; a coefficient becomes significant exactly once.
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct Coefficient{magnitude:u32,negative:bool,refined:bool,visited:bool}
impl Coefficient{
    pub fn significant(self)->bool{self.magnitude!=0}
    pub fn magnitude(self)->u32{self.magnitude}
    pub fn negative(self)->bool{self.negative}
    pub fn refined(self)->bool{self.refined}
    pub fn visited(self)->bool{self.visited}
    pub fn signed(self)->i32{if self.negative{-(self.magnitude as i32)}else{self.magnitude as i32}}
}
pub struct Block{width:usize,height:usize,coefficients:Vec<Coefficient>}
impl Block{
    pub fn new(width:usize,height:usize)->Result<Self>{
        let count=width.checked_mul(height).ok_or(Error::Limit("JPX EBCOT block area"))?;
        if width==0||height==0||width>1024||height>1024||count>4096{
            return Err(Error::Limit("JPX EBCOT block dimensions"));
        }
        Ok(Self{width,height,coefficients:vec![Coefficient::default();count]})
    }
    /// Row-major signed quantized coefficient values (no dequantization).
    pub fn signed_coefficients(&self)->Vec<i32>{self.coefficients.iter().map(|c|c.signed()).collect()}
    fn index(&self,x:usize,y:usize)->Result<usize>{
        if x>=self.width||y>=self.height{return Err(Error::Invalid("JPX EBCOT coordinate"));}
        Ok(y*self.width+x)
    }
    pub fn coefficient(&self,x:usize,y:usize)->Result<Coefficient>{
        Ok(self.coefficients[self.index(x,y)?])
    }
    pub fn make_significant(&mut self,x:usize,y:usize,plane:u8,negative:bool)->Result<()>{
        let i=self.index(x,y)?;
        if plane>30||self.coefficients[i].significant(){return Err(Error::Invalid("JPX EBCOT significance"));}
        self.coefficients[i]=Coefficient{magnitude:1u32<<plane,negative,refined:false,visited:false};Ok(())
    }
    fn sample(&self,x:i32,y:i32)->i8{
        if x<0||y<0{return 0;}
        match self.coefficient(x as usize,y as usize){
            Ok(c) if c.significant()=>if c.negative{-1}else{1},
            _=>0,
        }
    }
    pub fn neighbours(&self,x:usize,y:usize)->Result<(Neighbours,i8,i8)>{
        self.index(x,y)?;
        let (x,y)=(x as i32,y as i32);
        let h=[self.sample(x-1,y),self.sample(x+1,y)];
        let v=[self.sample(x,y-1),self.sample(x,y+1)];
        let d=[self.sample(x-1,y-1),self.sample(x+1,y-1),self.sample(x-1,y+1),self.sample(x+1,y+1)];
        Ok((Neighbours{h:h.iter().filter(|&&s|s!=0).count() as u8,
            v:v.iter().filter(|&&s|s!=0).count() as u8,
            d:d.iter().filter(|&&s|s!=0).count() as u8},
            (h[0]+h[1]).clamp(-1,1),(v[0]+v[1]).clamp(-1,1)))
    }
}

fn cleanup(b:&mut Block,band:Band,plane:u8,mut read:impl FnMut(usize)->Result<u8>)->Result<()>{
    if plane>30{return Err(Error::Invalid("JPX EBCOT plane"));}
    for top in (0..b.height).step_by(4){
        let bottom=(top+4).min(b.height);
        for x in 0..b.width{
            let mut y=top;
            let mut implicit=false;
            if bottom-top==4{
                let mut eligible=true;
                for row in top..bottom{
                    let c=b.coefficient(x,row)?;
                    let (n,_,_)=b.neighbours(x,row)?;
                    if c.significant()||c.visited||n.h+n.v+n.d!=0{eligible=false;break;}
                }
                if eligible{
                    if read(AGG_CONTEXT)?==0{continue;}
                    let hi=usize::from(read(UNI_CONTEXT)?);
                    let lo=usize::from(read(UNI_CONTEXT)?);
                    if hi>1||lo>1{return Err(Error::Invalid("JPX EBCOT run bit"));}
                    y+=2*hi+lo;
                    if y>=bottom{return Err(Error::Invalid("JPX EBCOT run position"));}
                    implicit=true;
                }
            }
            for row in y..bottom{
                let c=b.coefficient(x,row)?;
                if c.significant()||c.visited{continue;}
                let (n,h,v)=b.neighbours(x,row)?;
                if (implicit&&row==y)||read(zero_context(band,n)?)?!=0{
                    let (cx,pred)=sign_context(h,v)?;
                    let neg=(read(cx)?^pred)!=0;
                    b.make_significant(x,row,plane,neg)?;
                }
            }
        }
    }
    for c in &mut b.coefficients{c.visited=false;}
    Ok(())
}
fn initial_cleanup(width:usize,height:usize,band:Band,plane:u8,read:impl FnMut(usize)->Result<u8>)->Result<Block>{
    let mut b=Block::new(width,height)?;
    cleanup(&mut b,band,plane,read)?;
    Ok(b)
}
/// Decode the first (highest nonempty) cleanup bitplane of one baseline
/// MQ codeblock. No bypass, reset, segmentation symbol or later passes.
/// Contexts and decoder are fresh and private: errors return no partial block.
pub fn decode_initial_cleanup(data:&[u8],width:usize,height:usize,band:Band,plane:u8)->Result<Block>{
    let mut cx=Contexts::new()?;
    let mut mq=Decoder::new(data,4096*4)?;
    initial_cleanup(width,height,band,plane,|i|cx.decode(&mut mq,i))
}

fn propagation(b:&mut Block,band:Band,plane:u8,mut read:impl FnMut(usize)->Result<u8>)->Result<()>{
    if plane>30{return Err(Error::Invalid("JPX EBCOT plane"));}
    for top in (0..b.height).step_by(4){
        for x in 0..b.width{
            for y in top..(top+4).min(b.height){
                let i=b.index(x,y)?;
                let c=b.coefficients[i];
                if c.significant()||c.visited{continue;}
                let (n,h,v)=b.neighbours(x,y)?;
                if n.h+n.v+n.d==0{continue;}
                let bit=read(zero_context(band,n)?)?;
                if bit!=0{
                    let (cx,pred)=sign_context(h,v)?;
                    let negative=(read(cx)?^pred)!=0;
                    b.make_significant(x,y,plane,negative)?;
                }
                b.coefficients[i].visited=true;
            }
        }
    }
    Ok(())
}
/// Decode the first cleanup and the following significance propagation
/// from the same MQ segment, preserving all 19 probability contexts.
/// The result is a partial coefficient block, not a completed bitplane.
pub fn decode_initial_and_propagation(data:&[u8],width:usize,height:usize,band:Band,plane:u8)->Result<Block>{
    if !(1..=30).contains(&plane){return Err(Error::Invalid("JPX EBCOT propagation plane"));}
    let mut cx=Contexts::new()?;
    let mut mq=Decoder::new(data,4096*6)?;
    let mut b=initial_cleanup(width,height,band,plane,|i|cx.decode(&mut mq,i))?;
    propagation(&mut b,band,plane-1,|i|cx.decode(&mut mq,i))?;
    Ok(b)
}

fn refinement(b:&mut Block,plane:u8,mut read:impl FnMut(usize)->Result<u8>)->Result<()>{
    if plane>29{return Err(Error::Invalid("JPX EBCOT refinement plane"));}
    let mask=1u32<<plane;
    // Only coefficients significant before this bitplane may be refined.
    // Check the plane before consuming decisions.
    if b.coefficients.iter().any(|c|c.significant()&&!c.visited&&c.magnitude<2*mask){
        return Err(Error::Invalid("JPX EBCOT refinement order"));
    }
    for top in (0..b.height).step_by(4){
        for x in 0..b.width{
            for y in top..(top+4).min(b.height){
                let i=b.index(x,y)?;
                let c=b.coefficients[i];
                if !c.significant()||c.visited{continue;}
                let (n,_,_)=b.neighbours(x,y)?;
                let bit=read(magnitude_context(c.refined,n.h+n.v+n.d!=0))?;
                if bit>1{return Err(Error::Invalid("JPX EBCOT refinement bit"));}
                b.coefficients[i].magnitude=(c.magnitude&!mask)|(u32::from(bit)<<plane);
                b.coefficients[i].refined=true;
            }
        }
    }
    Ok(())
}
/// Decode the first three passes of a baseline MQ codeblock with continuous
/// arithmetic and context state. Lower-plane cleanup is not performed yet.
pub fn decode_initial_through_refinement(data:&[u8],width:usize,height:usize,band:Band,plane:u8)->Result<Block>{
    if !(1..=30).contains(&plane){return Err(Error::Invalid("JPX EBCOT refinement plane"));}
    let mut cx=Contexts::new()?;
    let mut mq=Decoder::new(data,4096*7)?;
    let mut b=initial_cleanup(width,height,band,plane,|i|cx.decode(&mut mq,i))?;
    propagation(&mut b,band,plane-1,|i|cx.decode(&mut mq,i))?;
    refinement(&mut b,plane-1,|i|cx.decode(&mut mq,i))?;
    Ok(b)
}

/// A baseline single MQ segment, with 1..=1+3*highest_plane passes.
/// Quality-layer truncation may stop after any complete pass; omitted bits
/// remain zero. The caller supplies the highest plane from packet metadata.
/// No bypass, RESET, TERMALL, VSC, PTERM or SEGSYM modes are accepted.
fn decode_passes(width:usize,height:usize,band:Band,highest_plane:u8,passes:u16,mut read:impl FnMut(usize)->Result<u8>)->Result<Block>{
    if highest_plane>30||passes==0||passes>1+3*u16::from(highest_plane){
        return Err(Error::Invalid("JPX EBCOT pass count/plane"));
    }
    let mut b=initial_cleanup(width,height,band,highest_plane,&mut read)?;
    let mut remaining=passes-1;
    for plane in (0..highest_plane).rev(){
        if remaining==0{break;}
        propagation(&mut b,band,plane,&mut read)?;
        remaining-=1;
        if remaining==0{break;}
        refinement(&mut b,plane,&mut read)?;
        remaining-=1;
        if remaining==0{break;}
        cleanup(&mut b,band,plane,&mut read)?;
        remaining-=1;
    }
    Ok(b)
}
/// Decode a bounded baseline codeblock. Returns signed quantized coefficients,
/// not pixels. MQ virtual termination is not a corruption authentication check.
pub fn decode_codeblock(data:&[u8],width:usize,height:usize,band:Band,highest_plane:u8,passes:u16,style:u8)->Result<Block>{
    if style!=0{return Err(Error::Unsupported("JPX EBCOT coding style"));}
    if highest_plane>30||passes==0||passes>1+3*u16::from(highest_plane){
        return Err(Error::Invalid("JPX EBCOT pass count/plane"));
    }
    // At most four decisions per sample per pass, including aggregation.
    let area=width.checked_mul(height).ok_or(Error::Limit("JPX EBCOT block area"))?;
    if width==0||height==0||width>1024||height>1024||area>4096{
        return Err(Error::Limit("JPX EBCOT block dimensions"));
    }
    let mut cx=Contexts::new()?;
    let mut mq=Decoder::new(data,area*usize::from(passes)*4)?;
    decode_passes(width,height,band,highest_plane,passes,|i|cx.decode(&mut mq,i))
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn zero_context_exhaustive(){
        // Rows horizontal counts, columns vertical counts, entries d=0..4.
        let ll=[[[0,1,2,2,2],[3;5],[4;5]],[[5,6,6,6,6],[7;5],[7;5]],[[8;5];3]];
        let hh=[[0,1,2,2,2],[3,4,5,5,5],[6,7,7,7,7],[8;5],[8;5]];
        for h in 0..=2{for v in 0..=2{for d in 0..=4{
            let n=Neighbours{h,v,d};
            assert_eq!(zero_context(Band::Ll,n).unwrap(),ll[h as usize][v as usize][d as usize]);
            assert_eq!(zero_context(Band::Lh,n).unwrap(),ll[h as usize][v as usize][d as usize]);
            assert_eq!(zero_context(Band::Hl,n).unwrap(),ll[v as usize][h as usize][d as usize]);
            assert_eq!(zero_context(Band::Hh,n).unwrap(),hh[d as usize][(h+v)as usize]);
        }}}
        assert!(zero_context(Band::Ll,Neighbours{h:3,v:0,d:0}).is_err());
    }
    #[test]fn sign_context_exhaustive(){
        let expected=[[(13,1),(12,1),(11,1)],[(10,1),(9,0),(10,0)],[(11,0),(12,0),(13,0)]];
        for h in -1..=1{for v in -1..=1{
            assert_eq!(sign_context(h,v).unwrap(),expected[(h+1)as usize][(v+1)as usize]);
        }}
        assert!(sign_context(2,0).is_err());
    }
    #[test]fn magnitude_and_reset(){
        for b in [false,true]{assert_eq!(magnitude_context(true,b),16);}
        assert_eq!(magnitude_context(false,false),14);assert_eq!(magnitude_context(false,true),15);
        let c=Contexts::new().unwrap();
        for i in 0..19{
            let s=c.get(i).unwrap();
            assert_eq!(s.state(),match i{0=>4,17=>3,18=>46,_=>0});assert_eq!(s.mps(),0);
        }
        assert!(c.get(19).is_err());
    }
    #[test]fn raster_edges_and_sign_cancellation(){
        let mut b=Block::new(3,3).unwrap();
        b.make_significant(0,1,5,false).unwrap();b.make_significant(2,1,4,true).unwrap();
        b.make_significant(1,0,3,true).unwrap();b.make_significant(0,0,2,false).unwrap();
        assert_eq!(b.neighbours(1,1).unwrap(),(Neighbours{h:2,v:1,d:1},0,-1));
        assert_eq!(b.neighbours(0,0).unwrap(),(Neighbours{h:1,v:1,d:0},-1,1));
        assert_eq!(b.coefficient(0,1).unwrap().magnitude(),32);
        assert!(b.make_significant(0,1,1,true).is_err());
        assert!(b.neighbours(3,0).is_err());
    }
    #[test]fn bounded_blocks_and_mq_bridge(){
        assert!(Block::new(0,1).is_err());assert!(Block::new(65,65).is_err());
        assert!(Block::new(usize::MAX,2).is_err());assert!(Block::new(1025,1).is_err());
        let mut b=Block::new(64,64).unwrap();assert!(b.make_significant(0,0,31,false).is_err());
        let mut c=Contexts::new().unwrap();let mut d=Decoder::new(&[0,0],1).unwrap();
        assert!(c.decode(&mut d,19).is_err());
        assert!(c.decode(&mut d,UNI_CONTEXT).is_ok());assert!(c.decode(&mut d,0).is_err());
    }

    fn scripted(w:usize,h:usize,seq:&[(usize,u8)])->Block{
        let mut it=seq.iter();
        let b=initial_cleanup(w,h,Band::Ll,3,|cx|{
            let &(want,bit)=it.next().expect("extra decision");
            assert_eq!(cx,want);Ok(bit)
        }).unwrap();
        assert!(it.next().is_none());b
    }
    #[test]fn cleanup_aggregate_zero(){
        let b=scripted(2,4,&[(17,0),(17,0)]);
        assert!(b.coefficients.iter().all(|c|!c.significant()));
    }
    #[test]fn cleanup_run_and_sign(){
        let b=scripted(1,4,&[(17,1),(18,1),(18,0),(9,1),(3,1),(10,0)]);
        assert!(!b.coefficient(0,0).unwrap().significant());
        assert!(!b.coefficient(0,1).unwrap().significant());
        for y in 2..4{let c=b.coefficient(0,y).unwrap();assert_eq!(c.magnitude(),8);assert!(c.negative());}
    }
    #[test]fn cleanup_partial_stripe_order(){
        let b=scripted(2,2,&[(0,1),(9,0),(3,0),(5,0),(1,0)]);
        assert_eq!(b.coefficient(0,0).unwrap().magnitude(),8);
        assert_eq!(b.coefficients.iter().filter(|c|c.significant()).count(),1);
    }
    #[test]fn cleanup_stripe_boundary(){
        let b=scripted(1,5,&[(17,1),(18,1),(18,1),(9,0),(3,0)]);
        assert!(b.coefficient(0,3).unwrap().significant());
        assert!(!b.coefficient(0,4).unwrap().significant());
    }
    #[test]fn cleanup_limits_and_mq(){
        assert!(decode_initial_cleanup(&[],1,1,Band::Ll,0).is_err());
        assert!(decode_initial_cleanup(&[0],65,65,Band::Ll,0).is_err());
        assert!(decode_initial_cleanup(&[0],1,1,Band::Ll,31).is_err());
        let b=decode_initial_cleanup(&[0,0],1,1,Band::Ll,0).unwrap();
        assert_eq!(b.coefficients.len(),1);
        assert!(initial_cleanup(1,4,Band::Ll,0,|_|Err(Error::Invalid("test"))).is_err());
    }

    #[test]fn propagation_cascade_and_visited(){
        let mut b=Block::new(4,1).unwrap();
        b.make_significant(0,0,3,false).unwrap();
        let mut seq=[(5,1),(12,0),(5,1),(12,1),(5,0)].into_iter();
        propagation(&mut b,Band::Ll,2,|cx|{let (want,bit)=seq.next().unwrap();assert_eq!(cx,want);Ok(bit)}).unwrap();
        assert!(seq.next().is_none());
        assert!(!b.coefficient(0,0).unwrap().visited());
        assert_eq!(b.coefficient(1,0).unwrap().magnitude(),4);
        assert!(!b.coefficient(1,0).unwrap().negative());
        assert!(b.coefficient(2,0).unwrap().negative());
        assert!(!b.coefficient(3,0).unwrap().significant());
        for x in 1..4{assert!(b.coefficient(x,0).unwrap().visited());}
    }
    #[test]fn propagation_does_not_revisit_earlier_sample(){
        let mut b=Block::new(3,1).unwrap();
        b.make_significant(2,0,3,false).unwrap();
        let mut seq=[(5,1),(12,0)].into_iter();
        propagation(&mut b,Band::Ll,2,|cx|{let (want,bit)=seq.next().unwrap();assert_eq!(cx,want);Ok(bit)}).unwrap();
        assert!(seq.next().is_none());
        assert!(!b.coefficient(0,0).unwrap().visited());
        assert!(b.coefficient(1,0).unwrap().visited());
    }
    #[test]fn propagation_no_neighbour_no_decision(){
        let mut b=Block::new(4,5).unwrap();
        propagation(&mut b,Band::Hh,0,|_|panic!("unexpected decision")).unwrap();
        assert!(b.coefficients.iter().all(|c|!c.visited()));
    }
    #[test]fn propagation_vertical_stripe_boundary(){
        let mut b=Block::new(1,5).unwrap();
        b.make_significant(0,3,4,true).unwrap();
        let mut seq=[(3,0),(3,1),(10,0)].into_iter();
        propagation(&mut b,Band::Ll,3,|cx|{let (want,bit)=seq.next().unwrap();assert_eq!(cx,want);Ok(bit)}).unwrap();
        assert!(seq.next().is_none());
        assert!(b.coefficient(0,2).unwrap().visited());
        assert!(b.coefficient(0,4).unwrap().negative());
        assert!(!b.coefficient(0,1).unwrap().visited());
    }
    #[test]fn propagation_mq_and_limits(){
        assert!(decode_initial_and_propagation(&[0,0],1,1,Band::Ll,0).is_err());
        assert!(decode_initial_and_propagation(&[],1,1,Band::Ll,1).is_err());
        assert!(decode_initial_and_propagation(&[0,0],65,65,Band::Ll,1).is_err());
        assert!(decode_initial_and_propagation(&[0,0],1,1,Band::Ll,1).is_ok());
        let mut b=Block::new(2,1).unwrap();
        b.make_significant(0,0,2,false).unwrap();
        assert!(propagation(&mut b,Band::Ll,1,|_|Err(Error::Invalid("test"))).is_err());
    }

    #[test]fn refinement_first_isolated_and_repeated(){
        let mut b=Block::new(1,1).unwrap();
        b.make_significant(0,0,3,true).unwrap();
        refinement(&mut b,2,|cx|{assert_eq!(cx,14);Ok(1)}).unwrap();
        let c=b.coefficient(0,0).unwrap();assert_eq!(c.magnitude(),12);assert!(c.refined());assert!(c.negative());
        refinement(&mut b,1,|cx|{assert_eq!(cx,16);Ok(0)}).unwrap();
        refinement(&mut b,0,|cx|{assert_eq!(cx,16);Ok(1)}).unwrap();
        assert_eq!(b.coefficient(0,0).unwrap().magnitude(),13);
    }
    #[test]fn refinement_skip_new_significance(){
        let mut b=Block::new(3,1).unwrap();
        b.make_significant(0,0,3,false).unwrap();
        let mut seq=[(5,1),(12,0),(5,0)].into_iter();
        propagation(&mut b,Band::Ll,2,|cx|{let (want,bit)=seq.next().unwrap();assert_eq!(cx,want);Ok(bit)}).unwrap();
        assert!(seq.next().is_none());
        let mut calls=0;
        refinement(&mut b,2,|cx|{calls+=1;assert_eq!(cx,15);Ok(1)}).unwrap();
        assert_eq!(calls,1);assert_eq!(b.coefficient(0,0).unwrap().magnitude(),12);
        assert_eq!(b.coefficient(1,0).unwrap().magnitude(),4);
        assert!(!b.coefficient(1,0).unwrap().refined());
        assert!(!b.coefficient(2,0).unwrap().significant());
    }
    #[test]fn refinement_stripe_order(){
        let mut b=Block::new(2,5).unwrap();
        for y in 0..5{for x in 0..2{b.make_significant(x,y,2,false).unwrap();}}
        let mut bits=[0,1,0,1,1,0,1,0,1,0].into_iter();
        refinement(&mut b,1,|cx|{assert_eq!(cx,15);Ok(bits.next().unwrap())}).unwrap();
        assert!(bits.next().is_none());
        assert_eq!(b.coefficients.iter().map(|c|c.magnitude()).collect::<Vec<_>>(),vec![4,6,6,4,4,6,6,4,6,4]);
    }
    #[test]fn refinement_empty_and_invalid(){
        let mut b=Block::new(2,2).unwrap();
        refinement(&mut b,0,|_|panic!("unexpected decision")).unwrap();
        b.make_significant(0,0,2,false).unwrap();
        assert!(refinement(&mut b,2,|_|panic!("invalid order")).is_err());
        assert!(refinement(&mut b,30,|_|panic!("invalid plane")).is_err());
        assert!(refinement(&mut b,1,|_|Ok(2)).is_err());
        assert_eq!(b.coefficient(0,0).unwrap().magnitude(),4);
        assert!(!b.coefficient(0,0).unwrap().refined());
    }
    #[test]fn refinement_mq_bridge_and_limits(){
        assert!(decode_initial_through_refinement(&[],1,1,Band::Ll,1).is_err());
        assert!(decode_initial_through_refinement(&[0,0],1,1,Band::Ll,0).is_err());
        assert!(decode_initial_through_refinement(&[0,0],1,1,Band::Ll,31).is_err());
        assert!(decode_initial_through_refinement(&[0,0],65,65,Band::Ll,1).is_err());
        let b=decode_initial_through_refinement(&[0,0],1,1,Band::Ll,1).unwrap();
        assert_eq!(b.coefficients.len(),1);
    }

    #[test]fn multipass_signed_single_sample(){
        let mut seq=[(0,1),(9,1),(14,1),(16,0)].into_iter();
        let b=decode_passes(1,1,Band::Ll,2,7,|cx|{
            let (want,bit)=seq.next().unwrap();assert_eq!(cx,want);Ok(bit)
        }).unwrap();
        assert!(seq.next().is_none());
        assert_eq!(b.signed_coefficients(),vec![-6]);
        assert!(!b.coefficient(0,0).unwrap().visited());
    }
    #[test]fn multipass_cleanup_clears_visited(){
        let mut b=Block::new(3,1).unwrap();
        b.make_significant(0,0,3,false).unwrap();
        let mut seq=[(5,0)].into_iter();
        propagation(&mut b,Band::Ll,2,|cx|{let(w,v)=seq.next().unwrap();assert_eq!(cx,w);Ok(v)}).unwrap();
        assert!(seq.next().is_none());
        assert!(b.coefficient(1,0).unwrap().visited());
        let mut seq=[(0,1),(9,1)].into_iter();
        cleanup(&mut b,Band::Ll,2,|cx|{let(w,v)=seq.next().unwrap();assert_eq!(cx,w);Ok(v)}).unwrap();
        assert!(seq.next().is_none());
        assert_eq!(b.signed_coefficients(),vec![8,0,-4]);
        assert!(b.coefficients.iter().all(|c|!c.visited()));
        let mut calls=0;
        propagation(&mut b,Band::Ll,1,|cx|{calls+=1;assert_eq!(cx,8);Ok(0)}).unwrap();
        assert_eq!(calls,1);
        assert!(b.coefficient(1,0).unwrap().visited());
    }
    #[test]fn multipass_truncated_pass_boundary(){
        let seq=[(0,1),(9,0),(14,1),(16,1)];
        for passes in 1..=7{
            let mut it=seq.into_iter();
            let b=decode_passes(1,1,Band::Ll,2,passes,|cx|{
                let(w,v)=it.next().unwrap();assert_eq!(cx,w);Ok(v)
            }).unwrap();
            let expected=if passes<3{4}else if passes<6{6}else{7};
            assert_eq!(b.signed_coefficients(),vec![expected]);
        }
    }
    #[test]fn multipass_all_zero_stripes(){
        let mut calls=0;
        let b=decode_passes(2,4,Band::Hh,2,7,|cx|{calls+=1;assert_eq!(cx,17);Ok(0)}).unwrap();
        assert_eq!(calls,6);assert_eq!(b.signed_coefficients(),vec![0;8]);
    }
    #[test]fn multipass_mq_and_invalid_modes(){
        assert!(decode_codeblock(&[0,0],1,1,Band::Ll,1,4,0).is_ok());
        assert!(decode_codeblock(&[],1,1,Band::Ll,1,4,0).is_err());
        assert!(decode_codeblock(&[0],1,1,Band::Ll,1,5,0).is_err());
        assert!(decode_codeblock(&[0],1,1,Band::Ll,1,0,0).is_err());
        assert!(decode_codeblock(&[0],1,1,Band::Ll,31,1,0).is_err());
        assert!(decode_codeblock(&[0],65,65,Band::Ll,1,4,0).is_err());
        for style in 1..=255{assert!(matches!(decode_codeblock(&[0],1,1,Band::Ll,1,4,style),Err(Error::Unsupported("JPX EBCOT coding style"))));}
    }

    // Pillow/OpenJPEG 2.5.4, lossless 8-bit grayscale, one resolution,
    // one layer and one codeblock. With no DWT the independently known
    // quantized coefficients are exactly the original samples minus 128.
    fn independent_gray(w:usize,h:usize,hex:&str,pixels:&[i32]){
        let data:Vec<u8>=hex.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect();
        let p=super::super::packet::inspect_packets(&data,0).unwrap();
        assert_eq!(p.packets.len(),1);
        let entries=&p.packets[0].contributions;
        assert_eq!(entries.len(),1);
        let e=&entries[0];
        assert_eq!(e.first_pass,0);
        assert_eq!(e.band,Band::Ll);
        assert_eq!((e.passes-1)%3,0);
        // QCD exponent 8, two guard bits: Mb=9.
        let plane=8-e.zero_bit_planes;
        assert_eq!(u16::from(plane)*3+1,e.passes);
        let b=decode_codeblock(&data[e.data.clone()],w,h,e.band,plane,e.passes,0).unwrap();
        assert_eq!(b.signed_coefficients(),pixels.iter().map(|v|v-128).collect::<Vec<_>>());
    }
    #[test]fn independent_real_single_coefficient(){
        independent_gray(1,1,"ff4fff510029000000000001000000010000000000000000000000010000000100000000000000000001070101ff52000c00000001000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000000120001ff93c0f84100ffd9",&[135]);
    }
    #[test]fn independent_real_mixed_coefficients(){
        independent_gray(4,4,"ff4fff510029000000000004000000040000000000000000000000040000000400000000000000000001070101ff52000c00000001000000000001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000001e0001ff93df806818aa09fcb2996738ca69cbe485ffd9",&[128,129,127,130,125,140,116,128,255,0,128,129,130,131,132,133]);
    }
    #[test]fn independent_real_partial_stripe(){
        independent_gray(3,5,"ff4fff510029000000000003000000050000000000000000000000030000000500000000000000000001070101ff52000c00000001000001010001ff5c00044040ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000001a0001ff93c1f209002e709cb049aa11cdffd9",&[128,127,130,125,132,123,134,121,136,119,138,117,140,115,142]);
    }
}
