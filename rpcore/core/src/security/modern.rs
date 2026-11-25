//! Modern Standard Security Handler R5/R6 (ISO 32000-2 Algorithm 2.B).
//! Caller supplies prepared UTF-8 password bytes; no implicit SASLprep.
//! No key export, guessing, or logging. No side-channel or zeroization audit.
use super::{Dict,EncryptionInfo,PasswordStatus,aes::{Aes128,Aes256},sha2::{sha256,sha384,sha512}};
use crate::{Error,Object,Result};

fn field<const N:usize>(d:&Dict,name:&[u8])->Result<[u8;N]>{
    match d.get(name){
        Some(Object::String(s))=>s.as_slice().try_into().map_err(|_|Error::Invalid("modern authentication field length")),
        _=>Err(Error::Invalid("modern authentication field")),
    }
}
fn equal(a:&[u8],b:&[u8])->bool{
    a.len()==b.len()&&a.iter().zip(b).fold(0u8,|v,(a,b)|v|(*a^*b))==0
}
fn hash(r:u8,password:&[u8],salt:&[u8],user:&[u8])->Result<[u8;32]>{
    if !matches!(r,5|6)||password.len()>127||salt.len()!=8||!matches!(user.len(),0|48){
        return Err(Error::Invalid("modern password hash inputs"));
    }
    let mut input=Vec::with_capacity(password.len()+8+user.len());
    input.extend_from_slice(password);input.extend_from_slice(salt);input.extend_from_slice(user);
    let initial=sha256(&input)?;
    if r==5{return Ok(initial);}
    let mut k=initial.to_vec();
    // Termination is guaranteed by round 287 (last byte <=255).
    // One round processes at most 64*(127+64+48) bytes.
    for round in 1usize..=287{
        let mut unit=Vec::with_capacity(password.len()+k.len()+user.len());
        unit.extend_from_slice(password);unit.extend_from_slice(&k);unit.extend_from_slice(user);
        let mut e=unit.repeat(64);
        let mut iv=[0;16];iv.copy_from_slice(&k[16..32]);
        Aes128::new(&k[..16])?.cbc_encrypt(iv,&mut e)?;
        let selector=e[..16].iter().map(|&b|usize::from(b)).sum::<usize>()%3;
        k=match selector{0=>sha256(&e)?.to_vec(),1=>sha384(&e)?.to_vec(),_=>sha512(&e)?.to_vec()};
        if round>=64&&usize::from(*e.last().ok_or(Error::Invalid("modern hash empty block"))?)<=round-32{
            return k[..32].try_into().map_err(|_|Error::Invalid("modern hash output"));
        }
    }
    Err(Error::Limit("modern password rounds"))
}
fn check_perms(key:&[u8;32],encrypted:[u8;16],info:&EncryptionInfo)->Result<()>{
    let plain=Aes256::new(key)?.decrypt(encrypted);
    let mut expected=[0xff;12];
    expected[..4].copy_from_slice(&info.permissions.to_le_bytes());
    expected[8]=if info.encrypt_metadata{b'T'}else{b'F'};
    expected[9..12].copy_from_slice(b"adb");
    if !equal(&plain[..12],&expected){return Err(Error::Invalid("modern PDF permissions block"));}
    Ok(())
}
pub(super) fn authenticate(d:&Dict,info:&EncryptionInfo,password:&[u8])->Result<(PasswordStatus,Option<[u8;32]>)>{
    if info.version!=5||!matches!(info.revision,5|6)||info.key_bits!=256{return Err(Error::Unsupported("modern password revision"));}
    // The caller supplies revision-appropriate prepared UTF-8. Truncate BYTES,
    // not characters, after preparation, as required by the PDF algorithms.
    let password=&password[..password.len().min(127)];
    let r=info.revision;
    let o=field::<48>(d,b"O")?;let u=field::<48>(d,b"U")?;
    let oe=field::<32>(d,b"OE")?;let ue=field::<32>(d,b"UE")?;
    let perms=field::<16>(d,b"Perms")?;
    let (role,mut key,wrapping)=if equal(&hash(r,password,&o[32..40],&u)?,&o[..32]){
        (PasswordStatus::Owner,oe,hash(r,password,&o[40..48],&u)?)
    }else if equal(&hash(r,password,&u[32..40],&[])?,&u[..32]){
        (PasswordStatus::User,ue,hash(r,password,&u[40..48],&[])?)
    }else{return Ok((PasswordStatus::Incorrect,None));};
    // OE/UE: exactly two AES-256 CBC blocks, zero IV, no padding.
    Aes256::new(&wrapping)?.cbc_decrypt([0;16],&mut key)?;
    check_perms(&key,perms,info)?;
    Ok((role,Some(key)))
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::Document;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    const ID:&str="0102030405060708090a0b0c0d0e0f10";
    struct Case{r:u8,p:i32,meta:bool,u:&'static str,ue:&'static str,o:&'static str,oe:&'static str,perms:&'static str}
    // Generated independently using pypdf AlgV5.generate_values on synthetic
    // reader/owner passwords and file key bytes 0..31. Random salts frozen.
    const CASES:[Case;2]=[
        Case{r:5,p:-4,meta:true,u:"82e0804746be548cbc434ab6746fced64be592beb241b6aa6300f02ab4095c30faf8372240b2da180bff1ba9a5494662",ue:"ae850c63e8e71c512ec5a12a1755fda6a8afb86e5b874e8af43b21a639a61bef",o:"b7b365189b81eaf8dc4be6bbce6e71981e6f1a2e62339129929b327e22a69eb3cb5a0331e1f54a6f81f629ab0d5b059d",oe:"2d124f5ac0a18cb175419307b855e6f4a87f8c134eda9ca92065f87c8db1bcd2",perms:"ffa9eb8e220fe89c8fd04ca4b39d0a08"},
        Case{r:6,p:-3904,meta:false,u:"159aef3d8f2f96e388ce96326e8bd2dafdee509ef6520629a9d72b768fd8698ca913229d6820bd1c66e9324d996b872d",ue:"e4ce52b578a37564d32a9582d4e9e7f0aea733014713ee753b4aa190634583a3",o:"615ed63547669c77926acfa8429b013a3ae25b5353c2cbf181a7d9cf6d7ad6b828681f7c8f3cf15b5ff66abd2980d690",oe:"31386736c60b954a55be595d9f21b374832c4bd2cae1fe94054119cc986f1852",perms:"da40ca7d7e294ca805fd180f2a19842a"},
    ];
    fn enc(c:&Case)->String{
        format!("<< /Filter /Standard /V 5 /R {} /Length 256 /P {} /EncryptMetadata {} /O <{}> /U <{}> /OE <{}> /UE <{}> /Perms <{}> /CF << /StdCF << /CFM /AESV3 /Length 32 >> >> /StmF /StdCF /StrF /StdCF >>",c.r,c.p,c.meta,c.o,c.u,c.oe,c.ue,c.perms)
    }
    fn dict(c:&Case)->Dict{crate::dict(crate::parse_object(enc(c).as_bytes()).unwrap()).unwrap()}
    #[test]fn independent_hash_vectors(){
        let salt:Vec<u8>=(0..8).collect();let u:Vec<u8>=(0..48).collect();
        for (r,p,ud,want) in [
            (5,b"reader".as_slice(),&[][..],"19dfc96498ffd53831f61559a15998d50b50c1d0ea4cd78c4ea11feaacbdedb0"),
            (6,b"reader".as_slice(),&[][..],"97d4266683088707a0226d11cb7ead30383e6ed7d738d2343282136db938cafe"),
            (6,b"owner".as_slice(),u.as_slice(),"f77bbaa82ba46eca0e3997ad8c91b9fa4a3f83750e13a950f901abef1f69b683"),
            (6,&[b'a';127][..],&[][..],"7da451f93264b80bf8c3f5cdae54fc07e8a6159048c608559f736a1e8e026a8c"),
        ]{assert_eq!(hash(r,p,&salt,ud).unwrap().as_slice(),hex(want));}
    }
    #[test]fn independent_user_owner_keys(){
        let key:Vec<u8>=(0..32).collect();
        for c in &CASES{
            let d=dict(c);let info=super::super::parse(&d).unwrap();
            for (pass,role) in [(b"reader".as_slice(),PasswordStatus::User),(b"owner".as_slice(),PasswordStatus::Owner)]{
                let (actual,got)=authenticate(&d,&info,pass).unwrap();
                assert_eq!(actual,role);assert_eq!(got.unwrap().as_slice(),key);
            }
            assert_eq!(authenticate(&d,&info,b"wrong").unwrap(),(PasswordStatus::Incorrect,None));
        }
    }
    #[test]fn permissions_and_wrapped_key_tamper(){
        let c=&CASES[0];let d=dict(c);let mut info=super::super::parse(&d).unwrap();
        info.permissions^=4;assert!(authenticate(&d,&info,b"reader").is_err());
        info.permissions=c.p;info.encrypt_metadata=false;assert!(authenticate(&d,&info,b"reader").is_err());
        info.encrypt_metadata=true;
        for name in [b"Perms".as_slice(),b"UE".as_slice()]{
            let mut damaged=d.clone();let Some(Object::String(s))=damaged.get_mut(name) else{panic!("field");};s[0]^=1;
            assert!(authenticate(&damaged,&info,b"reader").is_err());
        }
    }
    const TITLE:&str="000102030405060708090a0b0c0d0e0f6862a024bd732968189334707f04783f";
    const STREAM:&str="000102030405060708090a0b0c0d0e0f5051e48aeebc1e2952b6aec1b416f33d5cae6c95dc42061338bf59156cecc8f7";
    const OBJSTM:&str="000102030405060708090a0b0c0d0e0fef1d6a5b2b8ec8541163d8130dc6b0889906d51c4712adea1ee455d58f151d2edc1a75d63a25c2e09b40b3135e3eba01";
    const XML:&str="000102030405060708090a0b0c0d0e0f5f01abca678616af02e4625ea52f845a";
    fn stream(b:&mut Vec<u8>,id:usize,extra:&str,data:&[u8])->usize{
        let off=b.len();b.extend(format!("{id} 0 obj\n<< /Length {} {extra} >>\nstream\n",data.len()).bytes());b.extend(data);b.extend(b"\nendstream\nendobj\n");off
    }
    fn fixture(c:&Case)->Vec<u8>{
        let mut b=b"%PDF-1.7\n".to_vec();let mut off=[0usize;11];
        let page=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Title <{TITLE}> >>");
        for (i,o) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 1 /Kids [3 0 R] >>",page.as_str()].into_iter().enumerate(){
            off[i+1]=b.len();b.extend(format!("{} 0 obj\n{o}\nendobj\n",i+1).bytes());
        }
        off[4]=stream(&mut b,4,"/Filter /FlateDecode",&hex(STREAM));
        off[5]=b.len();b.extend(format!("5 7 obj\n<{TITLE}>\nendobj\n").bytes());
        let xml=if c.meta{hex(XML)}else{b"<xml/>".to_vec()};
        off[6]=stream(&mut b,6,"/Type /Metadata /Subtype /XML",&xml);
        off[7]=b.len();b.extend(format!("7 0 obj\n{}\nendobj\n",enc(c)).bytes());
        off[8]=stream(&mut b,8,"/Type /ObjStm /N 1 /First 4",&hex(OBJSTM));
        let x=b.len();off[10]=x;let mut rows=vec![];
        for j in 0..11{
            let (kind,pos,g)=match j{0=>(0u8,0u32,65535u16),5=>(1,off[j] as u32,7),9=>(2,8,0),_=>(1,off[j] as u32,0)};
            rows.push(kind);rows.extend(pos.to_be_bytes());rows.extend(g.to_be_bytes());
        }
        b.extend(format!("10 0 obj\n<< /Type /XRef /Size 11 /Root 1 0 R /Encrypt 7 0 R /ID [<{ID}> <{ID}>] /W [1 4 2] /Length {} >>\nstream\n",rows.len()).bytes());
        b.extend(rows);b.extend(format!("\nendstream\nendobj\nstartxref\n{x}\n%%EOF\n").bytes());b
    }
    fn check(doc:&Document<'_>){
        assert_eq!(doc.page_count().unwrap(),1);
        let d=crate::dict(doc.resolve(3,0).unwrap()).unwrap();
        assert_eq!(d.get(b"Title".as_slice()),Some(&Object::String(b"AES256 title".to_vec())));
        // AESV3 ignores object number and generation for object key selection.
        assert_eq!(doc.resolve(5,7).unwrap(),Object::String(b"AES256 title".to_vec()));
        assert_eq!(crate::filters::decode(&doc.resolve(4,0).unwrap(),1024).unwrap(),b"0 0 10 10 re f\n");
        let Object::Stream{data,..}=doc.resolve(6,0).unwrap() else{panic!("metadata");};assert_eq!(data,b"<xml/>");
        for _ in 0..2{
            let d=crate::dict(doc.resolve(9,0).unwrap()).unwrap();assert_eq!(d.get(b"Label".as_slice()),Some(&Object::String(b"compressed value".to_vec())));
        }
        assert!(doc.resolve(7,0).is_err());assert!(doc.resolve(10,0).is_err());
    }
    #[test]fn r5_aes256_document(){
        let b=fixture(&CASES[0]);check(&Document::open_with_password(&b,b"reader").unwrap());
        check(&Document::open_with_password(&b,b"owner").unwrap());assert!(Document::open(&b).is_err());
        assert!(Document::open_with_password(&b,b"wrong").is_err());
    }
    #[test]fn r6_aes256_document_and_permissions(){
        let b=fixture(&CASES[1]);check(&Document::open_with_password(&b,b"owner").unwrap());
        assert_eq!(Document::authenticate_password(&b,b"reader").unwrap(),PasswordStatus::User);
        assert!(matches!(Document::open_with_password(&b,b"reader"),Err(Error::Unsupported("restricted PDF user permissions"))));
        assert!(Document::open_with_password(&b,b"wrong").is_err());
    }
}
