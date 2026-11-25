//! JPEG 2000 packet-header bit stuffing and persistent tag-tree decoding.
//! Packet header fields and entropy payloads are handled in later stages.
use crate::{Error,Result,MAX_BYTES};
const MAX_TREE_NODES:usize=262_144;
const MAX_THRESHOLD:u16=256;

/// MSB-first reader. After a byte FF the next byte contributes seven bits;
/// its most significant bit must be zero. No entropy-decoder semantics.
#[derive(Clone,Copy)]
pub struct HeaderBits<'a>{data:&'a[u8],pos:usize,byte:u8,left:u8,after_ff:bool}
impl<'a> HeaderBits<'a>{
    pub fn new(data:&'a[u8])->Result<Self>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("JPX packet header bytes"));}
        Ok(Self{data,pos:0,byte:0,left:0,after_ff:false})
    }
    fn load(&mut self)->Result<()>{
        let b=*self.data.get(self.pos).ok_or(Error::Invalid("JPX truncated packet header"))?;
        if self.after_ff&&b&0x80!=0{return Err(Error::Invalid("JPX packet stuffed bit"));}
        self.left=if self.after_ff{7}else{8};self.byte=b;self.after_ff=b==255;self.pos+=1;Ok(())
    }
    pub fn bit(&mut self)->Result<u8>{
        if self.left==0{self.load()?;}
        self.left-=1;Ok((self.byte>>self.left)&1)
    }
    /// Atomic on error. Zero bits is allowed; at most 32 per read.
    pub fn bits(&mut self,n:u8)->Result<u32>{
        if n>32{return Err(Error::Limit("JPX packet bit count"));}
        let mut r=*self;let mut out=0;
        for _ in 0..n{out=(out<<1)|u32::from(r.bit()?);}
        *self=r;Ok(out)
    }
    /// Discard remaining alignment bits. If the current byte is FF, consume
    /// its mandatory stuffed successor as specified for packet alignment.
    /// Padding values other than the stuffed high bit are not validated here.
    pub fn align(&mut self)->Result<usize>{
        let mut r=*self;
        if r.after_ff{r.load()?;}
        r.left=0;r.after_ff=false;*self=r;Ok(self.pos)
    }
    pub fn bytes_loaded(&self)->usize{self.pos}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Node{low:u16,value:Option<u16>}
#[derive(Debug)]
struct Level{offset:usize,width:usize,height:usize}
/// Persistent inclusion/zero-bit-plane tag tree for one precinct subband.
/// Threshold queries indicate whether a leaf's value is below threshold.
pub struct TagTree{levels:Vec<Level>,nodes:Vec<Node>}
impl TagTree{
    pub fn new(mut width:usize,mut height:usize)->Result<Self>{
        if width==0||height==0{return Err(Error::Invalid("JPX tag tree dimensions"));}
        let mut levels=Vec::new();let mut total=0usize;
        loop{
            let n=width.checked_mul(height).ok_or(Error::Limit("JPX tag tree nodes"))?;
            levels.push(Level{offset:total,width,height});
            total=total.checked_add(n).ok_or(Error::Limit("JPX tag tree nodes"))?;
            if total>MAX_TREE_NODES{return Err(Error::Limit("JPX tag tree nodes"));}
            if width==1&&height==1{break;}
            width=width/2+width%2;height=height/2+height%2;
        }
        Ok(Self{levels,nodes:vec![Node{low:0,value:None};total]})
    }
    /// Decode one leaf, root to leaf. Lower thresholds and repeated queries
    /// are supported without re-reading bits. Reader and tree are unchanged
    /// on truncation or invalid input.
    pub fn below(&mut self,x:usize,y:usize,threshold:u16,bits:&mut HeaderBits<'_>)->Result<bool>{
        if threshold>MAX_THRESHOLD{return Err(Error::Limit("JPX tag tree threshold"));}
        let l=&self.levels[0];
        if x>=l.width||y>=l.height{return Err(Error::Invalid("JPX tag tree leaf"));}
        let mut path=Vec::with_capacity(self.levels.len());let (mut x,mut y)=(x,y);
        for l in &self.levels{path.push(l.offset+y*l.width+x);x/=2;y/=2;}
        let mut read=*bits;let mut low=0u16;
        let mut changes=Vec::with_capacity(path.len());let mut included=false;
        for i in path.into_iter().rev(){
            let mut node=self.nodes[i];low=low.max(node.low);
            while low<threshold&&node.value.is_none_or(|v|low<v){
                if read.bit()?==1{node.value=Some(low);}else{low+=1;}
            }
            node.low=low;
            included=node.value.is_some_and(|v|v<threshold);
            changes.push((i,node));
        }
        for (i,node) in changes{self.nodes[i]=node;}
        *bits=read;Ok(included)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn stuffed_bits_and_alignment(){
        let mut b=HeaderBits::new(&[0xff,0x55,0x91]).unwrap();
        assert_eq!(b.bits(8).unwrap(),255);assert_eq!(b.bits(7).unwrap(),0x55);
        assert_eq!(b.bits(4).unwrap(),9);assert_eq!(b.align().unwrap(),3);
        let mut b=HeaderBits::new(&[0xff,0,0x91]).unwrap();
        assert_eq!(b.bits(1).unwrap(),1);assert_eq!(b.align().unwrap(),2);
        assert_eq!(b.bits(8).unwrap(),0x91);
        let mut b=HeaderBits::new(&[0x80,0x91]).unwrap();
        assert_eq!(b.bit().unwrap(),1);assert_eq!(b.align().unwrap(),1);assert_eq!(b.bits(8).unwrap(),0x91);
    }
    #[test]fn malformed_stuffing_and_atomic_reads(){
        for bytes in [&[0xff,0x80][..],&[0xff][..]]{
            let mut b=HeaderBits::new(bytes).unwrap();assert_eq!(b.bits(8).unwrap(),255);
            assert!(b.bits(1).is_err());assert_eq!(b.bytes_loaded(),1);
            assert!(b.align().is_err());assert_eq!(b.bytes_loaded(),1);
        }
        let mut b=HeaderBits::new(&[0x12]).unwrap();
        assert!(b.bits(9).is_err());assert_eq!(b.bytes_loaded(),0);
        assert!(b.bits(33).is_err());assert_eq!(b.bits(0).unwrap(),0);
        assert_eq!(b.bits(8).unwrap(),0x12);
    }
    #[test]fn tag_tree_multi_layer_vector(){
        // Hand-derived 2x2 values [0,2;1,3], root min=0.
        // Query sequence: layer1 all, layer2 missing, layer3 missing,
        // layer4 final. Encoded decisions 11000010101.
        let mut b=HeaderBits::new(&[0xc2,0xa0]).unwrap();let mut t=TagTree::new(2,2).unwrap();
        let v=[0,2,1,3];
        for threshold in 1..=4{
            for i in 0..4{
                assert_eq!(t.below(i%2,i/2,threshold,&mut b).unwrap(),v[i]<threshold);
            }
        }
        assert_eq!(b.bits(5).unwrap(),0);
        assert!(t.below(1,1,4,&mut b).unwrap());
        assert!(!t.below(1,1,3,&mut b).unwrap());
        assert_eq!(b.bytes_loaded(),2);
    }
    #[test]fn tag_tree_odd_shape_and_rollback(){
        let mut t=TagTree::new(3,1).unwrap();
        // root, first parent and first leaf all zero (111);
        // second leaf zero (1), other parent and leaf zero (11).
        let mut b=HeaderBits::new(&[0xfc]).unwrap();
        for x in 0..3{assert!(t.below(x,0,1,&mut b).unwrap());}
        assert_eq!(b.bits(2).unwrap(),0);
        let mut t=TagTree::new(1,1).unwrap();let mut b=HeaderBits::new(&[0]).unwrap();
        assert!(t.below(0,0,9,&mut b).is_err());assert_eq!(b.bytes_loaded(),0);
        assert_eq!(t.nodes[0],Node{low:0,value:None});
        let mut b=HeaderBits::new(&[0x80]).unwrap();
        assert!(t.below(0,0,1,&mut b).unwrap());
    }
    #[test]fn tag_tree_limits(){
        for (w,h) in [(0,1),(1,0),(usize::MAX,2),(262144,1)]{assert!(TagTree::new(w,h).is_err());}
        let mut t=TagTree::new(1,1).unwrap();let mut b=HeaderBits::new(&[]).unwrap();
        assert!(t.below(1,0,1,&mut b).is_err());assert!(t.below(0,0,257,&mut b).is_err());
        assert!(!t.below(0,0,0,&mut b).unwrap());
    }
}
