//! Bounded PDF CMap subset. Parses mapping data, never executes PostScript.
use crate::{Error,Object,Parser,Result};
use std::collections::BTreeMap;
pub const MAX_CMAP_BYTES:usize=4*1024*1024;
const MAX_MAPPINGS:usize=65_536;
const MAX_RANGES:usize=256;
type Key=(u8,u32);
#[derive(Debug,Clone,Default)]
pub struct CMap {
    spaces:Vec<(Key,Key)>,
    unicode:BTreeMap<Key,String>,
    cid:BTreeMap<Key,u32>,
    vertical:bool,
}
#[derive(Debug)]
enum Token { Object(Object), Word(Vec<u8>) }
fn token(p:&mut Parser<'_>)->Result<Option<Token>>{
    p.skip();
    let Some(&b)=p.bytes.get(p.pos) else{return Ok(None)};
    if matches!(b,b'/'|b'('|b'<'|b'['){
        return Ok(Some(Token::Object(p.object(0)?)));
    }
    if matches!(b,b'{'|b'}'){return Err(Error::Unsupported("CMap procedure"));}
    Ok(Some(Token::Word(p.word()?.to_vec())))
}
fn next(p:&mut Parser<'_>)->Result<Token>{token(p)?.ok_or(Error::Invalid("truncated CMap"))}
fn bytes(p:&mut Parser<'_>)->Result<Vec<u8>>{
    match next(p)?{Token::Object(Object::String(s))=>Ok(s),_=>Err(Error::Invalid("CMap string"))}
}
fn key(s:&[u8])->Result<Key>{
    if s.is_empty()||s.len()>4{return Err(Error::Invalid("CMap code length"));}
    Ok((s.len() as u8,s.iter().fold(0u32,|v,&b|(v<<8)|u32::from(b))))
}
fn range(p:&mut Parser<'_>)->Result<(Key,Key)>{
    let a=key(&bytes(p)?)?;let b=key(&bytes(p)?)?;
    if a.0!=b.0||a.1>b.1{return Err(Error::Invalid("CMap code range"));}
    Ok((a,b))
}
fn integer(t:Token)->Result<usize>{
    if let Token::Word(w)=t {
        if !w.is_empty()&&w.iter().all(u8::is_ascii_digit){
            if let Ok(n)=std::str::from_utf8(&w).unwrap_or("").parse::<usize>(){return Ok(n);}
        }
    }
    Err(Error::Invalid("CMap integer"))
}
fn unicode(s:&[u8])->Result<String>{
    if s.is_empty()||s.len()>128||s.len()%2!=0{return Err(Error::Invalid("CMap UTF-16BE length"));}
    let units:Vec<_>=s.chunks_exact(2).map(|a|u16::from_be_bytes([a[0],a[1]])).collect();
    String::from_utf16(&units).map_err(|_|Error::Invalid("CMap UTF-16BE"))
}
fn increment(s:&mut [u8])->Result<()>{
    for b in s.iter_mut().rev(){let (n,carry)=b.overflowing_add(1);*b=n;if !carry{return Ok(());}}
    Err(Error::Invalid("CMap range destination overflow"))
}
fn expect(p:&mut Parser<'_>,w:&[u8])->Result<()>{
    if matches!(next(p)?,Token::Word(v) if v==w){Ok(())}else{Err(Error::Invalid("CMap section terminator"))}
}
impl CMap {
    pub fn parse(data:&[u8])->Result<Self>{
        if data.len()>MAX_CMAP_BYTES{return Err(Error::Limit("CMap bytes"));}
        let mut out=Self::default();
        let mut p=Parser::new(data,0);
        let mut count=None;
        let mut wmode=false;
        while let Some(t)=token(&mut p)?{
            let w=match t{
                Token::Object(Object::Name(n))=>{wmode=n==b"WMode";count=None;continue;}
                Token::Word(w)=>w,
                _=>{wmode=false;count=None;continue;}
            };
            if w==b"usecmap"{return Err(Error::Unsupported("CMap usecmap"));}
            if wmode{
                out.vertical=match w.as_slice(){b"0"=>false,b"1"=>true,_=>return Err(Error::Invalid("CMap WMode"))};
                wmode=false;
            }
            let end=match w.as_slice(){
                b"begincodespacerange"=>b"endcodespacerange".as_slice(),
                b"beginbfchar"=>b"endbfchar",
                b"beginbfrange"=>b"endbfrange",
                b"begincidchar"=>b"endcidchar",
                b"begincidrange"=>b"endcidrange",
                b"beginnotdefchar"|b"beginnotdefrange"=>return Err(Error::Unsupported("CMap notdef mapping")),
                _=>{count=integer(Token::Word(w)).ok();continue;}
            };
            let n=count.take().ok_or(Error::Invalid("CMap section count"))?;
            if n>MAX_MAPPINGS{return Err(Error::Limit("CMap section count"));}
            for _ in 0..n{
                match w.as_slice(){
                    b"begincodespacerange"=>{
                        if out.spaces.len()>=MAX_RANGES{return Err(Error::Limit("CMap codespaces"));}
                        let r=range(&mut p)?;
                        // Reject ambiguous overlapping ranges, including prefixes of longer codes.
                        for &(a,b) in &out.spaces{
                            let (c,d)=r;
                            let overlap=if a.0<=c.0{
                                let shift=8*u32::from(c.0-a.0);
                                a.1<=d.1>>shift && c.1>>shift<=b.1
                            }else{
                                let shift=8*u32::from(a.0-c.0);
                                c.1<=b.1>>shift && a.1>>shift<=d.1
                            };
                            if overlap{return Err(Error::Invalid("overlapping CMap codespaces"));}
                        }
                        out.spaces.push(r);
                    }
                    b"beginbfchar"=>{let k=key(&bytes(&mut p)?)?;let v=unicode(&bytes(&mut p)?)?;out.add_unicode(k,v)?;}
                    b"beginbfrange"=>{
                        let (a,b)=range(&mut p)?;
                        let n=u64::from(b.1)-u64::from(a.1)+1;
                        if n>MAX_MAPPINGS.saturating_sub(out.unicode.len()) as u64{return Err(Error::Limit("CMap mappings"));}
                        match next(&mut p)?{
                            Token::Object(Object::String(mut s))=>{
                                for i in a.1..=b.1{
                                    out.add_unicode((a.0,i),unicode(&s)?)?;
                                    if i!=b.1{increment(&mut s)?;}
                                }
                            }
                            Token::Object(Object::Array(items)) if items.len()==n as usize=>{
                                for (i,v) in (a.1..=b.1).zip(items){
                                    let Object::String(s)=v else{return Err(Error::Invalid("CMap range array"));};
                                    out.add_unicode((a.0,i),unicode(&s)?)?;
                                }
                            }
                            _=>return Err(Error::Invalid("CMap range destination")),
                        }
                    }
                    b"begincidchar"=>{let k=key(&bytes(&mut p)?)?;let cid=integer(next(&mut p)?)?;out.add_cid(k,cid)?;}
                    b"begincidrange"=>{
                        let (a,b)=range(&mut p)?;
                        let cid=integer(next(&mut p)?)?;
                        let n=u64::from(b.1)-u64::from(a.1)+1;
                        if n>MAX_MAPPINGS.saturating_sub(out.cid.len()) as u64{return Err(Error::Limit("CMap CID mappings"));}
                        for (i,c) in (a.1..=b.1).enumerate(){
                            out.add_cid((a.0,c),cid.checked_add(i).ok_or(Error::Limit("CMap CID"))?)?;
                        }
                    }
                    _=>unreachable!()
                }
            }
            expect(&mut p,end)?;
        }
        if out.spaces.is_empty(){return Err(Error::Invalid("missing CMap codespaces"));}
        for &k in out.unicode.keys().chain(out.cid.keys()){
            if !out.contains(k){return Err(Error::Invalid("CMap code outside codespace"));}
        }
        Ok(out)
    }
    fn add_unicode(&mut self,k:Key,v:String)->Result<()>{
        if self.unicode.len()>=MAX_MAPPINGS{return Err(Error::Limit("CMap mappings"));}
        if self.unicode.insert(k,v).is_some(){return Err(Error::Invalid("duplicate CMap mapping"));}
        Ok(())
    }
    fn add_cid(&mut self,k:Key,v:usize)->Result<()>{
        if self.cid.len()>=MAX_MAPPINGS{return Err(Error::Limit("CMap CID mappings"));}
        if v>65535{return Err(Error::Invalid("CID range"));}
        if self.cid.insert(k,v as u32).is_some(){return Err(Error::Invalid("duplicate CID mapping"));}
        Ok(())
    }
    fn contains(&self,k:Key)->bool{self.spaces.iter().any(|&(a,b)|a.0==k.0&&a.1<=k.1&&k.1<=b.1)}
    pub fn code_len(&self,input:&[u8])->Result<usize>{
        for n in 1..=input.len().min(4){
            if self.contains(key(&input[..n])?){return Ok(n);}
        }
        Err(Error::Invalid("character outside CMap codespace"))
    }
    pub fn unicode(&self,code:&[u8])->Option<&str>{self.unicode.get(&key(code).ok()?).map(String::as_str)}
    pub fn cid(&self,code:&[u8])->Option<u32>{self.cid.get(&key(code).ok()?).copied()}
    pub fn vertical(&self)->bool{self.vertical}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn unicode_ranges(){
        let c=CMap::parse(b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap 2 begincodespacerange <00> <7f> <8100> <81ff> endcodespacerange 1 beginbfchar <41> <4e2d> endbfchar 1 beginbfrange <42> <43> <0061> endbfrange 1 beginbfrange <8100> <8101> [<D83DDE00> <00660069>] endbfrange endcmap end end").unwrap();
        assert_eq!(c.unicode(b"A"),Some("中"));assert_eq!(c.unicode(b"C"),Some("b"));
        assert_eq!(c.unicode(&[0x81,0]),Some("😀"));assert_eq!(c.unicode(&[0x81,1]),Some("fi"));
        assert_eq!(c.code_len(&[0x81,1]).unwrap(),2);assert!(c.code_len(&[0x81]).is_err());
    }
    #[test]
    fn rejects_malformed(){
        for s in [
            b"1 begincodespacerange <00> <ff> endcodespacerange 1 beginbfchar <41> <d800> endbfchar".as_slice(),
            b"1 begincodespacerange <00> <ff> endcodespacerange 2 beginbfchar <41> <0041> endbfchar",
            b"2 begincodespacerange <00> <ff> <8100> <81ff> endcodespacerange",
            b"1 begincodespacerange <00000000> <ffffffff> endcodespacerange 1 beginbfrange <00000000> <ffffffff> <0041> endbfrange",
            b"/Identity-H usecmap",
        ]{assert!(CMap::parse(s).is_err());}
    }
}
