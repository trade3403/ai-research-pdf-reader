//! PDF Standard handler R2/R3 authentication only, ISO 32000-1 7.6.3.
//! Passwords are caller-supplied legacy encoded bytes. No guessing, key
//! export, logging or object decryption. Not side-channel/zeroization audited.
use super::{Dict,EncryptionInfo,legacy::{md5,Rc4}};
use crate::{Error,Object,Result};

const PAD:[u8;32]=[
    0x28,0xbf,0x4e,0x5e,0x4e,0x75,0x8a,0x41,0x64,0x00,0x4e,0x56,0xff,0xfa,0x01,0x08,
    0x2e,0x2e,0x00,0xb6,0xd0,0x68,0x3e,0x80,0x2f,0x0c,0xa9,0xfe,0x64,0x53,0x69,0x7a,
];

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PasswordStatus{Unencrypted,User,Owner,Incorrect}

fn padded(password:&[u8])->[u8;32]{
    let mut p=[0;32];let n=password.len().min(32);
    p[..n].copy_from_slice(&password[..n]);
    p[n..].copy_from_slice(&PAD[..32-n]);p
}
fn field<'a>(d:&'a Dict,k:&[u8])->Result<&'a [u8;32]>{
    match d.get(k){
        Some(Object::String(v))=>v.as_slice().try_into().map_err(|_|Error::Invalid("legacy authentication field length")),
        _=>Err(Error::Invalid("legacy authentication field")),
    }
}
fn crypt(key:&[u8],data:&mut [u8])->Result<()>{
    Rc4::new(key)?.apply(data)
}
fn xor_round(key:&[u8],round:u8,data:&mut [u8])->Result<()>{
    let mut k=[0u8;16];
    for (dst,src) in k.iter_mut().zip(key){*dst=*src^round;}
    crypt(&k[..key.len()],data)
}
fn file_key(password:&[u8;32],o:&[u8;32],p:i32,id:&[u8],r:u8,n:usize)->Result<[u8;16]>{
    let mut input=Vec::with_capacity(68+id.len());
    input.extend_from_slice(password);input.extend_from_slice(o);
    input.extend_from_slice(&p.to_le_bytes());input.extend_from_slice(id);
    let mut hash=md5(&input)?;
    if r==3{for _ in 0..50{hash=md5(&hash[..n])?;}}
    Ok(hash)
}
fn equal(a:&[u8],b:&[u8])->bool{
    a.len()==b.len()&&a.iter().zip(b).fold(0u8,|v,(a,b)|v|(*a^*b))==0
}
fn validate_user(key:&[u8],u:&[u8;32],id:&[u8],r:u8)->Result<bool>{
    if r==2{
        let mut expected=PAD;crypt(key,&mut expected)?;Ok(equal(&expected,u))
    }else{
        let mut seed=Vec::with_capacity(32+id.len());seed.extend_from_slice(&PAD);seed.extend_from_slice(id);
        let mut expected=md5(&seed)?;
        crypt(key,&mut expected)?;
        for round in 1..=19{xor_round(key,round,&mut expected)?;}
        // R3 trailing 16 bytes are arbitrary; never used in authentication.
        Ok(equal(&expected,&u[..16]))
    }
}
fn recover_owner(password:&[u8],o:&[u8;32],r:u8,n:usize)->Result<[u8;32]>{
    let mut hash=md5(&padded(password))?;
    if r==3{for _ in 0..50{hash=md5(&hash)?;}}
    let mut user=*o;
    if r==2{crypt(&hash[..n],&mut user)?;}
    else{for round in (0..=19).rev(){xor_round(&hash[..n],round,&mut user)?;}}
    Ok(user)
}
fn authenticate(d:&Dict,id:&[u8],info:&EncryptionInfo,password:&[u8])->Result<(PasswordStatus,Option<[u8;16]>)>{
    let r=info.revision;
    if !matches!((info.version,r),(1,2)|(2,3)){return Err(Error::Unsupported("legacy password revision"));}
    let n=usize::from(info.key_bits/8);
    if !(5..=16).contains(&n)||info.key_bits%8!=0||r==2&&n!=5{return Err(Error::Invalid("legacy password key length"));}
    if id.is_empty()||id.len()>1024{return Err(Error::Invalid("encrypted document ID"));}
    let o=field(d,b"O")?;let u=field(d,b"U")?;
    // If a supplied password is valid as both roles, prefer Owner.
    let recovered=recover_owner(password,o,r,n)?;
    let key=file_key(&recovered,o,info.permissions,id,r,n)?;
    if validate_user(&key[..n],u,id,r)?{return Ok((PasswordStatus::Owner,Some(key)));}
    let key=file_key(&padded(password),o,info.permissions,id,r,n)?;
    if validate_user(&key[..n],u,id,r)?{return Ok((PasswordStatus::User,Some(key)));}
    Ok((PasswordStatus::Incorrect,None))
}
pub(super) fn verify(d:&Dict,id:&[u8],info:&EncryptionInfo,password:&[u8])->Result<PasswordStatus>{
    authenticate(d,id,info,password).map(|(status,_)|status)
}

#[cfg(test)]
mod tests{
    use super::*;
    // Fixed synthetic O/U/file-key fixtures produced independently by pypdf
    // AlgV4 with its cryptography RC4 backend. No external runtime dependency.
    struct Case{r:u8,bits:u16,p:i32,user:&'static str,owner:&'static str,o:&'static str,u:&'static str,key:&'static str}
    const ID:&str="0102030405060708090a0b0c0d0e0f10";
    const CASES:[Case;4]=[
        Case{r:2,bits:40,p:-4,user:"726561646572",owner:"6f776e6572",o:"93fe0d5254ab4c9644693f33c07cb54f587dce1e2682fe9ecea6107a1ef630dd",u:"93813eaf533272230ec7cc0a0d30697a13a12e33aec4e50a9f4d9b8faa8e72f7",key:"178745574f"},
        Case{r:3,bits:40,p:-44,user:"",owner:"6f776e6572",o:"61840a4e66456bd408feb9a45fc05fce6d8f57574f1350bf0362c1836240898c",u:"a4ed68b98106dbe745cfa1bb83c66f8328bf4e5e4e758a4164004e56fffa0108",key:"c4d95e4bd9"},
        Case{r:3,bits:128,p:-3904,user:"726561646572",owner:"6f776e6572",o:"0cb58749c5346569e765906caf64e4429a4c20d6e996fdef963e9b5080f9e083",u:"905857d6a63f6a75a59cd5967cd4255228bf4e5e4e758a4164004e56fffa0108",key:"69f6f9891bdbd8953d40395910a30087"},
        Case{r:3,bits:128,p:-4,user:"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728",owner:"6f776e657280",o:"5c0529c63823359e0e544d9a680e505d83f0ccd93977e19f61f69839fbda71be",u:"5a5cb22075b430065a4f11c0d983ddf528bf4e5e4e758a4164004e56fffa0108",key:"22cf2a743233ce8153381dd093d6b884"},
    ];
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn text(c:&Case)->String{format!("<< /Filter /Standard /V {} /R {} /Length {} /P {} /O <{}> /U <{}> >>",c.r-1,c.r,c.bits,c.p,c.o,c.u)}
    fn dict(c:&Case)->Dict{crate::dict(crate::parse_object(text(c).as_bytes()).unwrap()).unwrap()}
    fn check(c:&Case,pass:&[u8],role:PasswordStatus){
        let d=dict(c);let info=super::super::parse(&d).unwrap();
        let (status,key)=authenticate(&d,&hex(ID),&info,pass).unwrap();
        assert_eq!(status,role);
        if role==PasswordStatus::Incorrect{assert!(key.is_none());}
        else{assert_eq!(&key.unwrap()[..usize::from(c.bits/8)],hex(c.key).as_slice());}
    }
    #[test]fn independent_user_keys(){for c in &CASES{check(c,&hex(c.user),PasswordStatus::User);}}
    #[test]fn independent_owner_keys(){for c in &CASES{check(c,&hex(c.owner),PasswordStatus::Owner);}}
    #[test]fn wrong_password_rejected(){for c in &CASES{check(c,b"wrong password",PasswordStatus::Incorrect);}}
    #[test]fn padding_and_truncation(){
        let c=&CASES[3];let pass=hex(c.user);
        check(c,&pass[..32],PasswordStatus::User);
        check(c,&pass[..31],PasswordStatus::Incorrect);
        assert_eq!(padded(b""),PAD);
    }
    #[test]fn revision3_arbitrary_u_tail(){
        let c=&CASES[2];let mut d=dict(c);let info=super::super::parse(&d).unwrap();
        let mut u=hex(c.u);u[16..].fill(0xa5);d.insert(b"U".to_vec(),Object::String(u.clone()));
        assert_eq!(verify(&d,&hex(ID),&info,&hex(c.user)).unwrap(),PasswordStatus::User);
        u[0]^=1;d.insert(b"U".to_vec(),Object::String(u));
        assert_eq!(verify(&d,&hex(ID),&info,&hex(c.user)).unwrap(),PasswordStatus::Incorrect);
    }
    #[test]fn id_permissions_and_revision_checks(){
        let c=&CASES[2];let d=dict(c);let mut info=super::super::parse(&d).unwrap();
        assert_eq!(verify(&d,b"other id",&info,&hex(c.user)).unwrap(),PasswordStatus::Incorrect);
        info.permissions^=4;assert_eq!(verify(&d,&hex(ID),&info,&hex(c.user)).unwrap(),PasswordStatus::Incorrect);
        info.version=4;info.revision=4;assert!(matches!(verify(&d,&hex(ID),&info,&hex(c.user)),Err(Error::Unsupported("legacy password revision"))));
    }
    #[test]fn public_authentication_does_not_open_document(){
        let c=&CASES[2];let enc=text(c);
        let mut b=b"%PDF-1.4\n".to_vec();let mut offsets=vec![];
        for (i,o) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 0 /Kids [] >>",enc.as_str()].into_iter().enumerate(){
            offsets.push(b.len());b.extend(format!("{} 0 obj\n{o}\nendobj\n",i+1).bytes());
        }
        let x=b.len();b.extend(b"xref\n0 4\n0000000000 65535 f \n");
        for off in offsets{b.extend(format!("{off:010} 00000 n \n").bytes());}
        b.extend(format!("trailer\n<< /Size 4 /Root 1 0 R /Encrypt 3 0 R /ID [<{ID}> <{ID}>] >>\nstartxref\n{x}\n%%EOF\n").bytes());
        use crate::Document;
        assert_eq!(Document::authenticate_legacy_password(&b,b"reader").unwrap(),PasswordStatus::User);
        assert_eq!(Document::authenticate_legacy_password(&b,b"owner").unwrap(),PasswordStatus::Owner);
        assert_eq!(Document::authenticate_legacy_password(&b,b"wrong").unwrap(),PasswordStatus::Incorrect);
        assert!(matches!(Document::open(&b),Err(Error::Unsupported("encryption"))));
    }
}
