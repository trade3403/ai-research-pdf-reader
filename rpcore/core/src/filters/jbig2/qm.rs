//! JBIG2 arithmetic binary decisions, ITU-T T.88 Annex E.
//! The existing native MQ mathematical core uses the same 47-state table
//! and positive-C decoding equations (not JPEG 2000-specific contexts).
//! This adapter admits a single explicitly terminated arithmetic payload.
//! It is not a region decoder, nor a proof that all encoded decisions exist.
use crate::{Error,Result,MAX_BYTES};
use crate::filters::jpx::mq;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Context(mq::Context);
impl Context{
    pub fn new(state:u8,mps:u8)->Result<Self>{
        mq::Context::new(state,mps).map(Self).map_err(|_|Error::Invalid("JBIG2 QM context"))
    }
    pub fn state(&self)->u8{self.0.state()}
    pub fn mps(&self)->u8{self.0.mps()}
}
impl Default for Context{
    fn default()->Self{Self(mq::Context::new(0,0).expect("valid initial QM state"))}
}

/// Locate the first marker following stuffed entropy bytes. Only the
/// explicit FF AC termination marker is admitted by this scoped entry
/// point, and it must be the final two bytes. A marker is not a checksum.
fn termination(data:&[u8])->Result<usize>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 QM input"));}
    for (i,pair) in data.windows(2).enumerate(){
        if pair[0]==255&&pair[1]>0x8f{
            if pair[1]!=0xac||i+2!=data.len(){
                return Err(Error::Invalid("JBIG2 QM unexpected marker"));
            }
            return Ok(i);
        }
    }
    Err(Error::Invalid("JBIG2 QM missing termination"))
}

pub struct Decoder<'a>{inner:mq::Decoder<'a>,marker:usize,decisions_used:usize}
impl<'a> Decoder<'a>{
    pub fn new(data:&'a[u8],decisions:usize)->Result<Self>{
        let marker=termination(data)?;
        let inner=mq::Decoder::new(data,decisions).map_err(|e|match e{
            Error::Limit(_)=>Error::Limit("JBIG2 QM decisions"),
            _=>Error::Invalid("JBIG2 QM initialization"),
        })?;
        Ok(Self{inner,marker,decisions_used:0})
    }
    /// The caller supplies the context and a finite total decision budget.
    /// Normative marker fill is allowed within that budget.
    #[inline]
    pub fn decode(&mut self,cx:&mut Context)->Result<u8>{
        let bit=self.inner.decode(&mut cx.0).map_err(|e|match e{
            Error::Limit(_)=>Error::Limit("JBIG2 QM decisions"),
            _=>Error::Invalid("JBIG2 QM decision"),
        })?;
        self.decisions_used+=1;
        Ok(bit)
    }
    /// Successful binary decisions, excluding failed budget checks.
    pub fn decisions_used(&self)->usize{self.decisions_used}
    pub fn marker_offset(&self)->usize{self.marker}
    /// Number of bytes at or before the arithmetic input pointer. This
    /// includes a marker prefix if the pointer has reached it, but excludes
    /// the peeked AC byte. It is not the length of the encoded decisions.
    pub fn bytes_consumed(&self)->usize{self.inner.bytes_consumed()}
    pub fn at_marker_prefix(&self)->bool{self.bytes_consumed()==self.marker+1}
    pub fn used_virtual_input(&self)->bool{self.inner.used_virtual_input()}
}

#[cfg(test)]
mod tests{
    use super::*;
    // ITU-T T.88 (2000) Annex H.2: published arithmetic coder test data.
    // https://www.itu.int/rec/T-REC-T.88-200002-S/en
    const CODE:[u8;30]=[
        0x84,0xc7,0x3b,0xfc,0xe1,0xa1,0x43,0x04,0x02,0x20,0x00,0x00,0x41,0x0d,0xbb,0x86,
        0xf4,0x31,0x7f,0xff,0x88,0xff,0x37,0x47,0x1a,0xdb,0x6a,0xdf,0xff,0xac];
    const EXPECT:[u8;32]=[
        0x00,0x02,0x00,0x51,0x00,0x00,0x00,0xc0,0x03,0x52,0x87,0x2a,0xaa,0xaa,0xaa,0xaa,
        0x82,0xc0,0x20,0x00,0xfc,0xd7,0x9e,0xf6,0xbf,0x7f,0xed,0x90,0x4f,0x46,0xa3,0xbf];
    #[test]fn qm_annex_h2_decisions(){
        let mut d=Decoder::new(&CODE,256).unwrap();
        let mut cx=Context::default();
        for (i,b) in EXPECT.iter().enumerate(){
            for k in 0..8{
                assert_eq!(d.decode(&mut cx).unwrap(),(b>>(7-k))&1,"decision {}",i*8+k+1);
            }
        }
        assert_eq!(d.marker_offset(),28);assert_eq!(d.bytes_consumed(),29);
        assert!(d.at_marker_prefix());assert!(!d.used_virtual_input());
        assert_eq!((cx.state(),cx.mps()),(18,1));
        assert_eq!(d.decisions_used(),256);
        assert!(matches!(d.decode(&mut cx),Err(Error::Limit(_))));
        assert_eq!(d.decisions_used(),256);
    }
    #[test]fn qm_annex_h1_initial_context_trace(){
        // Table H.1 gives states and MPS values BEFORE each decision.
        let mut d=Decoder::new(&CODE,12).unwrap();let mut cx=Context::default();
        for (i,state) in [0,1,2,2,2,2,2,3,3,3,3,3].into_iter().enumerate(){
            assert_eq!((cx.state(),cx.mps()),(state,0),"event {}",i+1);
            assert_eq!(d.decode(&mut cx).unwrap(),(EXPECT[i/8]>>(7-i%8))&1);
        }
    }
    #[test]fn qm_context_domain_and_reset(){
        assert_eq!(Context::default(),Context::new(0,0).unwrap());
        for i in 0..47{for m in 0..=1{
            let c=Context::new(i,m).unwrap();assert_eq!((c.state(),c.mps()),(i,m));
        }}
        assert!(Context::new(47,0).is_err());assert!(Context::new(0,2).is_err());
        let mut d=Decoder::new(&CODE,2).unwrap();let mut a=Context::default();let b=a;
        d.decode(&mut a).unwrap();assert_ne!(a,b);assert_eq!(b,Context::default());
    }
    #[test]fn qm_marker_and_stuffing_boundaries(){
        for bytes in [&[0xff,0x00,0xff,0xac][..],&[0xff,0x8f,0xff,0xac][..]]{
            let d=Decoder::new(bytes,1).unwrap();
            assert_eq!(d.marker_offset(),2);assert_eq!(d.bytes_consumed(),2);
            assert!(!d.at_marker_prefix());
        }
        let d=Decoder::new(&[0xff,0xac],1).unwrap();assert!(d.at_marker_prefix());
        for bytes in [&[][..],&[0xff][..],&[0xff,0x90,0xff,0xac][..],
            &[0xff,0xac,0][..],&[0xff,0xff,0xac][..],&[0x12,0x34][..]]{
            assert!(Decoder::new(bytes,1).is_err());
        }
        for end in 0..CODE.len(){assert!(Decoder::new(&CODE[..end],256).is_err());}
    }
    #[test]fn qm_bounded_marker_fill(){
        assert!(matches!(Decoder::new(&CODE,16_777_217),Err(Error::Limit(_))));
        let mut d=Decoder::new(&[0xff,0xac],1024).unwrap();let mut c=Context::default();
        for _ in 0..1024{
            assert!(d.decode(&mut c).unwrap()<=1);assert_eq!(d.bytes_consumed(),1);
            assert!(!d.used_virtual_input());
        }
        let saved=c;
        assert!(matches!(d.decode(&mut c),Err(Error::Limit(_))));assert_eq!(c,saved);
    }
}
