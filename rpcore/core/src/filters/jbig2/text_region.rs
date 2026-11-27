//! Native text-region placement and composition, T.88 (2000) 6.4.5–9.
//! Placement and bounded arithmetic entropy decoding with optional refinement.
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

pub const MAX_TEXT_DECISIONS:usize=16_777_216;

/// T.88 text refinement geometry: target dimensions are signed deltas
/// from the source; reference displacement uses floor(delta/2).
pub fn refinement_geometry(src:&Bitmap,dw:i64,dh:i64,dx:i64,dy:i64)
    ->Result<super::refinement::RefinementOptions>{
    fn dimension(base:u32,delta:i64)->Result<u32>{
        let n=i64::from(base).checked_add(delta).ok_or(Error::Limit("JBIG2 text refinement dimension"))?;
        if n<=0{return Err(Error::Invalid("JBIG2 text refinement dimension"));}
        if n>i64::from(bitmap::MAX_SIDE){return Err(Error::Limit("JBIG2 text refinement dimension"));}
        Ok(n as u32)
    }
    let width=dimension(src.width(),dw)?;
    let height=dimension(src.height(),dh)?;
    let dx=dw.div_euclid(2).checked_add(dx).ok_or(Error::Limit("JBIG2 text refinement offset"))?;
    let dy=dh.div_euclid(2).checked_add(dy).ok_or(Error::Limit("JBIG2 text refinement offset"))?;
    if i32::try_from(dx).is_err()||i32::try_from(dy).is_err(){
        return Err(Error::Limit("JBIG2 text refinement offset"));
    }
    if u64::from(width)*u64::from(height)>bitmap::MAX_PIXELS||
        u64::from(width).div_ceil(8)*u64::from(height)>bitmap::MAX_PACKED_BYTES as u64{
        return Err(Error::Limit("JBIG2 text refinement bitmap"));
    }
    Ok(super::refinement::RefinementOptions{width,height,dx,dy,typical_prediction:false})
}
#[cfg(test)]
mod text_refinement_geometry_tests{
    use super::*;
    #[test]
    fn refinement_geometry_signed_floor(){
        let b=Bitmap::new(10,10,false).unwrap();
        for (d,half) in [(-5,-3),(-3,-2),(-2,-1),(-1,-1),(0,0),(1,0),(2,1),(3,1),(5,2)]{
            let o=refinement_geometry(&b,d,d,4,-4).unwrap();
            assert_eq!((o.width,o.height,o.dx,o.dy),( (10+d) as u32,(10+d) as u32,half+4,half-4));
            assert!(!o.typical_prediction);
        }
    }
    #[test]
    fn refinement_geometry_limits(){
        let b=Bitmap::new(2,2,false).unwrap();
        for (dw,dh,dx,dy) in [(-2,0,0,0),(0,-2,0,0),(i64::MAX,0,0,0),
            (0,i64::MAX,0,0),(2,0,i64::MAX,0),(0,2,0,i64::MAX),
            (0,0,i64::MIN,0),(1000000,1000000,0,0)]{
            assert!(refinement_geometry(&b,dw,dh,dx,dy).is_err());
        }
    }
    #[test]
    fn refinement_geometry_uses_target_dimensions(){
        let b=Bitmap::new(3,5,false).unwrap();
        let o=refinement_geometry(&b,2,-3,-1,2).unwrap();
        assert_eq!((o.width,o.height,o.dx,o.dy),(5,2,0,0));
        let p=placement(Corner::BottomRight,false,2,3,o.width,o.height).unwrap();
        assert_eq!(p.next_s,6);
        assert_eq!((p.x,p.y),(2,2));
    }
}

/// Decode a known-length, explicitly terminated, non-refinement arithmetic
/// text payload. Symbols are borrowed in stream identifier order.
pub fn decode_arithmetic(data:&[u8],options:Options,symbols:&[&Bitmap],
    decision_limit:usize,work_limit:u64)->Result<DecodedText>{
    decode_arithmetic_inner(data,options,symbols,decision_limit,work_limit,None)
}
/// Standalone arithmetic text with refinement enabled. The same QM stream
/// and persistent GR bank are used for every refined instance.
pub fn decode_arithmetic_refined(data:&[u8],options:Options,symbols:&[&Bitmap],
    neighborhood:&super::refinement::Neighborhood,decision_limit:usize,
    work_limit:u64)->Result<DecodedText>{
    decode_arithmetic_inner(data,options,symbols,decision_limit,work_limit,Some(neighborhood))
}
fn decode_arithmetic_inner(data:&[u8],options:Options,symbols:&[&Bitmap],
    decision_limit:usize,work_limit:u64,
    neighborhood:Option<&super::refinement::Neighborhood>)->Result<DecodedText>{
    use super::integer::IaidDecoder;
    use super::refinement::ContextBank;
    if decision_limit>MAX_TEXT_DECISIONS||symbols.len()>MAX_INSTANCES as usize{
        return Err(Error::Limit("JBIG2 text arithmetic limits"));
    }
    if options.instances!=0&&symbols.is_empty(){
        return Err(Error::Invalid("JBIG2 text empty symbol namespace"));
    }
    let count=symbols.len() as u32;
    let width=if count<=1{0}else{(32-(count-1).leading_zeros()) as u8};
    let context_work=if let Some(n)=neighborhood{
        (1u64<<width)+9*512+n.template().context_count() as u64
    }else{0};
    let available=work_limit.checked_sub(context_work).ok_or(Error::Limit("JBIG2 text context work"))?;
    if work_limit>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 text region limits"));}
    let mut gr=match neighborhood{Some(n)=>Some(ContextBank::new(n.template())?),None=>None};
    let mut contexts=ArithmeticTextContexts::new() ;
    let mut decoder=super::qm::Decoder::new(data,decision_limit)?;
    let mut ids=IaidDecoder::new(width)?;
    let mut result=decode_arithmetic_shared(options,symbols,&mut decoder,&mut ids,&mut contexts,
        neighborhood,gr.as_mut(),available)?;
    result.stats.work_used+=context_work;
    Ok(result)
}

/// Persistent text syntax banks, independent of the QM registers and IAID/GR.
/// A caller sharing a dictionary stream must keep these alive across regions.
pub(crate) struct ArithmeticTextContexts{
    dt:super::integer::IntegerDecoder,fs:super::integer::IntegerDecoder,
    ds:super::integer::IntegerDecoder,it:super::integer::IntegerDecoder,
    extra:[super::integer::IntegerDecoder;5],
}
impl ArithmeticTextContexts{
    pub(crate) fn new()->Self{
        use super::integer::IntegerDecoder;
        Self{dt:IntegerDecoder::new(),fs:IntegerDecoder::new(),ds:IntegerDecoder::new(),
            it:IntegerDecoder::new(),extra:std::array::from_fn(|_|IntegerDecoder::new())}
    }
    /// Dictionary single-reference refinement uses the same displacement
    /// syntax banks as embedded text-region refinement.
    pub(crate) fn decode_displacement(&mut self,decoder:&mut super::qm::Decoder<'_>)->Result<(i64,i64)>{
        use super::integer::Integer;
        let Integer::Value(dx)=self.extra[3].decode(decoder)? else{return Err(Error::Invalid("JBIG2 refinement X OOB"));};
        let Integer::Value(dy)=self.extra[4].decode(decoder)? else{return Err(Error::Invalid("JBIG2 refinement Y OOB"));};
        Ok((dx,dy))
    }

}

/// Decode one text region on caller-owned QM, IAID and GR banks.
/// The available symbol slice may grow, while IAID width remains fixed.
/// The caller accounts for context initialization and cumulative entropy budget.
/// Abort the enclosing stream on error; probability states are not rolled back.
pub(crate) fn decode_arithmetic_shared(options:Options,symbols:&[&Bitmap],
    decoder:&mut super::qm::Decoder<'_>,ids:&mut super::integer::IaidDecoder,
    contexts:&mut ArithmeticTextContexts,neighborhood:Option<&super::refinement::Neighborhood>,
    gr:Option<&mut super::refinement::ContextBank>,available:u64)->Result<DecodedText>{
    decode_arithmetic_shared_bounded(options,symbols,decoder,ids,contexts,neighborhood,gr,available,bitmap::MAX_PACKED_BYTES)
}

/// Caller supplies remaining packed bytes after reserving persistent output
/// and the region canvas. Only one temporary refinement bitmap is live.
pub(crate) fn decode_arithmetic_shared_bounded(options:Options,symbols:&[&Bitmap],
    decoder:&mut super::qm::Decoder<'_>,ids:&mut super::integer::IaidDecoder,
    contexts:&mut ArithmeticTextContexts,neighborhood:Option<&super::refinement::Neighborhood>,
    mut gr:Option<&mut super::refinement::ContextBank>,available:u64,
    temporary_packed:usize)->Result<DecodedText>{
    use super::integer::Integer;
    use super::refinement;
    if symbols.len()>MAX_INSTANCES as usize||symbols.len()>(1usize<<ids.width()){
        return Err(Error::Limit("JBIG2 text shared namespace"));
    }
    if options.instances!=0&&symbols.is_empty(){
        return Err(Error::Invalid("JBIG2 text empty symbol namespace"));
    }
    if neighborhood.map(|n|n.template())!=gr.as_ref().map(|g|g.template()){
        return Err(Error::Invalid("JBIG2 text shared GR template"));
    }
    let count=symbols.len() as u32;
    let ArithmeticTextContexts{dt,fs,ds,it,extra}=contexts;
    let mut extra=Some(extra);
    fn required(v:Integer)->Result<i64>{
        match v{Integer::Value(n)=>Ok(n),Integer::Oob=>Err(Error::Invalid("JBIG2 text unexpected OOB"))}
    }
    let initial=required(dt.decode(decoder)?)?;
    let mut region=Region::new(options,initial,available)?;
    while region.remaining_instances()!=0{
        let delta=required(dt.decode(decoder)?)?;
        let first=required(fs.decode(decoder)?)?;
        region.begin_strip(delta,first)?;
        let mut offset=None;
        loop{
            let curt=if options.strips==1{0}else{
                u8::try_from(required(it.decode(decoder)?)?)
                    .map_err(|_|Error::Invalid("JBIG2 text CURT"))?
            };
            if curt>=options.strips{return Err(Error::Invalid("JBIG2 text CURT"));}
            let id=ids.decode_for_count(decoder,count)?;
            let src=symbols[id as usize];
            let mut refined=None;
            if let (Some(n),Some(bank),Some(fields))=(neighborhood,gr.as_deref_mut(),extra.as_deref_mut()){
                match required(fields[0].decode(decoder)?)?{
                    0=>{},
                    1=>{
                        let dw=required(fields[1].decode(decoder)?)?;
                        let dh=required(fields[2].decode(decoder)?)?;
                        let dx=required(fields[3].decode(decoder)?)?;
                        let dy=required(fields[4].decode(decoder)?)?;
                        let opt=refinement_geometry(src,dw,dh,dx,dy)?;
                        let cost=refinement::work_bound(n.template(),opt)?;
                        let bytes=u64::from(opt.width).div_ceil(8)*u64::from(opt.height);
                        if bytes>temporary_packed as u64{
                            return Err(Error::Limit("JBIG2 text temporary packed budget"));
                        }
                        let left=region.left.checked_sub(cost).ok_or(Error::Limit("JBIG2 text refinement work"))?;
                        let decoded=refinement::decode_bitmap(src,n,opt,bank,decoder,cost)?;
                        region.left=left;
                        region.stats.work_used+=cost;
                        refined=Some(decoded.bitmap);
                    },
                    _=>return Err(Error::Invalid("JBIG2 text refinement indicator")),
                }
            }
            region.place(offset,curt,refined.as_ref().unwrap_or(src))?;
            match ds.decode(decoder)?{
                Integer::Oob=>{region.end_strip()?;break;}
                Integer::Value(n)=>{
                    if region.remaining_instances()==0{
                        return Err(Error::Invalid("JBIG2 text excess instances"));
                    }
                    offset=Some(n);
                }
            }
        }
    }
    region.finish()
}
#[cfg(test)]
mod arithmetic_text_tests{
    use super::*;
    fn options()->Options{Options{width:8,height:2,default_black:false,
        op:ComposeOp::Or,strips:1,corner:Corner::TopLeft,transposed:false,
        ds_offset:0,instances:0}}
    #[test]
    fn arithmetic_text_empty_initial_dt(){
        // Existing independently encoded Annex E integer vector begins with zero.
        let data=[0xaa,0xf2,0x8c,0x65,0xc5,0x81,0x19,0xc3,0xf5,0xdd,0x6b,0x58,0x2e,0x73,0xf2,
        0xa3,0x73,0x46,0x0a,0x8e,0x90,0xf7,0x7a,0x76,0x20,0xf8,0x7f,0xff,0xac];
        let out=decode_arithmetic(&data,options(),&[],4,16).unwrap();
        assert_eq!(out.bitmap.packed(),[0,0]);
        assert_eq!(out.stats.instances,0);
        assert!(decode_arithmetic(&data,options(),&[],3,16).is_err());
        assert!(decode_arithmetic(&data,options(),&[],4,15).is_err());
    }
    #[test]
    fn arithmetic_text_invalid_namespace(){
        let mut o=options();o.instances=1;
        assert!(decode_arithmetic(&[255,172],o,&[],100,100).is_err());
    }
    #[test]
    fn arithmetic_text_missing_marker(){
        assert!(decode_arithmetic(&[0,0],options(),&[],100,100).is_err());
    }
    #[test]
    fn arithmetic_text_decision_cap(){
        assert!(decode_arithmetic(&[255,172],options(),&[],MAX_TEXT_DECISIONS+1,100).is_err());
    }
}

#[cfg(test)]
mod arithmetic_text_vectors {
    use super::*;
    // Original Annex E encoder fixtures, independent of the Rust decoder.
    const GOOD:&[u8]=&[147,150,68,103,175,63,255,172];
    const BAD_ID:&[u8]=&[169,109,255,172];
    const EXCESS:&[u8]=&[169,114,15,255,172];
    const OOB:&[u8]=&[207,255,172];
    fn options(instances:u32)->Options{Options{width:8,height:4,default_black:false,
        op:ComposeOp::Or,strips:2,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances}}
    fn symbols()->[Bitmap;3]{
        [Bitmap::from_packed(1,1,&[128]).unwrap(),Bitmap::from_packed(1,1,&[128]).unwrap(),
         Bitmap::from_packed(1,1,&[128]).unwrap()]
    }
    #[test]
    fn arithmetic_text_two_strips_pixels(){
        let s=symbols();let refs=[&s[0],&s[1],&s[2]];
        let d=decode_arithmetic(GOOD,options(3),&refs,50,100).unwrap();
        assert_eq!(d.bitmap.packed(),[16,64,128,0]);
        assert_eq!(d.stats.instances,3);assert_eq!(d.stats.strips,2);
        assert_eq!(d.stats.composed_pixels,3);
    }
    #[test]
    fn arithmetic_text_exact_decisions(){
        let s=symbols();let refs=[&s[0],&s[1],&s[2]];
        assert!(decode_arithmetic(GOOD,options(3),&refs,49,100).is_err());
        assert!(decode_arithmetic(GOOD,options(3),&refs,50,100).is_ok());
    }
    #[test]
    fn arithmetic_text_reject_invalid_id(){
        let s=symbols();let refs=[&s[0],&s[1],&s[2]];
        assert!(matches!(decode_arithmetic(BAD_ID,options(1),&refs,100,100),
            Err(Error::Invalid("JBIG2 IAID symbol index"))));
    }
    #[test]
    fn arithmetic_text_reject_excess(){
        let s=symbols();let refs=[&s[0],&s[1],&s[2]];
        assert!(matches!(decode_arithmetic(EXCESS,options(1),&refs,100,100),
            Err(Error::Invalid("JBIG2 text excess instances"))));
    }
    #[test]
    fn arithmetic_text_reject_initial_oob(){
        assert!(matches!(decode_arithmetic(OOB,options(0),&[],100,100),
            Err(Error::Invalid("JBIG2 text unexpected OOB"))));
    }
}

#[cfg(test)]
mod refined_text_integration_tests{
    use super::*;
    use super::super::refinement::{Neighborhood,Template};
    const EMPTY:&[u8]=&[0xaa,0xf2,0x8c,0x65,0xc5,0x81,0x19,0xc3,0xf5,0xdd,0x6b,0x58,0x2e,0x73,0xf2,
        0xa3,0x73,0x46,0x0a,0x8e,0x90,0xf7,0x7a,0x76,0x20,0xf8,0x7f,0xff,0xac];
    fn options()->Options{Options{width:8,height:2,default_black:false,
        op:ComposeOp::Or,strips:1,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:0}}
    #[test]
    fn refined_text_empty_context_budget(){
        for t in [Template::T0,Template::T1]{
            let n=Neighborhood::nominal(t);
            let cost=1+9*512+t.context_count() as u64+16;
            let d=decode_arithmetic_refined(EMPTY,options(),&[],&n,4,cost).unwrap();
            assert_eq!(d.bitmap.packed(),[0,0]);
            assert_eq!(d.stats.work_used,cost);
            assert!(decode_arithmetic_refined(EMPTY,options(),&[],&n,4,cost-1).is_err());
        }
    }
    #[test]
    fn refined_text_empty_initial_decision_required(){
        let n=Neighborhood::nominal(Template::T1);
        assert!(decode_arithmetic_refined(EMPTY,options(),&[],&n,3,20000).is_err());
    }
    #[test]
    fn refined_text_input_limits(){
        let n=Neighborhood::nominal(Template::T1);
        assert!(decode_arithmetic_refined(&[0],options(),&[],&n,100,20000).is_err());
        assert!(decode_arithmetic_refined(EMPTY,options(),&[],&n,MAX_TEXT_DECISIONS+1,20000).is_err());
        assert!(decode_arithmetic_refined(EMPTY,options(),&[],&n,100,bitmap::MAX_COMPOSE_WORK+1).is_err());
    }
}

#[cfg(test)]
mod refined_text_vectors{
    use super::*;
    use super::super::refinement::{Neighborhood,Template};
    const T0:&[u8]=&[169,113,170,191,21,40,159,255,172];
    const T1:&[u8]=&[169,113,170,191,21,40,159,255,172];
    const BAD_RI:&[u8]=&[169,99,255,172];
    const BAD_DIM:&[u8]=&[169,105,177,227,255,172];
    fn opts(n:u32)->Options{Options{width:8,height:1,default_black:false,op:ComposeOp::Or,
        strips:1,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:n}}
    fn run(t:Template,data:&[u8],n:u32,decisions:usize,work:u64)->Result<DecodedText>{
        let src=Bitmap::from_packed(1,1,&[128])?;
        decode_arithmetic_refined(data,opts(n),&[&src],&Neighborhood::nominal(t),decisions,work)
    }
    #[test]
    fn refined_text_mixed_reference_pixels(){
        for (t,data) in [(Template::T0,T0),(Template::T1,T1)]{
            let out=run(t,data,3,70,20000).unwrap();
            assert_eq!(out.bitmap.packed(),[192]);
            assert_eq!((out.stats.instances,out.stats.strips,out.stats.composed_pixels),(3,1,3));
        }
    }
    #[test]
    fn refined_text_shared_gr_exact_budget(){
        for (t,data) in [(Template::T0,T0),(Template::T1,T1)]{
            let work=1+9*512+t.context_count() as u64+16+2*(t.bits() as u64+1);
            let out=run(t,data,3,70,work).unwrap();
            assert_eq!(out.stats.work_used,work);
            assert!(run(t,data,3,70,work-1).is_err());
            assert!(run(t,data,3,69,work).is_err());
        }
    }
    #[test]
    fn refined_text_reject_indicator(){
        assert!(matches!(run(Template::T0,BAD_RI,1,100,20000),
            Err(Error::Invalid("JBIG2 text refinement indicator"))));
    }
    #[test]
    fn refined_text_reject_zero_dimension(){
        assert!(matches!(run(Template::T1,BAD_DIM,1,100,20000),
            Err(Error::Invalid("JBIG2 text refinement dimension"))));
    }
}

#[cfg(test)]
mod shared_text_tests{
    use super::*;
    use super::super::{integer::IaidDecoder,qm::Decoder,refinement::{ContextBank,Neighborhood,Template}};
    fn opts()->Options{Options{width:8,height:1,default_black:false,op:ComposeOp::Or,
        strips:2,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:0}}
    #[test]
    fn shared_text_fixed_namespace_preflight(){
        let mut d=Decoder::new(&[255,172],0).unwrap();
        let mut id=IaidDecoder::new(0).unwrap();let mut c=ArithmeticTextContexts::new();
        let b=Bitmap::new(1,1,false).unwrap();
        assert!(matches!(decode_arithmetic_shared(opts(),&[&b,&b],&mut d,&mut id,&mut c,None,None,100),
            Err(Error::Limit("JBIG2 text shared namespace"))));
    }
    #[test]
    fn shared_text_gr_template_preflight(){
        let mut d=Decoder::new(&[255,172],0).unwrap();
        let mut id=IaidDecoder::new(0).unwrap();let mut c=ArithmeticTextContexts::new();
        let n=Neighborhood::nominal(Template::T0);let mut g=ContextBank::new(Template::T1).unwrap();
        assert!(matches!(decode_arithmetic_shared(opts(),&[],&mut d,&mut id,&mut c,Some(&n),Some(&mut g),100),
            Err(Error::Invalid("JBIG2 text shared GR template"))));
    }
    #[test]
    fn shared_text_existing_reference_vector(){
        let data=[147,150,68,103,175,63,255,172];
        let mut d=Decoder::new(&data,50).unwrap();let mut id=IaidDecoder::new(2).unwrap();
        let mut c=ArithmeticTextContexts::new();let b=Bitmap::from_packed(1,1,&[128]).unwrap();
        let o=Options{height:4,instances:3,..opts()};
        let out=decode_arithmetic_shared(o,&[&b,&b,&b],&mut d,&mut id,&mut c,None,None,100).unwrap();
        assert_eq!(out.bitmap.packed(),[16,64,128,0]);
    }
}

/// Canonical Huffman symbol IDs, ordered by symbol namespace index.
/// Zero lengths disable IDs. The all-zero singleton is deliberately
/// excluded from this scoped entry; wire-format table parsing is separate.
pub struct HuffmanSymbolIds{
    table:super::huffman::Table,
    count:usize,
}
impl HuffmanSymbolIds{
    pub fn new(lengths:&[u8])->Result<Self>{
        use super::huffman::{Row,Range,Table,MAX_ROWS};
        if lengths.len()>MAX_ROWS{return Err(Error::Limit("JBIG2 Huffman symbol table rows"));}
        if lengths.is_empty(){return Err(Error::Invalid("JBIG2 empty Huffman symbol table"));}
        let mut rows=Vec::new();
        rows.try_reserve_exact(lengths.len()).map_err(|_|Error::Limit("JBIG2 Huffman symbol rows"))?;
        for (id,&prefix_len) in lengths.iter().enumerate(){
            rows.push(Row{prefix_len,range:Range::Normal{base:id as i64,extra:0}});
        }
        Ok(Self{table:Table::new(&rows)?,count:lengths.len()})
    }
    /// Read the 35 four-bit run-code lengths, then expand symbol lengths.
    /// Leaves the reader at the next bit; the region caller owns alignment.
    pub fn from_wire(reader:&mut crate::filters::ccitt::BitReader<'_>,count:usize)->Result<Self>{
        use super::huffman::{Row,Range,Table,MAX_ROWS};
        use super::integer::Integer;
        if count==0{return Err(Error::Invalid("JBIG2 empty Huffman symbol table"));}
        if count>MAX_ROWS{return Err(Error::Limit("JBIG2 Huffman symbol table rows"));}
        fn bits(r:&mut crate::filters::ccitt::BitReader<'_>,n:u8)->Result<usize>{
            let mut v=0usize;for _ in 0..n{v=(v<<1)|usize::from(r.bit()?);}Ok(v)
        }
        let mut rows=[Row{prefix_len:0,range:Range::Normal{base:0,extra:0}};35];
        for (i,row) in rows.iter_mut().enumerate(){
            *row=Row{prefix_len:bits(reader,4)? as u8,range:Range::Normal{base:i as i64,extra:0}};
        }
        let run=Table::new(&rows)?;
        let mut lengths=Vec::new();
        lengths.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 Huffman symbol lengths"))?;
        while lengths.len()<count{
            let Integer::Value(code)=run.decode(reader)? else{return Err(Error::Invalid("JBIG2 symbol length code"));};
            let (length,n)=match code{
                0..=31=>(code as u8,1),
                32=>(*lengths.last().ok_or(Error::Invalid("JBIG2 symbol length repeat without previous"))?,bits(reader,2)?+3),
                33=>(0,bits(reader,3)?+3),
                34=>(0,bits(reader,7)?+11),
                _=>return Err(Error::Invalid("JBIG2 symbol length code")),
            };
            let end=lengths.len().checked_add(n).ok_or(Error::Limit("JBIG2 symbol length run"))?;
            if end>count{return Err(Error::Invalid("JBIG2 symbol length run exceeds count"));}
            lengths.resize(end,length);
        }
        Self::new(&lengths)
    }
    pub fn count(&self)->usize{self.count}
    pub fn decode(&self,reader:&mut crate::filters::ccitt::BitReader<'_>)->Result<usize>{
        match self.table.decode(reader)?{
            super::integer::Integer::Value(id) if id>=0&&(id as usize)<self.count=>Ok(id as usize),
            _=>Err(Error::Invalid("JBIG2 Huffman symbol ID")),
        }
    }
}

#[cfg(test)]
mod huffman_symbol_id_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    #[test]
    fn huffman_symbol_ids_canonical_order(){
        // Independent codewords: ID 1=0, ID 3=10, ID 0=110, ID 4=111.
        let t=HuffmanSymbolIds::new(&[3,1,0,2,3]).unwrap();
        let mut r=BitReader::new(&[0x5b,0x80],9).unwrap();
        for id in [1,3,0,4]{assert_eq!(t.decode(&mut r).unwrap(),id);}
        assert_eq!(r.remaining(),0);assert_eq!(t.count(),5);
    }
    #[test]
    fn huffman_symbol_ids_invalid_and_truncated(){
        for lengths in [&[][..],&[0,0],&[1,1,1],&[33]]{
            assert!(HuffmanSymbolIds::new(lengths).is_err());
        }
        assert!(HuffmanSymbolIds::new(&vec![13;super::super::huffman::MAX_ROWS+1]).is_err());
        let t=HuffmanSymbolIds::new(&[0,2]).unwrap();
        assert!(t.decode(&mut BitReader::new(&[0x80],2).unwrap()).is_err());
        assert!(t.decode(&mut BitReader::new(&[0],1).unwrap()).is_err());
        assert_eq!(t.decode(&mut BitReader::new(&[0],2).unwrap()).unwrap(),1);
    }
}

#[cfg(test)]
mod huffman_symbol_wire_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    // Hand-built wire prefix: run alphabet ID 3=00, 32=01,
    // 33=10, 34=11. Four-bit lengths at those indices are all 2.
    fn wire(tail:&str)->(Vec<u8>,usize){
        let mut bits=String::new();
        for i in 0..35{bits.push_str(if [3,32,33,34].contains(&i){"0010"}else{"0000"});}
        bits.push_str(tail);
        let n=bits.len();let mut data=vec![0u8;n.div_ceil(8)];
        for (i,b) in bits.bytes().enumerate(){if b==b'1'{data[i/8]|=128>>(i%8);}}
        (data,n)
    }
    #[test]
    fn huffman_symbol_wire_all_run_types(){
        // literal 3; repeat previous three; zero run three; zero run eleven.
        // Four enabled IDs have codes 000..011. Then read ID 3.
        let (data,n)=wire("00010010000110000000011");
        let mut r=BitReader::new(&data,n).unwrap();
        let ids=HuffmanSymbolIds::from_wire(&mut r,18).unwrap();
        assert_eq!(ids.count(),18);assert_eq!(r.position(),160);
        assert_eq!(ids.decode(&mut r).unwrap(),3);assert_eq!(r.remaining(),0);
    }
    #[test]
    fn huffman_symbol_wire_rejects_bad_runs(){
        for (tail,count) in [("0100",3),("000100",3),("0010000",3),("00110000000",11)]{
            let (data,n)=wire(tail);
            assert!(HuffmanSymbolIds::from_wire(&mut BitReader::new(&data,n).unwrap(),count).is_err());
        }
        let (data,n)=wire("00010010000110000000011");
        for end in 0..160{assert!(HuffmanSymbolIds::from_wire(&mut BitReader::new(&data,end).unwrap(),18).is_err());}
        let mut r=BitReader::new(&data,n).unwrap();
        assert!(HuffmanSymbolIds::from_wire(&mut r,0).is_err());
        assert!(HuffmanSymbolIds::from_wire(&mut r,super::super::huffman::MAX_ROWS+1).is_err());
        assert_eq!(r.position(),0);
    }
}

/// Resolved Huffman text integer tables. Selectors and custom segment
/// dependency resolution are handled separately from entropy decoding.
pub struct HuffmanTextIntegers{
    fs:super::huffman::Table,
    ds:super::huffman::Table,
    dt:super::huffman::Table,
}
impl HuffmanTextIntegers{
    /// Standard table numbers, not header selector bit patterns.
    pub fn standard(fs:u8,ds:u8,dt:u8)->Result<Self>{
        use super::huffman::standard::table;
        if !matches!(fs,6|7)||!matches!(ds,8|9|10)||!matches!(dt,11|12|13){
            return Err(Error::Invalid("JBIG2 Huffman text table selection"));
        }
        Self::new(table(fs)?,table(ds)?,table(dt)?)
    }
    pub fn new(fs:super::huffman::Table,ds:super::huffman::Table,dt:super::huffman::Table)->Result<Self>{
        // OOB capability is permitted only on the subsequent-symbol table.
        // Inspect the enabled code/row pairs through the table API below.
        if fs.has_oob()||!ds.has_oob()||dt.has_oob(){
            return Err(Error::Invalid("JBIG2 Huffman text table OOB capability"));
        }
        Ok(Self{fs,ds,dt})
    }
    fn required(v:super::integer::Integer)->Result<i64>{
        match v{super::integer::Integer::Value(n)=>Ok(n),_=>Err(Error::Invalid("JBIG2 Huffman text unexpected OOB"))}
    }
    pub fn first(&self,r:&mut crate::filters::ccitt::BitReader<'_>)->Result<i64>{
        Self::required(self.fs.decode(r)?)
    }
    pub fn delta(&self,r:&mut crate::filters::ccitt::BitReader<'_>)->Result<super::integer::Integer>{
        self.ds.decode(r)
    }
    pub fn strip(&self,r:&mut crate::filters::ccitt::BitReader<'_>)->Result<i64>{
        Self::required(self.dt.decode(r)?)
    }
    pub fn curt(r:&mut crate::filters::ccitt::BitReader<'_>,strips:u8)->Result<u8>{
        if !matches!(strips,1|2|4|8){return Err(Error::Invalid("JBIG2 Huffman text strips"));}
        let mut n=0u8;
        for _ in 0..strips.trailing_zeros(){n=(n<<1)|r.bit()?;}
        Ok(n)
    }
}
#[cfg(test)]
mod huffman_text_integer_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    use super::super::{huffman::{Table,Row,Range},integer::Integer};
    #[test]
    fn huffman_text_integer_reference(){
        let t=HuffmanTextIntegers::standard(6,8,11).unwrap();
        // B.11 DT=1 (0); B.6 FS=3 (00 0000011);
        // CURT=3 (11); B.8 DS=-1 (1010), OOB (01).
        let mut r=BitReader::new(&[0,250,64],18).unwrap();
        assert_eq!(t.strip(&mut r).unwrap(),1);
        assert_eq!(t.first(&mut r).unwrap(),3);
        assert_eq!(HuffmanTextIntegers::curt(&mut r,4).unwrap(),3);
        assert_eq!(t.delta(&mut r).unwrap(),Integer::Value(-1));
        assert_eq!(t.delta(&mut r).unwrap(),Integer::Oob);
        assert_eq!(r.remaining(),0);
    }
    #[test]
    fn huffman_text_integer_selection_and_bounds(){
        for fs in [6,7]{for ds in [8,9,10]{for dt in [11,12,13]{
            assert!(HuffmanTextIntegers::standard(fs,ds,dt).is_ok());
        }}}
        assert!(HuffmanTextIntegers::standard(8,8,11).is_err());
        assert!(HuffmanTextIntegers::standard(6,6,11).is_err());
        assert!(HuffmanTextIntegers::standard(6,8,8).is_err());
        let normal=||Table::new(&[Row{prefix_len:1,range:Range::Normal{base:0,extra:0}}]).unwrap();
        let oob=||Table::new(&[Row{prefix_len:1,range:Range::Oob}]).unwrap();
        assert!(HuffmanTextIntegers::new(oob(),oob(),normal()).is_err());
        assert!(HuffmanTextIntegers::new(normal(),normal(),normal()).is_err());
        assert!(HuffmanTextIntegers::new(normal(),oob(),oob()).is_err());
        let mut r=BitReader::new(&[0xe0],3).unwrap();
        assert_eq!(HuffmanTextIntegers::curt(&mut r,1).unwrap(),0);
        assert!(HuffmanTextIntegers::curt(&mut r,3).is_err());
        assert_eq!(r.position(),0);
        assert_eq!(HuffmanTextIntegers::curt(&mut r,8).unwrap(),7);
        assert!(HuffmanTextIntegers::curt(&mut r,2).is_err());
    }
}
