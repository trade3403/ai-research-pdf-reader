//! Native packed bi-level bitmap and T.88 (2000) region information.
//! A set bit represents black/ink; rows are MSB first with zero padding.
//! No entropy decoding or PDF rendering is performed by this module.
use crate::{Error,Result};

pub const MAX_SIDE:u32=1_048_576;
pub const MAX_PIXELS:u64=100_000_000;
pub const MAX_PACKED_BYTES:usize=16*1024*1024;
pub const MAX_COMPOSE_WORK:u64=200_000_000;

fn layout(width:u32,height:u32)->Result<(usize,usize)>{
    if width==0||height==0{return Err(Error::Invalid("JBIG2 bitmap dimensions"));}
    if width>MAX_SIDE||height>MAX_SIDE||u64::from(width)*u64::from(height)>MAX_PIXELS{
        return Err(Error::Limit("JBIG2 bitmap dimensions"));
    }
    let stride=(width as usize).div_ceil(8);
    let len=stride.checked_mul(height as usize).ok_or(Error::Limit("JBIG2 bitmap bytes"))?;
    if len>MAX_PACKED_BYTES{return Err(Error::Limit("JBIG2 bitmap bytes"));}
    Ok((stride,len))
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ComposeOp{Or,And,Xor,Xnor,Replace}
impl ComposeOp{
    pub fn from_code(code:u8)->Result<Self>{
        match code{
            0=>Ok(Self::Or),1=>Ok(Self::And),2=>Ok(Self::Xor),
            3=>Ok(Self::Xnor),4=>Ok(Self::Replace),
            _=>Err(Error::Unsupported("JBIG2 combination operator")),
        }
    }
    fn apply(self,d:bool,s:bool)->bool{
        match self{Self::Or=>d|s,Self::And=>d&s,Self::Xor=>d^s,
            Self::Xnor=>!(d^s),Self::Replace=>s}
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RegionInfo{
    pub width:u32,pub height:u32,pub x:u32,pub y:u32,pub op:ComposeOp,
}
/// T.88 section 7.4.1: consume exactly the common 17-byte prefix.
/// Later amendments with colored-region flags are outside this subset.
pub fn parse_region(data:&[u8])->Result<(RegionInfo,&[u8])>{
    let h=data.get(..17).ok_or(Error::Invalid("JBIG2 region header"))?;
    let word=|i:usize|u32::from_be_bytes([h[i],h[i+1],h[i+2],h[i+3]]);
    let (width,height)=(word(0),word(4));
    layout(width,height)?;
    if h[16]&0xf8!=0{return Err(Error::Unsupported("JBIG2 region flags"));}
    Ok((RegionInfo{width,height,x:word(8),y:word(12),op:ComposeOp::from_code(h[16])?},&data[17..]))
}

#[derive(Debug,PartialEq,Eq)]
pub struct Bitmap{width:u32,height:u32,stride:usize,bytes:Vec<u8>}
impl Bitmap{
    pub fn new(width:u32,height:u32,black:bool)->Result<Self>{
        let (stride,len)=layout(width,height)?;
        let mut bytes=Vec::new();
        bytes.try_reserve_exact(len).map_err(|_|Error::Limit("JBIG2 bitmap allocation"))?;
        bytes.resize(len,if black{255}else{0});
        let mut out=Self{width,height,stride,bytes};out.clear_padding();Ok(out)
    }
    pub fn from_packed(width:u32,height:u32,data:&[u8])->Result<Self>{
        let (_,len)=layout(width,height)?;
        if data.len()!=len{return Err(Error::Invalid("JBIG2 packed bitmap length"));}
        let mut out=Self::new(width,height,false)?;
        out.bytes.copy_from_slice(data);out.clear_padding();Ok(out)
    }
    pub fn width(&self)->u32{self.width}
    pub fn height(&self)->u32{self.height}
    pub fn stride(&self)->usize{self.stride}
    pub fn packed(&self)->&[u8]{&self.bytes}
    fn clear_padding(&mut self){
        let rem=self.width%8;
        if rem!=0{
            let mask=255u8<<(8-rem);
            for row in self.bytes.chunks_exact_mut(self.stride){row[self.stride-1]&=mask;}
        }
    }
    fn offset(&self,x:u32,y:u32)->Option<(usize,u8)>{
        if x>=self.width||y>=self.height{return None;}
        Some((y as usize*self.stride+x as usize/8,0x80>>(x%8)))
    }
    pub fn get(&self,x:u32,y:u32)->Option<bool>{
        let (i,m)=self.offset(x,y)?;Some(self.bytes[i]&m!=0)
    }
    /// Outside a generic coding template's image, reference pixels are zero.
    pub fn sample(&self,x:i64,y:i64)->bool{
        let (Ok(x),Ok(y))=(u32::try_from(x),u32::try_from(y))else{return false;};
        self.get(x,y).unwrap_or(false)
    }
    pub fn set(&mut self,x:u32,y:u32,black:bool)->Result<()>{
        let (i,m)=self.offset(x,y).ok_or(Error::Invalid("JBIG2 bitmap coordinate"))?;
        if black{self.bytes[i]|=m;}else{self.bytes[i]&=!m;}
        Ok(())
    }
    /// Copy a completed previous row, including canonical zero padding.
    pub(super) fn copy_previous_row(&mut self,y:u32)->Result<()>{
        if y==0||y>=self.height{return Err(Error::Invalid("JBIG2 previous row coordinate"));}
        let end=y as usize*self.stride;
        self.bytes.copy_within(end-self.stride..end,end);
        Ok(())
    }
    /// Combine only pixels inside both images. Negative placement is
    /// supported for future symbol/region positioning; i128 intersections
    /// avoid overflow even for extreme caller coordinates.
    /// Charge the complete overlap before writing, so budget failure is atomic.
    pub fn compose(&mut self,src:&Bitmap,x:i64,y:i64,op:ComposeOp,budget:&mut ComposeBudget)->Result<()>{
        let (x,y)=(i128::from(x),i128::from(y));
        let left=x.max(0);let top=y.max(0);
        let right=(x+i128::from(src.width)).min(i128::from(self.width));
        let bottom=(y+i128::from(src.height)).min(i128::from(self.height));
        if left>=right||top>=bottom{return Ok(());}
        let count=(right-left)as u64*(bottom-top)as u64;
        budget.charge(count)?;
        // Convert the clipped rectangle once. All indices below are inside
        // their respective rows; a full destination byte covers eight valid
        // source pixels, including the next source byte if unaligned.
        let left=left as usize;let right=right as usize;
        let sx0=(left as i128-x)as usize;
        let head_end=left.div_ceil(8).saturating_mul(8).min(right);
        let body_end=(right/8*8).max(head_end);
        for dy in top..bottom{
            let sy=(dy-y)as usize;
            let s=&src.bytes[sy*src.stride..(sy+1)*src.stride];
            let dy=dy as usize;
            let d=&mut self.bytes[dy*self.stride..(dy+1)*self.stride];
            let edge=|d:&mut [u8],dx:usize|{
                let sx=sx0+dx-left;
                let sm=0x80u8>>(sx%8);let dm=0x80u8>>(dx%8);
                let bit=op.apply(d[dx/8]&dm!=0,s[sx/8]&sm!=0);
                if bit{d[dx/8]|=dm;}else{d[dx/8]&=!dm;}
            };
            for dx in left..head_end{edge(d,dx);}
            let n=(body_end-head_end)/8;
            if n!=0{
                let sx=sx0+head_end-left;let start=sx/8;let shift=sx%8;
                let dest=&mut d[head_end/8..head_end/8+n];
                if shift==0{
                    let source=&s[start..start+n];
                    match op{
                        ComposeOp::Replace=>dest.copy_from_slice(source),
                        ComposeOp::Or=>dest.iter_mut().zip(source).for_each(|(d,&s)|*d|=s),
                        ComposeOp::And=>dest.iter_mut().zip(source).for_each(|(d,&s)|*d&=s),
                        ComposeOp::Xor=>dest.iter_mut().zip(source).for_each(|(d,&s)|*d^=s),
                        ComposeOp::Xnor=>dest.iter_mut().zip(source).for_each(|(d,&s)|*d=!(*d^s)),
                    }
                }else{
                    for (d,pair) in dest.iter_mut().zip(s[start..start+n+1].windows(2)){
                        let v=(pair[0]<<shift)|(pair[1]>>(8-shift));
                        *d=match op{ComposeOp::Or=>*d|v,ComposeOp::And=>*d&v,
                            ComposeOp::Xor=>*d^v,ComposeOp::Xnor=>!(*d^v),ComposeOp::Replace=>v};
                    }
                }
            }
            for dx in body_end..right{edge(d,dx);}
        }
        Ok(())
    }
    pub fn compose_region(&mut self,src:&Bitmap,r:RegionInfo,budget:&mut ComposeBudget)->Result<()>{
        if (src.width,src.height)!=(r.width,r.height){
            return Err(Error::Invalid("JBIG2 region bitmap dimensions"));
        }
        self.compose(src,i64::from(r.x),i64::from(r.y),r.op,budget)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ComposeBudget{remaining:u64}
impl ComposeBudget{
    pub fn new(limit:u64)->Result<Self>{
        if limit>MAX_COMPOSE_WORK{return Err(Error::Limit("JBIG2 composition budget"));}
        Ok(Self{remaining:limit})
    }
    pub fn remaining(&self)->u64{self.remaining}
    fn charge(&mut self,n:u64)->Result<()>{
        self.remaining=self.remaining.checked_sub(n).ok_or(Error::Limit("JBIG2 composition work"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn header(w:u32,h:u32,x:u32,y:u32,op:u8)->Vec<u8>{
        let mut v=Vec::new();
        for n in [w,h,x,y]{v.extend(n.to_be_bytes());}
        v.push(op);v
    }
    #[test]fn bitmap_msb_padding(){
        let mut b=Bitmap::new(9,2,true).unwrap();
        assert_eq!((b.width(),b.height(),b.stride()),(9,2,2));
        assert_eq!(b.packed(),[255,128,255,128]);
        b.set(0,0,false).unwrap();b.set(8,1,false).unwrap();
        assert_eq!(b.packed(),[127,128,255,0]);
        assert_eq!(b.get(8,0),Some(true));assert_eq!(b.get(9,0),None);
        assert!(b.set(9,1,true).is_err());
        for (x,y) in [(-1,0),(0,-1),(9,0),(0,2),(i64::MAX,0)]{assert!(!b.sample(x,y));}
        assert_eq!(Bitmap::from_packed(9,2,&[0,255,0,127]).unwrap().packed(),[0,128,0,0]);
    }
    #[test]fn bitmap_invalid_geometry(){
        for (w,h) in [(0,1),(1,0),(MAX_SIDE+1,1),(1,MAX_SIDE+1),(100_000,100_000)]{
            assert!(Bitmap::new(w,h,false).is_err());
        }
        assert!(Bitmap::from_packed(9,2,&[0;3]).is_err());
        assert!(Bitmap::from_packed(9,2,&[0;5]).is_err());
        assert!(ComposeBudget::new(MAX_COMPOSE_WORK+1).is_err());
    }
    #[test]fn region_header_and_flags(){
        for code in 0..5{
            let mut h=header(13,7,300,400,code);h.extend([1,2,3]);
            let (r,p)=parse_region(&h).unwrap();
            assert_eq!((r.width,r.height,r.x,r.y),(13,7,300,400));
            assert_eq!(r.op,ComposeOp::from_code(code).unwrap());assert_eq!(p,[1,2,3]);
            for n in 0..17{assert!(parse_region(&h[..n]).is_err());}
        }
        for code in 5..=255{assert!(parse_region(&header(13,7,0,0,code)).is_err());}
        assert!(parse_region(&header(0,7,0,0,0)).is_err());
        assert!(parse_region(&header(u32::MAX,7,0,0,0)).is_err());
    }
    #[test]fn bitmap_compose_truth_table(){
        let src=Bitmap::from_packed(4,1,&[0x50]).unwrap();
        // Destination 0011 and source 0101 exercise all four input pairs.
        for (op,want) in [(ComposeOp::Or,0x70),(ComposeOp::And,0x10),
            (ComposeOp::Xor,0x60),(ComposeOp::Xnor,0x90),(ComposeOp::Replace,0x50)]{
            let mut d=Bitmap::from_packed(4,1,&[0x30]).unwrap();
            let mut budget=ComposeBudget::new(4).unwrap();
            d.compose(&src,0,0,op,&mut budget).unwrap();
            assert_eq!(d.packed(),[want]);assert_eq!(budget.remaining(),0);
        }
    }
    #[test]fn bitmap_clipped_reference(){
        // Independent unpacked boolean-grid reference, all operators,
        // crossing all four edges, unaligned placements and empty overlap.
        let s=Bitmap::from_packed(5,3,&[0xa8,0x50,0xe0]).unwrap();
        let initial=[0x92,0x80,0x49,0,0x24,0x80,0x12,0];
        for code in 0..5{
            for y in -4i64..=5{for x in -6i64..=10{
                let mut d=Bitmap::from_packed(9,4,&initial).unwrap();
                let mut budget=ComposeBudget::new(100).unwrap();
                d.compose(&s,x,y,ComposeOp::from_code(code).unwrap(),&mut budget).unwrap();
                let mut n=0;
                for dy in 0..4i64{for dx in 0..9i64{
                    let old=initial[dy as usize*2+dx as usize/8]&(128>>(dx%8))!=0;
                    let sx=dx-x;let sy=dy-y;
                    let want=if (0..5).contains(&sx)&&(0..3).contains(&sy){
                        n+=1;
                        let bit=[0xa8u8,0x50,0xe0][sy as usize]&(128>>sx)!=0;
                        match code{0=>old||bit,1=>old&&bit,2=>old!=bit,3=>old==bit,_=>bit}
                    }else{old};
                    assert_eq!(d.get(dx as u32,dy as u32),Some(want),"op {code}, {x},{y}, {dx},{dy}");
                }}
                assert_eq!(budget.remaining(),100-n);
                assert!(d.packed().chunks_exact(2).all(|r|r[1]&127==0));
            }}
        }
    }
    #[test]fn bitmap_budget_atomic_and_extremes(){
        let s=Bitmap::new(5,3,true).unwrap();let mut d=Bitmap::new(9,4,false).unwrap();
        let mut budget=ComposeBudget::new(14).unwrap();
        assert!(matches!(d.compose(&s,1,1,ComposeOp::Or,&mut budget),Err(Error::Limit(_))));
        assert_eq!(budget.remaining(),14);assert!(d.packed().iter().all(|&b|b==0));
        for (x,y) in [(i64::MIN,0),(i64::MAX,0),(0,i64::MIN),(0,i64::MAX)]{
            d.compose(&s,x,y,ComposeOp::Xnor,&mut budget).unwrap();
        }
        assert_eq!(budget.remaining(),14);assert!(d.packed().iter().all(|&b|b==0));
        d.compose(&s,8,3,ComposeOp::Or,&mut budget).unwrap();
        assert_eq!(d.get(8,3),Some(true));assert_eq!(budget.remaining(),13);
    }
    #[test]fn bitmap_packed_compose_matches_scalar_all_alignments(){
        let mut rng=0x12345678u32;
        let mut bytes=|n:usize|->Vec<u8>{(0..n).map(|_|{
            rng=rng.wrapping_mul(1664525).wrapping_add(1013904223);(rng>>24)as u8
        }).collect()};
        for sw in [1u32,7,8,9,15,16,17,23,24,25,31,32,33,65]{
            for dw in [1u32,8,9,16,17,31,32,33,64,67]{
                let src=Bitmap::from_packed(sw,3,&bytes((sw as usize).div_ceil(8)*3)).unwrap();
                let initial=bytes((dw as usize).div_ceil(8)*4);
                for op in [ComposeOp::Or,ComposeOp::And,ComposeOp::Xor,ComposeOp::Xnor,ComposeOp::Replace]{
                    for x in -(sw as i64)-1..=dw as i64+1{
                        for y in [-3i64,-1,0,2,4]{
                            let mut a=Bitmap::from_packed(dw,4,&initial).unwrap();
                            let mut want=Bitmap::from_packed(dw,4,&initial).unwrap();
                            let mut count=0;
                            for dy in 0..4{
                                for dx in 0..dw{
                                    let sx=i64::from(dx)-x;let sy=i64::from(dy)-y;
                                    if (0..i64::from(sw)).contains(&sx)&&(0..3).contains(&sy){
                                        let v=op.apply(want.get(dx,dy).unwrap(),src.sample(sx,sy));
                                        want.set(dx,dy,v).unwrap();count+=1;
                                    }
                                }
                            }
                            let mut budget=ComposeBudget::new(count).unwrap();
                            a.compose(&src,x,y,op,&mut budget).unwrap();
                            assert_eq!(a,want,"{sw} {dw} {x} {y} {op:?}");
                            assert_eq!(budget.remaining(),0);
                            if count>0{
                                let mut a=Bitmap::from_packed(dw,4,&initial).unwrap();
                                let mut budget=ComposeBudget::new(count-1).unwrap();
                                assert!(a.compose(&src,x,y,op,&mut budget).is_err());
                                assert_eq!(a,Bitmap::from_packed(dw,4,&initial).unwrap());
                                assert_eq!(budget.remaining(),count-1);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]fn bitmap_previous_row_copies_padding_and_rejects_bounds(){
        let mut b=Bitmap::from_packed(9,3,&[0xa5,0xff,0,0,0x55,0x7f]).unwrap();
        assert!(b.copy_previous_row(0).is_err());
        assert!(b.copy_previous_row(3).is_err());
        b.copy_previous_row(1).unwrap();b.copy_previous_row(2).unwrap();
        assert_eq!(b.packed(),[0xa5,0x80,0xa5,0x80,0xa5,0x80]);
    }
    #[test]fn bitmap_region_integration(){
        let s=Bitmap::new(3,2,true).unwrap();let mut d=Bitmap::new(8,4,false).unwrap();
        let h=header(3,2,2,1,4);let (r,_)=parse_region(&h).unwrap();
        let mut budget=ComposeBudget::new(6).unwrap();
        d.compose_region(&s,r,&mut budget).unwrap();
        assert_eq!(d.packed(),[0,0x38,0x38,0]);
        let wrong=RegionInfo{width:4,..r};
        assert!(d.compose_region(&s,wrong,&mut budget).is_err());
        assert_eq!(d.packed(),[0,0x38,0x38,0]);
    }
}
