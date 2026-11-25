//! FIPS 180-4 SHA-256/384/512, private bounded one-shot digests.
//! Fixed-size working storage; no external runtime dependencies.
use crate::{Error,Result,MAX_BYTES};

const K256:[u32;64]=[
0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
const K512:[u64;80]=[
0x428a2f98d728ae22,0x7137449123ef65cd,0xb5c0fbcfec4d3b2f,0xe9b5dba58189dbbc,
0x3956c25bf348b538,0x59f111f1b605d019,0x923f82a4af194f9b,0xab1c5ed5da6d8118,
0xd807aa98a3030242,0x12835b0145706fbe,0x243185be4ee4b28c,0x550c7dc3d5ffb4e2,
0x72be5d74f27b896f,0x80deb1fe3b1696b1,0x9bdc06a725c71235,0xc19bf174cf692694,
0xe49b69c19ef14ad2,0xefbe4786384f25e3,0x0fc19dc68b8cd5b5,0x240ca1cc77ac9c65,
0x2de92c6f592b0275,0x4a7484aa6ea6e483,0x5cb0a9dcbd41fbd4,0x76f988da831153b5,
0x983e5152ee66dfab,0xa831c66d2db43210,0xb00327c898fb213f,0xbf597fc7beef0ee4,
0xc6e00bf33da88fc2,0xd5a79147930aa725,0x06ca6351e003826f,0x142929670a0e6e70,
0x27b70a8546d22ffc,0x2e1b21385c26c926,0x4d2c6dfc5ac42aed,0x53380d139d95b3df,
0x650a73548baf63de,0x766a0abb3c77b2a8,0x81c2c92e47edaee6,0x92722c851482353b,
0xa2bfe8a14cf10364,0xa81a664bbc423001,0xc24b8b70d0f89791,0xc76c51a30654be30,
0xd192e819d6ef5218,0xd69906245565a910,0xf40e35855771202a,0x106aa07032bbd1b8,
0x19a4c116b8d2d0c8,0x1e376c085141ab53,0x2748774cdf8eeb99,0x34b0bcb5e19b48a8,
0x391c0cb3c5c95a63,0x4ed8aa4ae3418acb,0x5b9cca4f7763e373,0x682e6ff3d6b2b8a3,
0x748f82ee5defb2fc,0x78a5636f43172f60,0x84c87814a1f0ab72,0x8cc702081a6439ec,
0x90befffa23631e28,0xa4506cebde82bde9,0xbef9a3f7b2c67915,0xc67178f2e372532b,
0xca273eceea26619c,0xd186b8c721c0c207,0xeada7dd6cde0eb1e,0xf57d4f7fee6ed178,
0x06f067aa72176fba,0x0a637dc5a2c898a6,0x113f9804bef90dae,0x1b710b35131c471b,
0x28db77f523047d84,0x32caab7b40c72493,0x3c9ebe0a15c9bebc,0x431d67c49c100d4c,
0x4cc5d4becb3e42b6,0x597f299cfc657e2a,0x5fcb6fab3ad6faec,0x6c44198c4a475817];

macro_rules! compression{
    ($fn:ident,$t:ty,$bytes:expr,$rounds:expr,$k:ident,$a:expr,$b:expr,$c:expr,$d:expr)=>{
        fn $fn(state:&mut [$t;8],block:&[u8]){
            let mut w=[0 as $t;$rounds];
            for (word,src) in w.iter_mut().zip(block.chunks_exact($bytes)){
                let mut b=[0u8;$bytes];b.copy_from_slice(src);*word=<$t>::from_be_bytes(b);
            }
            for i in 16..$rounds{
                let x=w[i-15];let y=w[i-2];
                let s0=x.rotate_right($a.0)^x.rotate_right($a.1)^(x>>$a.2);
                let s1=y.rotate_right($b.0)^y.rotate_right($b.1)^(y>>$b.2);
                w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
            }
            let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h]=*state;
            for i in 0..$rounds{
                let s1=e.rotate_right($d.0)^e.rotate_right($d.1)^e.rotate_right($d.2);
                let ch=(e&f)^(!e&g);
                let t1=h.wrapping_add(s1).wrapping_add(ch).wrapping_add($k[i]).wrapping_add(w[i]);
                let s0=a.rotate_right($c.0)^a.rotate_right($c.1)^a.rotate_right($c.2);
                let maj=(a&b)^(a&c)^(b&c);let t2=s0.wrapping_add(maj);
                h=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2);
            }
            for (x,y) in state.iter_mut().zip([a,b,c,d,e,f,g,h]){*x=x.wrapping_add(y);}
        }
    }
}
compression!(block256,u32,4,64,K256,(7,18,3),(17,19,10),(2,13,22),(6,11,25));
compression!(block512,u64,8,80,K512,(1,8,7),(19,61,6),(28,34,39),(14,18,41));

pub(super) fn sha256(data:&[u8])->Result<[u8;32]>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("SHA-2 input bytes"));}
    let mut s=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    let mut chunks=data.chunks_exact(64);
    for block in &mut chunks{block256(&mut s,block);}
    let rem=chunks.remainder();let mut tail=[0;64];
    tail[..rem.len()].copy_from_slice(rem);tail[rem.len()]=0x80;
    if rem.len()>=56{block256(&mut s,&tail);tail.fill(0);}
    tail[56..].copy_from_slice(&((data.len() as u64)*8).to_be_bytes());block256(&mut s,&tail);
    let mut out=[0;32];for (b,x) in out.chunks_exact_mut(4).zip(s){b.copy_from_slice(&x.to_be_bytes());}Ok(out)
}
fn wide(data:&[u8],mut s:[u64;8])->Result<[u8;64]>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("SHA-2 input bytes"));}
    let mut chunks=data.chunks_exact(128);
    for block in &mut chunks{block512(&mut s,block);}
    let rem=chunks.remainder();let mut tail=[0;128];
    tail[..rem.len()].copy_from_slice(rem);tail[rem.len()]=0x80;
    if rem.len()>=112{block512(&mut s,&tail);tail.fill(0);}
    tail[112..].copy_from_slice(&((data.len() as u128)*8).to_be_bytes());block512(&mut s,&tail);
    let mut out=[0;64];for (b,x) in out.chunks_exact_mut(8).zip(s){b.copy_from_slice(&x.to_be_bytes());}Ok(out)
}
pub(super) fn sha512(data:&[u8])->Result<[u8;64]>{
    wide(data,[0x6a09e667f3bcc908,0xbb67ae8584caa73b,0x3c6ef372fe94f82b,0xa54ff53a5f1d36f1,0x510e527fade682d1,0x9b05688c2b3e6c1f,0x1f83d9abfb41bd6b,0x5be0cd19137e2179])
}
pub(super) fn sha384(data:&[u8])->Result<[u8;48]>{
    let full=wide(data,[0xcbbb9d5dc1059ed8,0x629a292a367cd507,0x9159015a3070dd17,0x152fecd8f70e5939,0x67332667ffc00b31,0x8eb44a8768581511,0xdb0c2e0d64f98fa7,0x47b5481dbefa4fa4])?;
    let mut out=[0;48];out.copy_from_slice(&full[..48]);Ok(out)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(b:&[u8])->String{b.iter().map(|v|format!("{v:02x}")).collect::<Vec<_>>().join("")}
    #[test]fn known_abc(){
        assert_eq!(hex(&sha256(b"abc").unwrap()),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hex(&sha384(b"abc").unwrap()),"cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7");
        assert_eq!(hex(&sha512(b"abc").unwrap()),"ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    }
    #[test]fn independent_binary_boundaries(){
        // Independently generated Python hashlib vectors, bytes (i*37+11)%256.
        for (n,a,b,c) in [
(0,"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b","cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"),
(1,"e7cf46a078fed4fafd0b5e3aff144802b853f8ae459a4f0c14add3314b7cc3a6","2deb5d512a82fb61b846799f41367b0172a8a82a4b1539fa9f3fd148bdd1db7eb053b452eb916523186cbbe41e9f2fbb","527cff2b6fdfbc0f54fe092b17d6d8c7e22500242635fa56981e85a64da6ce8a12a3a66cf69fd48f588bcba9bad141b8e351a0cdd4925ae57289933eec1fc153"),
(55,"2900465fcb533e05a158fd2b3be0e5e3b03740d83060aa3580e0d98a96bf2384","5af70c3661eb084de491e3a9cc1d584c28c2e752933aa13f047f7d2cd5285607b9d9750373e59fc0b5ff84ccf9099738","2fd061a12ae849786141611e02a18ad9dad5d1cf6980f67fd4021fe369e4fef874548e9e5146e53622a919b3dcbd652f9abdd9578c4bd0c8e28d38008ea922b5"),
(56,"31454ff48ef36af2f08fd511bdc37d9d5855ac23e992e5ff5445cb6b7674a674","c1a6736be1fcc8891a626b08a10097fbf2fcce3acd19ec22154761c485f874ac8ca986a1c213b790ebb57adeb0008b52","78ade6b0c755effd2b093d02bbeb8354d57c3e0c36457f941b47e2b384bc0a279c4cdaae1f9968e47bfa1e94438c33af1253067677087890f35f944a6fb8693e"),
(63,"5f6401b96532c36de4e65beec0409b69b1d181864c8009b7a04f43e5d56350d1","a4664ae68697964c5f1a1e4dde77f0237dab9e9ea7a6e5ceb2e50e984cdfad14bc55fd1c8830a5acff820e564a3c6a34","853d2f48b4e8c74aa7c547ad831787b4f2bb85aa2851241a7fd1a38f651c2eb5816990092a4cd336d86dba73f0191f69a8825c7f5f2991bdcc2fb9661af22f46"),
(64,"94eb5de4943613fd048dc93393ab06877405faa39c11f53e9386083339833e7e","13c3b81071b4ff54bcb80912eeb97bfadc12c29df001f83defaf970a4e43b104aeda6a6e6fa71d0d2cc6c745f2952406","be7cccdacc7551d40828f4f3568f741a224ec5e5274ca547e06ad5dc8313456706033259aa46937a061bc30f9913597e0b591c6492da5334e9148bbb91ce805b"),
(111,"aeca4f5a02aead30b044df2ce00b97f2f4874c25352eaa1916a1831819dcc3dd","d6517d210fc664af0e137a4ca19d7a299465109f40cc0fa9ac7f9d4ac9c77e6687f3cb060890ef7e96f9f5252dc528ab","4d1db900250c96436052fbca79c13acbf378aad9c35b87d94c3803264df61fd22cbd327c8938d024db372abf4208934ee09367d571d6c670bf74ee07b83e7506"),
(112,"cec7a189fcea0a38f025f11208b1bd2b12ec291ff00ec5448a325c5c8db0c7ac","76078618f7c580d9ca1f9990c2adb38a899f7feda86153dda507b72f1bbafb046ab835aa8d325a89a67e3deb81ad1ad5","dfb715ca3478a894302ace39c42d1d6646e1044f2247a6274d8b42d155d2fdbe7017195e85cfba96bedc51f84c44638978a540039ff09c64cef6c0c5ccc8f7b6"),
(127,"0fe729ff19257bd6fec853acc2ea355f6b34b58e6c0f684c3e188fcdfcd9baae","17ed2932ea5246073ae8240a06ec458b15bfd72d792520d0c319b84f97532d408fb7d2c39cd3710165914994f44a70f8","f93a0e7465b294188e8aa2b1cc2e98bc8d5115d46f51c7a9ec599b9d9f96a80fef6a4f226b648c89bd9eac23b3d64264898b568d915c66666c44cd0319e2ef56"),
(128,"0aedd4856f8eba0963627336ad5144a9a7dbe12498e6066f0165fc97d8ddee4c","13274b7eaabb6a3203608240e792e825ddb816853a66fde047b368cc7638d61b9f215323c4160677301ae4d178cd8579","0b4815d35f9d07b1a30de2790e1be2a720234295cd7b4d9e9af51719ff90019f1fe6d4e402a7dcc4177085023dc460ab743dad9b2c1dda42662bda5d3b2e155b"),
(129,"4f1757ae4bffbae86d775b831765b75af154d52f7deaa46dd378051a2d3ad57f","f07f92376b47fb9d34ff613a44f18b04b55fb1c480b07de4e08a0e9bd484ffa765c21671c341c70b4024e6bdcda709e4","1809db04d02717483e04bc4333a14308bd2d0213ba7bf2c63f11eb1b8a0af8252e67fd104fd466fb95f945539824d8e4183155fa5ced0bee3dad46d9384a0bd5"),
(4097,"042a02b55ba342cdd30322331816b6e4bca073f33bdb80149dc74a6f949802f4","c45d967208b168f458a7b4d09e07627e4d2aaabe7c19fe6a3c972a4514d0d1f8694731a3c58440aa13a6424136373bb1","96681ccb8c621cd60d4551ccbcc61683b5cdb15db79acfaff68daa0d02b93bb29a80295574a3b082110d949456c9b73c1c3f0156b8211446ca6d333a96821138"),
        ]{
            let data:Vec<u8>=(0..n).map(|i|((i*37+11)%256)as u8).collect();
            assert_eq!(hex(&sha256(&data).unwrap()),a,"SHA256 {n}");
            assert_eq!(hex(&sha384(&data).unwrap()),b,"SHA384 {n}");
            assert_eq!(hex(&sha512(&data).unwrap()),c,"SHA512 {n}");
        }
    }
}
