//! JPEG 2000 Part 1 MQ binary arithmetic decoder, independent of Tier-1.
//! Virtual FF termination is normative; it cannot authenticate a truncated segment.
use crate::{Error,Result,MAX_BYTES};
const MAX_SYMBOLS:usize=16_777_216;
// Qe, next MPS, next LPS, switch MPS.
const STATES:[(u32,u8,u8,bool);47]=[
(0x5601,1,1,true),(0x3401,2,6,false),(0x1801,3,9,false),(0x0ac1,4,12,false),
(0x0521,5,29,false),(0x0221,38,33,false),(0x5601,7,6,true),(0x5401,8,14,false),
(0x4801,9,14,false),(0x3801,10,14,false),(0x3001,11,17,false),(0x2401,12,18,false),
(0x1c01,13,20,false),(0x1601,29,21,false),(0x5601,15,14,true),(0x5401,16,14,false),
(0x5101,17,15,false),(0x4801,18,16,false),(0x3801,19,17,false),(0x3401,20,18,false),
(0x3001,21,19,false),(0x2801,22,19,false),(0x2401,23,20,false),(0x2201,24,21,false),
(0x1c01,25,22,false),(0x1801,26,23,false),(0x1601,27,24,false),(0x1401,28,25,false),
(0x1201,29,26,false),(0x1101,30,27,false),(0x0ac1,31,28,false),(0x09c1,32,29,false),
(0x08a1,33,30,false),(0x0521,34,31,false),(0x0441,35,32,false),(0x02a1,36,33,false),
(0x0221,37,34,false),(0x0141,38,35,false),(0x0111,39,36,false),(0x0085,40,37,false),
(0x0049,41,38,false),(0x0025,42,39,false),(0x0015,43,40,false),(0x0009,44,41,false),
(0x0005,45,42,false),(0x0001,45,43,false),(0x5601,46,46,false)];
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Context{state:u8,mps:u8}
impl Context{
    pub fn new(state:u8,mps:u8)->Result<Self>{
        if state>=47||mps>1{return Err(Error::Invalid("JPX MQ context"));}
        Ok(Self{state,mps})
    }
    pub fn state(&self)->u8{self.state}
    pub fn mps(&self)->u8{self.mps}
}
pub struct Decoder<'a>{data:&'a[u8],pos:usize,a:u32,c:u32,ct:u8,left:usize,virtual_input:bool}
impl<'a> Decoder<'a>{
    pub fn new(data:&'a[u8],symbols:usize)->Result<Self>{
        if data.is_empty(){return Err(Error::Invalid("JPX empty MQ segment"));}
        if data.len()>MAX_BYTES||symbols>MAX_SYMBOLS{return Err(Error::Limit("JPX MQ work"));}
        let mut d=Self{data,pos:0,a:0x8000,c:u32::from(data[0])<<16,ct:0,left:symbols,virtual_input:false};
        d.byte_in();d.c<<=7;d.ct-=7;Ok(d)
    }
    fn byte_in(&mut self){
        let b=self.data.get(self.pos).copied().unwrap_or(255);
        let next=self.data.get(self.pos+1).copied().unwrap_or(255);
        if b==255&&next>0x8f{
            self.c=self.c.wrapping_add(0xff00);self.ct=8;
            if self.pos+1>=self.data.len(){self.virtual_input=true;}
        }else{
            self.pos+=1;
            if self.pos>=self.data.len(){self.virtual_input=true;}
            if b==255{self.c=self.c.wrapping_add(u32::from(next)<<9);self.ct=7;}
            else{self.c=self.c.wrapping_add(u32::from(next)<<8);self.ct=8;}
        }
    }
    // The exchange equations always leave 0 < A <= 0xffff.
    // Shift to the next normalization or byte boundary, whichever comes
    // first. BYTEIN must observe C at exactly the same boundary as before.
    #[inline]
    fn renorm(&mut self){
        while self.a<0x8000{
            if self.ct==0{self.byte_in();}
            let needed=self.a.leading_zeros()-16;
            let n=needed.min(u32::from(self.ct));
            self.a<<=n;self.c<<=n;self.ct-=n as u8;
        }
    }
    #[inline]
    pub fn decode(&mut self,cx:&mut Context)->Result<u8>{
        if self.left==0{return Err(Error::Limit("JPX MQ symbols"));}
        self.left-=1;
        let (qe,nm,nl,sw)=STATES[usize::from(cx.state)];
        self.a-=qe;
        let m=cx.mps;
        let bit;
        if self.c>>16<qe{
            if self.a<qe{bit=m;cx.state=nm;}
            else{bit=m^1;cx.state=nl;if sw{cx.mps^=1;}}
            self.a=qe;self.renorm();
        }else{
            self.c-=qe<<16;
            if self.a>=0x8000{return Ok(m);}
            if self.a<qe{bit=m^1;cx.state=nl;if sw{cx.mps^=1;}}
            else{bit=m;cx.state=nm;}
            self.renorm();
        }
        Ok(bit)
    }
    pub fn used_virtual_input(&self)->bool{self.virtual_input}
    pub fn bytes_consumed(&self)->usize{(self.pos+1).min(self.data.len())}
}
#[cfg(test)]
mod tests{
    use super::*;
    // Previous scalar normalization, retained only as a test oracle.
    fn scalar_renorm(d:&mut Decoder<'_>){
        while d.a<0x8000{
            if d.ct==0{d.byte_in();}
            d.a<<=1;d.c<<=1;d.ct-=1;
        }
    }
    fn equal_registers(a:&Decoder<'_>,b:&Decoder<'_>){
        assert_eq!((a.a,a.c,a.ct,a.pos,a.left,a.virtual_input),
                   (b.a,b.c,b.ct,b.pos,b.left,b.virtual_input));
    }
    #[test]fn batched_renorm_all_intervals_and_byte_boundaries(){
        let streams:[&[u8];5]=[&[0,0x12,0x34],&[255,0x8f,0xff,0xac],&[255,0xac],
            &[0x78,255,0x90],&[0]];
        for data in streams{
            for a in 1..0x8000u32{
                for ct in 0..=8u8{
                    // Sweep every non-normalized interval, every bit count,
                    // varied code-register patterns and real/virtual bytes.
                    let c=a.wrapping_mul(0x9e3779b9)^u32::from(ct).wrapping_mul(0x1234567);
                    let pos=(a as usize)%data.len();
                    let mut x=Decoder{data,pos,a,c,ct,left:17,virtual_input:false};
                    let mut y=Decoder{data,pos,a,c,ct,left:17,virtual_input:false};
                    x.renorm();scalar_renorm(&mut y);equal_registers(&x,&y);
                }
            }
        }
    }
    #[test]fn batched_renorm_decision_trace_matches_scalar(){
        // Independently execute the exchange equations with the scalar
        // renormalizer, checking bits, probability states and all registers.
        fn scalar_decode(d:&mut Decoder<'_>,cx:&mut Context)->Result<u8>{
            if d.left==0{return Err(Error::Limit("JPX MQ symbols"));}
            d.left-=1;
            let (qe,nm,nl,sw)=STATES[usize::from(cx.state)];
            d.a-=qe;let m=cx.mps;let bit;
            if d.c>>16<qe{
                if d.a<qe{bit=m;cx.state=nm;}
                else{bit=m^1;cx.state=nl;if sw{cx.mps^=1;}}
                d.a=qe;scalar_renorm(d);
            }else{
                d.c-=qe<<16;
                if d.a>=0x8000{return Ok(m);}
                if d.a<qe{bit=m^1;cx.state=nl;if sw{cx.mps^=1;}}
                else{bit=m;cx.state=nm;}
                scalar_renorm(d);
            }
            Ok(bit)
        }
        let streams:[&[u8];4]=[&[0,0,0],&[255,0x8f,0,255,0xac],&[255,0xac],&[0x12,0x34,0x56,0x78]];
        for data in streams{
            let mut x=Decoder::new(data,4096).unwrap();
            let mut y=Decoder::new(data,4096).unwrap();
            let mut a:Vec<_>=(0..47).flat_map(|s|(0..2).map(move|m|Context::new(s,m).unwrap())).collect();
            let mut b=a.clone();let mut rng=0x12345678u32;
            for _ in 0..4096{
                rng=rng.wrapping_mul(1664525).wrapping_add(1013904223);
                let i=(rng as usize)%a.len();
                assert_eq!(x.decode(&mut a[i]),scalar_decode(&mut y,&mut b[i]));
                assert_eq!(a[i],b[i]);equal_registers(&x,&y);
            }
            assert!(x.decode(&mut a[0]).is_err());
            assert!(scalar_decode(&mut y,&mut b[0]).is_err());
            assert_eq!(a,b);equal_registers(&x,&y);
        }
    }
    // Observational, paired release microbenchmark; no timing assertion.
    #[test]fn mq_paired_renorm_performance(){
        if std::env::var_os("RPCORE_MQ_BENCH").is_none(){return;}
        use std::{hint::black_box,time::Instant};
        fn run(data:&[u8],scalar:bool)->(u128,u64){
            let start=Instant::now();
            let mut hash=0u64;
            for repeat in 0..32u32{
                for a in 1..0x8000u32{
                    let ct=((a+repeat)%9) as u8;
                    let mut d=Decoder{data,pos:(a as usize)%data.len(),a,c:a.wrapping_mul(0x9e3779b9)^repeat,ct,left:17,virtual_input:false};
                    if scalar{scalar_renorm(black_box(&mut d));}
                    else{black_box(&mut d).renorm();}
                    hash=hash.rotate_left(7)^u64::from(d.c)^u64::from(d.a)
                        ^((d.pos as u64)<<32)^((d.ct as u64)<<48)^((d.virtual_input as u64)<<56);
                }
            }
            (start.elapsed().as_nanos(),black_box(hash))
        }
        let streams:[&[u8];3]=[&[0,0x12,0x34,0x56],&[255,0x8f,0,255,0xac],&[255]];
        for (index,data) in streams.into_iter().enumerate(){
            let mut fast=Vec::new();let mut slow=Vec::new();
            for i in 0..9{
                let (x,y)=if i%2==0{let x=run(data,false);let y=run(data,true);(x,y)}
                    else{let y=run(data,true);let x=run(data,false);(x,y)};
                assert_eq!(x.1,y.1);
                if i>0{fast.push(x.0);slow.push(y.0);}
            }
            fast.sort_unstable();slow.sort_unstable();
            let f=(fast[3]+fast[4])/2;let s=(slow[3]+slow[4])/2;
            assert!(f>0&&s>0);
            println!("MQ paired stream {index}: batched_ns {f} scalar_ns {s} speedup {:.4} trials 8 intervals_per_trial 1048544",s as f64/f as f64);
        }
    }
    #[test]fn table_integrity(){
        assert_eq!(STATES.len(),47);
        for &(q,m,l,_) in &STATES{assert!(q>0&&q<=0x5601&&m<47&&l<47);}
        assert_eq!(STATES[46],(0x5601,46,46,false));
        assert_eq!(STATES.iter().filter(|s|s.3).count(),3);
    }
    #[test]fn context_and_work_limits(){
        assert!(Context::new(47,0).is_err());assert!(Context::new(0,2).is_err());
        assert!(Decoder::new(&[],1).is_err());assert!(Decoder::new(&[0],MAX_SYMBOLS+1).is_err());
        let mut d=Decoder::new(&[0,0],0).unwrap();let mut c=Context::new(0,0).unwrap();
        assert!(d.decode(&mut c).is_err());assert_eq!(c,Context::new(0,0).unwrap());
    }
    #[test]fn hand_derived_first_exchange(){
        let mut d=Decoder::new(&[0,0],1).unwrap();let mut c=Context::new(0,0).unwrap();
        assert_eq!(d.decode(&mut c).unwrap(),0);assert_eq!(c.state(),1);assert_eq!(c.mps(),0);
        let mut d=Decoder::new(&[0,0],1).unwrap();let mut c=Context::new(1,0).unwrap();
        assert_eq!(d.decode(&mut c).unwrap(),1);assert_eq!(c.state(),6);assert_eq!(c.mps(),0);
    }
    #[test]fn byte_in_stuffing_and_marker(){
        let d=Decoder::new(&[255,0x8f,0],1).unwrap();assert_eq!(d.bytes_consumed(),2);assert_eq!(d.ct,0);
        let d=Decoder::new(&[255,0x90,0],1).unwrap();assert_eq!(d.bytes_consumed(),1);assert_eq!(d.ct,1);
        let d=Decoder::new(&[255],1).unwrap();assert!(d.used_virtual_input());
    }
    #[test]fn bounded_repeated_symbols(){
        for data in [[0,0,0],[255,0x8f,0],[0x12,0x34,0x56]]{
            let mut d=Decoder::new(&data,1024).unwrap();let mut c=Context::new(0,0).unwrap();
            for _ in 0..1024{assert!(d.decode(&mut c).unwrap()<=1);assert!(d.a>=0x8000&&d.a<=0xffff);}
            assert!(d.decode(&mut c).is_err());assert!(d.bytes_consumed()<=data.len());
        }
    }

    // Offline reference fixtures: Debian OpenJPEG 2.5.3 static mqc.c.o.
    // resetstates; setstate(context=18,mps,state); init_enc with leading
    // zero sentinel; segmark_enc repeated 64 times; flush; numbytes.
    // Every segmentation symbol is 1010. Reference encoder is NOT linked
    // into RPCore or required by CI.
    #[test]fn independent_segmark_vectors(){
        let vectors=[
            (0,0,"b6c22000000000000000000000000000000000000000000000000000000000003f"),
            (0,1,"15804000000000000000000000000000000000000000000000000000000000007f"),
            (1,0,"2b008000000000000000000000000000000000000000000000000000000000007f"),
            (1,1,"74623000000000000000000000000000000000000000000000000000000000003f"),
            (6,0,"fca2f000000000000000000000000000000000000000000000000000000000003f"),
            (6,1,"0000000000000000000000000000000000000000000000000000000000000000"),
            (14,0,"fca2f000000000000000000000000000000000000000000000000000000000003f"),
            (14,1,"0000000000000000000000000000000000000000000000000000000000000000"),
            (45,0,"0000002401509315663b3c0d91864a800000000000000000000000000000000000000000000000000f"),
            (45,1,"0002001200a8498ab31d9e06c8c325400000000000000000000000000000000000000000000000000f"),
            (46,0,"ba577fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7f"),
            (46,1,"3955ff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7fff7f"),
        ];
        for ((state,mps,hex),expected_len) in vectors.into_iter().zip([33,33,33,33,33,32,33,32,41,41,35,34]){
            assert_eq!(hex.len()%2,0);
            let bytes:Vec<u8>=hex.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect();
            assert_eq!(bytes.len(),expected_len);
            let mut d=Decoder::new(&bytes,256).unwrap();
            let mut c=Context::new(state,mps).unwrap();
            for i in 0..256{
                assert_eq!(d.decode(&mut c).unwrap(),1-(i%2) as u8,"state {state}, mps {mps}, symbol {i}");
            }
            assert!(d.decode(&mut c).is_err());
        }
    }

    // Offline C reference encoder using OpenJPEG 2.5.3 state table,
    // byteout and flush; separate encoder decision equations.
    #[test]fn independent_multicontext_vectors(){
        let vectors=[
            "700f6d9ff6a1c6e35b52bf0ba1ee221c1b64394a9783834b8575b4febb62632f94498e4f4190dcf0aca780b69d0524e9bcf781bd5bd424cc59425da468b8af43044c4a273c",
            "3e56416ed4c80179f5e85455eae2bd3ad17b148d148c735d1e6916075e2c8a400b24416f663cabeafe6f9386dd08858d0e3081e9a428f927d07865928c3761a58fb5a6d864def265463d80a043218a822477",
            "0000c64b27bc050000850011a8e70011557a888020ac020a0d33181f68d86a8991a21b7ada39bf44165ef4b8045d0aa9b07bfcc2fd6b84ca5cd916cb47e29b6c60003faca10bd80c2205eb4eb4c0320fb423918e489ed2429b3ffe4f4badc7b0d072820d274793ad2a952ee46f",
            "01fe2492d873dec55183ef6af37b260341bd2324a8b73e6d75d67be003778a4d100a3924badb63d7fbb4c2917c1fe47b0f996cd1e8d0e18198333a4662af0c9b1cc4616fb16a7f",
            "00000004e0808a1000004eb582281acaab00f6e7f640a2106d6c66327121048110404a00ac111c7199c200000024d94b91e0be7952d52c4548017079f748f8148021300a00f58bc70de10c518a9a6ad405afb635293834a355b16c841f0959593bc2f2e9111ef22a1e1bb70e997b312d484e9cf50b",
        ];
        for (g,(hex,len)) in vectors.into_iter().zip([69,82,109,71,117]).enumerate(){
            assert_eq!(hex.len(),len*2);
            let bytes:Vec<u8>=hex.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect();
            let mut contexts:Vec<_>=(0..19).map(|k|Context::new(((g*19+k)%47) as u8,((g+k)%2) as u8).unwrap()).collect();
            let mut d=Decoder::new(&bytes,512).unwrap();
            let mut rng=0x12345678u32+g as u32;
            for i in 0..512{
                rng=rng.wrapping_mul(1664525).wrapping_add(1013904223);
                let k=((rng>>16)%19) as usize;
                assert_eq!(d.decode(&mut contexts[k]).unwrap(),(rng>>31) as u8,"group {g}, symbol {i}, context {k}");
            }
            assert!(d.decode(&mut contexts[0]).is_err());
        }
    }
}
