//! Authenticated R2/R3 object decoding; not an encryption writer.
//! No key export, no Debug, no guaranteed zeroization or side-channel audit.
use super::legacy::{md5,Rc4};
use crate::{Error,Object,Result,MAX_DEPTH,MAX_BYTES};

pub(crate) struct ObjectCrypto{key:[u8;16],len:usize,encrypt_ref:Option<(u32,u16)>}
impl ObjectCrypto{
    pub(super) fn new(key:[u8;16],len:usize,encrypt_ref:Option<(u32,u16)>)->Result<Self>{
        if !(5..=16).contains(&len){return Err(Error::Invalid("RC4 file key length"));}
        Ok(Self{key,len,encrypt_ref})
    }
    pub(crate) fn decrypt(&self,id:u32,generation:u16,value:&mut Object)->Result<()>{
        if self.encrypt_ref==Some((id,generation)){return Err(Error::Unsupported("encryption dictionary access"));}
        let mut seed=[0u8;21];
        seed[..self.len].copy_from_slice(&self.key[..self.len]);
        seed[self.len..self.len+3].copy_from_slice(&id.to_le_bytes()[..3]);
        seed[self.len+3..self.len+5].copy_from_slice(&generation.to_le_bytes());
        let key=md5(&seed[..self.len+5])?;
        let mut remaining=MAX_BYTES;
        visit(value,&key[..(self.len+5).min(16)],0,&mut remaining)
    }
}
fn decrypt_bytes(data:&mut [u8],key:&[u8],remaining:&mut usize)->Result<()>{
    *remaining=remaining.checked_sub(data.len()).ok_or(Error::Limit("encrypted object bytes"))?;
    // Each string and stream starts a separate RC4 state, including siblings.
    Rc4::new(key)?.apply(data)
}
fn special_dict(d:&std::collections::BTreeMap<Vec<u8>,Object>)->Result<()>{
    if matches!(d.get(b"Type".as_slice()),Some(Object::Name(n)) if n==b"Sig"||n==b"DocTimeStamp"||n==b"XRef")
        ||(d.contains_key(b"ByteRange".as_slice())&&d.contains_key(b"Contents".as_slice())){
        return Err(Error::Unsupported("encrypted special dictionary"));
    }
    Ok(())
}
fn visit(value:&mut Object,key:&[u8],depth:usize,remaining:&mut usize)->Result<()>{
    if depth>=MAX_DEPTH{return Err(Error::Limit("encrypted object depth"));}
    match value{
        Object::String(data)=>decrypt_bytes(data,key,remaining)?,
        Object::Array(a)=>for v in a{visit(v,key,depth+1,remaining)?;},
        Object::Dict(d)=>{special_dict(d)?;for v in d.values_mut(){visit(v,key,depth+1,remaining)?;}},
        Object::Stream{dict,data}=>{
            special_dict(dict)?;
            // R2/R3 does not support explicit crypt filters. Fail closed.
            match dict.get(b"Filter".as_slice()){
                None|Some(Object::Null)=>{},
                Some(Object::Name(n)) if n!=b"Crypt"=>{},
                Some(Object::Array(a)) if a.iter().all(|v|matches!(v,Object::Name(n) if n!=b"Crypt"))=>{},
                _=>return Err(Error::Unsupported("legacy encrypted stream filter")),
            }
            for v in dict.values_mut(){visit(v,key,depth+1,remaining)?;}
            decrypt_bytes(data,key,remaining)?;
        }
        _=>{},
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
