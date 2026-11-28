//! Private AES-128/AES-256 block and CBC primitives for PDF compatibility.
//! FIPS 197 and NIST SP 800-38A. No encryption writer, public crypto API,
//! external linkage, or unsafe code. Table lookups are not constant-time;
//! key zeroization and side-channel resistance have not been audited.
use crate::{Error,Result,MAX_BYTES};

const fn mul(mut a:u8,mut b:u8)->u8{
    let mut out=0u8;
    let mut i=0;
    while i<8{
        if b&1!=0{out^=a;}
        let high=a&0x80!=0;
        a<<=1;
        if high{a^=0x1b;}
        b>>=1;i+=1;
    }
    out
}
const fn inverse(x:u8)->u8{
    // Multiplicative inverse x^254 in GF(2^8); zero maps to zero.
    let mut y=1;let mut a=x;let mut n=254u8;
    while n!=0{if n&1!=0{y=mul(y,a);}a=mul(a,a);n>>=1;}
    y
}
const fn tables()->([u8;256],[u8;256]){
    let mut s=[0;256];let mut inv=[0;256];let mut i=0;
    while i<256{
        let x=inverse(i as u8);
        let y=x^x.rotate_left(1)^x.rotate_left(2)^x.rotate_left(3)^x.rotate_left(4)^0x63;
        s[i]=y;inv[y as usize]=i as u8;i+=1;
    }
    (s,inv)
}
const TABLES:([u8;256],[u8;256])=tables();
const S:[u8;256]=TABLES.0;
const SI:[u8;256]=TABLES.1;

pub(super) struct Aes128{keys:[[u8;16];11]}
impl Aes128{
    pub(super) fn new(key:&[u8])->Result<Self>{
        let key:[u8;16]=key.try_into().map_err(|_|Error::Invalid("AES-128 key length"))?;
        let mut keys=[[0;16];11];keys[0]=key;
        let mut rc=1u8;
        for i in 1..=10{
            let prev=keys[i-1];
            let t=[S[prev[13] as usize]^rc,S[prev[14] as usize],S[prev[15] as usize],S[prev[12] as usize]];
            for j in 0..4{keys[i][j]=prev[j]^t[j];}
            for j in 4..16{keys[i][j]=prev[j]^keys[i][j-4];}
            rc=mul(rc,2);
        }
        Ok(Self{keys})
    }
    pub(super) fn encrypt(&self,mut b:[u8;16])->[u8;16]{
        add_key(&mut b,&self.keys[0]);
        for r in 1..=10{
            sub(&mut b,&S);shift(&mut b,false);
            if r!=10{mix(&mut b,false);}
            add_key(&mut b,&self.keys[r]);
        }
        b
    }
    pub(super) fn decrypt(&self,mut b:[u8;16])->[u8;16]{
        add_key(&mut b,&self.keys[10]);
        for r in (0..10).rev(){
            shift(&mut b,true);sub(&mut b,&SI);
            add_key(&mut b,&self.keys[r]);
            if r!=0{mix(&mut b,true);}
        }
        b
    }
    // Raw CBC encryption for the R6 password hash, no padding.
    pub(super) fn cbc_encrypt(&self,mut iv:[u8;16],data:&mut [u8])->Result<()>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("AES CBC bytes"));}
        if data.len()%16!=0{return Err(Error::Invalid("AES CBC block length"));}
        for block in data.chunks_exact_mut(16){
            for i in 0..16{iv[i]^=block[i];}
            iv=self.encrypt(iv);block.copy_from_slice(&iv);
        }
        Ok(())
    }
    // Raw CBC, no padding, caller-provided IV. Length checked before mutation.
    pub(super) fn cbc_decrypt(&self,mut iv:[u8;16],data:&mut [u8])->Result<()>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("AES CBC bytes"));}
        if data.len()%16!=0{return Err(Error::Invalid("AES CBC block length"));}
        for block in data.chunks_exact_mut(16){
            let mut ct=[0;16];ct.copy_from_slice(block);
            let pt=self.decrypt(ct);
            for i in 0..16{block[i]=pt[i]^iv[i];}
            iv=ct;
        }
        Ok(())
    }
    /// PDF AESV2 representation: 16-byte IV followed by at least one
    /// CBC block and mandatory PKCS#7 padding. Return nothing on bad padding.
    pub(super) fn pdf_decrypt(&self,data:&[u8])->Result<Vec<u8>>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("AES PDF bytes"));}
        if data.len()<32||data.len()%16!=0{return Err(Error::Invalid("AES PDF ciphertext length"));}
        let mut iv=[0;16];iv.copy_from_slice(&data[..16]);
        let mut out=data[16..].to_vec();self.cbc_decrypt(iv,&mut out)?;
        let n=usize::from(*out.last().ok_or(Error::Invalid("AES PDF padding"))?);
        if !(1..=16).contains(&n)||n>out.len(){return Err(Error::Invalid("AES PDF padding"));}
        let end=out.len()-n;
        if out[end..].iter().fold(0u8,|a,b|a|(*b^n as u8))!=0{
            return Err(Error::Invalid("AES PDF padding"));
        }
        out.truncate(end);Ok(out)
    }
}
pub(super) struct Aes256{keys:[[u8;16];15]}
impl Aes256{
    pub(super) fn new(key:&[u8])->Result<Self>{
        if key.len()!=32{return Err(Error::Invalid("AES-256 key length"));}
        let mut w=[[0u8;4];60];
        for (dst,src) in w[..8].iter_mut().zip(key.chunks_exact(4)){dst.copy_from_slice(src);}
        let mut rc=1;
        for i in 8..60{
            let mut t=w[i-1];
            if i%8==0{t.rotate_left(1);for b in &mut t{*b=S[*b as usize];}t[0]^=rc;rc=mul(rc,2);}
            else if i%8==4{for b in &mut t{*b=S[*b as usize];}}
            for j in 0..4{w[i][j]=w[i-8][j]^t[j];}
        }
        let mut keys=[[0;16];15];
        for (dst,src) in keys.iter_mut().zip(w.chunks_exact(4)){
            for (word,bytes) in dst.chunks_exact_mut(4).zip(src){word.copy_from_slice(bytes);}
        }
        Ok(Self{keys})
    }
    pub(super) fn encrypt(&self,mut b:[u8;16])->[u8;16]{
        add_key(&mut b,&self.keys[0]);
        for r in 1..=14{
            sub(&mut b,&S);shift(&mut b,false);
            if r!=14{mix(&mut b,false);}
            add_key(&mut b,&self.keys[r]);
        }
        b
    }
    pub(super) fn decrypt(&self,mut b:[u8;16])->[u8;16]{
        add_key(&mut b,&self.keys[14]);
        for r in (0..14).rev(){
            shift(&mut b,true);sub(&mut b,&SI);add_key(&mut b,&self.keys[r]);
            if r!=0{mix(&mut b,true);}
        }
        b
    }
    pub(super) fn cbc_decrypt(&self,mut iv:[u8;16],data:&mut [u8])->Result<()>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("AES CBC bytes"));}
        if data.len()%16!=0{return Err(Error::Invalid("AES CBC block length"));}
        for block in data.chunks_exact_mut(16){
            let mut ct=[0;16];ct.copy_from_slice(block);
            let pt=self.decrypt(ct);
            for i in 0..16{block[i]=pt[i]^iv[i];}
            iv=ct;
        }
        Ok(())
    }
    pub(super) fn pdf_decrypt(&self,data:&[u8])->Result<Vec<u8>>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("AES PDF bytes"));}
        if data.len()<32||data.len()%16!=0{return Err(Error::Invalid("AES PDF ciphertext length"));}
        let mut iv=[0;16];iv.copy_from_slice(&data[..16]);
        let mut out=data[16..].to_vec();self.cbc_decrypt(iv,&mut out)?;
        let n=usize::from(*out.last().ok_or(Error::Invalid("AES PDF padding"))?);
        if !(1..=16).contains(&n)||n>out.len(){return Err(Error::Invalid("AES PDF padding"));}
        let end=out.len()-n;
        if out[end..].iter().fold(0u8,|a,b|a|(*b^n as u8))!=0{return Err(Error::Invalid("AES PDF padding"));}
        out.truncate(end);Ok(out)
    }
}
fn add_key(b:&mut [u8;16],k:&[u8;16]){
    for (a,c) in b.iter_mut().zip(k){*a^=*c;}
}
fn sub(b:&mut [u8;16],table:&[u8;256]){
    for a in b{*a=table[*a as usize];}
}
fn shift(b:&mut [u8;16],inv:bool){
    // FIPS 197 column-major state: index 4*column+row.
    let src=*b;
    for row in 0..4{for col in 0..4{
        let c=if inv{(col+4-row)%4}else{(col+row)%4};
        b[4*col+row]=src[4*c+row];
    }}
}
fn mix(b:&mut [u8;16],inv:bool){
    for col in b.chunks_exact_mut(4){
        let a=[col[0],col[1],col[2],col[3]];
        let k=if inv{[14,11,13,9]}else{[2,3,1,1]};
        for r in 0..4{
            col[r]=mul(a[r],k[0])^mul(a[(r+1)%4],k[1])^mul(a[(r+2)%4],k[2])^mul(a[(r+3)%4],k[3]);
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn block(s:&str)->[u8;16]{hex(s).try_into().unwrap()}
    #[test]fn finite_field_and_sbox(){
        assert_eq!(mul(0x57,0x13),0xfe);assert_eq!(S[0],0x63);assert_eq!(S[0x53],0xed);
        for i in 0..256{assert_eq!(SI[S[i] as usize],i as u8);}
        for i in 1..256{assert_eq!(mul(i as u8,inverse(i as u8)),1);}
    }
    #[test]fn fips197_key_schedule(){
        let a=Aes128::new(&hex("000102030405060708090a0b0c0d0e0f")).unwrap();
        assert_eq!(a.keys[1],block("d6aa74fdd2af72fadaa678f1d6ab76fe"));
        assert_eq!(a.keys[10],block("13111d7fe3944a17f307a78b4d2b30c5"));
    }
    #[test]fn fips197_block(){
        let a=Aes128::new(&hex("000102030405060708090a0b0c0d0e0f")).unwrap();
        let p=block("00112233445566778899aabbccddeeff");let c=block("69c4e0d86a7b0430d8cdb78070b4c55a");
        assert_eq!(a.encrypt(p),c);assert_eq!(a.decrypt(c),p);
        let zero=Aes128::new(&[0;16]).unwrap();
        let expected=block("66e94bd4ef8a2c3b884cfa59ca342b2e");
        assert_eq!(zero.encrypt([0;16]),expected);assert_eq!(zero.decrypt(expected),[0;16]);
    }
    #[test]fn nist_sp800_38a_ecb(){
        let a=Aes128::new(&hex("2b7e151628aed2a6abf7158809cf4f3c")).unwrap();
        for (p,c) in [
            ("6bc1bee22e409f96e93d7e117393172a","3ad77bb40d7a3660a89ecaf32466ef97"),
            ("ae2d8a571e03ac9c9eb76fac45af8e51","f5d3d58503b9699de785895a96fdbaaf"),
            ("30c81c46a35ce411e5fbc1191a0a52ef","43b1cd7f598ece23881b00e3ed030688"),
            ("f69f2445df4f9b17ad2b417be66c3710","7b0c785e27e8ad3f8223207104725dd4"),
        ]{assert_eq!(a.encrypt(block(p)),block(c));assert_eq!(a.decrypt(block(c)),block(p));}
    }
    #[test]fn nist_sp800_38a_cbc(){
        let a=Aes128::new(&hex("2b7e151628aed2a6abf7158809cf4f3c")).unwrap();
        let mut data=hex(concat!("7649abac8119b246cee98e9b12e9197d","5086cb9b507219ee95db113a917678b2","73bed6b8e3c1743b7116e69e22229516","3ff1caa1681fac09120eca307586e1a7"));
        a.cbc_decrypt(block("000102030405060708090a0b0c0d0e0f"),&mut data).unwrap();
        assert_eq!(data,hex(concat!("6bc1bee22e409f96e93d7e117393172a","ae2d8a571e03ac9c9eb76fac45af8e51","30c81c46a35ce411e5fbc1191a0a52ef","f69f2445df4f9b17ad2b417be66c3710")));
    }
    #[test]fn independent_pdf_padding_boundaries(){
        // Generated with Python cryptography AES/CBC and PKCS7(128),
        // key bytes 0..15, IV bytes 16..31, binary plaintext i*37+11.
        let a=Aes128::new(&hex("000102030405060708090a0b0c0d0e0f")).unwrap();
        for (n,ct) in [
            (0,"0a940bb5416ef045f1c39458c653ea5a"),
            (1,"e20a549b9f3892124fe7cd2a10392b7e"),
            (15,"0bc822e3cd95e8e5077df9446d9a4fb9"),
            (16,"f13561fd5604d1935c9c69da7d196345860a4c9aa3897ae2a079075064a8f522"),
            (17,"f13561fd5604d1935c9c69da7d196345d0929c14f37416c7ffc3587f9620287c"),
            (31,"f13561fd5604d1935c9c69da7d196345d5db1e55055b338c46b724f4843c7ba2"),
            (32,"f13561fd5604d1935c9c69da7d19634537d7d7b466a34e0f820069a784714da3dbd8cca04a14730f962d9819c6f0d919"),
            (33,"f13561fd5604d1935c9c69da7d19634537d7d7b466a34e0f820069a784714da3b6738a2f8ffeb75c5f215d915669a157"),
        ]{
            let mut data=hex("101112131415161718191a1b1c1d1e1f");data.extend(hex(ct));
            let expected:Vec<u8>=(0..n).map(|i|((i*37+11)%256) as u8).collect();
            assert_eq!(a.pdf_decrypt(&data).unwrap(),expected,"length {n}");
        }
    }
    #[test]fn malformed_key_length_and_padding(){
        for n in [0,15,17,24,32]{assert!(Aes128::new(&vec![0;n]).is_err());}
        let a=Aes128::new(&[0;16]).unwrap();
        let mut bad=[0x5a;17];assert!(a.cbc_decrypt([0;16],&mut bad).is_err());assert_eq!(bad,[0x5a;17]);
        for n in [0,15,16,17,31,33,47]{assert!(a.pdf_decrypt(&vec![0;n]).is_err());}
        for pt in [[0;16],[17;16],{let mut b=[2;16];b[14]=1;b}]{
            let mut data=vec![0;16];data.extend(a.encrypt(pt)); // zero IV
            assert!(matches!(a.pdf_decrypt(&data),Err(Error::Invalid("AES PDF padding"))));
        }
    }
}


#[cfg(test)]
mod modern_tests{
    use super::*;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn block(s:&str)->[u8;16]{hex(s).try_into().unwrap()}
    #[test]fn aes256_fips197(){
        let key:Vec<u8>=(0..32).collect();let a=Aes256::new(&key).unwrap();
        assert_eq!(a.keys[2],block("a573c29fa176c498a97fce93a572c09c"));
        assert_eq!(a.keys[14],block("24fc79ccbf0979e9371ac23c6d68de36"));
        let p=block("00112233445566778899aabbccddeeff");let c=block("8ea2b7ca516745bfeafc49904b496089");
        assert_eq!(a.encrypt(p),c);assert_eq!(a.decrypt(c),p);
    }
    #[test]fn aes256_nist_cbc(){
        let a=Aes256::new(&hex("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")).unwrap();
        let mut ct=hex(concat!("f58c4c04d6e5f1ba779eabfb5f7bfbd6","9cfc4e967edb808d679f777bc6702c7d","39f23369a9d9bacfa530e26304231461","b2eb05e2c39be9fcda6c19078c6a9d1b"));
        a.cbc_decrypt(block("000102030405060708090a0b0c0d0e0f"),&mut ct).unwrap();
        assert_eq!(ct,hex(concat!("6bc1bee22e409f96e93d7e117393172a","ae2d8a571e03ac9c9eb76fac45af8e51","30c81c46a35ce411e5fbc1191a0a52ef","f69f2445df4f9b17ad2b417be66c3710")));
    }
    #[test]fn aes256_independent_padding(){
        // Python cryptography AES/CBC, key 0..31, IV 16..31.
        let a=Aes256::new(&(0..32).collect::<Vec<u8>>()).unwrap();
        for (n,ct) in [
            (0,"5a6e045708fb7196f02e553d02c3a692"),
            (1,"c95134e1edd8b1629404b74ac050b97a"),
            (15,"96cf942e0dd63d9044b651b61cf7ff0c"),
            (16,"1248c62648fddf935f02769570e37a042a6b65c046cfc3fbcbcc306dd9e10e6b"),
            (17,"1248c62648fddf935f02769570e37a0458923e589bb38fd717b7b0c20fcc3a12"),
            (33,"1248c62648fddf935f02769570e37a049b83505ef3e8593a0615e1ee06c7eeabcf234c4bf385594da1122be32b6d855e"),
        ]{
            let mut data=hex("101112131415161718191a1b1c1d1e1f");data.extend(hex(ct));
            let expected:Vec<u8>=(0..n).map(|i|((i*37+11)%256)as u8).collect();
            assert_eq!(a.pdf_decrypt(&data).unwrap(),expected);
        }
    }
    #[test]fn aes256_invalid_inputs(){
        for n in [0,16,24,31,33]{assert!(Aes256::new(&vec![0;n]).is_err());}
        let a=Aes256::new(&[0;32]).unwrap();let mut data=[0x5a;17];
        assert!(a.cbc_decrypt([0;16],&mut data).is_err());assert_eq!(data,[0x5a;17]);
        for n in [0,16,31,33]{assert!(a.pdf_decrypt(&vec![0;n]).is_err());}
        let mut ct=vec![0;16];ct.extend(a.encrypt([0;16]));
        assert!(a.pdf_decrypt(&ct).is_err());
    }
    #[test]fn aes128_cbc_encrypt_reference(){
        let a=Aes128::new(&hex("2b7e151628aed2a6abf7158809cf4f3c")).unwrap();
        let mut data=hex(concat!("6bc1bee22e409f96e93d7e117393172a","ae2d8a571e03ac9c9eb76fac45af8e51","30c81c46a35ce411e5fbc1191a0a52ef","f69f2445df4f9b17ad2b417be66c3710"));
        a.cbc_encrypt(block("000102030405060708090a0b0c0d0e0f"),&mut data).unwrap();
        assert_eq!(data,hex(concat!("7649abac8119b246cee98e9b12e9197d","5086cb9b507219ee95db113a917678b2","73bed6b8e3c1743b7116e69e22229516","3ff1caa1681fac09120eca307586e1a7")));
        let mut bad=[0x5a;17];assert!(a.cbc_encrypt([0;16],&mut bad).is_err());assert_eq!(bad,[0x5a;17]);
    }
}
