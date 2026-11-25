//! Structural inspection and authenticated R2/R3/R4 RC4/AESV2 decoding.
//! Only the explicit authenticated open path exposes supported encrypted documents. EncryptionInfo intentionally excludes O/U/OE/UE/Perms and IDs.
mod legacy;
mod auth;
mod object_crypto;
// Block encryption is retained for primitive tests and future R6 integration.
#[allow(dead_code)]
mod aes;
// Private digest primitives until modern password authentication is integrated.
#[allow(dead_code)]
mod sha2;
pub(crate) use object_crypto::ObjectCrypto;
pub use auth::PasswordStatus;
use crate::{Document,Error,Object,Result,xref::Entry};
use std::collections::BTreeMap;
type Dict=BTreeMap<Vec<u8>,Object>;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CryptMethod{Identity,Rc4,Aes128,Aes256}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct EncryptionInfo{
    pub version:u8,
    pub revision:u8,
    pub key_bits:u16,
    pub permissions:i32,
    pub encrypt_metadata:bool,
    pub strings:CryptMethod,
    pub streams:CryptMethod,
    pub embedded_files:CryptMethod,
}

fn integer(d:&Dict,key:&[u8],label:&'static str)->Result<i32>{
    match d.get(key){
        Some(Object::Number(n)) if n.is_finite()&&n.fract()==0.&&*n>=i32::MIN as f64&&*n<=i32::MAX as f64=>Ok(*n as i32),
        _=>Err(Error::Invalid(label)),
    }
}
fn optional_integer(d:&Dict,key:&[u8],default:i32,label:&'static str)->Result<i32>{
    if d.contains_key(key){integer(d,key,label)}else{Ok(default)}
}
fn name<'a>(d:&'a Dict,key:&[u8],default:Option<&'a [u8]>,label:&'static str)->Result<&'a [u8]>{
    match d.get(key){Some(Object::Name(n))=>Ok(n),None=>default.ok_or(Error::Invalid(label)),_=>Err(Error::Invalid(label))}
}
fn string_len(d:&Dict,key:&[u8],len:usize)->Result<()>{
    if matches!(d.get(key),Some(Object::String(s)) if s.len()==len){Ok(())}
    else{Err(Error::Invalid("encryption authentication field length"))}
}
fn filter(d:&Dict,key:&[u8],default:&[u8],version:i32,bits:i32)->Result<CryptMethod>{
    let selected=name(d,key,Some(default),"crypt filter name")?;
    if selected==b"Identity"{return Ok(CryptMethod::Identity);}
    let Some(Object::Dict(cf))=d.get(b"CF".as_slice()) else{return Err(Error::Invalid("crypt filter dictionary"));};
    let Some(Object::Dict(entry))=cf.get(selected) else{return Err(Error::Invalid("missing crypt filter"));};
    let method=name(entry,b"CFM",Some(b"None"),"crypt filter method")?;
    let event=name(entry,b"AuthEvent",Some(b"DocOpen"),"crypt filter auth event")?;
    if event!=b"DocOpen"&&(event!=b"EFOpen"||key!=b"EFF"){return Err(Error::Unsupported("crypt filter auth event"));}
    let length=optional_integer(entry,b"Length",bits/8,"crypt filter Length")?;
    match method{
        b"None"=>Ok(CryptMethod::Identity),
        b"V2" if version==4&&(5..=16).contains(&length)&&length*8==bits=>Ok(CryptMethod::Rc4),
        b"AESV2" if version==4&&bits==128&&length==16=>Ok(CryptMethod::Aes128),
        b"AESV3" if version==5&&bits==256&&length==32=>Ok(CryptMethod::Aes256),
        b"V2"|b"AESV2"|b"AESV3"=>Err(Error::Invalid("crypt filter key length/version")),
        _=>Err(Error::Unsupported("crypt filter method")),
    }
}
fn parse(d:&Dict)->Result<EncryptionInfo>{
    if name(d,b"Filter",None,"encryption Filter")?!=b"Standard"{return Err(Error::Unsupported("non-Standard encryption handler"));}
    if d.contains_key(b"SubFilter".as_slice()){return Err(Error::Unsupported("encryption SubFilter"));}
    let v=integer(d,b"V","encryption V")?;
    let r=integer(d,b"R","encryption R")?;
    let bits=optional_integer(d,b"Length",if v==5{256}else{40},"encryption Length")?;
    match (v,r){
        (1,2) if bits==40=>{},
        (2,3) if (40..=128).contains(&bits)&&bits%8==0=>{},
        (4,4) if (40..=128).contains(&bits)&&bits%8==0=>{},
        (5,5|6) if bits==256=>{},
        (1,2)|(2,3)|(4,4)|(5,5|6)=>return Err(Error::Invalid("encryption key length")),
        _=>return Err(Error::Unsupported("encryption V/R combination")),
    }
    let permissions=integer(d,b"P","encryption P")?;
    let encrypt_metadata=match d.get(b"EncryptMetadata".as_slice()){
        None|Some(Object::Bool(true))=>true,
        Some(Object::Bool(false)) if v>=4=>false,
        _=>return Err(Error::Invalid("encryption EncryptMetadata")),
    };
    string_len(d,b"O",if r>=5{48}else{32})?;
    string_len(d,b"U",if r>=5{48}else{32})?;
    if r>=5{string_len(d,b"OE",32)?;string_len(d,b"UE",32)?;string_len(d,b"Perms",16)?;}
    let (strings,streams,embedded_files)=if v>=4{
        if let Some(value)=d.get(b"CF".as_slice()){
            let Object::Dict(cf)=value else{return Err(Error::Invalid("crypt filter dictionary"));};
            if cf.len()>64{return Err(Error::Limit("crypt filters"));}
        }
        let st=name(d,b"StmF",Some(b"Identity"),"stream crypt filter")?;
        (filter(d,b"StrF",b"Identity",v,bits)?,filter(d,b"StmF",b"Identity",v,bits)?,filter(d,b"EFF",st,v,bits)?)
    }else{(CryptMethod::Rc4,CryptMethod::Rc4,CryptMethod::Rc4)};
    Ok(EncryptionInfo{version:v as u8,revision:r as u8,key_bits:bits as u16,permissions,encrypt_metadata,strings,streams,embedded_files})
}

// Newest-first trailers. Retain only bounded first IDs and the encryption
// reference/dictionary. Reject conflicting security metadata conservatively;
// the second ID may change in an incremental save. No raw IDs in diagnostics.
#[derive(Default)]
pub(crate) struct TrailerSecurity{
    encrypt:Option<Object>,first_id:Option<Vec<u8>>,invalid_id:bool,conflicting_id:bool,
}
impl TrailerSecurity{
    pub(crate) fn observe(&mut self,d:&Dict)->Result<()>{
        if let Some(e)=d.get(b"Encrypt".as_slice()){
            if let Some(old)=&self.encrypt{if old!=e{return Err(Error::Unsupported("conflicting encryption dictionaries"));}}
            else{self.encrypt=Some(e.clone());}
        }
        if let Some(id)=d.get(b"ID".as_slice()){
            match id{
                Object::Array(a) if a.len()==2=>match (&a[0],&a[1]){
                    (Object::String(first),Object::String(second)) if !first.is_empty()&&!second.is_empty()&&first.len()<=1024&&second.len()<=1024=>{
                        if let Some(old)=&self.first_id{if old!=first{self.conflicting_id=true;}}
                        else{self.first_id=Some(first.clone());}
                    }
                    _=>self.invalid_id=true,
                },
                _=>self.invalid_id=true,
            }
        }
        Ok(())
    }
    fn bootstrap(self,doc:&Document<'_>)->Result<Option<(Dict,Vec<u8>)>>{
        let Some(e)=self.encrypt else{return Ok(None);};
        if self.invalid_id||self.first_id.is_none(){return Err(Error::Invalid("encrypted document ID"));}
        if self.conflicting_id{return Err(Error::Unsupported("conflicting encryption document ID"));}
        let d=match e{
            Object::Dict(d)=>d,
            Object::Ref(id,g)=>{
                // Encryption dictionary must be a direct (not compressed)
                // object. Resolve exactly one unencrypted bootstrap object.
                if !matches!(doc.entries.get(&id),Some(Entry::Direct{generation,..}) if *generation==g){
                    return Err(Error::Invalid("encryption dictionary reference"));
                }
                match doc.resolve(id,g)?{Object::Dict(d)=>d,_=>return Err(Error::Invalid("encryption dictionary"))}
            }
            _=>return Err(Error::Invalid("encryption dictionary")),
        };
        Ok(Some((d,self.first_id.ok_or(Error::Invalid("encrypted document ID"))?)))
    }
    pub(crate) fn inspect(self,doc:&Document<'_>)->Result<Option<EncryptionInfo>>{
        self.bootstrap(doc)?.map(|(d,_)|parse(&d)).transpose()
    }
    pub(crate) fn unlock(self,doc:&Document<'_>,password:&[u8])->Result<Option<ObjectCrypto>>{
        let reference=match &self.encrypt{Some(Object::Ref(id,g))=>Some((*id,*g)),_=>None};
        let Some((d,id))=self.bootstrap(doc)? else{return Ok(None);};
        let info=parse(&d)?;
        let (role,key)=auth::authenticate(&d,&id,&info,password)?;
        let key=key.ok_or(Error::Invalid("incorrect PDF password"))?;
        if role==PasswordStatus::User{
            let mask=if info.revision==2{0x3c}else{0xf3c};
            if info.permissions&mask!=mask{return Err(Error::Unsupported("restricted PDF user permissions"));}
        }
        if info.revision==4{
            // Separate embedded-file authentication is not yet implemented.
            if let Some(Object::Dict(cf))=d.get(b"CF".as_slice()){
                if cf.values().any(|v|matches!(v,Object::Dict(e) if matches!(e.get(b"AuthEvent".as_slice()),Some(Object::Name(n)) if n==b"EFOpen"))){
                    return Err(Error::Unsupported("separate embedded file authentication"));
                }
            }
        }
        Ok(Some(ObjectCrypto::with_info(key,reference,&info)?))
    }
    pub(crate) fn authenticate(self,doc:&Document<'_>,password:&[u8])->Result<PasswordStatus>{
        let Some((d,id))=self.bootstrap(doc)? else{return Ok(PasswordStatus::Unencrypted);};
        let info=parse(&d)?;
        auth::verify(&d,&id,&info,password)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn standard(v:i32,r:i32,length:i32,extra:&str)->String{
        let n=if r>=5{48}else{32};
        let modern=if r>=5{format!("/OE <{}> /UE <{}> /Perms <{}>","00".repeat(32),"00".repeat(32),"00".repeat(16))}else{String::new()};
        format!("<< /Filter /Standard /V {v} /R {r} /Length {length} /P -4 /O <{}> /U <{}> {modern} {extra} >>","00".repeat(n),"00".repeat(n))
    }
    fn parse_text(s:&str)->Result<EncryptionInfo>{parse(&crate::dict(crate::parse_object(s.as_bytes())?)?)}
    fn classic(encrypt:&str,id:&str)->Vec<u8>{
        let mut b=b"%PDF-1.7\n".to_vec();
        let mut offsets=vec![];
        for (i,obj) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 0 /Kids [] >>",encrypt].into_iter().enumerate(){
            offsets.push(b.len());b.extend(format!("{} 0 obj\n{obj}\nendobj\n",i+1).bytes());
        }
        let x=b.len();b.extend(b"xref\n0 4\n0000000000 65535 f \n");
        for off in offsets{b.extend(format!("{off:010} 00000 n \n").bytes());}
        b.extend(format!("trailer\n<< /Size 4 /Root 1 0 R /Encrypt 3 0 R {id} >>\nstartxref\n{x}\n%%EOF\n").bytes());b
    }
    const ID:&str="/ID [<01020304> <05060708>]";
    #[test]fn legacy_rc4(){
        for (v,r,bits) in [(1,2,40),(2,3,128)]{
            let info=parse_text(&standard(v,r,bits,"")).unwrap();
            assert_eq!(info.key_bits,bits as u16);assert_eq!(info.streams,CryptMethod::Rc4);
        }
    }
    #[test]fn aes128_and_identity(){
        let s=standard(4,4,128,"/CF << /StdCF << /CFM /AESV2 /Length 16 >> >> /StmF /StdCF /StrF /Identity /EncryptMetadata false");
        let i=parse_text(&s).unwrap();assert_eq!(i.streams,CryptMethod::Aes128);assert_eq!(i.embedded_files,CryptMethod::Aes128);assert_eq!(i.strings,CryptMethod::Identity);assert!(!i.encrypt_metadata);
    }
    #[test]fn aes256_revisions(){
        for r in [5,6]{
            let i=parse_text(&standard(5,r,256,"/CF << /StdCF << /CFM /AESV3 /Length 32 >> >> /StmF /StdCF /StrF /StdCF")).unwrap();
            assert_eq!(i.streams,CryptMethod::Aes256);assert_eq!(i.strings,CryptMethod::Aes256);
        }
    }
    #[test]fn invalid_and_unsupported_fields(){
        for s in [standard(2,3,41,""),standard(5,6,128,""),standard(4,4,128,"/StmF /Missing"),standard(4,4,128,"/CF << /Bad << /CFM /AESV2 /Length 5 >> >> /StmF /Bad"),standard(1,2,40,"").replace("/Standard","/Adobe.PubSec"),standard(5,6,256,"").replace("/Perms <","/Other <")]{
            assert!(parse_text(&s).is_err(),"{s}");
        }
    }
    #[test]fn inspect_classic_but_open_rejects(){
        let b=classic(&standard(1,2,40,""),ID);
        assert_eq!(Document::encryption_info(&b).unwrap().unwrap().revision,2);
        assert!(matches!(Document::open(&b),Err(Error::Unsupported("encryption"))));
        let b=classic(&standard(1,2,40,""),"");
        assert!(matches!(Document::encryption_info(&b),Err(Error::Invalid("encrypted document ID"))));
        let b=classic("42",ID);assert!(Document::encryption_info(&b).is_err());
    }
    #[test]fn incremental_identifier_policy(){
        let mut b=classic(&standard(1,2,40,""),ID);
        let start=b.windows(9).rposition(|w|w==b"startxref").unwrap();
        let mut p=crate::Parser::new(&b,start+9);let prev=p.uint().unwrap();
        let x=b.len();b.extend(format!("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 4 /Root 1 0 R /Prev {prev} /ID [<01020304> <9999>] >>\nstartxref\n{x}\n%%EOF\n").bytes());
        assert_eq!(Document::encryption_info(&b).unwrap().unwrap().revision,2);
        assert!(Document::open(&b).is_err());
        let x=b.len();b.extend(format!("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 4 /Root 1 0 R /Prev {prev} /ID [<FFFFFFFF> <9999>] >>\nstartxref\n{x}\n%%EOF\n").bytes());
        assert!(matches!(Document::encryption_info(&b),Err(Error::Unsupported("conflicting encryption document ID"))));
    }
    #[test]fn xref_stream_inspection(){
        let mut b=b"%PDF-1.7\n".to_vec();
        let mut offsets=vec![];
        let enc=standard(1,2,40,"");
        for (i,obj) in ["<< /Type /Catalog /Pages 2 0 R >>","<< /Type /Pages /Count 0 /Kids [] >>",enc.as_str()].into_iter().enumerate(){
            offsets.push(b.len());b.extend(format!("{} 0 obj\n{obj}\nendobj\n",i+1).bytes());
        }
        let x=b.len();offsets.push(x);
        let mut rows=vec![0,0,0,0,0,255,255];
        for off in offsets{rows.push(1);rows.extend((off as u32).to_be_bytes());rows.extend([0,0]);}
        b.extend(format!("4 0 obj\n<< /Type /XRef /Size 5 /Root 1 0 R /W [1 4 2] /Encrypt 3 0 R {ID} /Length {} >>\nstream\n",rows.len()).bytes());
        b.extend(rows);b.extend(format!("\nendstream\nendobj\nstartxref\n{x}\n%%EOF\n").bytes());
        assert_eq!(Document::encryption_info(&b).unwrap().unwrap().revision,2);
        assert!(matches!(Document::open(&b),Err(Error::Unsupported("encryption"))));
    }
    #[test]fn plain_document_unchanged(){
        let mut b=b"%PDF-1.4\n".to_vec();let off=b.len();
        b.extend(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let off2=b.len();b.extend(b"2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n");
        let x=b.len();b.extend(format!("xref\n0 3\n0000000000 65535 f \n{off:010} 00000 n \n{off2:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").bytes());
        assert_eq!(Document::encryption_info(&b).unwrap(),None);
        assert_eq!(Document::open(&b).unwrap().page_count().unwrap(),0);
    }
}
