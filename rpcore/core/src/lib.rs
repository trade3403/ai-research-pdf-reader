#![forbid(unsafe_code)]
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 100 * 1024 * 1024;
const MAX_OBJECTS: usize = 100_000;
const MAX_DEPTH: usize = 64;
const MAX_TOKEN: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Bool(bool),
    Number(f64),
    Name(Vec<u8>),
    String(Vec<u8>),
    Array(Vec<Object>),
    Dict(BTreeMap<Vec<u8>, Object>),
    Ref(u32, u16),
    Stream { dict: BTreeMap<Vec<u8>, Object>, data: Vec<u8> },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    Limit(&'static str),
    Unsupported(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;
fn ws(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}
fn delim(b: u8) -> bool {
    ws(b) || b"()<>[]{}/%".contains(&b)
}
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8], pos: usize) -> Self {
        Self { bytes, pos }
    }
    fn skip(&mut self) {
        loop {
            while self.bytes.get(self.pos).is_some_and(|b| ws(*b)) {
                self.pos += 1;
            }
            if self.bytes.get(self.pos) != Some(&b'%') {
                break;
            }
            while self.bytes.get(self.pos).is_some_and(|b| !matches!(*b, 10 | 13)) {
                self.pos += 1;
            }
        }
    }
    fn starts(&self, s: &[u8]) -> bool {
        self.bytes.get(self.pos..).is_some_and(|b| b.starts_with(s))
    }
    fn word(&mut self) -> Result<&'a [u8]> {
        self.skip();
        let start = self.pos;
        while self.bytes.get(self.pos).is_some_and(|b| !delim(*b)) {
            self.pos += 1;
            if self.pos - start > MAX_TOKEN {
                return Err(Error::Limit("token"));
            }
        }
        if start == self.pos {
            return Err(Error::Invalid("expected token"));
        }
        Ok(&self.bytes[start..self.pos])
    }
    fn uint(&mut self) -> Result<usize> {
        let w = self.word()?;
        if !w.iter().all(u8::is_ascii_digit) {
            return Err(Error::Invalid("unsigned integer"));
        }
        std::str::from_utf8(w).ok().and_then(|s| s.parse().ok()).ok_or(Error::Invalid("integer"))
    }
    fn expect(&mut self, word: &[u8]) -> Result<()> {
        if self.word()? == word { Ok(()) } else { Err(Error::Invalid("unexpected keyword")) }
    }
    fn name(&mut self) -> Result<Vec<u8>> {
        self.skip();
        if !self.starts(b"/") { return Err(Error::Invalid("name")); }
        self.pos += 1;
        let mut out = Vec::new();
        while let Some(&b) = self.bytes.get(self.pos) {
            if delim(b) { break; }
            self.pos += 1;
            if b == b'#' {
                let a = *self.bytes.get(self.pos).ok_or(Error::Invalid("name escape"))?;
                let c = *self.bytes.get(self.pos + 1).ok_or(Error::Invalid("name escape"))?;
                out.push(hex(a)? * 16 + hex(c)?);
                self.pos += 2;
            } else { out.push(b); }
            if out.len() > MAX_TOKEN { return Err(Error::Limit("name")); }
        }
        Ok(out)
    }
    fn literal(&mut self) -> Result<Vec<u8>> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut depth = 1usize;
        while let Some(&b) = self.bytes.get(self.pos) {
            self.pos += 1;
            match b {
                b'(' => { depth += 1; if depth > MAX_DEPTH { return Err(Error::Limit("string nesting")); } out.push(b); }
                b')' => { depth -= 1; if depth == 0 { return Ok(out); } out.push(b); }
                b'\\' => {
                    let c = *self.bytes.get(self.pos).ok_or(Error::Invalid("string escape"))?;
                    self.pos += 1;
                    match c {
                        b'n' => out.push(10), b'r' => out.push(13), b't' => out.push(9),
                        b'b' => out.push(8), b'f' => out.push(12),
                        10 => {}, 13 => { if self.bytes.get(self.pos) == Some(&10) { self.pos += 1; } }
                        b'0'..=b'7' => {
                            let mut v = u16::from(c - b'0');
                            for _ in 0..2 {
                                match self.bytes.get(self.pos) {
                                    Some(&d @ b'0'..=b'7') => { v = v * 8 + u16::from(d - b'0'); self.pos += 1; }
                                    _ => break,
                                }
                            }
                            out.push(v as u8);
                        }
                        _ => out.push(c),
                    }
                }
                13 => { if self.bytes.get(self.pos) == Some(&10) { self.pos += 1; } out.push(10); }
                _ => out.push(b),
            }
            if out.len() > MAX_TOKEN { return Err(Error::Limit("string")); }
        }
        Err(Error::Invalid("unterminated string"))
    }
    fn hex_string(&mut self) -> Result<Vec<u8>> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut high = None;
        while let Some(&b) = self.bytes.get(self.pos) {
            self.pos += 1;
            if b == b'>' { if let Some(h) = high { out.push(h * 16); } return Ok(out); }
            if ws(b) { continue; }
            let n = hex(b)?;
            if let Some(h) = high.take() { out.push(h * 16 + n); } else { high = Some(n); }
            if out.len() > MAX_TOKEN { return Err(Error::Limit("hex string")); }
        }
        Err(Error::Invalid("unterminated hex string"))
    }
    fn object(&mut self, depth: usize) -> Result<Object> {
        if depth >= MAX_DEPTH { return Err(Error::Limit("nesting")); }
        self.skip();
        if self.starts(b"<<") {
            self.pos += 2;
            let mut map = BTreeMap::new();
            loop {
                self.skip();
                if self.starts(b">>") { self.pos += 2; return Ok(Object::Dict(map)); }
                if map.len() >= MAX_OBJECTS { return Err(Error::Limit("dictionary")); }
                let key = self.name()?;
                let value = self.object(depth + 1)?;
                if map.insert(key, value).is_some() { return Err(Error::Invalid("duplicate key")); }
            }
        }
        match self.bytes.get(self.pos).copied() {
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip();
                    if self.starts(b"]") { self.pos += 1; return Ok(Object::Array(items)); }
                    if items.len() >= MAX_OBJECTS { return Err(Error::Limit("array")); }
                    items.push(self.object(depth + 1)?);
                }
            }
            Some(b'/') => Ok(Object::Name(self.name()?)),
            Some(b'(') => Ok(Object::String(self.literal()?)),
            Some(b'<') => Ok(Object::String(self.hex_string()?)),
            _ => {
                let token = self.word()?;
                match token {
                    b"null" => Ok(Object::Null),
                    b"true" => Ok(Object::Bool(true)),
                    b"false" => Ok(Object::Bool(false)),
                    _ => {
                        if !token.iter().all(|b| b.is_ascii_digit() || b"+-.".contains(b)) {
                            return Err(Error::Invalid("object token"));
                        }
                        let n = std::str::from_utf8(token).ok().and_then(|s| s.parse::<f64>().ok()).filter(|n| n.is_finite()).ok_or(Error::Invalid("number"))?;
                        let after = self.pos;
                        if let Ok(id) = std::str::from_utf8(token).unwrap_or("").parse::<u32>() {
                            if let Ok(g) = self.uint() {
                                if let Ok(gen) = u16::try_from(g) {
                                    if self.expect(b"R").is_ok() { return Ok(Object::Ref(id, gen)); }
                                }
                            }
                        }
                        self.pos = after;
                        Ok(Object::Number(n))
                    }
                }
            }
        }
    }
}
fn hex(b: u8) -> Result<u8> {
    match b { b'0'..=b'9' => Ok(b-b'0'), b'a'..=b'f' => Ok(b-b'a'+10), b'A'..=b'F' => Ok(b-b'A'+10), _ => Err(Error::Invalid("hex digit")) }
}
pub fn parse_object(bytes: &[u8]) -> Result<Object> {
    if bytes.len() > MAX_BYTES { return Err(Error::Limit("input")); }
    let mut p = Parser::new(bytes, 0);
    let obj = p.object(0)?;
    p.skip();
    if p.pos != bytes.len() { return Err(Error::Invalid("trailing input")); }
    Ok(obj)
}
fn dict(o: Object) -> Result<BTreeMap<Vec<u8>, Object>> {
    if let Object::Dict(d) = o { Ok(d) } else { Err(Error::Invalid("dictionary expected")) }
}
fn reference(o: Option<&Object>) -> Result<(u32,u16)> {
    if let Some(Object::Ref(id,g)) = o { Ok((*id,*g)) } else { Err(Error::Invalid("reference expected")) }
}
mod xref;
pub struct Document<'a> {
    bytes: &'a [u8],
    entries: BTreeMap<u32,xref::Entry>,
    root: (u32,u16),
    object_stream_cache: std::sync::Mutex<Option<(u32,std::sync::Arc<xref::ObjectStream>)>>,
}
impl<'a> Document<'a> {
    pub fn page_count(&self) -> Result<usize> {
        let catalog = dict(self.resolve(self.root.0,self.root.1)?)?;
        if catalog.get(b"Type".as_slice()) != Some(&Object::Name(b"Catalog".to_vec())) { return Err(Error::Invalid("catalog type")); }
        let pages = reference(catalog.get(b"Pages".as_slice()))?;
        self.walk(pages,0,&mut BTreeSet::new())
    }
    fn walk(&self, id: (u32,u16), depth: usize, seen: &mut BTreeSet<(u32,u16)>) -> Result<usize> {
        if depth >= MAX_DEPTH || seen.len() >= MAX_OBJECTS { return Err(Error::Limit("page tree")); }
        if !seen.insert(id) { return Err(Error::Invalid("page tree cycle/duplicate")); }
        let node = dict(self.resolve(id.0,id.1)?)?;
        match node.get(b"Type".as_slice()) {
            Some(Object::Name(n)) if n == b"Page" => Ok(1),
            Some(Object::Name(n)) if n == b"Pages" => {
                let Some(Object::Array(kids)) = node.get(b"Kids".as_slice()) else { return Err(Error::Invalid("page kids")); };
                let mut sum = 0usize;
                for kid in kids {
                    sum = sum.checked_add(self.walk(reference(Some(kid))?,depth+1,seen)?).ok_or(Error::Limit("page count"))?;
                }
                if node.get(b"Count".as_slice()) != Some(&Object::Number(sum as f64)) { return Err(Error::Invalid("page count mismatch")); }
                Ok(sum)
            }
            _ => Err(Error::Invalid("page type")),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(kids: &str) -> Vec<u8> {
        let mut b = b"%PDF-1.4\n".to_vec();
        let objs = [String::from("<< /Type /Catalog /Pages 2 0 R >>"),format!("<< /Type /Pages /Kids [{kids}] /Count 1 >>"),String::from("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>")];
        let mut offsets = Vec::new();
        for (i,o) in objs.iter().enumerate() {
            offsets.push(b.len());
            b.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n",i+1).as_bytes());
        }
        let x = b.len();
        b.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for off in offsets { b.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()); }
        b.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").as_bytes());
        b
    }
    #[test]
    fn objects() {
        assert_eq!(parse_object(b"[1 0 R /A#42 (a\\n) <F> true null]").unwrap(),Object::Array(vec![Object::Ref(1,0),Object::Name(b"AB".to_vec()),Object::String(b"a\n".to_vec()),Object::String(vec![240]),Object::Bool(true),Object::Null]));
        assert!(parse_object(b"<< /A 1 /A 2 >>").is_err());
        assert!(parse_object(b"(").is_err());
        assert!(parse_object(&vec![b'[';100]).is_err());
    }
    #[test]
    fn page_tree() { let b=fixture("3 0 R"); assert_eq!(Document::open(&b).unwrap().page_count().unwrap(),1); }
    #[test]
    fn cycle() { let b=fixture("2 0 R"); assert!(Document::open(&b).unwrap().page_count().is_err()); }
    #[test]
    fn bad_header() { assert!(Document::open(b"bad").is_err()); }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageInfo {
    pub object_id: (u32, u16),
    pub media_box: [f64; 4],
    pub crop_box: [f64; 4],
    pub rotation: i32,
    pub resources: Option<Object>,
}

#[derive(Default, Clone)]
struct Inherited {
    media: Option<[f64; 4]>,
    crop: Option<[f64; 4]>,
    rotate: Option<i32>,
    resources: Option<Object>,
}

impl Document<'_> {
    fn dereference(&self, object: &Object) -> Result<Object> {
        let mut current = object.clone();
        let mut seen = BTreeSet::new();
        for _ in 0..MAX_DEPTH {
            match current {
                Object::Ref(id, gen) => {
                    if !seen.insert((id, gen)) { return Err(Error::Invalid("reference cycle")); }
                    current = self.resolve(id, gen)?;
                }
                _ => return Ok(current),
            }
        }
        Err(Error::Limit("reference depth"))
    }

    fn rectangle(&self, o: &Object) -> Result<[f64; 4]> {
        let Object::Array(values) = self.dereference(o)? else { return Err(Error::Invalid("page box array")); };
        if values.len() != 4 { return Err(Error::Invalid("page box size")); }
        let mut rect = [0.0; 4];
        for (i, v) in values.iter().enumerate() {
            let Object::Number(n) = self.dereference(v)? else { return Err(Error::Invalid("page box coordinate")); };
            if !n.is_finite() || n.abs() > 1e9 { return Err(Error::Limit("page coordinate")); }
            rect[i] = n;
        }
        if rect[2] <= rect[0] || rect[3] <= rect[1] { return Err(Error::Invalid("page box area")); }
        Ok(rect)
    }

    pub fn pages(&self) -> Result<Vec<PageInfo>> {
        let catalog = dict(self.resolve(self.root.0,self.root.1)?)?;
        if catalog.get(b"Type".as_slice()) != Some(&Object::Name(b"Catalog".to_vec())) { return Err(Error::Invalid("catalog type")); }
        let root = reference(catalog.get(b"Pages".as_slice()))?;
        let mut pages = Vec::new();
        self.collect_pages(root, 0, Inherited::default(), &mut BTreeSet::new(), &mut pages)?;
        Ok(pages)
    }

    fn collect_pages(&self, id: (u32,u16), depth: usize, mut inherited: Inherited, seen: &mut BTreeSet<(u32,u16)>, out: &mut Vec<PageInfo>) -> Result<()> {
        if depth >= MAX_DEPTH || seen.len() >= MAX_OBJECTS { return Err(Error::Limit("page tree")); }
        if !seen.insert(id) { return Err(Error::Invalid("page tree cycle/duplicate")); }
        let node = dict(self.resolve(id.0,id.1)?)?;
        if let Some(v) = node.get(b"MediaBox".as_slice()) { inherited.media = Some(self.rectangle(v)?); }
        if let Some(v) = node.get(b"CropBox".as_slice()) { inherited.crop = Some(self.rectangle(v)?); }
        if let Some(v) = node.get(b"Rotate".as_slice()) {
            let Object::Number(n) = self.dereference(v)? else { return Err(Error::Invalid("rotation")); };
            if n.fract() != 0.0 || n.abs() > i32::MAX as f64 || n % 90.0 != 0.0 { return Err(Error::Invalid("rotation")); }
            inherited.rotate = Some((n as i32).rem_euclid(360));
        }
        if let Some(v) = node.get(b"Resources".as_slice()) {
            // Keep indirect dictionaries as references, avoid expanding resource graphs.
            if !matches!(v,Object::Ref(_,_)|Object::Dict(_)) { return Err(Error::Invalid("resources")); }
            inherited.resources = Some(v.clone());
        }
        match node.get(b"Type".as_slice()) {
            Some(Object::Name(n)) if n == b"Page" => {
                let media = inherited.media.ok_or(Error::Invalid("missing MediaBox"))?;
                out.push(PageInfo { object_id:id, media_box:media, crop_box:inherited.crop.unwrap_or(media), rotation:inherited.rotate.unwrap_or(0), resources:inherited.resources });
            }
            Some(Object::Name(n)) if n == b"Pages" => {
                let Some(Object::Array(kids)) = node.get(b"Kids".as_slice()) else { return Err(Error::Invalid("page kids")); };
                let start = out.len();
                for kid in kids { self.collect_pages(reference(Some(kid))?,depth+1,inherited.clone(),seen,out)?; }
                if node.get(b"Count".as_slice()) != Some(&Object::Number((out.len()-start) as f64)) { return Err(Error::Invalid("page count mismatch")); }
            }
            _ => return Err(Error::Invalid("page type")),
        }
        Ok(())
    }
}
pub mod filters;

pub mod content;

pub mod fonts;

pub mod text;

pub mod graphics;

pub mod raster;
