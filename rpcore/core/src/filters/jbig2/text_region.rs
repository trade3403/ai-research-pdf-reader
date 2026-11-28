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
/// Zero lengths disable IDs; they never encode a zero-bit singleton.
/// Only the full payload's zero-instance path admits an unused namespace.
pub struct HuffmanSymbolIds{
    table:Option<super::huffman::Table>,
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
        Ok(Self{table:Some(Table::new(&rows)?),count:lengths.len()})
    }
    /// Read the 35 four-bit run-code lengths, then expand symbol lengths.
    /// Leaves the reader at the next bit; the region caller owns alignment.
    pub fn from_wire(reader:&mut crate::filters::ccitt::BitReader<'_>,count:usize)->Result<Self>{
        Self::from_wire_impl(reader,count,false)
    }
    fn from_wire_impl(reader:&mut crate::filters::ccitt::BitReader<'_>,count:usize,allow_unused:bool)->Result<Self>{
        use super::huffman::{Row,Range,Table,MAX_ROWS};
        use super::integer::Integer;
        if count==0&&!allow_unused{return Err(Error::Invalid("JBIG2 empty Huffman symbol table"));}
        if count>MAX_ROWS{return Err(Error::Limit("JBIG2 Huffman symbol table rows"));}
        fn bits(r:&mut crate::filters::ccitt::BitReader<'_>,n:u8)->Result<usize>{
            let mut v=0usize;for _ in 0..n{v=(v<<1)|usize::from(r.bit()?);}Ok(v)
        }
        let mut rows=[Row{prefix_len:0,range:Range::Normal{base:0,extra:0}};35];
        for (i,row) in rows.iter_mut().enumerate(){
            *row=Row{prefix_len:bits(reader,4)? as u8,range:Range::Normal{base:i as i64,extra:0}};
        }
        // No symbol lengths to expand: still consume the complete 35-nibble
        // alphabet and validate any enabled run codes before alignment.
        if count==0{
            if rows.iter().any(|r|r.prefix_len!=0){Table::new(&rows)?;}
            return Ok(Self{table:None,count:0});
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
        if allow_unused&&lengths.iter().all(|&n|n==0){return Ok(Self{table:None,count});}
        Self::new(&lengths)
    }
    pub fn count(&self)->usize{self.count}
    pub fn decode(&self,reader:&mut crate::filters::ccitt::BitReader<'_>)->Result<usize>{
        match self.table.as_ref().ok_or(Error::Invalid("JBIG2 no enabled symbol IDs"))?.decode(reader)?{
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

/// Decode a non-refining Huffman text region with already resolved tables
/// and symbol IDs. The reader starts at the first HTDT, after any wire
/// table alignment. Its exact bit limit bounds entropy input separately
/// from the region's pixel/placement work limit.
pub fn decode_huffman_direct(reader:&mut crate::filters::ccitt::BitReader<'_>,
    options:Options,symbols:&[&Bitmap],ids:&HuffmanSymbolIds,
    integers:&HuffmanTextIntegers,bit_limit:usize,work_limit:u64)->Result<DecodedText>{
    use super::integer::Integer;
    if symbols.len()!=ids.count(){return Err(Error::Invalid("JBIG2 Huffman text symbol namespace"));}
    if options.instances!=0&&ids.table.is_none(){return Err(Error::Invalid("JBIG2 no enabled symbol IDs"));}
    if bit_limit>MAX_TEXT_DECISIONS{return Err(Error::Limit("JBIG2 Huffman text bits"));}
    // Bound the reader window before decoding, not just after a field.
    // This scoped entry requires the caller to delimit its entropy span.
    if reader.remaining()>bit_limit{return Err(Error::Limit("JBIG2 Huffman text bits"));}
    let initial=integers.strip(reader)?;
    let mut region=Region::new(options,initial,work_limit)?;
    while region.remaining_instances()!=0{
        let dt=integers.strip(reader)?;
        let fs=integers.first(reader)?;
        region.begin_strip(dt,fs)?;
        let mut offset=None;
        loop{
            let curt=HuffmanTextIntegers::curt(reader,options.strips)?;
            let id=ids.decode(reader)?;
            let symbol=symbols.get(id).ok_or(Error::Invalid("JBIG2 Huffman text symbol ID"))?;
            region.place(offset,curt,symbol)?;
            match integers.delta(reader)?{
                Integer::Oob=>{region.end_strip()?;break;}
                Integer::Value(n)=>{
                    if region.remaining_instances()==0{return Err(Error::Invalid("JBIG2 Huffman text excess instances"));}
                    offset=Some(n);
                }
            }
        }
    }
    region.finish()
}

#[cfg(test)]
mod huffman_direct_text_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    fn options(n:u32)->Options{Options{width:8,height:4,default_black:false,
        op:ComposeOp::Or,strips:2,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:n}}
    // Independent bit syntax: initial DT=1; strip DT=1 FS=1,
    // CURT=0 ID=0; DS=2 CURT=1 ID=1; OOB;
    // strip DT=1 FS=0 CURT=0 ID=0 OOB.
    // B.11 DT=1:0, B.6 FS=1:000000001, FS=0:000000000,
    // B.8 DS=2:11010, OOB:01. IDs use one bit each.
    const GOOD:&[u8]=&[0,38,180,0,16];
    const N:usize=36;
    fn run(data:&[u8],bits:usize,n:u32,cap:usize,work:u64)->Result<DecodedText>{
        let a=Bitmap::from_packed(1,1,&[128])?;
        let b=Bitmap::from_packed(2,1,&[192])?;
        let ids=HuffmanSymbolIds::new(&[1,1])?;
        let t=HuffmanTextIntegers::standard(6,8,11)?;
        let mut r=BitReader::new(data,bits)?;
        let d=decode_huffman_direct(&mut r,options(n),&[&a,&b],&ids,&t,cap,work)?;
        assert_eq!(r.remaining(),0);
        Ok(d)
    }
    #[test]
    fn huffman_direct_text_two_strips_pixels(){
        let d=run(GOOD,N,3,N,43).unwrap();
        assert_eq!(d.bitmap.packed(),[64,24,64,0]);
        assert_eq!(d.stats,Stats{strips:2,instances:3,work_used:43,composed_pixels:4});
    }
    #[test]
    fn huffman_direct_text_exact_bounds(){
        assert!(run(GOOD,N,3,N,42).is_err());
        assert!(run(GOOD,N,3,N-1,43).is_err());
        for end in 0..N{assert!(run(GOOD,end,3,N,43).is_err());}
        assert!(run(GOOD,N,1,N,43).is_err());
        assert!(run(GOOD,N,4,N,100).is_err());
    }
    #[test]
    fn huffman_direct_text_empty_and_namespace(){
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        let ids=HuffmanSymbolIds::new(&[1]).unwrap();
        let t=HuffmanTextIntegers::standard(6,8,11).unwrap();
        let mut r=BitReader::new(&[0],1).unwrap();
        assert!(decode_huffman_direct(&mut r,options(0),&[],&ids,&t,1,32).is_err());
        assert_eq!(r.position(),0);
        let d=decode_huffman_direct(&mut r,options(0),&[&a],&ids,&t,1,32).unwrap();
        assert_eq!(d.bitmap.packed(),[0;4]);assert_eq!(d.stats.work_used,32);
        assert_eq!(r.remaining(),0);
    }
}

/// Resolved refinement delta and block-size tables. The four deltas
/// can independently select Annex B.14/B.15 or a resolved custom table.
pub struct HuffmanTextRefinementTables{
    deltas:[super::huffman::Table;4],
    size:super::huffman::Table,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HuffmanTextRefinementFields{
    pub dw:i64,pub dh:i64,pub dx:i64,pub dy:i64,pub bytes:usize,
}
impl HuffmanTextRefinementTables{
    pub fn standard(numbers:[u8;4])->Result<Self>{
        use super::huffman::standard::table;
        if numbers.iter().any(|&n|!matches!(n,14|15)){
            return Err(Error::Invalid("JBIG2 Huffman text refinement table selection"));
        }
        Self::new([table(numbers[0])?,table(numbers[1])?,table(numbers[2])?,table(numbers[3])?],table(1)?)
    }
    pub fn new(deltas:[super::huffman::Table;4],size:super::huffman::Table)->Result<Self>{
        if deltas.iter().any(|t|t.has_oob())||size.has_oob(){
            return Err(Error::Invalid("JBIG2 Huffman text refinement OOB table"));
        }
        Ok(Self{deltas,size})
    }
    /// One RI bit followed by RDW/RDH/RDX/RDY and RSIZE when set.
    /// No alignment or block bytes are consumed here.
    pub fn read(&self,r:&mut crate::filters::ccitt::BitReader<'_>)->Result<Option<HuffmanTextRefinementFields>>{
        if r.bit()?==0{return Ok(None);}
        let mut v=[0i64;4];
        for (out,t) in v.iter_mut().zip(&self.deltas){
            *out=HuffmanTextIntegers::required(t.decode(r)?)?;
        }
        let bytes=usize::try_from(HuffmanTextIntegers::required(self.size.decode(r)?)?)
            .map_err(|_|Error::Invalid("JBIG2 Huffman text negative refinement size"))?;
        if bytes<2{return Err(Error::Invalid("JBIG2 Huffman text short refinement block"));}
        Ok(Some(HuffmanTextRefinementFields{dw:v[0],dh:v[1],dx:v[2],dy:v[3],bytes}))
    }
}
/// Resolve a declared byte-aligned refinement payload in the SAME input
/// window as the reader. This helper only calculates bounds; the caller
/// consumes alignment and bytes or creates a new reader at the end.
pub fn huffman_refinement_span(bit_position:usize,remaining_bits:usize,bytes:usize)
    ->Result<std::ops::Range<usize>>{
    if bytes<2{return Err(Error::Invalid("JBIG2 Huffman text short refinement block"));}
    let start=bit_position.div_ceil(8);
    let end=start.checked_add(bytes).ok_or(Error::Limit("JBIG2 Huffman text refinement end"))?;
    let end_bits=end.checked_mul(8).ok_or(Error::Limit("JBIG2 Huffman text refinement bits"))?;
    let limit=bit_position.checked_add(remaining_bits).ok_or(Error::Limit("JBIG2 Huffman text input bits"))?;
    if end_bits>limit{return Err(Error::Invalid("JBIG2 Huffman text truncated refinement block"));}
    Ok(start..end)
}
#[cfg(test)]
mod huffman_text_refinement_field_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    #[test]
    fn huffman_text_refinement_fields_reference(){
        let t=HuffmanTextRefinementTables::standard([14,15,14,15]).unwrap();
        // RI=1, N:-2=100, O:+1=101, N:-1=101, O:0=0,
        // B.1 size=3:00011. 16 bits, followed by a 3-byte QM block.
        let data=[203,67,0xbb,0xff,0xac];
        let mut r=BitReader::new(&data,40).unwrap();
        assert_eq!(t.read(&mut r).unwrap(),Some(HuffmanTextRefinementFields{dw:-2,dh:1,dx:-1,dy:0,bytes:3}));
        assert_eq!(r.position(),16);
        assert_eq!(huffman_refinement_span(r.position(),r.remaining(),3).unwrap(),2..5);
    }
    #[test]
    fn huffman_text_refinement_field_bounds(){
        let t=HuffmanTextRefinementTables::standard([14;4]).unwrap();
        let mut r=BitReader::new(&[0],1).unwrap();
        assert_eq!(t.read(&mut r).unwrap(),None);assert_eq!(r.remaining(),0);
        assert!(HuffmanTextRefinementTables::standard([13;4]).is_err());
        // RI=1, four zero deltas, size 1.
        assert!(t.read(&mut BitReader::new(&[128,64],10).unwrap()).is_err());
        assert_eq!(huffman_refinement_span(13,27,3).unwrap(),2..5);
        assert!(huffman_refinement_span(13,26,3).is_err());
        assert!(huffman_refinement_span(13,27,1).is_err());
        assert!(huffman_refinement_span(usize::MAX,1,2).is_err());
        assert!(huffman_refinement_span(0,40,usize::MAX).is_err());
    }
}

/// Refined Huffman text region with a fresh GR bank shared by all QM
/// blocks. Input begins at HTDT, after table alignment. Each block is
/// bounded by its declared byte length; resume at that end, not QM read-ahead.
pub fn decode_huffman_refined(data:&[u8],bit_len:usize,options:Options,
    symbols:&[&Bitmap],ids:&HuffmanSymbolIds,integers:&HuffmanTextIntegers,
    tables:&HuffmanTextRefinementTables,neighborhood:&super::refinement::Neighborhood,
    decision_limit:usize,work_limit:u64)->Result<DecodedText>{
    decode_huffman_refined_bounded(data,bit_len,options,symbols,ids,integers,tables,neighborhood,
        decision_limit,work_limit,bitmap::MAX_PACKED_BYTES,&mut 0)
}
fn decode_huffman_refined_bounded(data:&[u8],bit_len:usize,options:Options,
    symbols:&[&Bitmap],ids:&HuffmanSymbolIds,integers:&HuffmanTextIntegers,
    tables:&HuffmanTextRefinementTables,neighborhood:&super::refinement::Neighborhood,
    decision_limit:usize,work_limit:u64,temporary_packed:usize,decisions_used:&mut usize)->Result<DecodedText>{
    use super::{integer::Integer,refinement,qm};
    use crate::filters::ccitt::BitReader;
    if symbols.len()!=ids.count(){return Err(Error::Invalid("JBIG2 Huffman text symbol namespace"));}
    if options.instances!=0&&ids.table.is_none(){return Err(Error::Invalid("JBIG2 no enabled symbol IDs"));}
    if bit_len>MAX_TEXT_DECISIONS||decision_limit>MAX_TEXT_DECISIONS||
        work_limit>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 Huffman refined text limits"));}
    let mut reader=BitReader::new(data,bit_len)?;
    let template=neighborhood.template();
    // Precharge the complete bounded input and GR initialization.
    let mut extra=(bit_len as u64).checked_add(template.context_count() as u64)
        .ok_or(Error::Limit("JBIG2 Huffman refined text work"))?;
    let region_limit=work_limit.checked_sub(extra).ok_or(Error::Limit("JBIG2 Huffman refined text work"))?;
    let mut gr=refinement::ContextBank::new(template)?;
    let mut remaining=decision_limit;
    let mut base=0usize;
    let initial=integers.strip(&mut reader)?;
    let mut region=Region::new(options,initial,region_limit)?;
    while region.remaining_instances()!=0{
        let dt=integers.strip(&mut reader)?;
        let fs=integers.first(&mut reader)?;
        region.begin_strip(dt,fs)?;
        let mut offset=None;
        loop{
            let curt=HuffmanTextIntegers::curt(&mut reader,options.strips)?;
            let id=ids.decode(&mut reader)?;
            let src=symbols.get(id).ok_or(Error::Invalid("JBIG2 Huffman text symbol ID"))?;
            if let Some(fields)=tables.read(&mut reader)?{
                let span=huffman_refinement_span(reader.position(),reader.remaining(),fields.bytes)?;
                let start=base.checked_add(span.start).ok_or(Error::Limit("JBIG2 Huffman text block start"))?;
                let end=base.checked_add(span.end).ok_or(Error::Limit("JBIG2 Huffman text block end"))?;
                let block=data.get(start..end).ok_or(Error::Invalid("JBIG2 Huffman text truncated block"))?;
                let opt=refinement_geometry(src,fields.dw,fields.dh,fields.dx,fields.dy)?;
                if u64::from(opt.width).div_ceil(8)*u64::from(opt.height)>temporary_packed as u64{
                    return Err(Error::Limit("JBIG2 Huffman text temporary packed budget"));
                }
                let area=usize::try_from(u64::from(opt.width)*u64::from(opt.height))
                    .map_err(|_|Error::Limit("JBIG2 Huffman text decisions"))?;
                remaining=remaining.checked_sub(area).ok_or(Error::Limit("JBIG2 Huffman text decisions"))?;
                let work=refinement::work_bound(template,opt)?;
                extra=extra.checked_add(work).ok_or(Error::Limit("JBIG2 Huffman refined text work"))?;
                // Reserve refinement work from the same budget as placement.
                region.left=region.left.checked_sub(work).ok_or(Error::Limit("JBIG2 Huffman refined text work"))?;
                let mut decoder=qm::Decoder::new(block,area)?;
                let decoded=refinement::decode_bitmap(src,neighborhood,opt,&mut gr,&mut decoder,work)?;
                region.place(offset,curt,&decoded.bitmap)?;
                base=end;
                let used=base.checked_mul(8).ok_or(Error::Limit("JBIG2 Huffman text block end"))?;
                let rest=bit_len.checked_sub(used).ok_or(Error::Invalid("JBIG2 Huffman text block end"))?;
                reader=BitReader::new(&data[base..],rest)?;
            }else{
                region.place(offset,curt,src)?;
            }
            match integers.delta(&mut reader)?{
                Integer::Oob=>{region.end_strip()?;break;}
                Integer::Value(n)=>{
                    if region.remaining_instances()==0{return Err(Error::Invalid("JBIG2 Huffman text excess instances"));}
                    offset=Some(n);
                }
            }
        }
    }
    if reader.remaining()>7{return Err(Error::Invalid("JBIG2 Huffman refined text trailing bits"));}
    let mut out=region.finish()?;
    out.stats.work_used=out.stats.work_used.checked_add(extra).ok_or(Error::Limit("JBIG2 Huffman refined text work"))?;
    *decisions_used=decision_limit-remaining;
    Ok(out)
}
#[cfg(test)]
mod huffman_refined_integration_tests{
    use super::*;
    use super::super::refinement::{Neighborhood,Template};
    fn run(data:&[u8],bits:usize,decisions:usize,work:u64)->Result<DecodedText>{
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0])?;
        let o=Options{width:8,height:4,default_black:false,op:ComposeOp::Or,
            strips:1,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:1};
        let ids=HuffmanSymbolIds::new(&[1])?;
        let ints=HuffmanTextIntegers::standard(6,8,11)?;
        let tables=HuffmanTextRefinementTables::standard([15;4])?;
        decode_huffman_refined(data,bits,o,&[&a],&ids,&ints,&tables,&Neighborhood::nominal(Template::T1),decisions,work)
    }
    // Reference block from the independently encoded dictionary T1 fixture:
    // 7x6 source, target 3x2, dx=1,dy=-1. Text deltas compensate floor(-4/2).
    // initial DT=1, DT=1, FS=0, ID=0, RI=1; DW=-4 DH=-4 DX=3 DY=1;
    // RSIZE=3, pad, QM [187,255,172], DS OOB.
    const DATA:&[u8]=&[0,15,28,117,70,187,255,172,64];
    #[test]
    fn huffman_refined_text_single_block(){
        let d=run(DATA,DATA.len()*8,6,20000).unwrap();
        assert_eq!(d.bitmap.packed(),[160,64,0,0]);
        assert_eq!(d.stats.instances,1);
        assert_eq!(run(DATA,DATA.len()*8,6,d.stats.work_used).unwrap(),d);
        assert!(run(DATA,DATA.len()*8,6,d.stats.work_used-1).is_err());
        assert!(run(DATA,DATA.len()*8,5,20000).is_err());
    }
    #[test]
    fn huffman_refined_text_block_bounds(){
        for n in 0..DATA.len(){assert!(run(&DATA[..n],n*8,6,20000).is_err());}
        let mut bad=DATA.to_vec();let n=bad.len();bad[n-2]=0x90;
        assert!(run(&bad,bad.len()*8,6,20000).is_err());
    }
}

#[cfg(test)]
mod huffman_refined_multi_tests{
    use super::*;
    use super::super::refinement::{Neighborhood,Template};
    // Repacked Huffman fields around the two independently encoded QM
    // blocks from dictionary::huffman_refinement_tests. The second
    // reference is an imported copy of the first target, not a dynamic ID.
    // First target 3x2 [160,64], second 4x2 [192,48].
    // Third symbol is unrefined, at x=0,y=2.
    fn fixture(template:Template)->Vec<u8>{
        let mut data=vec![0,15,28,117,70,187,255,172,215,82,144];
        data.extend([if template==Template::T0{221}else{220},127,255,172]);
        data.extend([96,2,64]);data
    }
    fn run(template:Template,data:&[u8],decisions:usize,work:u64)->Result<DecodedText>{
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0])?;
        let b=Bitmap::from_packed(3,2,&[160,64])?;
        let o=Options{width:8,height:4,default_black:false,op:ComposeOp::Or,
            strips:1,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:3};
        let ids=HuffmanSymbolIds::new(&[1,1])?;
        let ints=HuffmanTextIntegers::standard(6,8,11)?;
        let tables=HuffmanTextRefinementTables::standard([15;4])?;
        decode_huffman_refined(data,data.len()*8,o,&[&a,&b],&ids,&ints,&tables,&Neighborhood::nominal(template),decisions,work)
    }
    #[test]
    fn huffman_refined_multi_shared_gr_both_templates(){
        for template in [Template::T0,Template::T1]{
            let data=fixture(template);
            let d=run(template,&data,14,40000).unwrap();
            assert_eq!(d.bitmap.packed(),[172,67,160,64]);
            assert_eq!((d.stats.instances,d.stats.strips,d.stats.composed_pixels),(3,2,20));
            assert_eq!(run(template,&data,14,d.stats.work_used).unwrap(),d);
            assert!(run(template,&data,13,40000).is_err());
            assert!(run(template,&data,14,d.stats.work_used-1).is_err());
        }
    }
    #[test]
    fn huffman_refined_multi_truncation_and_marker(){
        for template in [Template::T0,Template::T1]{
            let data=fixture(template);
            for n in 0..data.len(){assert!(run(template,&data[..n],14,40000).is_err());}
            let mut bad=data.clone();bad[7]=0x90;
            assert!(run(template,&bad,14,40000).is_err());
            let mut bad=data.clone();bad[14]=0x90;
            assert!(run(template,&bad,14,40000).is_err());
            let mut bad=data.clone();bad.push(0);
            assert!(run(template,&bad,14,40000).is_err());
        }
    }
}

/// T.88 (2000) 7.4.3.1.2, 7.4.3.1.6. Custom indices are in
/// referred table segment order, not the complete referred segment list.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TextTableChoice{Standard(u8),Custom(usize)}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct TextHuffmanSelections{
    pub integers:[TextTableChoice;3],
    pub refinement:Option<[TextTableChoice;5]>,
    pub custom_count:usize,
}
impl TextHuffmanSelections{
    pub fn parse(flags:u16,refine:bool)->Result<Self>{
        if flags&0x8000!=0||(!refine&&flags&0x7fc0!=0){
            return Err(Error::Invalid("JBIG2 text Huffman reserved flags"));
        }
        let mut custom_count=0usize;
        let mut select=|shift:u8,base:u8,allow_two:bool|->Result<TextTableChoice>{
            match ((flags>>shift)&3) as u8{
                3=>{let i=custom_count;custom_count+=1;Ok(TextTableChoice::Custom(i))},
                2 if !allow_two=>Err(Error::Invalid("JBIG2 text Huffman selector")),
                n=>Ok(TextTableChoice::Standard(base+n)),
            }
        };
        let integers=[select(0,6,false)?,select(2,8,true)?,select(4,11,true)?];
        let refinement=if refine{
            let mut fields=[select(6,14,false)?,select(8,14,false)?,
                select(10,14,false)?,select(12,14,false)?,TextTableChoice::Standard(1)];
            if flags&0x4000!=0{fields[4]=TextTableChoice::Custom(custom_count);custom_count+=1;}
            Some(fields)
        }else{None};
        Ok(Self{integers,refinement,custom_count})
    }
}
/// Parse the fixed text header before the variable symbol ID code table.
/// Arithmetic text and colored-region extensions are outside this entry.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct HuffmanTextHeader{
    pub region:bitmap::RegionInfo,
    pub options:Options,
    pub selections:TextHuffmanSelections,
    pub neighborhood:Option<super::refinement::Neighborhood>,
}
pub fn parse_huffman_text_header(data:&[u8])->Result<(HuffmanTextHeader,&[u8])>{
    use super::{generic::Offset,refinement::{Neighborhood,Template}};
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JBIG2 text segment bytes"));}
    let (region,rest)=bitmap::parse_region(data)?;
    let h=rest.get(..4).ok_or(Error::Invalid("JBIG2 text flags truncated"))?;
    let flags=u16::from_be_bytes([h[0],h[1]]);
    if flags&1==0{return Err(Error::Unsupported("JBIG2 arithmetic text header"));}
    let refine=flags&2!=0;
    let template=Template::from_code((flags>>15)as u8)?;
    if !refine&&template==Template::T1{return Err(Error::Invalid("JBIG2 text unused refinement template"));}
    let selections=TextHuffmanSelections::parse(u16::from_be_bytes([h[2],h[3]]),refine)?;
    let rest=&rest[4..];
    let n=if refine{template.adaptive_count()*2}else{0};
    let at=rest.get(..n).ok_or(Error::Invalid("JBIG2 text adaptive flags truncated"))?;
    let neighborhood=if refine{
        let mut offsets=[Offset{x:0,y:0};2];
        for (i,p) in at.chunks_exact(2).enumerate(){offsets[i]=Offset{x:p[0]as i8,y:p[1]as i8};}
        Some(Neighborhood::new(template,&offsets[..n/2])?)
    }else{None};
    let rest=&rest[n..];
    let count=rest.get(..4).ok_or(Error::Invalid("JBIG2 text instance count truncated"))?;
    let instances=u32::from_be_bytes([count[0],count[1],count[2],count[3]]);
    if instances>MAX_INSTANCES{return Err(Error::Limit("JBIG2 text instances"));}
    let ds=((flags>>10)&31) as i8;
    let options=Options{width:region.width,height:region.height,default_black:flags&0x200!=0,
        op:ComposeOp::from_code(((flags>>7)&3)as u8)?,strips:1u8<<((flags>>2)&3),
        corner:Corner::from_code(((flags>>4)&3)as u8)?,transposed:flags&64!=0,
        ds_offset:if ds>=16{ds-32}else{ds},instances};
    Ok((HuffmanTextHeader{region,options,selections,neighborhood},&rest[4..]))
}

#[cfg(test)]
mod huffman_text_header_tests{
    use super::*;
    use TextTableChoice::{Standard,Custom};
    fn header(flags:u16,huff:u16,at:&[u8],count:u32)->Vec<u8>{
        let mut v=vec![0,0,0,8,0,0,0,4,0,0,0,1,0,0,0,2,0];
        v.extend(flags.to_be_bytes());v.extend(huff.to_be_bytes());
        v.extend(at);v.extend(count.to_be_bytes());v.push(0xa5);v
    }
    #[test]
    fn huffman_text_header_annex_h_flags(){
        let data=header(0x0c09,0x0010,&[],5);
        let (h,rest)=parse_huffman_text_header(&data).unwrap();
        assert_eq!(h.options,Options{width:8,height:4,default_black:false,op:ComposeOp::Or,
            strips:4,corner:Corner::BottomLeft,transposed:false,ds_offset:3,instances:5});
        assert_eq!(h.selections.integers,[Standard(6),Standard(8),Standard(12)]);
        assert_eq!(h.selections.custom_count,0);assert_eq!(rest,[0xa5]);assert!(h.neighborhood.is_none());
    }
    #[test]
    fn huffman_text_header_custom_order_and_templates(){
        let data=header(0x7fff,0x7fff,&[255;4],3);
        let (h,rest)=parse_huffman_text_header(&data).unwrap();
        assert_eq!(h.selections.integers,[Custom(0),Custom(1),Custom(2)]);
        assert_eq!(h.selections.refinement,Some([Custom(3),Custom(4),Custom(5),Custom(6),Custom(7)]));
        assert_eq!(h.selections.custom_count,8);
        assert_eq!(h.options.ds_offset,-1);assert_eq!(h.options.strips,8);
        assert!(h.options.transposed&&h.options.default_black);
        assert_eq!(h.neighborhood.unwrap().template(),super::super::refinement::Template::T0);
        assert_eq!(rest,[0xa5]);
        let data=header(0x8003,0,&[],1);
        assert_eq!(parse_huffman_text_header(&data).unwrap().0.neighborhood.unwrap().template(),super::super::refinement::Template::T1);
    }
    #[test]
    fn huffman_text_header_rejects_reserved_and_truncated(){
        for flags in [0x8000,2,0x40,0x4000]{assert!(TextHuffmanSelections::parse(flags,false).is_err());}
        for shift in [0,6,8,10,12]{assert!(TextHuffmanSelections::parse(2<<shift,true).is_err());}
        let data=header(3,0,&[255;4],1);
        for n in 0..29{assert!(parse_huffman_text_header(&data[..n]).is_err());}
        assert!(parse_huffman_text_header(&header(0x8001,0,&[],1)).is_err());
        assert!(parse_huffman_text_header(&header(1,0,&[],MAX_INSTANCES+1)).is_err());
        assert!(parse_huffman_text_header(&header(3,0,&[0;4],1)).is_err());
    }
}

/// Resolve only caller-validated custom table references, in referred
/// table order. A segment graph caller must validate identity and scope.
pub fn bind_text_tables(selections:&TextHuffmanSelections,custom:&[&super::huffman::CustomTable])
    ->Result<(HuffmanTextIntegers,Option<HuffmanTextRefinementTables>)>{
    use super::huffman::{Table,standard};
    if custom.len()!=selections.custom_count||custom.len()>8{
        return Err(Error::Invalid("JBIG2 text custom table count"));
    }
    let mut next=0usize;
    let mut resolve=|choice:TextTableChoice,allowed:&[u8]|->Result<Table>{
        match choice{
            TextTableChoice::Standard(n) if allowed.contains(&n)=>standard::table(n),
            TextTableChoice::Custom(i) if i==next=>{
                let t=custom.get(i).ok_or(Error::Invalid("JBIG2 text custom table missing"))?;
                next+=1;Table::new(t.rows())
            }
            _=>Err(Error::Invalid("JBIG2 text table binding order")),
        }
    };
    let [fs,ds,dt]=selections.integers;
    let integers=HuffmanTextIntegers::new(resolve(fs,&[6,7])?,resolve(ds,&[8,9,10])?,resolve(dt,&[11,12,13])?)?;
    let refinement=if let Some([dw,dh,dx,dy,size])=selections.refinement{
        Some(HuffmanTextRefinementTables::new([resolve(dw,&[14,15])?,resolve(dh,&[14,15])?,
            resolve(dx,&[14,15])?,resolve(dy,&[14,15])?],resolve(size,&[1])?)?)
    }else{None};
    if next!=custom.len(){return Err(Error::Invalid("JBIG2 text unused custom table"));}
    Ok((integers,refinement))
}
/// Scoped complete Huffman region payload decoder. Caller supplies the
/// ordered imported symbols and custom tables, not yet a page executor.
/// The encoded symbol table is byte-aligned before HTDT. Trailing partial
/// byte padding is permitted, never an additional whole byte.
pub fn decode_huffman_text_payload(data:&[u8],symbols:&[&Bitmap],custom:&[&super::huffman::CustomTable],
    bit_limit:usize,decision_limit:usize,work_limit:u64)->Result<(HuffmanTextHeader,DecodedText)>{
    decode_huffman_text_payload_impl(data,symbols,custom,bit_limit,decision_limit,work_limit,None,&mut 0)
}
/// Packed budget includes the output canvas and any one temporary refinement.
pub fn decode_huffman_text_payload_bounded(data:&[u8],symbols:&[&Bitmap],custom:&[&super::huffman::CustomTable],
    bit_limit:usize,decision_limit:usize,work_limit:u64,packed_limit:usize)->Result<(HuffmanTextHeader,DecodedText)>{
    decode_huffman_text_payload_impl(data,symbols,custom,bit_limit,decision_limit,work_limit,Some(packed_limit),&mut 0)
}
fn decode_huffman_text_payload_impl(data:&[u8],symbols:&[&Bitmap],custom:&[&super::huffman::CustomTable],
    bit_limit:usize,decision_limit:usize,work_limit:u64,packed_limit:Option<usize>,decisions_used:&mut usize)->Result<(HuffmanTextHeader,DecodedText)>{
    use crate::filters::ccitt::BitReader;
    if bit_limit>MAX_TEXT_DECISIONS||decision_limit>MAX_TEXT_DECISIONS||
        work_limit>bitmap::MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 text payload limits"));}
    let (header,body)=parse_huffman_text_header(data)?;
    let temporary=if let Some(cap)=packed_limit{
        if cap>bitmap::MAX_PACKED_BYTES{return Err(Error::Limit("JBIG2 Huffman text packed budget"));}
        let canvas=(header.region.width as usize).div_ceil(8).checked_mul(header.region.height as usize)
            .ok_or(Error::Limit("JBIG2 Huffman text packed canvas"))?;
        cap.checked_sub(canvas).ok_or(Error::Limit("JBIG2 Huffman text packed canvas"))?
    }else{bitmap::MAX_PACKED_BYTES};
    // Bound caller-provided namespaces before even traversing custom rows.
    if custom.len()!=header.selections.custom_count||custom.len()>8{
        return Err(Error::Invalid("JBIG2 text custom table count"));
    }
    if symbols.len()>super::huffman::MAX_ROWS{
        return Err(Error::Limit("JBIG2 Huffman symbol table rows"));
    }
    let bits=body.len().checked_mul(8).ok_or(Error::Limit("JBIG2 text payload bits"))?;
    if bits>bit_limit{return Err(Error::Limit("JBIG2 text payload bits"));}
    // Conservative table construction charge: each row has up to 32
    // prefix nodes. Include both code-length and custom table creation.
    let rows=custom.iter().try_fold(symbols.len().checked_add(35+8*32)
        .ok_or(Error::Limit("JBIG2 text table work"))?,|n,t|n.checked_add(t.rows().len())
        .ok_or(Error::Limit("JBIG2 text table work")))?;
    let overhead=(rows as u64).checked_mul(32).and_then(|v|v.checked_add((data.len()-body.len())as u64))
        .ok_or(Error::Limit("JBIG2 text table work"))?;
    let left=work_limit.checked_sub(overhead).ok_or(Error::Limit("JBIG2 text table work"))?;
    let (ints,refinement)=bind_text_tables(&header.selections,custom)?;
    let mut reader=BitReader::new(body,bits)?;
    let ids=HuffmanSymbolIds::from_wire_impl(&mut reader,symbols.len(),header.options.instances==0)?;
    let start=reader.position().div_ceil(8);
    let entropy=body.get(start..).ok_or(Error::Invalid("JBIG2 text alignment"))?;
    let entropy_bits=entropy.len()*8;
    let prefix_work=(start as u64)*8;
    let left=left.checked_sub(prefix_work).ok_or(Error::Limit("JBIG2 text table bits work"))?;
    let mut decoded=match (&header.neighborhood,refinement){
        (Some(n),Some(t))=>decode_huffman_refined_bounded(entropy,entropy_bits,header.options,symbols,&ids,&ints,&t,n,decision_limit,left,temporary,decisions_used)?,
        (None,None)=>{
            let left=left.checked_sub(entropy_bits as u64).ok_or(Error::Limit("JBIG2 text entropy work"))?;
            let mut r=BitReader::new(entropy,entropy_bits)?;
            let mut d=decode_huffman_direct(&mut r,header.options,symbols,&ids,&ints,entropy_bits,left)?;
            if r.remaining()>7{return Err(Error::Invalid("JBIG2 text trailing bytes"));}
            d.stats.work_used+=entropy_bits as u64;*decisions_used=0;d
        },
        _=>return Err(Error::Invalid("JBIG2 text refinement table presence")),
    };
    decoded.stats.work_used+=overhead+prefix_work;
    Ok((header,decoded))
}
#[cfg(test)]
mod huffman_text_payload_tests{
    use super::*;
    fn body()->Vec<u8>{
        let mut v=vec![0,0,0,8,0,0,0,4,0,0,0,0,0,0,0,0,0,0,0x15,0,0,0,0,0,3];
        // Run table: only literal length 1 has a one-bit prefix.
        // 140-bit table + two zero codes + two arbitrary pad bits.
        let mut table=[0u8;18];table[0]=1;table[17]=3;
        v.extend(table);v.extend([0,38,180,0,16]);v
    }
    #[test]
    fn huffman_text_payload_direct_alignment(){
        let data=body();
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        let b=Bitmap::from_packed(2,1,&[192]).unwrap();
        let (h,d)=decode_huffman_text_payload(&data,&[&a,&b],&[],184,0,30000).unwrap();
        assert_eq!(h.options.strips,2);assert_eq!(d.bitmap.packed(),[64,24,64,0]);
        assert_eq!(decode_huffman_text_payload(&data,&[&a,&b],&[],184,0,d.stats.work_used).unwrap().1,d);
        assert!(decode_huffman_text_payload(&data,&[&a,&b],&[],184,0,d.stats.work_used-1).is_err());
        assert!(decode_huffman_text_payload(&data,&[&a,&b],&[],183,0,30000).is_err());
        let mut extra=data.clone();extra.push(0);
        assert!(decode_huffman_text_payload(&extra,&[&a,&b],&[],192,0,30000).is_err());
    }
    #[test]
    fn huffman_text_bind_custom_order(){
        let custom=super::super::huffman::parse_custom(&[0x42,0,0,0,0,0,1,1,0x10,0x49,0x23,0x81,0x80]).unwrap();
        let s=TextHuffmanSelections::parse(3,false).unwrap();
        assert!(bind_text_tables(&s,&[&custom]).is_ok());
        assert!(bind_text_tables(&s,&[]).is_err());
        assert!(bind_text_tables(&s,&[&custom,&custom]).is_err());
        let mut bad=s;bad.integers[0]=TextTableChoice::Custom(1);
        assert!(bind_text_tables(&bad,&[&custom]).is_err());
        let s=TextHuffmanSelections::parse(12,false).unwrap();
        assert!(bind_text_tables(&s,&[&custom]).is_err()); // DS needs OOB.
    }
}

#[cfg(test)]
mod huffman_payload_dictionary_integration_tests{
    use super::*;
    use super::super::dictionary::{execute_pdf_dictionaries,ExecutionLimits};
    // Existing Annex H.1 third segment data (collective raw bitmap).
    // Text stream below is synthetic, NOT the Annex H text segment.
    const DICT:&[u8]=&[0,1,0,0,0,2,0,0,0,2,0xe5,0xcd,0xf8,0,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,0x10,0x79,0xf0,0,0x80];
    fn seg(id:u8,kind:u8,page:u8,data:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,id,kind,0,page];
        v.extend((data.len()as u32).to_be_bytes());v.extend(data);v
    }
    fn payload()->Vec<u8>{
        // One instance at origin, top left, two imported symbols.
        let mut v=vec![0,0,0,8,0,0,0,6,0,0,0,0,0,0,0,0,0,0,0x11,0,0,0,0,0,1];
        let mut table=[0u8;18];table[0]=1;v.extend(table);
        // HTDT=1, DT=1, FS=0, ID=0, DS=OOB.
        v.extend([0,4]);v
    }
    #[test]
    fn huffman_segment_custom_table_binding(){
        // Type 53 constant zero, normal prefix zero. Custom FS selector.
        let mut globals=seg(1,0,0,DICT);
        globals.extend(seg(2,53,0,&[0,0,0,0,0,0,0,0,1,0x80]));
        let mut info=Vec::new();for n in [8u32,6,300,300]{info.extend(n.to_be_bytes());}
        info.extend([0,0,0]);let mut page=seg(3,48,1,&info);
        let mut data=payload();data[20]=3;
        // Initial DT=1, DT=1, custom FS=0, ID=0, OOB=01.
        data.truncate(data.len()-2);data.push(4);
        let mut region=vec![0,0,0,4,7,0x40,2,1,1];
        region.extend((data.len() as u32).to_be_bytes());region.extend(&data);
        page.extend(region);
        let d=execute_pdf_text_segment(&globals,&page,4,TextSegmentLimits::default()).unwrap();
        assert_eq!((d.custom_tables,d.symbol_slots),(1,2));
        assert_eq!(d.text.bitmap.packed(),[120,132,128,128,132,120]);
        // Missing selected custom table, and a table supplied to arithmetic.
        let mut bad=page.clone();bad[30+6]=1;
        assert!(execute_pdf_text_segment(&globals,&bad,4,TextSegmentLimits::default()).is_err());
        let mut bad=page;bad[30+13+18]&=!1;
        assert!(execute_pdf_text_segment(&globals,&bad,4,TextSegmentLimits::default()).is_err());
    }
    #[test]
    fn huffman_payload_annex_dictionary_join(){
        let globals=seg(1,0,0,DICT);
        let mut info=Vec::new();for n in [8u32,6,300,300]{info.extend(n.to_be_bytes());}
        info.extend([0,0,0]);
        let page=seg(2,48,1,&info);
        let d=execute_pdf_dictionaries(&globals,&page,ExecutionLimits::default()).unwrap();
        let a=d.symbol(1,0).unwrap();let b=d.symbol(1,1).unwrap();
        let mut data=payload();
        let (_,out)=decode_huffman_text_payload(&data,&[a,b],&[],160,0,40000).unwrap();
        assert_eq!(out.bitmap.packed(),[120,132,128,128,132,120]);
        // Flip the final symbol-ID bit, keep all other fields identical.
        let n=data.len();data[n-1]|=0x10;
        let (_,out)=decode_huffman_text_payload(&data,&[a,b],&[],160,0,40000).unwrap();
        assert_eq!(out.bitmap.packed(),[120,4,124,132,132,124]);
    }
    #[test]
    fn huffman_payload_refined_packed_budget(){
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap();
        let mut data=vec![0,0,0,8,0,0,0,4,0,0,0,0,0,0,0,0,0];
        data.extend(0x8013u16.to_be_bytes());data.extend(0x1540u16.to_be_bytes());
        data.extend(1u32.to_be_bytes());
        let mut table=[0u8;18];table[0]=1;data.extend(table);
        data.extend([0,15,28,117,70,187,255,172,64]);
        let (_,out)=decode_huffman_text_payload_bounded(&data,&[&a],&[],216,6,60000,6).unwrap();
        assert_eq!(out.bitmap.packed(),[160,64,0,0]);
        assert!(decode_huffman_text_payload_bounded(&data,&[&a],&[],216,6,60000,5).is_err());
    }
    #[test]
    fn huffman_payload_refined_both_templates(){
        use super::super::refinement::Template;
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap();
        let b=Bitmap::from_packed(3,2,&[160,64]).unwrap();
        for t in [Template::T0,Template::T1]{
            let flags=if t==Template::T0{0x0013u16}else{0x8013};
            let mut data=vec![0,0,0,8,0,0,0,4,0,0,0,0,0,0,0,0,0];
            data.extend(flags.to_be_bytes());data.extend(0x1540u16.to_be_bytes());
            if t==Template::T0{data.extend([255;4]);}
            data.extend(3u32.to_be_bytes());
            let mut table=[0u8;18];table[0]=1;data.extend(table);
            data.extend([0,15,28,117,70,187,255,172,215,82,144]);
            data.extend([if t==Template::T0{221}else{220},127,255,172,96,2,64]);
            let (_,out)=decode_huffman_text_payload(&data,&[&a,&b],&[],288,14,60000).unwrap();
            assert_eq!(out.bitmap.packed(),[172,67,160,64]);
            assert_eq!(decode_huffman_text_payload(&data,&[&a,&b],&[],288,14,out.stats.work_used).unwrap().1,out);
            assert!(decode_huffman_text_payload(&data,&[&a,&b],&[],288,13,60000).is_err());
        }
    }
}

#[cfg(test)]
mod annex_h_original_huffman_text_tests {
    use super::*;
    use super::super::dictionary::{execute_pdf_dictionaries, ExecutionLimits};
    // ITU-T T.88 (02/2000), Annex H.1, printed pp.121,123–129.
    // https://www.itu.int/rec/T-REC-T.88-200002-S/en
    // Original segment bytes, no standalone file header. Segment 0 is global,
    // segments 1,2,3 are page-associated. Text payload offsets 0x82..0xb3.
    const GLOBAL: &[u8] = &[0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x18,0x00,0x01,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x01,0xe9,0xcb,0xf4,0x00,0x26,0xaf,0x04,0xbf,0xf0,0x78,0x2f,0xe0,0x00,0x40];
    const PAGE_PREFIX: &[u8] = &[0x00,0x00,0x00,0x01,0x30,0x00,0x01,0x00,0x00,0x00,0x13,0x00,0x00,0x00,0x40,0x00,0x00,0x00,0x38,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x02,0x00,0x01,0x01,0x00,0x00,0x00,0x1c,0x00,0x01,0x00,0x00,0x00,0x02,0x00,0x00,0x00,0x02,0xe5,0xcd,0xf8,0x00,0x79,0xe0,0x84,0x10,0x81,0xf0,0x82,0x10,0x86,0x10,0x79,0xf0,0x00,0x80];
    const TEXT: &[u8] = &[0x00,0x00,0x00,0x03,0x07,0x42,0x00,0x02,0x01,0x00,0x00,0x00,0x31,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x01,0x00,0x0c,0x09,0x00,0x10,0x00,0x00,0x00,0x05,0x01,0x10,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0c,0x40,0x07,0x08,0x70,0x41,0xd0];
    const EXPECTED: &[u8] = &[
        120,120,240,240,240, 132,4,136,9,8,
        128,124,136,249,0, 128,132,137,9,0,
        132,132,241,9,8, 120,124,128,248,240,
        0,0,128,0,0, 0,0,128,0,0,
    ]; // Figure H.5, MSB-first, zero row padding; independently transcribed.
    fn page() -> Vec<u8> {
        let mut page = PAGE_PREFIX.to_vec();
        page.extend_from_slice(TEXT);
        page
    }
    #[test]
    fn annex_h_huffman_automatic_segment_dispatch(){
        let page=page();let l=TextSegmentLimits::default();
        let d=execute_pdf_text_segment(GLOBAL,&page,3,l).unwrap();
        assert!(d.huffman);assert_eq!(d.symbol_slots,3);assert_eq!(d.custom_tables,0);
        assert_eq!(d.text.bitmap.packed(),EXPECTED);
        let cap=TextSegmentLimits{work:d.work_used,packed_bytes:d.dictionary_stats.packed_bytes+40,..l};
        assert_eq!(execute_pdf_text_segment(GLOBAL,&page,3,cap).unwrap(),d);
        assert!(execute_pdf_text_segment(GLOBAL,&page,3,TextSegmentLimits{packed_bytes:cap.packed_bytes-1,..cap}).is_err());
        assert!(execute_pdf_text_segment(GLOBAL,&page,3,TextSegmentLimits{symbol_slots:2,..l}).is_err());
        assert!(execute_pdf_text_segment(GLOBAL,&page,3,TextSegmentLimits{huffman_bits:191,..l}).is_err());
        let mut bad=page.clone();bad[PAGE_PREFIX.len()+7]=99;
        assert!(execute_pdf_text_segment(GLOBAL,&bad,3,l).is_err());
        assert!(execute_pdf_text_segment(GLOBAL,&page,2,l).is_err());
    }
    #[test]
    fn annex_h_original_huffman_text_pixels() {
        let page = page();
        let dictionaries = execute_pdf_dictionaries(GLOBAL, &page, ExecutionLimits::default()).unwrap();
        let p = dictionaries.symbol(0,0).unwrap();
        let c = dictionaries.symbol(2,0).unwrap();
        let a = dictionaries.symbol(2,1).unwrap();
        assert_eq!(p.packed(), &[240,136,136,136,240,128,128,128]);
        assert_eq!(c.packed(), &[120,132,128,128,132,120]);
        assert_eq!(a.packed(), &[120,4,124,132,132,124]);
        let (h,d) = decode_huffman_text_payload(&TEXT[13..], &[p,c,a], &[], 192, 0, 40000).unwrap();
        assert_eq!((h.options.width,h.options.height,h.options.strips,h.options.instances,h.options.ds_offset),(37,8,4,5,3));
        assert_eq!(h.options.corner, Corner::BottomLeft);
        assert_eq!(h.selections.integers,[TextTableChoice::Standard(6),TextTableChoice::Standard(8),TextTableChoice::Standard(12)]);
        assert_eq!(h.selections.custom_count,0);
        assert_eq!(d.bitmap.packed(),EXPECTED);
        assert_eq!((d.stats.strips,d.stats.instances,d.stats.composed_pixels),(1,5,184));
    }
    #[test]
    fn annex_h_original_huffman_text_bounds() {
        let page = page();
        let dictionaries = execute_pdf_dictionaries(GLOBAL, &page, ExecutionLimits::default()).unwrap();
        let symbols = [dictionaries.symbol(0,0).unwrap(),dictionaries.symbol(2,0).unwrap(),dictionaries.symbol(2,1).unwrap()];
        let data = &TEXT[13..];
        let (_,d) = decode_huffman_text_payload(data,&symbols,&[],192,0,40000).unwrap();
        assert_eq!(decode_huffman_text_payload(data,&symbols,&[],192,0,d.stats.work_used).unwrap().1,d);
        assert!(decode_huffman_text_payload(data,&symbols,&[],192,0,d.stats.work_used-1).is_err());
        assert!(decode_huffman_text_payload(data,&symbols,&[],191,0,40000).is_err());
        for n in 0..data.len() {
            assert!(decode_huffman_text_payload(&data[..n],&symbols,&[],192,0,40000).is_err(),"truncation at {n}");
        }
        let mut extra = data.to_vec(); extra.push(0);
        assert!(decode_huffman_text_payload(&extra,&symbols,&[],200,0,40000).is_err());
        let mut bad = page.clone(); bad[PAGE_PREFIX.len()+7]=99;
        assert!(execute_pdf_dictionaries(GLOBAL,&bad,ExecutionLimits::default()).is_err());
        let mut bad = page.clone(); bad[PAGE_PREFIX.len()+12]=50;
        assert!(execute_pdf_dictionaries(GLOBAL,&bad,ExecutionLimits::default()).is_err());
    }
}

#[cfg(test)]
mod huffman_unused_namespace_tests {
    use super::*;
    use crate::filters::ccitt::BitReader;
    fn payload(flags:u16,instances:u32,table:&[u8;18],entropy:&[u8])->Vec<u8>{
        let mut v=vec![0,0,0,8,0,0,0,4,0,0,0,0,0,0,0,0,0];
        v.extend(flags.to_be_bytes());v.extend([0,0]);
        v.extend(instances.to_be_bytes());v.extend(table);v.extend(entropy);v
    }
    #[test]
    fn huffman_empty_namespace_default_pixels(){
        // Synthetic zero-instance region: 35 zero run lengths, no symbol
        // lengths, byte padding, initial HTDT=1 (B.11 code 0).
        for (flags,expected) in [(0x11,0),(0x211,255),(0x8013,0)]{
            let data=payload(flags,0,&[0;18],&[0]);
            let (_,d)=decode_huffman_text_payload(&data,&[],&[],152,0,50000).unwrap();
            assert_eq!(d.bitmap.packed(),[expected;4]);
            assert_eq!((d.stats.instances,d.stats.strips,d.stats.composed_pixels),(0,0,0));
            assert_eq!(decode_huffman_text_payload(&data,&[],&[],152,0,d.stats.work_used).unwrap().1,d);
            assert!(decode_huffman_text_payload(&data,&[],&[],152,0,d.stats.work_used-1).is_err());
            assert!(decode_huffman_text_payload(&data,&[],&[],151,0,50000).is_err());
            for n in 0..data.len(){assert!(decode_huffman_text_payload(&data[..n],&[],&[],152,0,50000).is_err());}
            let mut extra=data.clone();extra.push(0);
            assert!(decode_huffman_text_payload(&extra,&[],&[],160,0,50000).is_err());
            let mut bad=data;bad[24]=1;
            assert!(decode_huffman_text_payload(&bad,&[],&[],152,0,50000).is_err());
        }
        let mut bad=[0;18];bad[0]=0x11;bad[1]=0x10; // three one-bit codes
        assert!(decode_huffman_text_payload(&payload(0x11,0,&bad,&[0]),&[],&[],152,0,50000).is_err());
    }
    #[test]
    fn huffman_unused_symbols_never_decode_zero_bit_id(){
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        let mut table=[0;18];table[0]=0x10; // literal length zero has code 0
        let data=payload(0x11,0,&table,&[0]);
        let (_,d)=decode_huffman_text_payload(&data,&[&a],&[],152,0,50000).unwrap();
        assert_eq!(d.bitmap.packed(),[0;4]);
        let mut r=BitReader::new(&table,144).unwrap();
        let ids=HuffmanSymbolIds::from_wire_impl(&mut r,1,true).unwrap();
        assert_eq!(r.position(),141);
        assert!(ids.decode(&mut r).is_err());assert_eq!(r.position(),141);
        let t=HuffmanTextIntegers::standard(6,8,11).unwrap();
        let mut opt=parse_huffman_text_header(&data).unwrap().0.options;opt.instances=1;
        assert!(decode_huffman_direct(&mut r,opt,&[&a],&ids,&t,3,50000).is_err());
        assert_eq!(r.position(),141);
        assert!(decode_huffman_text_payload(&payload(0x11,1,&table,&[0,4]),&[&a],&[],160,0,50000).is_err());
        assert!(HuffmanSymbolIds::new(&[0]).is_err());
        // One actually enabled ID still consumes a real one-bit prefix.
        table[0]=1;
        let (_,d)=decode_huffman_text_payload(&payload(0x11,1,&table,&[0,4]),&[&a],&[],160,0,50000).unwrap();
        assert_eq!(d.bitmap.packed(),[128,0,0,0]);
        let ids=HuffmanSymbolIds::new(&[1]).unwrap();
        assert!(ids.decode(&mut BitReader::new(&[0],0).unwrap()).is_err());
        assert!(ids.decode(&mut BitReader::new(&[128],1).unwrap()).is_err());
    }
}

#[cfg(test)]
mod huffman_selector_conformance_tests {
    use super::*;
    use crate::filters::ccitt::BitReader;
    use super::super::{huffman::{parse_custom,CustomTable},integer::Integer};

    #[test]
    fn huffman_selector_exhaustive_flags(){
        // Independent explicit mapping, T.88 (02/2000) 7.4.3.1.2.
        // 0 = forbidden; -1 = custom; positive = Annex B table.
        let map=[[6,7,0,-1],[8,9,10,-1],[11,12,13,-1],
            [14,15,0,-1],[14,15,0,-1],[14,15,0,-1],[14,15,0,-1]];
        for refine in [false,true]{
            let mut accepted=0;
            for flags in 0..=u16::MAX{
                let fields=if refine{7}else{3};
                let mut valid=flags&0x8000==0&&(refine||flags&0x7fc0==0);
                let mut expected=[TextTableChoice::Standard(1);8];
                let mut next=0;
                for i in 0..fields{
                    let n=map[i][usize::from((flags>>(2*i))&3)];
                    match n{
                        0=>valid=false,
                        -1=>{expected[i]=TextTableChoice::Custom(next);next+=1;},
                        n=>expected[i]=TextTableChoice::Standard(n as u8),
                    }
                }
                if refine&&flags&0x4000!=0{expected[7]=TextTableChoice::Custom(next);next+=1;}
                let actual=TextHuffmanSelections::parse(flags,refine);
                if !valid{assert!(actual.is_err(),"flags {flags:04x}, refine {refine}");continue;}
                let actual=actual.unwrap();accepted+=1;
                assert_eq!(actual.integers,[expected[0],expected[1],expected[2]]);
                assert_eq!(actual.refinement,if refine{Some([expected[3],expected[4],expected[5],expected[6],expected[7]])}else{None});
                assert_eq!(actual.custom_count,next);
            }
            assert_eq!(accepted,if refine{7776}else{48});
        }
    }

    // Independent wire syntax: one normal value, prefix 0, no extensions.
    // If DS, add OOB at prefix 1. Distinct normal values detect table swaps.
    fn constant(value:i32,oob:bool)->CustomTable{
        let mut bytes=vec![u8::from(oob)];
        bytes.extend(value.to_be_bytes());
        bytes.extend((value+1).to_be_bytes());
        bytes.push(if oob{0x88}else{0x80});
        parse_custom(&bytes).unwrap()
    }
    #[test]
    fn huffman_all_custom_fields_order_and_oob(){
        let owned=[constant(2,false),constant(3,true),constant(1,false),
            constant(-2,false),constant(1,false),constant(-1,false),
            constant(0,false),constant(3,false)];
        let mut custom:Vec<_>=owned.iter().collect();
        let s=TextHuffmanSelections::parse(0x7fff,true).unwrap();
        let (ints,refinement)=bind_text_tables(&s,&custom).unwrap();
        let mut r=BitReader::new(&[0x10],4).unwrap(); // FS 0, DS 0, DT 0, OOB 1
        assert_eq!(ints.first(&mut r).unwrap(),2);
        assert_eq!(ints.delta(&mut r).unwrap(),Integer::Value(3));
        assert_eq!(ints.strip(&mut r).unwrap(),1);
        assert_eq!(ints.delta(&mut r).unwrap(),Integer::Oob);
        assert_eq!(r.remaining(),0);
        let mut r=BitReader::new(&[0x80],6).unwrap(); // RI then five zero prefixes
        assert_eq!(refinement.unwrap().read(&mut r).unwrap(),Some(
            HuffmanTextRefinementFields{dw:-2,dh:1,dx:-1,dy:0,bytes:3}));
        assert_eq!(r.remaining(),0);
        assert!(bind_text_tables(&s,&custom[..7]).is_err());
        custom.push(&owned[0]);assert!(bind_text_tables(&s,&custom).is_err());custom.pop();
        for i in [0,2,3,4,5,6,7]{
            custom.swap(1,i);assert!(bind_text_tables(&s,&custom).is_err());custom.swap(1,i);
        }
        let mut forged=s;forged.integers[0]=TextTableChoice::Custom(1);
        assert!(bind_text_tables(&forged,&custom).is_err());
    }
    #[test]
    fn huffman_standard_payload_selector_matrix(){
        let symbol=Bitmap::from_packed(1,1,&[128]).unwrap();
        // Explicit B.6/B.7 code for FS=1, B.8/B.9/B.10 OOB codes.
        // DT=1 in B.11/B.12/B.13 is 0; single symbol ID is 0.
        for (fs,fs_bits) in [(0u16,"000000001"),(1,"101100001")]{
            for (ds,oob) in [(0u16,"01"),(1,"00"),(2,"10")]{
                for dt in 0u16..3{
                    let flags=fs|(ds<<2)|(dt<<4);
                    let mut data=vec![0,0,0,8,0,0,0,4,0,0,0,0,0,0,0,0,0,0,0x11];
                    data.extend(flags.to_be_bytes());data.extend(1u32.to_be_bytes());
                    let mut table=[0u8;18];table[0]=1;data.extend(table);
                    let stream=format!("00{fs_bits}0{oob}");
                    let mut entropy=vec![0u8;stream.len().div_ceil(8)];
                    for (i,b) in stream.bytes().enumerate(){if b==b'1'{entropy[i/8]|=128>>(i%8);}}
                    data.extend(entropy);
                    let (_,d)=decode_huffman_text_payload(&data,&[&symbol],&[],160,0,40000).unwrap();
                    assert_eq!(d.bitmap.packed(),[64,0,0,0],"flags {flags:04x}");
                    assert_eq!(decode_huffman_text_payload(&data,&[&symbol],&[],160,0,d.stats.work_used).unwrap().1,d);
                    assert!(decode_huffman_text_payload(&data,&[&symbol],&[],160,0,d.stats.work_used-1).is_err());
                    assert!(decode_huffman_text_payload(&data,&vec![&symbol;super::super::huffman::MAX_ROWS+1],&[],160,0,40000).is_err());
                    let extra=constant(1,false);
                    assert!(decode_huffman_text_payload(&data,&[&symbol],&[&extra],160,0,40000).is_err());
                }
            }
        }
    }
}

/// Scoped Table 17 Huffman aggregate bitmap. Unlike a standalone text
/// payload, this consumes only the aggregate syntax from the shared cursor,
/// leaves following dictionary fields untouched, and borrows the GR bank.
/// Caller reserves the output bitmap separately from temporary refinement.
#[derive(Debug,PartialEq,Eq)]
pub struct HuffmanAggregateBitmap{
    pub decoded:DecodedText,
    pub pixel_decisions:usize,
    pub entropy_bytes:usize,
}
pub fn decode_huffman_aggregate_bitmap(
    reader:&mut crate::filters::ccitt::BitReader<'_>,width:u32,height:u32,
    instances:u32,symbols:&[&Bitmap],total_symbols:u32,
    neighborhood:&super::refinement::Neighborhood,gr:&mut super::refinement::ContextBank,
    decision_limit:usize,work_limit:u64,temporary_packed_limit:usize,
)->Result<HuffmanAggregateBitmap>{
    use super::{integer::Integer,refinement,qm};
    if instances<2||instances>MAX_INSTANCES||symbols.is_empty()||
        symbols.len()>=total_symbols as usize||total_symbols>super::dictionary::MAX_SYMBOLS{
        return Err(Error::Invalid("JBIG2 Huffman aggregate namespace or count"));
    }
    if gr.template()!=neighborhood.template(){return Err(Error::Invalid("JBIG2 aggregate GR template"));}
    if decision_limit>MAX_TEXT_DECISIONS||work_limit>bitmap::MAX_COMPOSE_WORK||
        temporary_packed_limit>bitmap::MAX_PACKED_BYTES{
        return Err(Error::Limit("JBIG2 Huffman aggregate limits"));
    }
    let bits=super::dictionary::huffman_refinement_id_bits(0,total_symbols)?;
    // Worst-case syntax per instance: strip DT/FS, DS, ID, RI,
    // four refinement deltas, RSIZE. Each integer uses <=64 bits.
    // This reserves work without charging unrelated following fields.
    let mut extra=64u64+u64::from(instances)*(8*64+u64::from(bits)+1);
    let left=work_limit.checked_sub(extra).ok_or(Error::Limit("JBIG2 aggregate syntax work"))?;
    let integers=HuffmanTextIntegers::standard(6,8,11)?;
    let tables=HuffmanTextRefinementTables::standard([15;4])?;
    let options=Options{width,height,default_black:false,op:ComposeOp::Or,
        strips:1,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances};
    let initial=integers.strip(reader)?;
    let mut region=Region::new(options,initial,left)?;
    let mut remaining=decision_limit;
    let mut entropy_bytes=0usize;
    while region.remaining_instances()!=0{
        let dt=integers.strip(reader)?;
        let fs=integers.first(reader)?;
        region.begin_strip(dt,fs)?;
        let mut offset=None;
        loop{
            let mut id=0usize;
            for _ in 0..bits{id=(id<<1)|usize::from(reader.bit()?);}
            let src=symbols.get(id).ok_or(Error::Invalid("JBIG2 Huffman aggregate forward symbol"))?;
            if let Some(fields)=tables.read(reader)?{
                let opt=refinement_geometry(src,fields.dw,fields.dh,fields.dx,fields.dy)?;
                let packed=(opt.width as usize).div_ceil(8).checked_mul(opt.height as usize)
                    .ok_or(Error::Limit("JBIG2 aggregate temporary packed bytes"))?;
                if packed>temporary_packed_limit{return Err(Error::Limit("JBIG2 aggregate temporary packed bytes"));}
                let area=usize::try_from(u64::from(opt.width)*u64::from(opt.height))
                    .map_err(|_|Error::Limit("JBIG2 aggregate decisions"))?;
                remaining=remaining.checked_sub(area).ok_or(Error::Limit("JBIG2 aggregate decisions"))?;
                let work=refinement::work_bound(neighborhood.template(),opt)?;
                let block_work=(fields.bytes as u64).checked_mul(8).and_then(|v|v.checked_add(7))
                    .and_then(|v|v.checked_add(work)).ok_or(Error::Limit("JBIG2 aggregate block work"))?;
                region.left=region.left.checked_sub(block_work).ok_or(Error::Limit("JBIG2 aggregate block work"))?;
                extra=extra.checked_add(block_work).ok_or(Error::Limit("JBIG2 aggregate work"))?;
                let block=reader.take_aligned_bytes(fields.bytes)?;
                let mut decoder=qm::Decoder::new(block,area)?;
                let decoded=refinement::decode_bitmap(src,neighborhood,opt,gr,&mut decoder,work)?;
                region.place(offset,0,&decoded.bitmap)?;
                entropy_bytes=entropy_bytes.checked_add(fields.bytes).ok_or(Error::Limit("JBIG2 aggregate entropy bytes"))?;
            }else{
                region.place(offset,0,src)?;
            }
            match integers.delta(reader)?{
                Integer::Oob=>{region.end_strip()?;break;},
                Integer::Value(n)=>{
                    if region.remaining_instances()==0{return Err(Error::Invalid("JBIG2 aggregate excess instances"));}
                    offset=Some(n);
                },
            }
        }
    }
    let mut decoded=region.finish()?;
    decoded.stats.work_used=decoded.stats.work_used.checked_add(extra).ok_or(Error::Limit("JBIG2 aggregate work"))?;
    Ok(HuffmanAggregateBitmap{decoded,pixel_decisions:decision_limit-remaining,entropy_bytes})
}

#[cfg(test)]
mod huffman_aggregate_bitmap_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    use super::super::refinement::{ContextBank,Neighborhood,Template};
    fn pack(bits:&str)->Vec<u8>{
        let mut v=vec![0;bits.len().div_ceil(8)];
        for (i,b) in bits.bytes().enumerate(){if b==b'1'{v[i/8]|=128>>(i%8);}}
        v
    }
    #[test]
    fn huffman_aggregate_fixed_id_and_shared_cursor(){
        // Starts three bits into a dictionary. Total namespace 4, only
        // ID 0 available. Initial DT=1, DT=1, FS=0, ID=00 RI=0,
        // DS=2, ID=00 RI=0, OOB; trailing 101 belongs to caller.
        let body="00000000000";
        let stream=format!("101{body}0001101000001101");
        let data=pack(&stream);
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        let n=Neighborhood::nominal(Template::T1);
        let run=|limit:usize,work:u64,total:u32|->Result<(HuffmanAggregateBitmap,usize)>{
            let mut r=BitReader::new(&data,limit)?;
            for _ in 0..3{r.bit()?;}
            let mut g=ContextBank::new(Template::T1)?;
            let d=decode_huffman_aggregate_bitmap(&mut r,8,4,2,&[&a],total,&n,&mut g,0,work,0)?;
            let end=r.position();
            assert_eq!([r.bit()?,r.bit()?,r.bit()?],[1,0,1]);
            Ok((d,end))
        };
        let (d,end)=run(stream.len(),10000,4).unwrap();
        assert_eq!(end,stream.len()-3);
        assert_eq!(d.decoded.bitmap.packed(),[160,0,0,0]);
        assert_eq!(d.pixel_decisions,0);assert_eq!(d.entropy_bytes,0);
        assert_eq!(run(stream.len(),d.decoded.stats.work_used,4).unwrap().0,d);
        assert!(run(stream.len(),d.decoded.stats.work_used-1,4).is_err());
        for limit in 3..end{assert!(run(limit,10000,4).is_err());}
        // First ID is 01, forward relative to the one available symbol.
        let bad=pack(&format!("101{body}0101101000001101"));
        let mut r=BitReader::new(&bad,stream.len()).unwrap();
        for _ in 0..3{r.bit().unwrap();}
        let mut g=ContextBank::new(Template::T1).unwrap();
        assert!(decode_huffman_aggregate_bitmap(&mut r,8,4,2,&[&a],4,&n,&mut g,0,10000,0).is_err());
    }
    #[test]
    fn huffman_aggregate_refinement_and_following_fields(){
        // Previously independently encoded QM T1 block (3x2 target).
        // OOB then another strip with one direct symbol, then sentinel.
        let tail="0100000000000001101";
        let mut data=vec![0,15,28,117,70,187,255,172];
        data.extend(pack(tail));
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap();
        let n=Neighborhood::nominal(Template::T1);
        let run=|decision_limit:usize,packed:usize,work:u64|->Result<HuffmanAggregateBitmap>{
            let mut r=BitReader::new(&data,64+tail.len())?;
            let mut g=ContextBank::new(Template::T1)?;
            let d=decode_huffman_aggregate_bitmap(&mut r,8,8,2,&[&a],2,&n,&mut g,decision_limit,work,packed)?;
            assert_eq!(r.position(),64+tail.len()-3);
            assert_eq!([r.bit()?,r.bit()?,r.bit()?],[1,0,1]);
            Ok(d)
        };
        let d=run(6,2,30000).unwrap();
        assert_eq!(d.decoded.bitmap.packed(),[160,64,124,124,124,0,0,0]);
        assert_eq!((d.pixel_decisions,d.entropy_bytes),(6,3));
        assert_eq!(run(6,2,d.decoded.stats.work_used).unwrap(),d);
        assert!(run(5,2,30000).is_err());
        assert!(run(6,1,30000).is_err());
        assert!(run(6,2,d.decoded.stats.work_used-1).is_err());
    }
}

#[cfg(test)]
mod huffman_aggregate_shared_gr_tests{
    use super::*;
    use crate::filters::ccitt::BitReader;
    use super::super::refinement::{ContextBank,Neighborhood,Template};
    #[test]
    fn huffman_aggregate_two_qm_blocks_shared_gr(){
        // 9d3 independent QM vectors, IDs widened to two bits, two
        // imported symbols and one not-yet-decoded symbol. Final OOB
        // followed by a 3-bit caller sentinel, not text payload padding.
        let a=Bitmap::from_packed(7,6,&[0,124,124,124,0,0]).unwrap();
        let b=Bitmap::from_packed(3,2,&[160,64]).unwrap();
        for template in [Template::T0,Template::T1]{
            let n=Neighborhood::nominal(template);
            let mut data=vec![0,7,142,58,163,187,255,172,211,169,72];
            data.extend([if template==Template::T0{221}else{220},127,255,172]);
            // 5-byte header + 3-byte QM + 3-byte header + 4-byte QM.
            // The second block ends at byte 15, not byte 14.
            assert_eq!(data.len(),15);
            let end=data.len()*8;
            data.push(0x68); // OOB 01, sentinel 101.
            // Previous fixture's erroneous bit limit cut the second QM block.
            let mut truncated=BitReader::new(&data,end-3).unwrap();
            let mut gr=ContextBank::new(template).unwrap();
            assert!(matches!(decode_huffman_aggregate_bitmap(&mut truncated,8,4,2,&[&a,&b],3,
                &n,&mut gr,14,40000,2),Err(Error::Invalid("CCITT truncated aligned bytes"))));
            let run=|decisions,work,packed|->Result<HuffmanAggregateBitmap>{
                let mut r=BitReader::new(&data,end+5)?;
                let mut gr=ContextBank::new(template)?;
                let d=decode_huffman_aggregate_bitmap(&mut r,8,4,2,&[&a,&b],3,&n,&mut gr,decisions,work,packed)?;
                assert_eq!(r.position(),end+2);
                assert_eq!([r.bit()?,r.bit()?,r.bit()?],[1,0,1]);
                Ok(d)
            };
            let d=run(14,40000,2).unwrap();
            assert_eq!(d.decoded.bitmap.packed(),[172,67,0,0]);
            assert_eq!((d.pixel_decisions,d.entropy_bytes),(14,7));
            assert_eq!(run(14,d.decoded.stats.work_used,2).unwrap(),d);
            assert!(run(13,40000,2).is_err());
            assert!(run(14,40000,1).is_err());
            assert!(run(14,d.decoded.stats.work_used-1,2).is_err());
        }
    }
}

/// Known-length arithmetic text segment payload (T.88 7.4.3).
/// No Huffman selector word or symbol-ID code-length table is present.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ArithmeticTextHeader{
    pub region:bitmap::RegionInfo,
    pub options:Options,
    pub neighborhood:Option<super::refinement::Neighborhood>,
}
pub fn parse_arithmetic_text_header(data:&[u8])->Result<(ArithmeticTextHeader,&[u8])>{
    use super::{generic::Offset,refinement::{Neighborhood,Template}};
    if data.len()>crate::MAX_BYTES{return Err(Error::Limit("JBIG2 arithmetic text segment bytes"));}
    let (region,rest)=bitmap::parse_region(data)?;
    let h=rest.get(..2).ok_or(Error::Invalid("JBIG2 arithmetic text flags truncated"))?;
    let flags=u16::from_be_bytes([h[0],h[1]]);
    if flags&1!=0{return Err(Error::Unsupported("JBIG2 Huffman text header"));}
    let refine=flags&2!=0;
    let template=Template::from_code((flags>>15)as u8)?;
    if !refine&&template==Template::T1{return Err(Error::Invalid("JBIG2 text unused refinement template"));}
    let rest=&rest[2..];
    let n=if refine{template.adaptive_count()*2}else{0};
    let at=rest.get(..n).ok_or(Error::Invalid("JBIG2 arithmetic text adaptive flags truncated"))?;
    let neighborhood=if refine{
        let mut offsets=[Offset{x:0,y:0};2];
        for (i,p) in at.chunks_exact(2).enumerate(){offsets[i]=Offset{x:p[0]as i8,y:p[1]as i8};}
        Some(Neighborhood::new(template,&offsets[..n/2])?)
    }else{None};
    let rest=&rest[n..];
    let count=rest.get(..4).ok_or(Error::Invalid("JBIG2 arithmetic text instance count truncated"))?;
    let instances=u32::from_be_bytes([count[0],count[1],count[2],count[3]]);
    if instances>MAX_INSTANCES{return Err(Error::Limit("JBIG2 text instances"));}
    let ds=((flags>>10)&31) as i8;
    let options=Options{width:region.width,height:region.height,default_black:flags&0x200!=0,
        op:ComposeOp::from_code(((flags>>7)&3)as u8)?,strips:1u8<<((flags>>2)&3),
        corner:Corner::from_code(((flags>>4)&3)as u8)?,transposed:flags&64!=0,
        ds_offset:if ds>=16{ds-32}else{ds},instances};
    Ok((ArithmeticTextHeader{region,options,neighborhood},&rest[4..]))
}
/// Complete known-length arithmetic text payload. Caller resolves the
/// imported namespace in segment-reference order. Work charges input
/// scanning, all allocated syntax/IAID/GR contexts and bitmap operations;
/// entropy decisions have an independent hard cap. Packed budget covers
/// output canvas plus the one live temporary refined bitmap, not imports.
/// Explicit FF AC termination is required, not an entropy-integrity proof.
pub fn decode_arithmetic_text_payload(data:&[u8],symbols:&[&Bitmap],
    decision_limit:usize,work_limit:u64,packed_limit:usize)->Result<(ArithmeticTextHeader,DecodedText)>{
    decode_arithmetic_text_payload_counted(data,symbols,decision_limit,work_limit,packed_limit,&mut 0)
}
fn decode_arithmetic_text_payload_counted(data:&[u8],symbols:&[&Bitmap],
    decision_limit:usize,work_limit:u64,packed_limit:usize,decisions_used:&mut usize)->Result<(ArithmeticTextHeader,DecodedText)>{
    use super::{integer::IaidDecoder,refinement::ContextBank,qm::Decoder};
    if symbols.len()>MAX_INSTANCES as usize||decision_limit>MAX_TEXT_DECISIONS||
        work_limit>bitmap::MAX_COMPOSE_WORK||packed_limit>bitmap::MAX_PACKED_BYTES{
        return Err(Error::Limit("JBIG2 arithmetic text payload limits"));
    }
    let (header,body)=parse_arithmetic_text_header(data)?;
    if header.options.instances!=0&&symbols.is_empty(){
        return Err(Error::Invalid("JBIG2 text empty symbol namespace"));
    }
    let count=symbols.len() as u32;
    let width=if count<=1{0}else{(32-(count-1).leading_zeros()) as u8};
    let canvas=(header.region.width as usize).div_ceil(8).checked_mul(header.region.height as usize)
        .ok_or(Error::Limit("JBIG2 arithmetic text packed canvas"))?;
    let temporary=packed_limit.checked_sub(canvas).ok_or(Error::Limit("JBIG2 arithmetic text packed canvas"))?;
    let context_work=(1u64<<width)+9*512+
        header.neighborhood.as_ref().map_or(0,|n|n.template().context_count() as u64);
    let overhead=context_work.checked_add(data.len() as u64)
        .ok_or(Error::Limit("JBIG2 arithmetic text payload work"))?;
    let available=work_limit.checked_sub(overhead).ok_or(Error::Limit("JBIG2 arithmetic text payload work"))?;
    let mut decoder=Decoder::new(body,decision_limit)?;
    let mut ids=IaidDecoder::new(width)?;
    let mut contexts=ArithmeticTextContexts::new();
    let mut gr=match &header.neighborhood{Some(n)=>Some(ContextBank::new(n.template())?),None=>None};
    let mut decoded=decode_arithmetic_shared_bounded(header.options,symbols,&mut decoder,&mut ids,
        &mut contexts,header.neighborhood.as_ref(),gr.as_mut(),available,temporary)?;
    decoded.stats.work_used=decoded.stats.work_used.checked_add(overhead)
        .ok_or(Error::Limit("JBIG2 arithmetic text payload work"))?;
    *decisions_used=decoder.decisions_used();
    Ok((header,decoded))
}

#[cfg(test)]
mod arithmetic_text_payload_tests{
    use super::*;
    use super::super::refinement::Template;
    // Same independently encoded QM reference vectors as the earlier
    // entropy-only tests, now with explicit complete binary region headers.
    const DIRECT:&[u8]=&[147,150,68,103,175,63,255,172];
    const REFINED:&[u8]=&[169,113,170,191,21,40,159,255,172];
    fn payload(width:u32,height:u32,flags:u16,at:&[u8],instances:u32,entropy:&[u8])->Vec<u8>{
        let mut v=Vec::new();
        for n in [width,height,3,4]{v.extend(n.to_be_bytes());}
        v.push(2); // external XOR differs from the internal composition operator
        v.extend(flags.to_be_bytes());v.extend(at);v.extend(instances.to_be_bytes());v.extend(entropy);v
    }
    #[test]
    fn arithmetic_payload_direct_header_and_exact_budgets(){
        let data=payload(8,4,0x14,&[],3,DIRECT);
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        let refs=[&a,&a,&a];
        let (h,d)=decode_arithmetic_text_payload(&data,&refs,50,20000,4).unwrap();
        assert_eq!((h.region.x,h.region.y,h.region.op),(3,4,ComposeOp::Xor));
        assert_eq!(h.options,Options{width:8,height:4,default_black:false,op:ComposeOp::Or,
            strips:2,corner:Corner::TopLeft,transposed:false,ds_offset:0,instances:3});
        assert!(h.neighborhood.is_none());assert_eq!(d.bitmap.packed(),[16,64,128,0]);
        assert_eq!(d.stats.work_used,decode_arithmetic(DIRECT,h.options,&refs,50,100).unwrap().stats.work_used+
            4+9*512+data.len() as u64);
        assert_eq!(decode_arithmetic_text_payload(&data,&refs,50,d.stats.work_used,4).unwrap().1,d);
        assert!(decode_arithmetic_text_payload(&data,&refs,50,d.stats.work_used-1,4).is_err());
        assert!(decode_arithmetic_text_payload(&data,&refs,49,20000,4).is_err());
        assert!(decode_arithmetic_text_payload(&data,&refs,50,20000,3).is_err());
        for n in 0..data.len(){assert!(decode_arithmetic_text_payload(&data[..n],&refs,50,20000,4).is_err());}
        let mut bad=data.clone();bad.push(0);
        assert!(decode_arithmetic_text_payload(&bad,&refs,50,20000,4).is_err());
        assert!(decode_arithmetic_text_payload(&data,&[],50,20000,4).is_err());
    }
    #[test]
    fn arithmetic_payload_refined_templates_and_temporary_budget(){
        let a=Bitmap::from_packed(1,1,&[128]).unwrap();
        for (flags,at,template) in [(0x12u16,&[255;4][..],Template::T0),(0x8012,&[][..],Template::T1)]{
            let data=payload(8,1,flags,at,3,REFINED);
            let (h,d)=decode_arithmetic_text_payload(&data,&[&a],70,30000,2).unwrap();
            let n=h.neighborhood.as_ref().unwrap();assert_eq!(n.template(),template);
            assert_eq!(d.bitmap.packed(),[192]);
            let old=decode_arithmetic_refined(REFINED,h.options,&[&a],n,70,30000).unwrap();
            assert_eq!(d.stats.work_used,old.stats.work_used+data.len() as u64);
            assert_eq!(decode_arithmetic_text_payload(&data,&[&a],70,d.stats.work_used,2).unwrap().1,d);
            assert!(decode_arithmetic_text_payload(&data,&[&a],70,d.stats.work_used,1).is_err());
            assert!(decode_arithmetic_text_payload(&data,&[&a],69,30000,2).is_err());
            assert!(decode_arithmetic_text_payload(&data,&[&a],70,d.stats.work_used-1,2).is_err());
            for end in 0..data.len(){assert!(decode_arithmetic_text_payload(&data[..end],&[&a],70,30000,2).is_err());}
        }
    }
    #[test]
    fn arithmetic_payload_header_fields_and_rejections(){
        let data=payload(8,4,0x7ffe,&[255;4],3,&[0xa5]);
        let (h,body)=parse_arithmetic_text_header(&data).unwrap();
        assert_eq!(body,[0xa5]);assert_eq!(h.options.ds_offset,-1);
        assert_eq!(h.options.strips,8);assert_eq!(h.options.corner,Corner::TopRight);
        assert_eq!(h.options.op,ComposeOp::Xnor);assert!(h.options.transposed&&h.options.default_black);
        assert_eq!(h.neighborhood.unwrap().template(),Template::T0);
        for n in 0..27{assert!(parse_arithmetic_text_header(&data[..n]).is_err());}
        for (flags,at,count) in [(1,&[][..],1),(0x8000,&[][..],1),(2,&[0;4][..],1),(0,&[][..],MAX_INSTANCES+1)]{
            assert!(parse_arithmetic_text_header(&payload(8,4,flags,at,count,&[])).is_err());
        }
        let mut bad=data;bad[16]=8;
        assert!(parse_arithmetic_text_header(&bad).is_err());
    }
}

#[cfg(test)]
mod annex_h_original_arithmetic_text_tests{
    use super::*;
    use super::super::dictionary::{execute_pdf_dictionaries,ExecutionLimits};
    // ITU-T T.88 (02/2000) Annex H.1, printed pp.133–135,137–142.
    // Original segments, extracted into independent embedded streams.
    // Non-text page-2 regions omitted; no re-encoding of the original bytes.
    // https://www.itu.int/rec/T-REC-T.88-200002-S/en
    const GLOBAL2:&[u8]=&[0x00,0x00,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x18,0x00,0x01,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x01,0xe9,0xcb,0xf4,0x00,0x26,0xaf,0x04,0xbf,0xf0,0x78,0x2f,0xe0,0x00,0x40];
    const PAGE2:&[u8]=&[0,0,0,8,48,0,2,0,0,0,19,0,0,0,64,0,0,0,56,0,0,0,0,0,0,0,0,1,0,0,0,0,0,9,0,1,2,0,0,0,27,8,0,2,255,0,0,0,2,0,0,0,2,79,231,140,32,14,29,199,207,1,17,196,178,111,255,172];
    const TEXT2:&[u8]=&[0,0,0,10,7,64,0,9,2,0,0,0,31,0,0,0,37,0,0,0,8,0,0,0,4,0,0,0,1,0,12,8,0,0,0,5,141,110,90,18,64,133,255,172];
    const GLOBAL3:&[u8]=&[0,0,0,16,0,1,0,0,0,0,22,8,0,2,255,0,0,0,1,0,0,0,1,79,231,141,104,27,20,47,63,255,172];
    const PAGE3:&[u8]=&[0,0,0,15,48,0,3,0,0,0,19,0,0,0,37,0,0,0,8,0,0,0,0,0,0,0,0,1,0,0,0,0,0,17,0,33,16,3,0,0,0,32,8,2,2,255,255,255,255,255,0,0,0,3,0,0,0,2,79,233,215,213,144,195,181,38,167,251,109,20,152,63,255,172];
    const TEXT3:&[u8]=&[0,0,0,18,7,32,17,3,0,0,0,37,0,0,0,37,0,0,0,8,0,0,0,0,0,0,0,0,0,140,18,0,0,0,4,169,92,139,244,195,125,150,106,40,229,118,143,255,172];
    const EXPECTED:&[u8]=&[
        120,120,240,240,240, 132,4,136,9,8,
        128,124,136,249,0, 128,132,137,9,0,
        132,132,241,9,8, 120,124,128,248,240,
        0,0,128,0,0, 0,0,128,0,0,
    ];
    fn join(prefix:&[u8],text:&[u8])->Vec<u8>{
        let mut page=prefix.to_vec();page.extend_from_slice(text);page
    }
    #[test]
    fn text_regions_mixed_modes_order_and_cumulative_caps(){
        let mut page=join(PAGE2,TEXT2);
        // Rebind the original Huffman text payload to equivalent arithmetic
        // dictionary 9. Synthetic graph wrapper; original entropy unchanged.
        let mut h=vec![0x00,0x00,0x00,0x03,0x07,0x42,0x00,0x02,0x01,0x00,0x00,0x00,0x31,0x00,0x00,0x00,0x25,0x00,0x00,0x00,0x08,0x00,0x00,0x00,0x04,0x00,0x00,0x00,0x01,0x00,0x0c,0x09,0x00,0x10,0x00,0x00,0x00,0x05,0x01,0x10,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0c,0x40,0x07,0x08,0x70,0x41,0xd0];
        h[3]=11;h[4]=4;h[7]=9;h[8]=2;
        page.extend_from_slice(&h);
        h[3]=12;h[4]=6;page.extend_from_slice(&h);
        let l=TextRegionsLimits::default();
        let d=execute_pdf_text_regions(GLOBAL2,&page,l).unwrap();
        assert_eq!(d.regions.iter().map(|r|(r.number,r.kind,r.huffman)).collect::<Vec<_>>(),
            [(10,7,false),(11,4,true),(12,6,true)]);
        for r in &d.regions{
            assert_eq!(r.text.bitmap.packed(),EXPECTED);
            let single=execute_pdf_text_segment(GLOBAL2,&page,r.number,l.text).unwrap();
            assert_eq!(r.text,single.text);assert_eq!(r.decisions_used,single.decisions_used);
        }
        assert_eq!((d.pixels,d.instances,d.symbol_slots,d.huffman_bits_used),(37*8*3,15,9,384));
        assert!(d.decisions_used>0);assert_eq!(d.decisions_used,d.regions[0].decisions_used);
        assert_eq!(d.packed_bytes,d.dictionary_stats.packed_bytes+120);
        let exact=TextRegionsLimits{regions:3,pixels:d.pixels,instances:d.instances,
            text:TextSegmentLimits{work:d.work_used,packed_bytes:d.packed_bytes,
                symbol_slots:d.symbol_slots,decisions:d.decisions_used,huffman_bits:d.huffman_bits_used,..l.text}};
        assert_eq!(execute_pdf_text_regions(GLOBAL2,&page,exact).unwrap(),d);
        for cap in [TextRegionsLimits{regions:2,..exact},TextRegionsLimits{pixels:exact.pixels-1,..exact},
            TextRegionsLimits{instances:14,..exact}]{
            assert!(execute_pdf_text_regions(GLOBAL2,&page,cap).is_err());
        }
        for t in [TextSegmentLimits{work:d.work_used-1,..exact.text},TextSegmentLimits{packed_bytes:d.packed_bytes-1,..exact.text},
            TextSegmentLimits{symbol_slots:8,..exact.text},TextSegmentLimits{decisions:d.decisions_used-1,..exact.text},
            TextSegmentLimits{huffman_bits:383,..exact.text}]{
            assert!(execute_pdf_text_regions(GLOBAL2,&page,TextRegionsLimits{text:t,..exact}).is_err());
        }
        // A malformed last payload fails the whole operation.
        let end=page.len();let last_start=end-h.len();page[last_start+13+16]=8;
        assert!(execute_pdf_text_regions(GLOBAL2,&page,l).is_err());
    }
    #[test]
    fn text_regions_refined_decisions_are_cumulative(){
        let mut page=join(PAGE3,TEXT3);
        let mut second=TEXT3.to_vec();second[3]=19;page.extend(second);
        let l=TextRegionsLimits::default();
        let d=execute_pdf_text_regions(GLOBAL3,&page,l).unwrap();
        assert_eq!(d.regions.len(),2);assert_eq!(d.regions[0].text,d.regions[1].text);
        assert_eq!(d.decisions_used,2*d.regions[0].decisions_used);
        assert!(d.decisions_used>0);
        assert_eq!(d.huffman_bits_used,0);
        let cap=TextRegionsLimits{text:TextSegmentLimits{decisions:d.decisions_used,..l.text},..l};
        assert_eq!(execute_pdf_text_regions(GLOBAL3,&page,cap).unwrap(),d);
        assert!(execute_pdf_text_regions(GLOBAL3,&page,TextRegionsLimits{text:TextSegmentLimits{
            decisions:d.decisions_used-1,..l.text},..l}).is_err());
        let empty=execute_pdf_text_regions(GLOBAL3,PAGE3,l).unwrap();
        assert!(empty.regions.is_empty());assert_eq!(empty.packed_bytes,empty.dictionary_stats.packed_bytes);
    }
    #[test]
    fn annex_h_arithmetic_automatic_segment_dispatch(){
        for (global,prefix,segment,number) in [(GLOBAL2,PAGE2,TEXT2,10),(GLOBAL3,PAGE3,TEXT3,18)]{
            let page=join(prefix,segment);
            let l=TextSegmentLimits::default();
            let d=execute_pdf_text_segment(global,&page,number,l).unwrap();
            assert!(!d.huffman);assert_eq!(d.kind,7);assert_eq!(d.symbol_slots,3);
            assert_eq!(d.text.bitmap.packed(),EXPECTED);
            assert_eq!(execute_pdf_text_segment(global,&page,number,TextSegmentLimits{work:d.work_used,..l}).unwrap(),d);
            for cap in [TextSegmentLimits{work:d.work_used-1,..l},TextSegmentLimits{symbol_slots:2,..l},
                TextSegmentLimits{packed_bytes:d.dictionary_stats.packed_bytes+39,..l},TextSegmentLimits{decisions:0,..l}]{
                assert!(execute_pdf_text_segment(global,&page,number,cap).is_err());
            }
            assert!(execute_pdf_text_segment(global,&page,number+1,l).is_err());
        }
    }
    #[test]
    fn annex_h_arithmetic_original_direct_region(){
        let page=join(PAGE2,TEXT2);
        let d=execute_pdf_dictionaries(GLOBAL2,&page,ExecutionLimits::default()).unwrap();
        let refs=[d.symbol(0,0).unwrap(),d.symbol(9,0).unwrap(),d.symbol(9,1).unwrap()];
        assert_eq!(refs[0].packed(),[240,136,136,136,240,128,128,128]);
        assert_eq!(refs[1].packed(),[120,132,128,128,132,120]);
        assert_eq!(refs[2].packed(),[120,4,124,132,132,124]);
        let (h,out)=decode_arithmetic_text_payload(&TEXT2[13..],&refs,2000,40000,40).unwrap();
        assert_eq!((h.region.x,h.region.y,h.options.width,h.options.height),(4,1,37,8));
        assert_eq!((h.options.strips,h.options.instances,h.options.ds_offset),(4,5,3));
        assert_eq!(h.options.corner,Corner::BottomLeft);
        assert!(h.neighborhood.is_none());
        assert_eq!(out.bitmap.packed(),EXPECTED);
        assert_eq!((out.stats.strips,out.stats.instances,out.stats.composed_pixels),(1,5,184));
    }
    #[test]
    fn annex_h_arithmetic_original_refined_region(){
        let page=join(PAGE3,TEXT3);
        let d=execute_pdf_dictionaries(GLOBAL3,&page,ExecutionLimits::default()).unwrap();
        let refs=[d.symbol(17,0).unwrap(),d.symbol(17,1).unwrap(),d.symbol(17,2).unwrap()];
        assert_eq!(refs[0].packed(),[120,4,124,132,132,124]);
        assert_eq!(refs[1].packed(),[120,132,128,128,132,120]);
        assert_eq!(refs[2].packed(),[120,120,4,132,124,128,132,128,132,132,124,120]);
        assert!(std::ptr::eq(d.symbol(16,0).unwrap(),refs[0]));
        let (h,out)=decode_arithmetic_text_payload(&TEXT3[12..],&refs,2000,40000,64).unwrap();
        assert_eq!((h.region.x,h.region.y,h.options.width,h.options.height),(0,0,37,8));
        assert_eq!((h.options.strips,h.options.instances,h.options.ds_offset),(1,4,3));
        assert_eq!(h.options.corner,Corner::TopLeft);
        assert_eq!(h.neighborhood.unwrap().template(),super::super::refinement::Template::T1);
        assert_eq!(out.bitmap.packed(),EXPECTED);
        assert_eq!(out.stats.instances,4);
    }
    #[test]
    fn annex_h_arithmetic_original_bounds(){
        for (global,prefix,text,header,ids) in [
            (GLOBAL2,PAGE2,TEXT2,13usize,[(0,0),(9,0),(9,1)]),
            (GLOBAL3,PAGE3,TEXT3,12usize,[(17,0),(17,1),(17,2)])]{
            let page=join(prefix,text);
            let d=execute_pdf_dictionaries(global,&page,ExecutionLimits::default()).unwrap();
            let refs=ids.map(|(s,i)|d.symbol(s,i).unwrap());
            let payload=&text[header..];
            let (_,out)=decode_arithmetic_text_payload(payload,&refs,2000,40000,64).unwrap();
            assert_eq!(decode_arithmetic_text_payload(payload,&refs,2000,out.stats.work_used,64).unwrap().1,out);
            assert!(decode_arithmetic_text_payload(payload,&refs,2000,out.stats.work_used-1,64).is_err());
            assert!(decode_arithmetic_text_payload(payload,&refs,0,40000,64).is_err());
            assert!(decode_arithmetic_text_payload(payload,&refs,2000,40000,39).is_err());
            for n in 0..payload.len(){
                assert!(decode_arithmetic_text_payload(&payload[..n],&refs,2000,40000,64).is_err(),"truncated at {n}");
            }
            let mut extra=payload.to_vec();extra.push(0);
            assert!(decode_arithmetic_text_payload(&extra,&refs,2000,40000,64).is_err());
            let mut bad=page.clone();bad[prefix.len()+6]=99;
            assert!(execute_pdf_dictionaries(global,&bad,ExecutionLimits::default()).is_err());
        }
    }
}

/// Decode one referenced text segment from original embedded streams.
/// No caller-supplied graph, bitmap identities, or custom tables. Other
/// region payloads are not executed; no page composition or retention.
#[derive(Debug,Clone,Copy)]
pub struct TextSegmentLimits{
    pub dictionaries:super::dictionary::ExecutionLimits,
    pub work:u64,
    pub packed_bytes:usize,
    pub symbol_slots:usize,
    pub decisions:usize,
    pub huffman_bits:usize,
}
impl Default for TextSegmentLimits{
    fn default()->Self{Self{dictionaries:super::dictionary::ExecutionLimits::default(),
        work:bitmap::MAX_COMPOSE_WORK,packed_bytes:bitmap::MAX_PACKED_BYTES,
        symbol_slots:MAX_INSTANCES as usize,decisions:MAX_TEXT_DECISIONS,huffman_bits:MAX_TEXT_DECISIONS}}
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedTextSegment{
    pub number:u32,pub kind:u8,pub page:u32,pub region:bitmap::RegionInfo,
    pub huffman:bool,pub symbol_slots:usize,pub custom_tables:usize,
    pub dictionary_stats:super::dictionary::ExecutionStats,
    /// Includes dictionary execution and this segment's setup/decode.
    /// Batch totals count shared dictionary work only once.
    pub work_used:u64,
    pub decisions_used:usize,
    pub huffman_bits_used:usize,
    pub text:DecodedText,
}
fn check_text_segment_limits(globals:&[u8],page:&[u8],limits:TextSegmentLimits)->Result<()> {
    if limits.work>bitmap::MAX_COMPOSE_WORK||limits.packed_bytes>bitmap::MAX_PACKED_BYTES||
        limits.symbol_slots>MAX_INSTANCES as usize||limits.decisions>MAX_TEXT_DECISIONS||
        limits.huffman_bits>MAX_TEXT_DECISIONS{
        return Err(Error::Limit("JBIG2 text segment limits"));
    }
    let input=globals.len().checked_add(page.len()).ok_or(Error::Limit("JBIG2 text segment input"))?;
    if input as u64>limits.work{return Err(Error::Limit("JBIG2 text segment work"));}
    Ok(())
}
pub fn execute_pdf_text_segment(globals:&[u8],page:&[u8],number:u32,limits:TextSegmentLimits)
    ->Result<DecodedTextSegment>{
    use super::dictionary;
    check_text_segment_limits(globals,page,limits)?;
    let inspected=super::inspect_pdf_streams(globals,page)?;
    let node=inspected.graph.iter().find(|n|n.number==number).ok_or(Error::Invalid("JBIG2 text segment missing"))?;
    let graph_work=inspected.graph.len() as u64;
    let mut dl=limits.dictionaries;
    dl.work=dl.work.min(limits.work.checked_sub(graph_work).ok_or(Error::Limit("JBIG2 text segment work"))?);
    dl.packed_bytes=dl.packed_bytes.min(limits.packed_bytes);
    let dictionaries=dictionary::execute_pdf_dictionaries_inspected(globals,page,dl,true,true,&inspected)?;
    execute_text_segment_inspected(globals,page,&inspected,node,&dictionaries,limits,graph_work,
        bitmap::MAX_PIXELS,u64::from(MAX_INSTANCES))
}
pub(super) fn execute_text_segment_inspected(globals:&[u8],page:&[u8],inspected:&super::PdfInspection,
    node:&super::ResolvedSegment,dictionaries:&super::dictionary::DictionarySet,limits:TextSegmentLimits,
    graph_work:u64,pixel_limit:u64,instance_limit:u64)->Result<DecodedTextSegment>{
    use super::{huffman,SegmentSource};
    let stream=|src|match src{SegmentSource::Globals=>globals,SegmentSource::Page=>page};
    let number=node.number;
    let segment=inspected.segment(node.location).ok_or(Error::Invalid("JBIG2 text segment location"))?;
    if !matches!(segment.kind,4|6|7)||node.location.source!=SegmentSource::Page{
        return Err(Error::Invalid("JBIG2 text segment kind"));
    }
    let data=stream(node.location.source).get(segment.payload.clone()).ok_or(Error::Invalid("JBIG2 text segment span"))?;
    let flags=data.get(17..19).ok_or(Error::Invalid("JBIG2 text segment flags"))?;
    let is_huffman=flags[1]&1!=0;
    let (region,instances,expected_custom,huffman_bits)=if is_huffman{
        let (h,body)=parse_huffman_text_header(data)?;
        let bits=body.len().checked_mul(8).ok_or(Error::Limit("JBIG2 text payload bits"))?;
        if bits>limits.huffman_bits{return Err(Error::Limit("JBIG2 text payload bits"));}
        (h.region,h.options.instances,h.selections.custom_count,bits)
    }else{
        let (h,_)=parse_arithmetic_text_header(data)?;(h.region,h.options.instances,0,0)
    };
    if u64::from(region.width)*u64::from(region.height)>pixel_limit||u64::from(instances)>instance_limit{
        return Err(Error::Limit("JBIG2 cumulative text pixels or instances"));
    }
    let mut custom_count=0usize;
    let mut setup_work=graph_work.checked_add(node.dependencies.len() as u64)
        .ok_or(Error::Limit("JBIG2 text segment work"))?;
    for &loc in &node.dependencies{
        let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 text dependency"))?;
        match dep.kind{
            0=>{},
            53 if is_huffman=>{
                custom_count+=1;
                if custom_count>expected_custom{return Err(Error::Invalid("JBIG2 text custom table count"));}
                setup_work=setup_work.checked_add((dep.payload.len() as u64).checked_mul(64)
                    .ok_or(Error::Limit("JBIG2 text custom table work"))?)
                    .ok_or(Error::Limit("JBIG2 text custom table work"))?;
            },
            _=>return Err(Error::Invalid("JBIG2 text dependency kind")),
        }
    }
    if custom_count!=expected_custom{return Err(Error::Invalid("JBIG2 text custom table count"));}
    let stats=dictionaries.stats();
    let mut count=0usize;
    let cap=limits.symbol_slots.min(if is_huffman{huffman::MAX_ROWS}else{MAX_INSTANCES as usize});
    for &loc in &node.dependencies{
        let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 text dependency"))?;
        if dep.kind==0{
            count=count.checked_add(dictionaries.dictionary(dep.number).ok_or(Error::Invalid("JBIG2 text missing dictionary"))?.len())
                .ok_or(Error::Limit("JBIG2 text symbol slots"))?;
            if count>cap{return Err(Error::Limit("JBIG2 text symbol slots"));}
        }
    }
    setup_work=setup_work.checked_add(count as u64).ok_or(Error::Limit("JBIG2 text segment work"))?;
    let left=limits.work.checked_sub(stats.work_used).and_then(|v|v.checked_sub(setup_work))
        .ok_or(Error::Limit("JBIG2 text segment work"))?;
    let packed=limits.packed_bytes.checked_sub(stats.packed_bytes).ok_or(Error::Limit("JBIG2 text segment packed budget"))?;
    let mut symbols=Vec::new();symbols.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 text symbol allocation"))?;
    let mut custom=Vec::new();custom.try_reserve_exact(custom_count).map_err(|_|Error::Limit("JBIG2 text custom allocation"))?;
    for &loc in &node.dependencies{
        let dep=inspected.segment(loc).ok_or(Error::Invalid("JBIG2 text dependency"))?;
        if dep.kind==0{
            for &id in dictionaries.dictionary(dep.number).ok_or(Error::Invalid("JBIG2 text missing dictionary"))?{
                symbols.push(dictionaries.bitmap(id).ok_or(Error::Invalid("JBIG2 text symbol identity"))?);
            }
        }else{
            custom.push(huffman::parse_custom(stream(loc.source).get(dep.payload.clone())
                .ok_or(Error::Invalid("JBIG2 text custom span"))?)?);
        }
    }
    let mut decisions_used=0;
    let text=if is_huffman{
        let refs:Vec<_>=custom.iter().collect(); // at most eight tables
        decode_huffman_text_payload_impl(data,&symbols,&refs,limits.huffman_bits,limits.decisions,left,Some(packed),&mut decisions_used)?.1
    }else{
        decode_arithmetic_text_payload_counted(data,&symbols,limits.decisions,left,packed,&mut decisions_used)?.1
    };
    Ok(DecodedTextSegment{number,kind:segment.kind,page:segment.page,region,huffman:is_huffman,
        symbol_slots:count,custom_tables:custom_count,dictionary_stats:stats,
        work_used:stats.work_used+setup_work+text.stats.work_used,decisions_used,huffman_bits_used:huffman_bits,text})
}

/// Batch limits: text.work and packed_bytes include dictionary decoding,
/// all retained text outputs, and active temporary refinement. Text slots,
/// decisions and Huffman bits are cumulative across text regions only;
/// dictionary-specific caps remain in text.dictionaries.
#[derive(Debug,Clone,Copy)]
pub struct TextRegionsLimits{
    pub text:TextSegmentLimits,
    pub regions:usize,
    pub pixels:u64,
    pub instances:u64,
}
impl Default for TextRegionsLimits{
    fn default()->Self{Self{text:TextSegmentLimits::default(),regions:super::MAX_SEGMENTS,
        pixels:bitmap::MAX_PIXELS,instances:u64::from(MAX_INSTANCES)}}
}
#[derive(Debug,PartialEq,Eq)]
pub struct DecodedTextRegions{
    pub regions:Vec<DecodedTextSegment>,
    pub dictionary_stats:super::dictionary::ExecutionStats,
    pub work_used:u64,
    pub packed_bytes:usize,
    pub pixels:u64,
    pub instances:u64,
    pub symbol_slots:usize,
    pub decisions_used:usize,
    pub huffman_bits_used:usize,
}
/// Decode every type 4/6/7 in original graph order using one dictionary
/// arena. Keep all output regions (including intermediate ones) and abort
/// atomically on failure. No page compositing, retention flags, or other
/// region decoding; unrelated dictionaries are still executed.
pub fn execute_pdf_text_regions(globals:&[u8],page:&[u8],limits:TextRegionsLimits)->Result<DecodedTextRegions>{
    use super::dictionary;
    check_text_segment_limits(globals,page,limits.text)?;
    if limits.regions>super::MAX_SEGMENTS||limits.pixels>bitmap::MAX_PIXELS||
        limits.instances>u64::from(MAX_INSTANCES){
        return Err(Error::Limit("JBIG2 text regions limits"));
    }
    let inspected=super::inspect_pdf_streams(globals,page)?;
    // Count and execute in two linear scans, no per-segment graph search.
    let graph_work=(inspected.graph.len() as u64)*2;
    let count=inspected.graph.iter().filter(|n|inspected.segment(n.location)
        .is_some_and(|s|matches!(s.kind,4|6|7))).count();
    if count>limits.regions{return Err(Error::Limit("JBIG2 text region count"));}
    let mut dl=limits.text.dictionaries;
    dl.work=dl.work.min(limits.text.work.checked_sub(graph_work).ok_or(Error::Limit("JBIG2 text regions work"))?);
    dl.packed_bytes=dl.packed_bytes.min(limits.text.packed_bytes);
    let dictionaries=dictionary::execute_pdf_dictionaries_inspected(globals,page,dl,true,true,&inspected)?;
    let stats=dictionaries.stats();
    let mut out=DecodedTextRegions{regions:Vec::new(),dictionary_stats:stats,
        work_used:stats.work_used+graph_work,packed_bytes:stats.packed_bytes,
        pixels:0,instances:0,symbol_slots:0,decisions_used:0,huffman_bits_used:0};
    out.regions.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 text regions allocation"))?;
    for node in &inspected.graph{
        let segment=inspected.segment(node.location).ok_or(Error::Invalid("JBIG2 text segment location"))?;
        if !matches!(segment.kind,4|6|7){continue;}
        let remaining=TextSegmentLimits{
            work:limits.text.work.checked_sub(out.work_used).ok_or(Error::Limit("JBIG2 text regions work"))?+stats.work_used,
            packed_bytes:limits.text.packed_bytes-out.packed_bytes+stats.packed_bytes,
            symbol_slots:limits.text.symbol_slots-out.symbol_slots,
            decisions:limits.text.decisions-out.decisions_used,
            huffman_bits:limits.text.huffman_bits-out.huffman_bits_used,
            ..limits.text
        };
        let d=execute_text_segment_inspected(globals,page,&inspected,node,&dictionaries,remaining,0,
            limits.pixels-out.pixels,limits.instances-out.instances)?;
        out.work_used+=d.work_used-stats.work_used;
        out.packed_bytes+=d.text.bitmap.packed().len();
        out.pixels+=u64::from(d.region.width)*u64::from(d.region.height);
        out.instances+=u64::from(d.text.stats.instances);
        out.symbol_slots+=d.symbol_slots;
        out.decisions_used+=d.decisions_used;
        out.huffman_bits_used+=d.huffman_bits_used;
        out.regions.push(d);
    }
    Ok(out)
}
