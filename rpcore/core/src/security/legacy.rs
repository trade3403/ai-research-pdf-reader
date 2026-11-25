//! Legacy PDF compatibility only: RFC 1321 MD5 and RC4.
//! No use for new encryption, signing, password storage or integrity checks.
//! No external crypto linkage and no unsafe code. Not a side-channel audited
//! implementation; temporary state is not guaranteed to be securely erased.
use crate::{Error,Result,MAX_BYTES};

const K:[u32;64]=[
    0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
    0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
    0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
    0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
    0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
    0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
    0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
    0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391,
];
const SHIFTS:[[u32;4];4]=[[7,12,17,22],[5,9,14,20],[4,11,16,23],[6,10,15,21]];

fn block(state:&mut [u32;4],bytes:&[u8;64]){
    let mut words=[0u32;16];
    for (word,b) in words.iter_mut().zip(bytes.chunks_exact(4)){
        *word=u32::from_le_bytes([b[0],b[1],b[2],b[3]]);
    }
    let [mut a,mut b,mut c,mut d]=*state;
    for i in 0..64{
        let (f,g)=match i/16{
            0=>((b&c)|(!b&d),i),
            1=>((d&b)|(!d&c),(5*i+1)%16),
            2=>(b^c^d,(3*i+5)%16),
            _=>(c^(b|!d),(7*i)%16),
        };
        let next=b.wrapping_add(a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(words[g]).rotate_left(SHIFTS[i/16][i%4]));
        a=d;d=c;c=b;b=next;
    }
    for (x,y) in state.iter_mut().zip([a,b,c,d]){*x=x.wrapping_add(y);}
}
/// Fixed-size working state, bounded input, little-endian RFC 1321 digest.
pub(super) fn md5(data:&[u8])->Result<[u8;16]>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("MD5 input bytes"));}
    let mut state=[0x67452301u32,0xefcdab89,0x98badcfe,0x10325476];
    let mut chunks=data.chunks_exact(64);
    for chunk in &mut chunks{
        // chunks_exact guarantees 64 bytes.
        let b:&[u8;64]=chunk.try_into().map_err(|_|Error::Invalid("MD5 block size"))?;
        block(&mut state,b);
    }
    let rem=chunks.remainder();
    let mut tail=[0u8;64];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()]=0x80;
    if rem.len()>=56{block(&mut state,&tail);tail.fill(0);}
    tail[56..].copy_from_slice(&((data.len() as u64)*8).to_le_bytes());
    block(&mut state,&tail);
    let mut out=[0u8;16];
    for (dest,v) in out.chunks_exact_mut(4).zip(state){dest.copy_from_slice(&v.to_le_bytes());}
    Ok(out)
}

/// One state per PDF string/stream (never reuse across objects). In-place
/// XOR; subsequent calls continue the same stream, with a cumulative limit.
/// Only PDF-compatible 5..=16-byte keys. No Debug or Clone of key state.
pub(super) struct Rc4{state:[u8;256],i:u8,j:u8,processed:usize}
impl Rc4{
    pub(super) fn new(key:&[u8])->Result<Self>{
        if !(5..=16).contains(&key.len()){return Err(Error::Invalid("RC4 PDF key length"));}
        let mut state=std::array::from_fn(|i|i as u8);
        let mut j=0u8;
        for i in 0..256{
            j=j.wrapping_add(state[i]).wrapping_add(key[i%key.len()]);
            state.swap(i,j as usize);
        }
        Ok(Self{state,i:0,j:0,processed:0})
    }
    pub(super) fn apply(&mut self,data:&mut [u8])->Result<()>{
        let next=self.processed.checked_add(data.len()).ok_or(Error::Limit("RC4 input bytes"))?;
        if next>MAX_BYTES{return Err(Error::Limit("RC4 input bytes"));}
        for b in data{
            self.i=self.i.wrapping_add(1);
            self.j=self.j.wrapping_add(self.state[self.i as usize]);
            self.state.swap(self.i as usize,self.j as usize);
            let k=self.state[self.i as usize].wrapping_add(self.state[self.j as usize]);
            *b^=self.state[k as usize];
        }
        self.processed=next;Ok(())
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join("")}
    #[test]fn md5_rfc1321(){
        for (input,want) in [
            ("","d41d8cd98f00b204e9800998ecf8427e"),
            ("a","0cc175b9c0f1b6a831c399e269772661"),
            ("abc","900150983cd24fb0d6963f7d28e17f72"),
            ("message digest","f96b697d7cb7938d525a2f31aaf161d0"),
            ("abcdefghijklmnopqrstuvwxyz","c3fcd3d76192e4007dfb496cca67e13b"),
            ("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789","d174ab98d277d9f5a5611c2c9f419d9f"),
            ("12345678901234567890123456789012345678901234567890123456789012345678901234567890","57edf4a22be3c955ac49da2e2107b67a"),
        ]{assert_eq!(hex(&md5(input.as_bytes()).unwrap()),want);}
    }
    #[test]fn md5_binary_padding_boundaries(){
        // Frozen expected results generated independently with Python hashlib.md5.
        for (n,want) in [
            (0,"d41d8cd98f00b204e9800998ecf8427e"),(1,"13c8ffd977013703a701cf8e11deac65"),
            (55,"d872aa0473a24da995ce4ac518ade767"),(56,"e23567645846677c205de80f9779081b"),
            (57,"1fe5e4a27e12484d7203d0b60c21c51f"),(63,"4775b66278a8fc132ff80923378216cd"),
            (64,"71e123b70c7aa64826fcfe472694cd1c"),(65,"5949948f26e35203661075214faa3966"),
            (119,"87f72c7241c5fe218bb4df8c0aba3232"),(120,"bf240b8b7407fba3805adeec63a03b77"),
            (121,"c51004ed1ef4271c82106468a13fe8cf"),(127,"520bdc2dbab7c25d64263ffb242d9e98"),
            (128,"3e93b378458b77da96b2357c3bda8cc2"),(129,"d1ae06bbf9128955a34bedb6231eee62"),
            (1024,"d4883a545d61557ce12ff4e95350e997"),(4097,"82b1834165b5032bacc5f34d594f6a36"),
        ]{
            let bytes:Vec<u8>=(0..n).map(|i|((i*37+11)%256)as u8).collect();
            assert_eq!(hex(&md5(&bytes).unwrap()),want,"length {n}");
        }
    }
    #[test]fn rc4_rfc6229_40bit(){
        let mut data=vec![0;4112];Rc4::new(&[1,2,3,4,5]).unwrap().apply(&mut data).unwrap();
        for (off,want) in [
            (0,"b2396305f03dc027ccc3524a0a1118a8"),(256,"1cfcf62b03eddb641d77dfcf7f8d8c93"),
            (512,"6459844432a7da923cfb3eb4980661f6"),(768,"eb62638d4f0ba1fe9fca20e05bf8ff2b"),
            (1024,"30abbcc7c20b01609f23ee2d5f6bb7df"),(1536,"d8729db41882259bee4f825325f5a130"),
            (2048,"cc582f8ba9f265e2b1be9112e975d2d7"),(3072,"ec0e11c479dc329dc8da7968fe965681"),
            (4096,"ff25b58995996707e51fbdf08b34d875"),
        ]{assert_eq!(hex(&data[off..off+16]),want);}
    }
    #[test]fn rc4_rfc6229_128bit(){
        let key:Vec<u8>=(1..=16).collect();let mut data=vec![0;4112];Rc4::new(&key).unwrap().apply(&mut data).unwrap();
        for (off,want) in [
            (0,"9ac7cc9a609d1ef7b2932899cde41b97"),(256,"d39d566bc6bce3010768151549f3873f"),
            (512,"c68c1d5c559a974123df1dbc52a43b89"),(768,"eccbe13de1fcc91c11a0b26c0bc8fa4d"),
            (1024,"bdf0324e6083dcc6d3cedd3ca8c53c16"),(1536,"ffa0b514647ec04f6306b892ae661181"),
            (2048,"8a44126411eaa78bd51e8d87a8879bf5"),(3072,"c05d88abd50357f935a63c59ee537623"),
            (4096,"a36a4c301ae8ac13610ccbc12256cacc"),
        ]{assert_eq!(hex(&data[off..off+16]),want);}
    }
    #[test]fn rc4_chunking_and_roundtrip(){
        let plain:Vec<u8>=(0..8193).map(|i|((i*37+11)%256)as u8).collect();
        for n in 5..=16{
            let key=vec![0x82;n];let mut single=plain.clone();let mut split=plain.clone();
            Rc4::new(&key).unwrap().apply(&mut single).unwrap();
            let mut rc=Rc4::new(&key).unwrap();
            for chunk in split.chunks_mut(127){rc.apply(chunk).unwrap();rc.apply(&mut []).unwrap();}
            assert_eq!(single,split);
            Rc4::new(&key).unwrap().apply(&mut split).unwrap();assert_eq!(split,plain);
        }
    }
    #[test]fn rc4_rejects_bad_key_and_limit_before_mutation(){
        for n in [0,1,4,17,256]{assert!(Rc4::new(&vec![0;n]).is_err());}
        let mut rc=Rc4::new(&[0;5]).unwrap();rc.processed=MAX_BYTES;
        let mut data=[0x5a];assert!(matches!(rc.apply(&mut data),Err(Error::Limit("RC4 input bytes"))));
        assert_eq!(data,[0x5a]);assert_eq!((rc.i,rc.j,rc.processed),(0,0,MAX_BYTES));
    }
}
