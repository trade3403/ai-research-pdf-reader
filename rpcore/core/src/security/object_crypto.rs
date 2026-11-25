//! Authenticated R2/R3/R4 object decoding, not an encryption writer.
//! No key export, Debug, guaranteed zeroization or side-channel audit.
use super::{legacy::{md5,Rc4},aes::Aes128,CryptMethod,EncryptionInfo};
use crate::{Error,Object,Result,MAX_DEPTH,MAX_BYTES};

pub(crate) struct ObjectCrypto{
    key:[u8;16],len:usize,encrypt_ref:Option<(u32,u16)>,
    revision:u8,strings:CryptMethod,streams:CryptMethod,embedded:CryptMethod,metadata:bool,
}
impl ObjectCrypto{
    #[cfg(test)]
    pub(super) fn new(key:[u8;16],len:usize,encrypt_ref:Option<(u32,u16)>)->Result<Self>{
        Self::with_info(key,encrypt_ref,&EncryptionInfo{version:1,revision:2,key_bits:(len*8) as u16,permissions:-4,encrypt_metadata:true,strings:CryptMethod::Rc4,streams:CryptMethod::Rc4,embedded_files:CryptMethod::Rc4})
    }
    pub(super) fn with_info(key:[u8;16],encrypt_ref:Option<(u32,u16)>,info:&EncryptionInfo)->Result<Self>{
        let len=usize::from(info.key_bits/8);
        if !(5..=16).contains(&len){return Err(Error::Invalid("legacy file key length"));}
        for method in [info.strings,info.streams,info.embedded_files]{
            match method{
                CryptMethod::Identity|CryptMethod::Rc4=>{},
                CryptMethod::Aes128 if len==16&&info.revision==4=>{},
                _=>return Err(Error::Unsupported("legacy object crypt method")),
            }
        }
        Ok(Self{key,len,encrypt_ref,revision:info.revision,strings:info.strings,streams:info.streams,embedded:info.embedded_files,metadata:info.encrypt_metadata})
    }
    pub(crate) fn decrypt(&self,id:u32,generation:u16,value:&mut Object)->Result<()>{
        if self.encrypt_ref==Some((id,generation)){return Err(Error::Unsupported("encryption dictionary access"));}
        let mut seed=[0u8;25];
        seed[..self.len].copy_from_slice(&self.key[..self.len]);
        seed[self.len..self.len+3].copy_from_slice(&id.to_le_bytes()[..3]);
        seed[self.len+3..self.len+5].copy_from_slice(&generation.to_le_bytes());
        let rc4=md5(&seed[..self.len+5])?;
        seed[self.len+5..self.len+9].copy_from_slice(b"sAlT");
        let aes=md5(&seed[..self.len+9])?;
        let keys=Keys{rc4,rc4_len:(self.len+5).min(16),aes};
        let mut remaining=MAX_BYTES;
        self.visit(value,&keys,0,&mut remaining)
    }
    fn visit(&self,value:&mut Object,keys:&Keys,depth:usize,remaining:&mut usize)->Result<()>{
        if depth>=MAX_DEPTH{return Err(Error::Limit("encrypted object depth"));}
        match value{
            Object::String(data)=>decrypt_bytes(data,self.strings,keys,remaining)?,
            Object::Array(a)=>for v in a{self.visit(v,keys,depth+1,remaining)?;},
            Object::Dict(d)=>{special_dict(d)?;for v in d.values_mut(){self.visit(v,keys,depth+1,remaining)?;}},
            Object::Stream{dict,data}=>{
                special_dict(dict)?;
                // Explicit Crypt filters and indirect filter chains are not
                // supported; never apply a second implicit decryption.
                match dict.get(b"Filter".as_slice()){
                    None|Some(Object::Null)=>{},
                    Some(Object::Name(n)) if n!=b"Crypt"=>{},
                    Some(Object::Array(a)) if a.iter().all(|v|matches!(v,Object::Name(n) if n!=b"Crypt"))=>{},
                    _=>return Err(Error::Unsupported("legacy encrypted stream filter")),
                }
                let method=match dict.get(b"Type".as_slice()){
                    Some(Object::Name(n)) if n==b"Metadata"&&!self.metadata=>CryptMethod::Identity,
                    Some(Object::Name(n)) if n==b"EmbeddedFile"=>self.embedded,
                    Some(Object::Ref(..)) if self.revision==4=>return Err(Error::Unsupported("indirect encrypted stream Type")),
                    _=>self.streams,
                };
                // The stream dictionary's strings follow StrF even when the
                // stream payload is exempt (e.g. unencrypted metadata).
                for v in dict.values_mut(){self.visit(v,keys,depth+1,remaining)?;}
                decrypt_bytes(data,method,keys,remaining)?;
                if method==CryptMethod::Aes128{
                    // The original encrypted Length was used for bounded
                    // reading; returned decrypted stream length is normalized.
                    dict.insert(b"Length".to_vec(),Object::Number(data.len() as f64));
                }
            }
            _=>{},
        }
        Ok(())
    }
}
struct Keys{rc4:[u8;16],rc4_len:usize,aes:[u8;16]}
fn decrypt_bytes(data:&mut Vec<u8>,method:CryptMethod,keys:&Keys,remaining:&mut usize)->Result<()>{
    *remaining=remaining.checked_sub(data.len()).ok_or(Error::Limit("encrypted object bytes"))?;
    match method{
        CryptMethod::Identity=>Ok(()),
        CryptMethod::Rc4=>Rc4::new(&keys.rc4[..keys.rc4_len])?.apply(data),
        CryptMethod::Aes128=>{*data=Aes128::new(&keys.aes)?.pdf_decrypt(data)?;Ok(())},
        _=>Err(Error::Unsupported("legacy object crypt method")),
    }
}
fn special_dict(d:&std::collections::BTreeMap<Vec<u8>,Object>)->Result<()>{
    if matches!(d.get(b"Type".as_slice()),Some(Object::Name(n)) if n==b"Sig"||n==b"DocTimeStamp"||n==b"XRef")
        ||(d.contains_key(b"ByteRange".as_slice())&&d.contains_key(b"Contents".as_slice())){
        return Err(Error::Unsupported("encrypted special dictionary"));
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::Document;
    const ID:&str="0102030405060708090a0b0c0d0e0f10";
    struct Case{v:u8,r:u8,bits:u16,p:i32,o:&'static str,u:&'static str,title:&'static str,nested:&'static str,stream:&'static str,gen:&'static str,objstm:&'static str}
    // Authentication fixtures from B11.1c; independent ciphertext generated
    // with Python hashlib.md5 + cryptography ARC4, zlib for the page stream.
    const CASES:[Case;2]=[
        Case{v:1,r:2,bits:40,p:-4,o:"93fe0d5254ab4c9644693f33c07cb54f587dce1e2682fe9ecea6107a1ef630dd",u:"93813eaf533272230ec7cc0a0d30697a13a12e33aec4e50a9f4d9b8faa8e72f7",title:"5eec3f49584d84548fb0c4c8",nested:"5cec2b5c5e50845687a8ddc8",stream:"d53fe2b9f2e7149a81fb677efbd1b497e744ce",gen:"0becdd144e29fd7c8db3e5d023046369",objstm:"5468e7f4158efad16c6c9ab8d83f0d5cae1badd5c0378a2e921795050a2ef1381e2d1e"},
        Case{v:2,r:3,bits:128,p:-3904,o:"0cb58749c5346569e765906caf64e4429a4c20d6e996fdef963e9b5080f9e083",u:"905857d6a63f6a75a59cd5967cd4255228bf4e5e4e758a4164004e56fffa0108",title:"7ba9f76f13ed1efa97df0010",nested:"79a9e37a15f01ef89fc71910",stream:"4d0e163931a8da9a607d54662930bbe110b0ce",gen:"22c9a983e9649360e3534efa095b97ed",objstm:"dfca929bfe38702d0834bb7a9536aca1c1d08fc80273a6f7cf5c183554f869c7ce082c"},
    ];
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn fixture(c:&Case,stream_xref:bool)->Vec<u8>{
        let mut b=b"%PDF-1.7\n".to_vec();
        let mut off=[0usize;10];
        let page=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Title <{}> /Extra [<< /Label <{}> >>] >>",c.title,c.nested);
        for (i,o) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 1 /Kids [3 0 R] >>",page.as_str()].into_iter().enumerate(){
            off[i+1]=b.len();b.extend(format!("{} 0 obj\n{o}\nendobj\n",i+1).bytes());
        }
        let payload=hex(c.stream);off[4]=b.len();
        b.extend(format!("4 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n",payload.len()).bytes());b.extend(payload);b.extend(b"\nendstream\nendobj\n");
        off[5]=b.len();b.extend(format!("5 7 obj\n<{}>\nendobj\n",c.gen).bytes());
        if stream_xref{
            let payload=hex(c.objstm);off[6]=b.len();
            b.extend(format!("6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {} >>\nstream\n",payload.len()).bytes());b.extend(payload);b.extend(b"\nendstream\nendobj\n");
        }
        off[8]=b.len();
        b.extend(format!("8 0 obj\n<< /Filter /Standard /V {} /R {} /Length {} /P {} /O <{}> /U <{}> >>\nendobj\n",c.v,c.r,c.bits,c.p,c.o,c.u).bytes());
        let x=b.len();
        if stream_xref{
            off[9]=x;let mut rows=vec![];
            for i in 0..10{
                let (kind,offset,g)=match i{0=>(0u8,0u32,65535u16),7=>(2,6,0),5=>(1,off[i] as u32,7),_=>(1,off[i] as u32,0)};
                rows.push(kind);rows.extend(offset.to_be_bytes());rows.extend(g.to_be_bytes());
            }
            b.extend(format!("9 0 obj\n<< /Type /XRef /Size 10 /Root 1 0 R /Encrypt 8 0 R /ID [<{ID}> <{ID}>] /W [1 4 2] /Length {} >>\nstream\n",rows.len()).bytes());b.extend(rows);b.extend(b"\nendstream\nendobj\n");
        }else{
            b.extend(b"xref\n0 9\n0000000000 65535 f \n");
            for (i,v) in off.iter().enumerate().take(9).skip(1){
                if *v==0{b.extend(b"0000000000 00000 f \n");}else{b.extend(format!("{v:010} {:05} n \n",if i==5{7}else{0}).bytes());}
            }
            b.extend(format!("trailer\n<< /Size 9 /Root 1 0 R /Encrypt 8 0 R /ID [<{ID}> <{ID}>] >>\n").bytes());
        }
        b.extend(format!("startxref\n{x}\n%%EOF\n").bytes());b
    }
    fn check(doc:&Document<'_>){
        assert_eq!(doc.page_count().unwrap(),1);
        let d=crate::dict(doc.resolve(3,0).unwrap()).unwrap();
        assert_eq!(d.get(b"Title".as_slice()),Some(&Object::String(b"legacy title".to_vec())));
        let mut nested=std::collections::BTreeMap::new();nested.insert(b"Label".to_vec(),Object::String(b"nested value".to_vec()));
        assert_eq!(d.get(b"Extra".as_slice()),Some(&Object::Array(vec![Object::Dict(nested)])));
        assert_eq!(doc.resolve(5,7).unwrap(),Object::String(b"generation seven".to_vec()));
        assert!(doc.resolve(5,0).is_err());
        assert!(doc.resolve(8,0).is_err());
        let data=crate::filters::decode(&doc.resolve(4,0).unwrap(),1024).unwrap();
        assert_eq!(data,b"0 0 10 10 re f\n");
        assert_eq!(doc.resolve(3,0).unwrap(),Object::Dict(d));
    }
    #[test]fn authenticated_r2_classic(){
        let b=fixture(&CASES[0],false);
        for password in [b"reader".as_slice(),b"owner".as_slice()]{check(&Document::open_with_legacy_password(&b,password).unwrap());}
        assert!(Document::open(&b).is_err());
        assert!(matches!(Document::open_with_legacy_password(&b,b"wrong"),Err(Error::Invalid("incorrect PDF password"))));
    }
    #[test]fn authenticated_r3_permissions(){
        let b=fixture(&CASES[1],false);
        check(&Document::open_with_legacy_password(&b,b"owner").unwrap());
        assert!(matches!(Document::open_with_legacy_password(&b,b"reader"),Err(Error::Unsupported("restricted PDF user permissions"))));
        assert!(Document::open_with_legacy_password(&b,b"wrong").is_err());
    }
    #[test]fn encrypted_object_stream_once(){
        let b=fixture(&CASES[1],true);let doc=Document::open_with_legacy_password(&b,b"owner").unwrap();
        check(&doc);
        for _ in 0..2{
            let d=crate::dict(doc.resolve(7,0).unwrap()).unwrap();
            assert_eq!(d.get(b"Label".as_slice()),Some(&Object::String(b"compressed value".to_vec())));
        }
        assert!(doc.resolve(9,0).is_err());
    }
    #[test]fn special_objects_and_filters_fail_closed(){
        let c=ObjectCrypto::new([1;16],5,None).unwrap();
        for raw in [b"<< /Type /Sig /Contents <00> >>".as_slice(),b"<< /ByteRange [0 1] /Contents <00> >>".as_slice(),b"<< /Type /XRef >>".as_slice()]{
            let mut o=crate::parse_object(raw).unwrap();assert!(c.decrypt(1,0,&mut o).is_err());
        }
        let mut d=std::collections::BTreeMap::new();d.insert(b"Filter".to_vec(),Object::Name(b"Crypt".to_vec()));
        let mut o=Object::Stream{dict:d,data:vec![0]};
        assert!(c.decrypt(1,0,&mut o).is_err());
    }
}


#[cfg(test)]
mod r4_tests{
    use super::*;
    use crate::Document;
    const ID:&str="0102030405060708090a0b0c0d0e0f10";
    const O:&str="0cb58749c5346569e765906caf64e4429a4c20d6e996fdef963e9b5080f9e083";
    // Reference O/U and key from pypdf AlgV4; object ciphertext from
    // Python hashlib and cryptography, independent of the native AES code.
    const U_TRUE:&str="d65cf7f405a0e0ca4ff087f69169b29c28bf4e5e4e758a4164004e56fffa0108";
    const U_FALSE:&str="a0ac4bde8e5fba80adc064fc72d8bfa628bf4e5e4e758a4164004e56fffa0108";
    const TITLE:[&str;2]=["000102030405060708090a0b0c0d0e0fb4da860224f08f66e560e5001559e4d4","000102030405060708090a0b0c0d0e0f33b497271e3fea7a93ac9d70e9074c05"];
    const STREAM:[&str;2]=["000102030405060708090a0b0c0d0e0f33ae664f6a4f0614900b7928af164d042d70eabca00741bfd67b268e14d20a81","000102030405060708090a0b0c0d0e0fe314fcdc2ff54b08ec018749a0ef5c3e02a9b8e1ff15186e6f9e7b5b86a72e2d"];
    const GENERATION:[&str;2]=["000102030405060708090a0b0c0d0e0fccb3f823e41066b2849b5141546a8c7f7bf2deffa6f44b33b34eb63b6be1b0f4","000102030405060708090a0b0c0d0e0f144a313e497c99efad5d7e0385e031134fea7e4e3ad94ec00641a99cf57021f4"];
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    fn stream(b:&mut Vec<u8>,id:usize,dict:&str,data:&[u8])->usize{
        let off=b.len();b.extend(format!("{id} 0 obj\n<< /Length {} {dict} >>\nstream\n",data.len()).bytes());
        b.extend(data);b.extend(b"\nendstream\nendobj\n");off
    }
    fn fixture(metadata:bool,mixed:bool)->Vec<u8>{
        assert!(!mixed||!metadata);
        let i=usize::from(!metadata);
        let title=if mixed{"f07c6ddaf7091ea5e7"}else{TITLE[i]};
        let mut b=b"%PDF-1.7\n".to_vec();let mut off=[0usize;12];
        let page=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Title <{title}> >>");
        for (j,o) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 1 /Kids [3 0 R] >>",page.as_str()].into_iter().enumerate(){
            off[j+1]=b.len();b.extend(format!("{} 0 obj\n{o}\nendobj\n",j+1).bytes());
        }
        off[4]=stream(&mut b,4,"/Filter /FlateDecode",&hex(STREAM[i]));
        // Mixed uses RC4 for strings; nonzero generation fixture is AES only.
        if !mixed{off[5]=b.len();b.extend(format!("5 7 obj\n<{}>\nendobj\n",GENERATION[i]).bytes());}
        let xml=if metadata{hex("000102030405060708090a0b0c0d0e0fa5d0430876b33f0a4d92c5f8a1c33243")}else{b"<xml/>".to_vec()};
        off[6]=stream(&mut b,6,"/Type /Metadata /Subtype /XML",&xml);
        let attachment=if mixed{b"attachment".to_vec()}else if metadata{hex("000102030405060708090a0b0c0d0e0f7806ea9301e1830e1c5fa9ae58ae75b3")}else{hex("000102030405060708090a0b0c0d0e0fdab53c20889fbe670e473acce77c04b3")};
        off[7]=stream(&mut b,7,"/Type /EmbeddedFile",&attachment);
        let objstm=if metadata{"000102030405060708090a0b0c0d0e0f0c2750a428a0c3ae41ce1629b8987d728c46d4abb805d78ce84e1c0a5acd577a292d14e20a4685787fdcceb567a8db04"}else{"000102030405060708090a0b0c0d0e0f0e815c0e31f44ed1694e8eac64ccc36f559ed299a88140f740620a0a0145129df43800dc0e055beeb48ec54e39579cf2"};
        off[8]=stream(&mut b,8,"/Type /ObjStm /N 1 /First 4",&hex(objstm));
        off[10]=b.len();let u=if metadata{U_TRUE}else{U_FALSE};
        let (strf,eff)=if mixed{("Legacy","Identity")}else{("StdCF","StdCF")};
        b.extend(format!("10 0 obj\n<< /Filter /Standard /V 4 /R 4 /Length 128 /P -4 /O <{O}> /U <{u}> /EncryptMetadata {metadata} /CF << /StdCF << /CFM /AESV2 /Length 16 >> /Legacy << /CFM /V2 /Length 16 >> >> /StmF /StdCF /StrF /{strf} /EFF /{eff} >>\nendobj\n").bytes());
        let x=b.len();off[11]=x;let mut rows=vec![];
        for j in 0..12{
            let (kind,pos,g)=match j{0=>(0u8,0u32,65535u16),9=>(2,8,0),5 if !mixed=>(1,off[j] as u32,7),5=>(0,0,0),_=>(1,off[j] as u32,0)};
            rows.push(kind);rows.extend(pos.to_be_bytes());rows.extend(g.to_be_bytes());
        }
        b.extend(format!("11 0 obj\n<< /Type /XRef /Size 12 /Root 1 0 R /Encrypt 10 0 R /ID [<{ID}> <{ID}>] /W [1 4 2] /Length {} >>\nstream\n",rows.len()).bytes());
        b.extend(rows);b.extend(format!("\nendstream\nendobj\nstartxref\n{x}\n%%EOF\n").bytes());b
    }
    fn check(doc:&Document<'_>,mixed:bool){
        assert_eq!(doc.page_count().unwrap(),1);
        let d=crate::dict(doc.resolve(3,0).unwrap()).unwrap();
        assert_eq!(d.get(b"Title".as_slice()),Some(&Object::String(b"AES title".to_vec())));
        assert_eq!(crate::filters::decode(&doc.resolve(4,0).unwrap(),1024).unwrap(),b"0 0 10 10 re f\n");
        if !mixed{assert_eq!(doc.resolve(5,7).unwrap(),Object::String(b"generation seven".to_vec()));}
        for (id,want) in [(6,b"<xml/>".as_slice()),(7,b"attachment".as_slice())]{
            let Object::Stream{data,..}=doc.resolve(id,0).unwrap() else{panic!("stream");};assert_eq!(data,want);
        }
        for _ in 0..2{
            let d=crate::dict(doc.resolve(9,0).unwrap()).unwrap();
            assert_eq!(d.get(b"Label".as_slice()),Some(&Object::String(b"compressed value".to_vec())));
        }
        assert!(doc.resolve(10,0).is_err());assert!(doc.resolve(11,0).is_err());
    }
    #[test]fn r4_aes_metadata_encrypted(){
        let b=fixture(true,false);
        for password in [b"reader".as_slice(),b"owner".as_slice()]{check(&Document::open_with_legacy_password(&b,password).unwrap(),false);}
        assert!(Document::open(&b).is_err());assert!(Document::open_with_legacy_password(&b,b"wrong").is_err());
    }
    #[test]fn r4_aes_metadata_exempt(){
        let b=fixture(false,false);check(&Document::open_with_legacy_password(&b,b"reader").unwrap(),false);
        assert!(Document::open_with_legacy_password(&b,b"wrong").is_err());
    }
    #[test]fn r4_mixed_rc4_aes_identity(){
        let b=fixture(false,true);check(&Document::open_with_legacy_password(&b,b"owner").unwrap(),true);
    }
    #[test]fn r4_rejects_truncated_ciphertext_and_indirect_type(){
        let info=EncryptionInfo{version:4,revision:4,key_bits:128,permissions:-4,encrypt_metadata:false,strings:CryptMethod::Aes128,streams:CryptMethod::Aes128,embedded_files:CryptMethod::Identity};
        let c=ObjectCrypto::with_info([0;16],None,&info).unwrap();
        assert!(c.decrypt(3,0,&mut Object::String(vec![0;31])).is_err());
        let mut d=std::collections::BTreeMap::new();d.insert(b"Type".to_vec(),Object::Ref(9,0));
        assert!(c.decrypt(4,0,&mut Object::Stream{dict:d,data:vec![0;32]}).is_err());
    }
}
