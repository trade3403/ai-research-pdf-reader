//! PDF font dictionaries, character mapping and horizontal advances.
//! This is not a TrueType/CFF outline renderer.
use crate::{Document,Error,Object,PageInfo,Result};
use std::collections::BTreeMap;
pub mod cmap;
use cmap::CMap;
const MAX_GLYPHS:usize=100_000;
type Dict=BTreeMap<Vec<u8>,Object>;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FontKind{Type1,TrueType,Type0Cid0,Type0Cid2}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum WritingMode{Horizontal,Vertical}
#[derive(Debug,Clone,PartialEq)]
pub struct Glyph{
    pub code:Vec<u8>,
    pub cid:Option<u32>,
    pub glyph_id:Option<u16>,
    pub unicode:Option<String>,
    /// Horizontal advance in 1/1000 text space units; None means unknown.
    pub width:Option<f64>,
}
#[derive(Debug,Clone)]
enum Encoding{Simple(Simple),Identity,Custom(CMap)}
#[derive(Debug,Clone,Copy)]
enum Base{Unknown,Standard,WinAnsi}
#[derive(Debug,Clone)]
struct Simple{base:Base,differences:BTreeMap<u8,Option<String>>}
#[derive(Debug,Clone)]
enum Gid{Unknown,Identity,Map(Vec<u8>)}
#[derive(Debug,Clone)]
pub struct Font{
    pub kind:FontKind,
    pub base_font:Option<Vec<u8>>,
    pub writing_mode:WritingMode,
    encoding:Encoding,
    to_unicode:Option<CMap>,
    widths:BTreeMap<u32,f64>,
    default_width:Option<f64>,
    gid:Gid,
}
fn get<'a>(d:&'a Dict,k:&[u8])->Option<&'a Object>{d.get(k)}
fn name(o:Option<&Object>)->Option<&[u8]>{if let Some(Object::Name(n))=o{Some(n)}else{None}}
fn integer(o:&Object,max:u32)->Result<u32>{
    if let Object::Number(n)=o{if *n>=0.&&*n<=f64::from(max)&&n.fract()==0.{return Ok(*n as u32);}}
    Err(Error::Invalid("font integer"))
}
fn width(o:&Object)->Result<f64>{
    if let Object::Number(n)=o{if n.is_finite()&&n.abs()<=1e7{return Ok(*n);}}
    Err(Error::Invalid("font width"))
}
fn dictionary(o:Object)->Result<Dict>{crate::dict(o)}
fn base(n:Option<&[u8]>)->Base{
    match n{Some(b"WinAnsiEncoding")=>Base::WinAnsi,Some(b"StandardEncoding")=>Base::Standard,_=>Base::Unknown}
}
fn glyph_name(n:&[u8])->Option<String>{
    let n=n.split(|&b|b==b'.').next()?;
    if n.len()==1&&n[0].is_ascii_alphabetic(){return Some(char::from(n[0]).to_string());}
    let special=match n{
        b"space"=>" ",b"hyphen"=>"-",b"minus"=>"−",b"period"=>".",b"comma"=>",",
        b"zero"=>"0",b"one"=>"1",b"two"=>"2",b"three"=>"3",b"four"=>"4",b"five"=>"5",b"six"=>"6",b"seven"=>"7",b"eight"=>"8",b"nine"=>"9",
        b"fi"=>"fi",b"fl"=>"fl",b"ffi"=>"ffi",b"ffl"=>"ffl",b"ff"=>"ff",
        b"Euro"=>"€",b"bullet"=>"•",b"endash"=>"–",b"emdash"=>"—",b"quotesingle"=>"'",b"quoteleft"=>"‘",b"quoteright"=>"’",
        b"quotedbl"=>"\"",b"quotedblleft"=>"“",b"quotedblright"=>"”",b"alpha"=>"α",b"beta"=>"β",b"gamma"=>"γ",b"Delta"=>"∆",b"Omega"=>"Ω",
        b"plus"=>"+",b"equal"=>"=",b"parenleft"=>"(",b"parenright"=>")",b"slash"=>"/",b"colon"=>":",b"semicolon"=>";",
        _=>{
            let s=std::str::from_utf8(n).ok()?;
            if let Some(h)=s.strip_prefix("uni"){
                if h.is_empty()||h.len()%4!=0||h.len()>64{return None;}
                let mut units=Vec::new();
                for p in h.as_bytes().chunks_exact(4){
                    units.push(u16::from_str_radix(std::str::from_utf8(p).ok()?,16).ok()?);
                }
                // Adobe uni names represent BMP scalar sequences, not surrogate pairs.
                if units.iter().any(|u|(0xd800..=0xdfff).contains(u)){return None;}
                return String::from_utf16(&units).ok();
            }
            if let Some(h)=s.strip_prefix('u'){
                if (4..=6).contains(&h.len()){return char::from_u32(u32::from_str_radix(h,16).ok()?).map(|c|c.to_string());}
            }
            return None;
        }
    };
    Some(special.to_owned())
}
fn base_char(base:Base,c:u8)->Option<String>{
    let ch=match base{
        Base::Unknown=>return None,
        Base::Standard=>match c{39=>'’',96=>'‘',32..=126=>char::from(c),_=>return None},
        Base::WinAnsi=>match c{
            32..=126|160..=255=>char::from(c),
            128=>'€',130=>'‚',131=>'ƒ',132=>'„',133=>'…',134=>'†',135=>'‡',136=>'ˆ',137=>'‰',138=>'Š',139=>'‹',140=>'Œ',142=>'Ž',
            145=>'‘',146=>'’',147=>'“',148=>'”',149=>'•',150=>'–',151=>'—',152=>'˜',153=>'™',154=>'š',155=>'›',156=>'œ',158=>'ž',159=>'Ÿ',_=>return None
        }
    };
    Some(ch.to_string())
}
impl Simple{
    fn parse(doc:&Document<'_>,o:Option<&Object>,fallback:Base)->Result<Self>{
        let mut out=Self{base:fallback,differences:BTreeMap::new()};
        if let Some(o)=o{
            match doc.dereference(o)?{
                Object::Name(n)=>out.base=base(Some(&n)),
                Object::Dict(d)=>{
                    if let Some(v)=get(&d,b"BaseEncoding"){out.base=base(name(Some(&doc.dereference(v)?)));}
                    if let Some(v)=get(&d,b"Differences"){
                        let Object::Array(items)=doc.dereference(v)? else{return Err(Error::Invalid("font Differences"));};
                        if items.len()>1024{return Err(Error::Limit("font Differences"));}
                        let mut next=None;
                        for item in items{
                            match item{
                                Object::Number(_)=>next=Some(integer(&item,255)?),
                                Object::Name(n)=>{
                                    let i=next.ok_or(Error::Invalid("font Differences start"))?;
                                    if i>255{return Err(Error::Invalid("font Differences range"));}
                                    out.differences.insert(i as u8,glyph_name(&n));next=Some(i+1);
                                }
                                _=>return Err(Error::Invalid("font Differences item")),
                            }
                        }
                    }
                }
                _=>return Err(Error::Invalid("font Encoding")),
            }
        }
        Ok(out)
    }
    fn unicode(&self,c:u8)->Option<String>{
        match self.differences.get(&c){Some(v)=>v.clone(),None=>base_char(self.base,c)}
    }
}
fn cid_widths(doc:&Document<'_>,d:&Dict)->Result<(BTreeMap<u32,f64>,Option<f64>)>{
    let default=match get(d,b"DW"){Some(v)=>width(&doc.dereference(v)?)?,None=>1000.};
    let mut widths=BTreeMap::new();
    if let Some(w)=get(d,b"W"){
        let Object::Array(items)=doc.dereference(w)? else{return Err(Error::Invalid("CID W"));};
        let mut it=items.iter();
        while let Some(first)=it.next(){
            let start=integer(first,65535)?;
            let second=it.next().ok_or(Error::Invalid("CID W entry"))?;
            match second{
                Object::Array(a)=>{
                    if a.is_empty()||a.len()>65536-start as usize{return Err(Error::Invalid("CID W range"));}
                    for (i,v) in a.iter().enumerate(){
                        if widths.insert(start+i as u32,width(v)?).is_some(){return Err(Error::Invalid("overlapping CID widths"));}
                    }
                }
                Object::Number(_)=>{
                    let end=integer(second,65535)?;
                    if end<start{return Err(Error::Invalid("CID W range"));}
                    let w=width(it.next().ok_or(Error::Invalid("CID W width"))?)?;
                    for i in start..=end{if widths.insert(i,w).is_some(){return Err(Error::Invalid("overlapping CID widths"));}}
                }
                _=>return Err(Error::Invalid("CID W entry")),
            }
        }
    }
    Ok((widths,Some(default)))
}
fn simple_widths(doc:&Document<'_>,d:&Dict)->Result<(BTreeMap<u32,f64>,Option<f64>)>{
    let mut widths=BTreeMap::new();
    if let Some(w)=get(d,b"Widths"){
        let Object::Array(items)=doc.dereference(w)? else{return Err(Error::Invalid("font Widths"));};
        let first=integer(&doc.dereference(get(d,b"FirstChar").ok_or(Error::Invalid("FirstChar"))?)?,255)?;
        let last=integer(&doc.dereference(get(d,b"LastChar").ok_or(Error::Invalid("LastChar"))?)?,255)?;
        if last<first||items.len()!=1+(last-first) as usize{return Err(Error::Invalid("font Widths range"));}
        for (i,v) in items.iter().enumerate(){widths.insert(first+i as u32,width(v)?);}
    }
    let default=if let Some(v)=get(d,b"FontDescriptor"){
        let desc=dictionary(doc.dereference(v)?)?;
        get(&desc,b"MissingWidth").map(|v|doc.dereference(v).and_then(|v|width(&v))).transpose()?
    }else{None};
    Ok((widths,default))
}
fn load_cmap(doc:&Document<'_>,o:&Object)->Result<CMap>{
    let stream=doc.dereference(o)?;
    if let Object::Stream{dict,..}=&stream{
        if get(dict,b"UseCMap").is_some(){return Err(Error::Unsupported("CMap UseCMap"));}
    }
    CMap::parse(&crate::filters::decode(&stream,cmap::MAX_CMAP_BYTES)?)
}
impl Font{
    pub fn load(doc:&Document<'_>,o:&Object)->Result<Self>{
        let d=dictionary(doc.dereference(o)?)?;
        let base_font=name(get(&d,b"BaseFont")).map(<[u8]>::to_vec);
        let to_unicode=get(&d,b"ToUnicode").map(|v|load_cmap(doc,v)).transpose()?;
        let (kind,encoding,writing_mode,widths,default_width,gid)=match name(get(&d,b"Subtype")){
            Some(b"Type1"|b"MMType1"|b"TrueType")=>{
                let kind=if name(get(&d,b"Subtype"))==Some(b"TrueType"){FontKind::TrueType}else{FontKind::Type1};
                // No invented built-in font encoding. Explicit encoding or ToUnicode only.
                let encoding=Encoding::Simple(Simple::parse(doc,get(&d,b"Encoding"),Base::Unknown)?);
                let (w,dw)=simple_widths(doc,&d)?;
                (kind,encoding,WritingMode::Horizontal,w,dw,Gid::Unknown)
            }
            Some(b"Type0")=>{
                let e=doc.dereference(get(&d,b"Encoding").ok_or(Error::Invalid("Type0 Encoding"))?)?;
                let (encoding,vertical)=match &e{
                    Object::Name(n) if n==b"Identity-H"||n==b"Identity-V"=>(Encoding::Identity,n==b"Identity-V"),
                    Object::Stream{..}=>{let c=load_cmap(doc,&e)?;let v=c.vertical();(Encoding::Custom(c),v)},
                    _=>return Err(Error::Unsupported("predefined CID CMap")),
                };
                let Object::Array(a)=doc.dereference(get(&d,b"DescendantFonts").ok_or(Error::Invalid("DescendantFonts"))?)? else{return Err(Error::Invalid("DescendantFonts"));};
                if a.len()!=1{return Err(Error::Invalid("DescendantFonts count"));}
                let desc=dictionary(doc.dereference(&a[0])?)?;
                let kind=match name(get(&desc,b"Subtype")){Some(b"CIDFontType0")=>FontKind::Type0Cid0,Some(b"CIDFontType2")=>FontKind::Type0Cid2,_=>return Err(Error::Unsupported("CIDFont subtype"))};
                let gid=if kind==FontKind::Type0Cid2{
                    match get(&desc,b"CIDToGIDMap"){
                        None=>Gid::Identity,
                        Some(v)=>match doc.dereference(v)?{
                            Object::Name(n) if n==b"Identity"=>Gid::Identity,
                            s@Object::Stream{..}=>{
                                let data=crate::filters::decode(&s,131072)?;
                                if data.len()%2!=0{return Err(Error::Invalid("CIDToGIDMap length"));}
                                Gid::Map(data)
                            }
                            _=>return Err(Error::Invalid("CIDToGIDMap")),
                        }
                    }
                }else{Gid::Unknown};
                let (w,dw)=cid_widths(doc,&desc)?;
                (kind,encoding,if vertical{WritingMode::Vertical}else{WritingMode::Horizontal},w,dw,gid)
            }
            _=>return Err(Error::Unsupported("font subtype")),
        };
        Ok(Self{kind,base_font,writing_mode,encoding,to_unicode,widths,default_width,gid})
    }
    pub fn decode(&self,input:&[u8])->Result<Vec<Glyph>>{
        if input.len()>MAX_GLYPHS*4{return Err(Error::Limit("font input"));}
        let mut out=Vec::new();let mut pos=0;
        while pos<input.len(){
            if out.len()>=MAX_GLYPHS{return Err(Error::Limit("font glyph count"));}
            let n=match &self.encoding{Encoding::Simple(_)=>1,Encoding::Identity=>2,Encoding::Custom(c)=>c.code_len(&input[pos..])?};
            let code=input.get(pos..pos+n).ok_or(Error::Invalid("truncated character code"))?;
            let cid=match &self.encoding{Encoding::Simple(_)=>None,Encoding::Identity=>Some(u32::from(u16::from_be_bytes([code[0],code[1]]))),Encoding::Custom(c)=>c.cid(code)};
            let unicode=self.to_unicode.as_ref().and_then(|c|c.unicode(code)).map(str::to_owned).or_else(||match &self.encoding{Encoding::Simple(s)=>s.unicode(code[0]),_=>None});
            let width_key=match &self.encoding{Encoding::Simple(_)=>Some(u32::from(code[0])),_=>cid};
            let advance=width_key.and_then(|k|self.widths.get(&k).copied().or(self.default_width));
            let glyph_id=cid.and_then(|c|match &self.gid{
                Gid::Unknown=>None,Gid::Identity=>u16::try_from(c).ok(),
                Gid::Map(data)=>{let i=c as usize*2;Some(u16::from_be_bytes([*data.get(i)?,*data.get(i+1)?]))}
            });
            out.push(Glyph{code:code.to_vec(),cid,glyph_id,unicode,width:advance});
            pos+=n;
        }
        Ok(out)
    }
}
impl Document<'_>{
    /// Resolve one named page font lazily, including inherited Resources.
    pub fn page_font(&self,page:&PageInfo,resource_name:&[u8])->Result<Font>{
        let resources=dictionary(self.dereference(page.resources.as_ref().ok_or(Error::Invalid("page Resources"))?)?)?;
        let fonts=dictionary(self.dereference(get(&resources,b"Font").ok_or(Error::Invalid("page Font resources"))?)?)?;
        Font::load(self,get(&fonts,resource_name).ok_or(Error::Invalid("font resource name"))?)
    }
}

pub mod truetype;
