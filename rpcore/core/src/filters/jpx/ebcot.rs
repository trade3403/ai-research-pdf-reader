//! JPEG 2000 Tier-1 context modelling for the baseline MQ coding style.
//! No coding passes are performed here.
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
pub struct Coefficient{magnitude:u32,negative:bool,refined:bool}
impl Coefficient{
    pub fn significant(self)->bool{self.magnitude!=0}
    pub fn magnitude(self)->u32{self.magnitude}
    pub fn negative(self)->bool{self.negative}
    pub fn refined(self)->bool{self.refined}
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
        self.coefficients[i]=Coefficient{magnitude:1u32<<plane,negative,refined:false};Ok(())
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
}
