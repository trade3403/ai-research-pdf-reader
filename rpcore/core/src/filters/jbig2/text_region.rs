//! Native text-region placement and composition, T.88 (2000) 6.4.5–9.
//! Consumes already decoded signed deltas and symbol bitmaps, not entropy.
//! Transposition changes coordinate axes, never the source pixel orientation.
use crate::{Error,Result};
use super::bitmap::{self,Bitmap,ComposeBudget,ComposeOp};

pub const MAX_INSTANCES:u32=1_048_576;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Corner{BottomLeft,TopLeft,BottomRight,TopRight}
impl Corner{
    pub fn from_code(code:u8)->Result<Self>{
        match code{0=>Ok(Self::BottomLeft),1=>Ok(Self::TopLeft),2=>Ok(Self::BottomRight),
            3=>Ok(Self::TopRight),_=>Err(Error::Invalid("JBIG2 text reference corner"))}
    }
    fn right(self)->bool{matches!(self,Self::BottomRight|Self::TopRight)}
    fn bottom(self)->bool{matches!(self,Self::BottomLeft|Self::BottomRight)}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Placement{
    pub x:i64,pub y:i64,pub s:i32,pub t:i32,pub next_s:i32,
}
fn coord(v:i64)->Result<i32>{i32::try_from(v).map_err(|_|Error::Limit("JBIG2 text coordinate"))}
/// CURS before step vi, TI, and actual instance dimensions (after any
/// refinement). Return the anchor, physical upper-left and post-step-x CURS.
pub fn placement(corner:Corner,transposed:bool,curs:i32,t:i32,width:u32,height:u32)->Result<Placement>{
    if width==0||height==0{return Err(Error::Invalid("JBIG2 text symbol dimensions"));}
    if width>bitmap::MAX_SIDE||height>bitmap::MAX_SIDE||
        u64::from(width)*u64::from(height)>bitmap::MAX_PIXELS{return Err(Error::Limit("JBIG2 text symbol dimensions"));}
    let w=i64::from(width)-1;let h=i64::from(height)-1;
    let advance=if transposed{h}else{w};
    let far=if transposed{corner.bottom()}else{corner.right()};
    let next_s=coord(i64::from(curs)+advance)?;
    let s=if far{next_s}else{curs};
    let (x,y)=if transposed{(i64::from(t),i64::from(s))}else{(i64::from(s),i64::from(t))};
    Ok(Placement{x:x-if corner.right(){w}else{0},y:y-if corner.bottom(){h}else{0},s,t,next_s})
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Options{
    pub width:u32,pub height:u32,pub default_black:bool,pub op:ComposeOp,
    pub strips:u8,pub corner:Corner,pub transposed:bool,pub ds_offset:i8,pub instances:u32,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Stats{pub strips:u32,pub instances:u32,pub work_used:u64,pub composed_pixels:u64}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedText{pub bitmap:Bitmap,pub stats:Stats}
/// State after decoding the initial IADT/HTDT integer. Input DT arguments
/// are unscaled decoded integers; this state applies SBSTRIPS exactly once.
/// Every opened strip must contain a symbol and close with decoded OOB,
/// including the final strip. A failed transition does not commit state.
pub struct Region{
    options:Options,image:Bitmap,strip_t:i32,first_s:i32,curs:i32,
    in_strip:bool,first_pending:bool,stats:Stats,left:u64,
}
impl Region{
    pub fn new(options:Options,initial_dt:i64,work_limit:u64)->Result<Self>{
        if !matches!(options.strips,1|2|4|8)||!(-16..=15).contains(&options.ds_offset)||
            options.op==ComposeOp::Replace{return Err(Error::Invalid("JBIG2 text options"));}
        if options.instances>MAX_INSTANCES||work_limit>bitmap::MAX_COMPOSE_WORK{
            return Err(Error::Limit("JBIG2 text region limits"));
        }
        let strip_t=coord(initial_dt.checked_mul(i64::from(options.strips))
            .and_then(i64::checked_neg).ok_or(Error::Limit("JBIG2 text initial strip"))?)?;
        let init=u64::from(options.width)*u64::from(options.height);
        let left=work_limit.checked_sub(init).ok_or(Error::Limit("JBIG2 text initialization work"))?;
        let image=Bitmap::new(options.width,options.height,options.default_black)?;
        Ok(Self{options,image,strip_t,first_s:0,curs:0,in_strip:false,first_pending:false,
            stats:Stats{strips:0,instances:0,work_used:init,composed_pixels:0},left})
    }
    pub fn stats(&self)->Stats{self.stats}
    pub fn remaining_instances(&self)->u32{self.options.instances-self.stats.instances}
    pub fn strip_t(&self)->i32{self.strip_t}
    pub fn first_s(&self)->i32{self.first_s}
    pub fn curs(&self)->i32{self.curs}
    pub fn begin_strip(&mut self,dt:i64,dfs:i64)->Result<()>{
        if self.in_strip||self.remaining_instances()==0{return Err(Error::Invalid("JBIG2 text strip state"));}
        let t=coord(dt.checked_mul(i64::from(self.options.strips))
            .and_then(|v|v.checked_add(i64::from(self.strip_t))).ok_or(Error::Limit("JBIG2 text strip delta"))?)?;
        let first=coord(dfs.checked_add(i64::from(self.first_s)).ok_or(Error::Limit("JBIG2 text first delta"))?)?;
        let left=self.left.checked_sub(1).ok_or(Error::Limit("JBIG2 text strip work"))?;
        self.strip_t=t;self.first_s=first;self.curs=first;
        self.in_strip=true;self.first_pending=true;self.left=left;
        self.stats.strips+=1;self.stats.work_used+=1;Ok(())
    }
    /// None is the first symbol; Some(IDS) is a subsequent symbol.
    /// CURT is already decoded (zero without entropy for strip height one).
    /// The caller resolves the ID and any refinement before placing.
    pub fn place(&mut self,delta:Option<i64>,curt:u8,symbol:&Bitmap)->Result<Placement>{
        if !self.in_strip||self.remaining_instances()==0||self.first_pending!=delta.is_none()||
            curt>=self.options.strips{return Err(Error::Invalid("JBIG2 text instance state"));}
        let curs=match delta{
            None=>self.curs,
            Some(v)=>coord(i64::from(self.curs).checked_add(v)
                .and_then(|n|n.checked_add(i64::from(self.options.ds_offset)))
                .ok_or(Error::Limit("JBIG2 text symbol delta"))?)?,
        };
        let t=coord(i64::from(self.strip_t)+i64::from(curt))?;
        let p=placement(self.options.corner,self.options.transposed,curs,t,symbol.width(),symbol.height())?;
        // Constant charge even for fully clipped symbols. Bitmap::compose
        // precharges overlap atomically; no state advances on budget failure.
        let left=self.left.checked_sub(1).ok_or(Error::Limit("JBIG2 text instance work"))?;
        let mut budget=ComposeBudget::new(left)?;
        self.image.compose(symbol,p.x,p.y,self.options.op,&mut budget)?;
        let pixels=left-budget.remaining();
        self.left=budget.remaining();self.stats.work_used+=1+pixels;self.stats.composed_pixels+=pixels;
        self.stats.instances+=1;self.curs=p.next_s;self.first_pending=false;Ok(p)
    }
    /// Caller has consumed IADS/HTDS OOB. Empty strips are not encoded.
    pub fn end_strip(&mut self)->Result<()>{
        if !self.in_strip||self.first_pending{return Err(Error::Invalid("JBIG2 text strip end"));}
        let left=self.left.checked_sub(1).ok_or(Error::Limit("JBIG2 text strip end work"))?;
        self.left=left;self.stats.work_used+=1;self.in_strip=false;Ok(())
    }
    pub fn finish(self)->Result<DecodedText>{
        if self.in_strip||self.remaining_instances()!=0{return Err(Error::Invalid("JBIG2 text incomplete region"));}
        Ok(DecodedText{bitmap:self.image,stats:self.stats})
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn opt()->Options{Options{width:8,height:6,default_black:false,op:ComposeOp::Xor,
        strips:2,corner:Corner::TopLeft,transposed:false,ds_offset:-1,instances:4}}
    fn a()->Bitmap{Bitmap::from_packed(3,2,&[160,64]).unwrap()}
    fn b()->Bitmap{Bitmap::from_packed(2,3,&[192,64,128]).unwrap()}
    #[test]
    fn text_corner_and_transposition(){
        let expected=[[(1,0,1,1,3),(1,1,1,1,3),(1,0,3,1,3),(1,1,3,1,3)],
            [(1,1,2,1,2),(1,1,1,1,2),(-1,1,2,1,2),(-1,1,1,1,2)]];
        for tr in [false,true]{for c in 0..4{
            let p=placement(Corner::from_code(c).unwrap(),tr,1,1,3,2).unwrap();
            assert_eq!((p.x,p.y,p.s,p.t,p.next_s),expected[usize::from(tr)][c as usize]);
        }}
        assert!(Corner::from_code(4).is_err());
        assert!(placement(Corner::TopLeft,false,0,0,0,1).is_err());
        assert!(placement(Corner::TopLeft,false,i32::MAX,0,2,1).is_err());
    }
    fn fixture(o:Options,work:u64)->Result<DecodedText>{
        let mut r=Region::new(o,1,work)?;let a=a();let b=b();
        assert_eq!(r.strip_t(),-2);
        r.begin_strip(1,1)?;r.place(None,1,&a)?;r.place(Some(1),0,&b)?;r.end_strip()?;
        r.begin_strip(-1,-2)?;assert_eq!((r.strip_t(),r.first_s(),r.curs()),(-2,-1,-1));
        r.place(None,1,&b)?;r.place(Some(2),0,&a)?;r.end_strip()?;r.finish()
    }
    #[test]
    fn text_two_strip_reference_pixels(){
        // Independent JS boolean-grid reference, asymmetric glyphs,
        // negative deltas, clipping and XOR overlap, all eight geometries.
        let expected=[[64,32,0,0,0,0],[152,88,48,0,0,0],[64,32,0,0,0,0],[152,88,48,0,0,0],
            [128,80,96,64,128,0],[128,80,96,64,128,0],[0,64,0,128,0,0],[0,64,0,128,0,0]];
        let overlap=[8,14,8,14,16,16,7,7];
        for tr in [false,true]{for c in 0..4{
            let i=usize::from(tr)*4+c as usize;
            let d=fixture(Options{corner:Corner::from_code(c).unwrap(),transposed:tr,..opt()},200).unwrap();
            assert_eq!(d.bitmap.packed(),expected[i]);
            assert_eq!(d.stats,Stats{strips:2,instances:4,work_used:56+overlap[i],composed_pixels:overlap[i]});
        }}
    }
    #[test]
    fn text_all_internal_operators(){
        let src=Bitmap::from_packed(4,1,&[0x50]).unwrap();
        for (op,black,want) in [(ComposeOp::Or,false,0x50),(ComposeOp::And,true,0x50),
            (ComposeOp::Xor,true,0xa0),(ComposeOp::Xnor,false,0xa0)]{
            let mut r=Region::new(Options{width:4,height:1,op,default_black:black,strips:1,instances:1,..opt()},0,20).unwrap();
            r.begin_strip(0,0).unwrap();r.place(None,0,&src).unwrap();r.end_strip().unwrap();
            assert_eq!(r.finish().unwrap().bitmap.packed(),[want]);
        }
    }
    #[test]
    fn text_exact_budget_and_atomic_failure(){
        let o=opt();let d=fixture(o,200).unwrap();
        assert_eq!(fixture(o,d.stats.work_used).unwrap(),d);
        assert!(fixture(o,d.stats.work_used-1).is_err());
        let mut r=Region::new(Options{instances:1,..o},0,50).unwrap();
        r.begin_strip(0,0).unwrap();let before=r.stats();
        assert!(r.place(None,0,&a()).is_err());assert_eq!(r.stats(),before);
        assert_eq!(r.curs(),0);
    }
    #[test]
    fn text_state_and_count_validation(){
        let mut r=Region::new(Options{instances:1,..opt()},0,200).unwrap();
        assert!(r.place(None,0,&a()).is_err());assert!(r.end_strip().is_err());
        r.begin_strip(0,0).unwrap();assert!(r.begin_strip(0,0).is_err());assert!(r.end_strip().is_err());
        assert!(r.place(Some(0),0,&a()).is_err());assert!(r.place(None,2,&a()).is_err());
        r.place(None,0,&a()).unwrap();assert!(r.place(Some(0),0,&a()).is_err());
        r.end_strip().unwrap();assert!(r.begin_strip(0,0).is_err());assert!(r.finish().is_ok());
        assert!(Region::new(opt(),0,200).unwrap().finish().is_err());
        let mut r=Region::new(Options{instances:1,..opt()},0,200).unwrap();
        r.begin_strip(0,0).unwrap();r.place(None,0,&a()).unwrap();assert!(r.finish().is_err());
    }
    #[test]
    fn text_signed_extremes_and_invalid_options(){
        for s in [0,3,16]{assert!(Region::new(Options{strips:s,..opt()},0,200).is_err());}
        for o in [Options{ds_offset:-17,..opt()},Options{ds_offset:16,..opt()},
            Options{op:ComposeOp::Replace,..opt()},Options{instances:MAX_INSTANCES+1,..opt()}]{
            assert!(Region::new(o,0,200).is_err());
        }
        for n in [i64::MIN,i64::MAX]{assert!(Region::new(opt(),n,200).is_err());}
        let mut r=Region::new(opt(),0,200).unwrap();
        assert!(r.begin_strip(i64::MAX,0).is_err());assert!(r.begin_strip(0,i64::MIN).is_err());
        r.begin_strip(-1,i64::from(i32::MIN)).unwrap();
        r.place(None,0,&a()).unwrap();let old=r.stats();
        assert!(r.place(Some(i64::MIN),0,&a()).is_err());assert_eq!(r.stats(),old);
        assert_eq!(r.stats().composed_pixels,0);
    }
    #[test]
    fn text_empty_and_fully_clipped_budget(){
        let d=Region::new(Options{instances:0,default_black:true,..opt()},0,48).unwrap().finish().unwrap();
        assert_eq!(d.bitmap.packed(),[255;6]);assert_eq!(d.stats.work_used,48);
        let mut r=Region::new(Options{instances:1,..opt()},0,51).unwrap();
        r.begin_strip(-100,-100).unwrap();r.place(None,0,&a()).unwrap();r.end_strip().unwrap();
        let d=r.finish().unwrap();assert_eq!(d.bitmap.packed(),[0;6]);
        assert_eq!((d.stats.work_used,d.stats.composed_pixels),(51,0));
    }
}
