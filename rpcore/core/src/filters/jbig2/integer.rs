//! T.88 Annex A arithmetic integer and symbol identifier decoding.
//! Each syntax procedure owns its own persistent adaptive context bank.
//! No symbol dictionary or text region reconstruction is performed here.
use crate::{Error,Result};
use super::qm::{Context,Decoder};

const INTEGER_CONTEXTS:usize=512;
pub const MAX_ID_BITS:u8=20;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Integer{Value(i64),Oob}

// Nine-bit rolling context: retain a leading one plus the eight most
// recent decisions once the history has reached eight decisions.
fn next_history(prev:usize,bit:u8)->usize{
    let n=(prev<<1)|usize::from(bit);
    if prev<256{n}else{(n&511)|256}
}
fn integer_bits(mut bit:impl FnMut(usize)->Result<u8>)->Result<Integer>{
    let mut prev=1usize;
    let mut read=||->Result<u8>{
        let b=bit(prev)?;
        prev=next_history(prev,b);
        Ok(b)
    };
    let negative=read()?!=0;
    let mut class=0;
    while class<5&&read()?!=0{class+=1;}
    let (width,base)=[(2,0i64),(4,4),(6,20),(8,84),(12,340),(32,4436)][class];
    let mut magnitude=0i64;
    for _ in 0..width{magnitude=(magnitude<<1)|i64::from(read()?);}
    magnitude+=base;
    Ok(if negative{
        if magnitude==0{Integer::Oob}else{Integer::Value(-magnitude)}
    }else{Integer::Value(magnitude)})
}

/// One independent IAx bank. Reuse across values of the same syntax field;
/// allocate distinct banks for different fields. Failure after a decision
/// can leave the arithmetic stream and bank partially advanced.
pub struct IntegerDecoder{contexts:[Context;INTEGER_CONTEXTS]}
impl Default for IntegerDecoder{
    fn default()->Self{Self{contexts:[Context::default();INTEGER_CONTEXTS]}}
}
impl IntegerDecoder{
    pub fn new()->Self{Self::default()}
    pub fn reset(&mut self){self.contexts.fill(Context::default());}
    pub fn decode(&mut self,decoder:&mut Decoder<'_>)->Result<Integer>{
        integer_bits(|i|decoder.decode(&mut self.contexts[i]))
    }
}

fn id_bits(width:u8,mut bit:impl FnMut(usize)->Result<u8>)->Result<u32>{
    let mut prev=1usize;
    for _ in 0..width{prev=(prev<<1)|usize::from(bit(prev)?);}
    Ok((prev-(1usize<<width))as u32)
}
/// Fixed-length IAID tree; supports 0..=20 bits in this bounded core.
/// Width zero represents a single identifier and consumes no decisions.
pub struct IaidDecoder{width:u8,contexts:Vec<Context>}
impl IaidDecoder{
    pub fn new(width:u8)->Result<Self>{
        if width>MAX_ID_BITS{return Err(Error::Limit("JBIG2 IAID context width"));}
        let count=1usize<<width;
        let mut contexts=Vec::new();
        contexts.try_reserve_exact(count).map_err(|_|Error::Limit("JBIG2 IAID contexts"))?;
        contexts.resize(count,Context::default());
        Ok(Self{width,contexts})
    }
    pub fn width(&self)->u8{self.width}
    pub fn reset(&mut self){self.contexts.fill(Context::default());}
    pub fn decode(&mut self,decoder:&mut Decoder<'_>)->Result<u32>{
        id_bits(self.width,|i|decoder.decode(&mut self.contexts[i]))
    }
    /// Reject unused identifiers in a non-power-of-two symbol table.
    /// A valid range check after decoding does not roll back decisions.
    pub fn decode_for_count(&mut self,decoder:&mut Decoder<'_>,count:u32)->Result<u32>{
        if count==0||count>(1u32<<self.width){return Err(Error::Invalid("JBIG2 IAID symbol count"));}
        let id=self.decode(decoder)?;
        if id>=count{return Err(Error::Invalid("JBIG2 IAID symbol index"));}
        Ok(id)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn scripted(s:&str)->(Integer,Vec<usize>){
        let mut bits=s.bytes();let mut trace=Vec::new();
        let v=integer_bits(|i|{
            trace.push(i);
            match bits.next(){Some(b'0')=>Ok(0),Some(b'1')=>Ok(1),_=>Err(Error::Invalid("test bit"))}
        }).unwrap();
        assert!(bits.next().is_none());(v,trace)
    }
    #[test]fn integer_annex_a_example(){
        // Annex A.2 example IADW = 12 and its exact context indices.
        assert_eq!(scripted("0101000"),(Integer::Value(12),vec![1,2,5,10,21,42,84]));
        assert_eq!(scripted("0000").0,Integer::Value(0));
        assert_eq!(scripted("1000").0,Integer::Oob);
        assert_eq!(scripted("1001").0,Integer::Value(-1));
    }
    #[test]fn integer_all_magnitude_boundaries(){
        for (class,(width,base)) in [(2,0i64),(4,4),(6,20),(8,84),(12,340),(32,4436)].into_iter().enumerate(){
            for tail in [0i64,(1i64<<width)-1]{
                for sign in [false,true]{
                    let prefix=format!("{}{}{}",if sign{"1"}else{"0"},"1".repeat(class),if class<5{"0"}else{""});
                    let code=format!("{prefix}{tail:0width$b}",width=width);
                    let v=base+tail;
                    let want=if sign&&v==0{Integer::Oob}else{Integer::Value(if sign{-v}else{v})};
                    let (actual,trace)=scripted(&code);
                    assert_eq!(actual,want);
                    assert!(trace.iter().all(|&i|i>0&&i<512));
                }
            }
        }
    }
    #[test]fn integer_rolling_context(){
        let code=format!("011111{}","01".repeat(16));
        let (v,trace)=scripted(&code);
        assert_eq!(v,Integer::Value(4436+0x55555555));
        for (i,&cx) in trace.iter().enumerate(){
            let p=&code[..i];let suffix=&p[p.len().saturating_sub(8)..];
            let expected=if suffix.is_empty(){1}else{(1<<suffix.len())|usize::from_str_radix(suffix,2).unwrap()};
            assert_eq!(cx,expected);
        }
        assert!(integer_bits(|_|Err(Error::Invalid("test exhaustion"))).is_err());
    }
    #[test]fn iaid_tree_and_limits(){
        for width in 0..=10u8{
            for id in [0,(1u32<<width)-1,(1u32<<width)/2]{
                if id>=(1u32<<width){continue;}
                let mut trace=Vec::new();
                let actual=id_bits(width,|i|{
                    let k=trace.len();trace.push(i);
                    Ok(((id>>(usize::from(width)-1-k))&1)as u8)
                }).unwrap();
                assert_eq!(actual,id);
                for (k,cx) in trace.into_iter().enumerate(){
                    assert_eq!(cx,(1usize<<k)|((id as usize)>>(usize::from(width)-k)));
                }
            }
        }
        assert!(IaidDecoder::new(MAX_ID_BITS+1).is_err());
        assert_eq!(IaidDecoder::new(MAX_ID_BITS).unwrap().contexts.len(),1<<MAX_ID_BITS);
        let mut d=Decoder::new(&[255,172],0).unwrap();
        let mut id=IaidDecoder::new(0).unwrap();
        assert_eq!(id.decode_for_count(&mut d,1).unwrap(),0);
        assert!(id.decode_for_count(&mut d,0).is_err());
        assert!(id.decode_for_count(&mut d,2).is_err());
    }
    // Offline JavaScript reference encoder uses the Annex E interval
    // encoding equations and byteout/flush, separate from the Rust decoder.
    // Shared normative 47-state table; interleaved IAx and IAID banks.
    const ENCODED:[u8;29]=[
        0xaa,0xf2,0x8c,0x65,0xc5,0x81,0x19,0xc3,0xf5,0xdd,0x6b,0x58,0x2e,0x73,0xf2,
        0xa3,0x73,0x46,0x0a,0x8e,0x90,0xf7,0x7a,0x76,0x20,0xf8,0x7f,0xff,0xac];
    #[test]fn integer_iaid_interleaved_reference(){
        let values=[Some(0),Some(1),Some(-1),Some(3),Some(-3),Some(4),Some(19),Some(-20),
            Some(83),Some(84),Some(-339),Some(340),Some(4435),Some(4436),Some(-4436),
            Some(4294971731),Some(-4294971731),None,Some(12),Some(0),Some(12)];
        let ids=[0,7,3,5,1,6,2,4,7,0];
        let mut d=Decoder::new(&ENCODED,320).unwrap();
        let mut ints=IntegerDecoder::new();let mut iaid=IaidDecoder::new(3).unwrap();
        for (i,v) in values.into_iter().enumerate(){
            assert_eq!(ints.decode(&mut d).unwrap(),v.map_or(Integer::Oob,Integer::Value),"integer {i}");
            if let Some(&id)=ids.get(i){assert_eq!(iaid.decode_for_count(&mut d,8).unwrap(),id,"identifier {i}");}
        }
        assert!(matches!(ints.decode(&mut d),Err(Error::Limit(_))));
        assert!(d.at_marker_prefix());
        ints.reset();iaid.reset();
        assert!(ints.contexts.iter().all(|&x|x==Context::default()));
        assert!(iaid.contexts.iter().all(|&x|x==Context::default()));
    }
    #[test]fn integer_iaid_budget_and_symbol_range(){
        let mut d=Decoder::new(&ENCODED,0).unwrap();
        assert!(matches!(IntegerDecoder::new().decode(&mut d),Err(Error::Limit(_))));
        assert!(matches!(IaidDecoder::new(3).unwrap().decode(&mut d),Err(Error::Limit(_))));
        let mut d=Decoder::new(&ENCODED,320).unwrap();
        let mut ints=IntegerDecoder::new();let mut ids=IaidDecoder::new(3).unwrap();
        assert_eq!(ints.decode(&mut d).unwrap(),Integer::Value(0));
        assert_eq!(ids.decode_for_count(&mut d,1).unwrap(),0);
        assert_eq!(ints.decode(&mut d).unwrap(),Integer::Value(1));
        assert!(matches!(ids.decode_for_count(&mut d,7),Err(Error::Invalid(_))));
    }
}
