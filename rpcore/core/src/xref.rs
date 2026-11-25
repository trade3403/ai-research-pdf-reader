//! Classic, stream, hybrid and incremental cross-reference indexing.
use crate::{dict,reference,parse_object,Document,Error,Object,Parser,Result,MAX_BYTES,MAX_DEPTH,MAX_OBJECTS};
use std::collections::{BTreeMap,BTreeSet};
use std::sync::{Arc,Mutex};
const MAX_REVISIONS: usize = 64;
const MAX_OBJECT_STREAM: usize = 16 * 1024 * 1024;
const MAX_XREF_BYTES: usize = MAX_OBJECTS * 24;
type Dict = BTreeMap<Vec<u8>,Object>;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub(super) enum Entry {
    Free,
    Direct { offset: usize, generation: u16 },
    Compressed { stream: u32, index: usize },
}
struct Section { entries: BTreeMap<u32,Entry>, trailer: Dict }

fn number(o: Option<&Object>, max: usize, label: &'static str) -> Result<usize> {
    match o {
        Some(Object::Number(n)) if *n >= 0.0 && n.fract()==0.0 && *n<=max as f64 => Ok(*n as usize),
        _ => Err(Error::Invalid(label)),
    }
}
fn optional_offset(d: &Dict, k: &[u8], len: usize) -> Result<Option<usize>> {
    d.get(k).map(|v|number(Some(v),len.saturating_sub(1),"xref link")).transpose()
}
fn size(d:&Dict)->Result<usize> {
    let n=number(d.get(b"Size".as_slice()),u32::MAX as usize,"xref Size")?;
    if n==0 {return Err(Error::Invalid("xref Size"));}
    Ok(n)
}
fn put(entries:&mut BTreeMap<u32,Entry>, id:usize, e:Entry)->Result<()> {
    if entries.len()>=MAX_OBJECTS {return Err(Error::Limit("xref entries"));}
    let id=u32::try_from(id).map_err(|_|Error::Limit("object id"))?;
    if id==0 && e!=Entry::Free {return Err(Error::Invalid("object zero"));}
    if entries.insert(id,e).is_some(){return Err(Error::Invalid("duplicate xref object"));}
    Ok(())
}
fn classic(bytes:&[u8], offset:usize)->Result<Section>{
    let mut p=Parser::new(bytes,offset);
    p.expect(b"xref")?;
    let mut entries=BTreeMap::new();
    loop {
        p.skip();
        if p.starts(b"trailer"){p.expect(b"trailer")?;break;}
        let first=p.uint()?;
        let count=p.uint()?;
        if count>MAX_OBJECTS.saturating_sub(entries.len()){return Err(Error::Limit("xref entries"));}
        let end=first.checked_add(count).ok_or(Error::Limit("xref range"))?;
        if end>u32::MAX as usize{return Err(Error::Limit("xref range"));}
        for id in first..end{
            let off=p.uint()?;
            let generation=u16::try_from(p.uint()?).map_err(|_|Error::Invalid("generation"))?;
            let e=match p.word()?{
                b"f"=>Entry::Free,
                b"n" if off<bytes.len() && generation!=u16::MAX=>Entry::Direct{offset:off,generation},
                _=>return Err(Error::Invalid("xref entry")),
            };
            put(&mut entries,id,e)?;
        }
    }
    let trailer=dict(p.object(0)?)?;
    let n=size(&trailer)?;
    if entries.keys().any(|&id|id as usize>=n){return Err(Error::Invalid("xref Size range"));}
    Ok(Section{entries,trailer})
}
// Stream data is bounded by the declared Length, never by searching endstream.
fn read_stream(p:&mut Parser<'_>, map:Dict, length:usize)->Result<Object>{
    p.expect(b"stream")?;
    match p.bytes.get(p.pos){
        Some(b'\n')=>p.pos+=1,
        Some(b'\r') if p.bytes.get(p.pos+1)==Some(&b'\n')=>p.pos+=2,
        _=>return Err(Error::Invalid("stream line ending")),
    }
    let end=p.pos.checked_add(length).ok_or(Error::Limit("stream"))?;
    let data=p.bytes.get(p.pos..end).ok_or(Error::Invalid("truncated stream"))?.to_vec();
    p.pos=end;
    p.expect(b"endstream")?;
    p.expect(b"endobj")?;
    Ok(Object::Stream{dict:map,data})
}
fn be_field(row:&[u8])->Result<u64>{
    if row.len()>8{return Err(Error::Unsupported("xref field width"));}
    Ok(row.iter().fold(0u64,|v,&b|(v<<8)|u64::from(b)))
}
fn stream(bytes:&[u8],offset:usize)->Result<Section>{
    let mut p=Parser::new(bytes,offset);
    let id=p.uint()?;
    let generation=p.uint()?;
    if id==0 || id>u32::MAX as usize || generation>=u16::MAX as usize{return Err(Error::Invalid("xref stream identity"));}
    p.expect(b"obj")?;
    let map=dict(p.object(0)?)?;
    if !matches!(map.get(b"Type".as_slice()),Some(Object::Name(n)) if n==b"XRef"){return Err(Error::Invalid("xref stream type"));}
    let n=size(&map)?;
    let Some(Object::Array(w))=map.get(b"W".as_slice()) else{return Err(Error::Invalid("xref W"));};
    if w.len()!=3{return Err(Error::Invalid("xref W"));}
    let widths=[number(w.first(),8,"xref W")?,number(w.get(1),8,"xref W")?,number(w.get(2),8,"xref W")?];
    let stride:usize=widths.iter().sum();
    if stride==0{return Err(Error::Invalid("xref zero width"));}
    let mut ranges=Vec::new();
    match map.get(b"Index".as_slice()){
        None=>ranges.push((0,n)),
        Some(Object::Array(a)) if a.len()%2==0 && !a.is_empty()=>{
            for pair in a.chunks_exact(2){
                ranges.push((number(pair.first(),n,"xref Index")?,number(pair.get(1),n,"xref Index")?));
            }
        }
        _=>return Err(Error::Invalid("xref Index")),
    }
    let mut total=0usize;
    let mut previous_end=0usize;
    for &(first,count) in &ranges{
        let end=first.checked_add(count).ok_or(Error::Limit("xref Index"))?;
        if first<previous_end || end>n{return Err(Error::Invalid("xref Index range"));}
        previous_end=end;
        total=total.checked_add(count).ok_or(Error::Limit("xref entries"))?;
        if total>MAX_OBJECTS{return Err(Error::Limit("xref entries"));}
    }
    let expected=total.checked_mul(stride).ok_or(Error::Limit("xref bytes"))?;
    if expected>MAX_XREF_BYTES{return Err(Error::Limit("xref bytes"));}
    // XRef stream bootstrap fields must be direct; no index exists yet.
    let length=number(map.get(b"Length".as_slice()),MAX_BYTES,"xref stream Length")?;
    let obj=read_stream(&mut p,map,length)?;
    let data=crate::filters::decode(&obj,expected)?;
    if data.len()!=expected{return Err(Error::Invalid("xref stream byte count"));}
    let Object::Stream{dict:trailer,..}=obj else{unreachable!()};
    let mut rows=data.chunks_exact(stride);
    let mut entries=BTreeMap::new();
    for (first,count) in ranges{
        for id in first..first+count{
            let row=rows.next().ok_or(Error::Invalid("xref row"))?;
            let a=if widths[0]==0{1}else{be_field(&row[..widths[0]])?};
            let b=be_field(&row[widths[0]..widths[0]+widths[1]])?;
            let c=be_field(&row[widths[0]+widths[1]..])?;
            let e=match a{
                0=>{if c>u16::MAX as u64{return Err(Error::Invalid("free generation"));} Entry::Free},
                1 if b<bytes.len() as u64 && c<u16::MAX as u64=>Entry::Direct{offset:b as usize,generation:c as u16},
                2 if b>0 && b<u32::MAX as u64 && c<MAX_OBJECTS as u64=>Entry::Compressed{stream:b as u32,index:c as usize},
                1|2=>return Err(Error::Invalid("xref stream entry")),
                _=>return Err(Error::Unsupported("xref stream entry type")),
            };
            put(&mut entries,id,e)?;
        }
    }
    Ok(Section{entries,trailer})
}
fn section(bytes:&[u8],offset:usize)->Result<(Section,bool)>{
    let mut p=Parser::new(bytes,offset);
    p.skip();
    if p.starts(b"xref"){Ok((classic(bytes,offset)?,true))}else{Ok((stream(bytes,offset)?,false))}
}
fn reject_encryption(d:&Dict)->Result<()>{
    if d.contains_key(b"Encrypt".as_slice()){return Err(Error::Unsupported("encryption"));}
    Ok(())
}

pub(super) struct ObjectStream { data: Vec<u8>, first: usize, index: Vec<(u32,usize)> }
impl ObjectStream {
    fn parse(obj:Object)->Result<Self>{
        let Object::Stream{dict,..}=&obj else{return Err(Error::Invalid("object stream expected"));};
        if !matches!(dict.get(b"Type".as_slice()),Some(Object::Name(n)) if n==b"ObjStm"){return Err(Error::Invalid("object stream type"));}
        let n=number(dict.get(b"N".as_slice()),MAX_OBJECTS,"object stream N")?;
        if n==0{return Err(Error::Invalid("empty object stream"));}
        let first=number(dict.get(b"First".as_slice()),MAX_OBJECT_STREAM,"object stream First")?;
        let data=crate::filters::decode(&obj,MAX_OBJECT_STREAM)?;
        let header=data.get(..first).ok_or(Error::Invalid("object stream header"))?;
        let mut p=Parser::new(header,0);
        let mut index=Vec::with_capacity(n);
        let mut ids=BTreeSet::new();
        for _ in 0..n{
            let id=u32::try_from(p.uint()?).map_err(|_|Error::Invalid("compressed object id"))?;
            let offset=p.uint()?;
            if id==0 || !ids.insert(id) || offset>=data.len()-first || index.last().is_some_and(|&(_,prev)|offset<=prev){
                return Err(Error::Invalid("object stream index"));
            }
            index.push((id,offset));
        }
        p.skip();
        if p.pos!=header.len(){return Err(Error::Invalid("object stream header trailing bytes"));}
        Ok(Self{data,first,index})
    }
    fn get(&self,id:u32,index:usize)->Result<Object>{
        let &(actual,start)=self.index.get(index).ok_or(Error::Invalid("compressed object index"))?;
        if actual!=id{return Err(Error::Invalid("compressed object identity"));}
        let end=self.index.get(index+1).map_or(self.data.len()-self.first,|&(_,off)|off);
        parse_object(&self.data[self.first+start..self.first+end])
    }
}

impl<'a> Document<'a>{
    pub fn open(bytes:&'a [u8])->Result<Self>{
        Self::index(bytes,false).map(|(doc,_)|doc)
    }
    /// Inspect unencrypted bootstrap metadata without exposing encrypted objects.
    pub fn encryption_info(bytes:&'a [u8])->Result<Option<crate::security::EncryptionInfo>>{
        let (doc,security)=Self::index(bytes,true)?;
        security.inspect(&doc)
    }
    /// Verify a supplied PDF password. No key or readable Document is returned.
    /// Caller supplies prepared bytes: legacy encoding for R2–R4, prepared UTF-8
    /// for R5/R6. This API does not perform Unicode normalization.
    pub fn authenticate_legacy_password(bytes:&'a [u8],password:&[u8])->Result<crate::security::PasswordStatus>{
        let (doc,security)=Self::index(bytes,true)?;
        security.authenticate(&doc,password)
    }
    /// Read-only authenticated R2–R6 open. Restricted user permissions are
    /// rejected until application enforcement is available. No password logging.
    pub fn open_with_legacy_password(bytes:&'a [u8],password:&[u8])->Result<Self>{
        let (mut doc,security)=Self::index(bytes,true)?;
        doc.encryption=security.unlock(&doc,password)?;
        Ok(doc)
    }
    /// Revision-neutral alias for the authenticated byte-password API.
    pub fn open_with_password(bytes:&'a [u8],password:&[u8])->Result<Self>{
        Self::open_with_legacy_password(bytes,password)
    }
    /// Revision-neutral alias; caller prepares password bytes for the revision.
    pub fn authenticate_password(bytes:&'a [u8],password:&[u8])->Result<crate::security::PasswordStatus>{
        Self::authenticate_legacy_password(bytes,password)
    }
    fn index(bytes:&'a [u8],inspect:bool)->Result<(Self,crate::security::TrailerSecurity)>{
        if bytes.len()>MAX_BYTES{return Err(Error::Limit("file size"));}
        if !bytes.starts_with(b"%PDF-"){return Err(Error::Invalid("PDF header"));}
        let start=bytes.windows(9).rposition(|w|w==b"startxref").ok_or(Error::Invalid("startxref"))?;
        let mut tail=Parser::new(bytes,start+9);
        let mut offset=tail.uint()?;
        let mut visited=BTreeSet::new();
        let mut entries=BTreeMap::new();
        let mut root=None;
        let mut security=crate::security::TrailerSecurity::default();
        let mut total=0usize;
        for _ in 0..MAX_REVISIONS{
            if offset>=bytes.len() || !visited.insert(offset){return Err(Error::Invalid("xref offset or cycle"));}
            let (mut sec,is_classic)=section(bytes,offset)?;
            if inspect{security.observe(&sec.trailer)?;}else{reject_encryption(&sec.trailer)?;}
            let prev=optional_offset(&sec.trailer,b"Prev",bytes.len())?;
            if let Some(supplement)=optional_offset(&sec.trailer,b"XRefStm",bytes.len())?{
                if !is_classic || !visited.insert(supplement){return Err(Error::Invalid("hybrid xref cycle"));}
                let extra=stream(bytes,supplement)?;
                if inspect{security.observe(&extra.trailer)?;}else{reject_encryption(&extra.trailer)?;}
                if extra.trailer.contains_key(b"XRefStm".as_slice()){return Err(Error::Invalid("nested hybrid xref"));}
                // Hybrid supplementary entries override the same revision's table.
                // Prev in the supplementary stream is not followed; the table owns the chain.
                total=total.checked_add(extra.entries.len()).ok_or(Error::Limit("xref entries"))?;
                for (id,e) in extra.entries{sec.entries.insert(id,e);}
                if !sec.trailer.contains_key(b"Root".as_slice()){
                    if let Some(r)=extra.trailer.get(b"Root".as_slice()){sec.trailer.insert(b"Root".to_vec(),r.clone());}
                }
            }
            total=total.checked_add(sec.entries.len()).ok_or(Error::Limit("xref entries"))?;
            if total>MAX_OBJECTS{return Err(Error::Limit("cumulative xref entries"));}
            if root.is_none() && sec.trailer.contains_key(b"Root".as_slice()){
                root=Some(reference(sec.trailer.get(b"Root".as_slice()))?);
            }
            // Index by object number, not generation. Free entries are tombstones.
            for (id,e) in sec.entries{entries.entry(id).or_insert(e);}
            if let Some(p)=prev{offset=p;}else{
                return Ok((Self{bytes,entries,root:root.ok_or(Error::Invalid("missing Root"))?,encryption:None,object_stream_cache:Mutex::new(None)},security));
            }
        }
        Err(Error::Limit("xref revisions"))
    }

    pub fn resolve(&self,id:u32,generation:u16)->Result<Object>{
        self.resolve_inner(id,generation,&mut BTreeSet::new())
    }
    fn resolve_inner(&self,id:u32,generation:u16,active:&mut BTreeSet<(u32,u16)>)->Result<Object>{
        if active.len()>=MAX_DEPTH{return Err(Error::Limit("object reference depth"));}
        if !active.insert((id,generation)){return Err(Error::Invalid("object reference cycle"));}
        let result=self.resolve_entry(id,generation,active);
        active.remove(&(id,generation));
        result
    }
    fn resolve_entry(&self,id:u32,generation:u16,active:&mut BTreeSet<(u32,u16)>)->Result<Object>{
        match self.entries.get(&id).copied(){
            Some(Entry::Direct{offset,generation:g}) if g==generation=>{
                let mut p=Parser::new(self.bytes,offset);
                if p.uint()?!=id as usize || p.uint()?!=generation as usize{return Err(Error::Invalid("object identity"));}
                p.expect(b"obj")?;
                let value=p.object(0)?;
                p.skip();
                if p.starts(b"stream"){
                    let map=dict(value)?;
                    let length=match map.get(b"Length".as_slice()){
                        Some(Object::Ref(i,g))=>self.stream_length(*i,*g,active)?,
                        other=>number(other,MAX_BYTES,"stream length")?,
                    };
                    let mut value=read_stream(&mut p,map,length)?;
                    if let Some(crypto)=&self.encryption{crypto.decrypt(id,generation,&mut value)?;}
                    return Ok(value);
                }
                p.expect(b"endobj")?;
                let mut value=value;
                if let Some(crypto)=&self.encryption{crypto.decrypt(id,generation,&mut value)?;}
                Ok(value)
            }
            Some(Entry::Compressed{stream,index}) if generation==0=>{
                if !matches!(self.entries.get(&stream),Some(Entry::Direct{generation:0,..})){
                    return Err(Error::Invalid("object stream must be direct"));
                }
                let cached={
                    let cache=self.object_stream_cache.lock().map_err(|_|Error::Invalid("object stream cache"))?;
                    cache.as_ref().filter(|(key,_)|*key==stream).map(|(_,value)|Arc::clone(value))
                };
                let container=match cached{
                    Some(value)=>value,
                    None=>{
                        let value=Arc::new(ObjectStream::parse(self.resolve_inner(stream,0,active)?)?);
                        *self.object_stream_cache.lock().map_err(|_|Error::Invalid("object stream cache"))?=Some((stream,Arc::clone(&value)));
                        value
                    }
                };
                container.get(id,index)
            }
            _=>Err(Error::Invalid("missing object or generation")),
        }
    }
    fn stream_length(&self,mut id:u32,mut generation:u16,active:&mut BTreeSet<(u32,u16)>)->Result<usize>{
        let mut seen=BTreeSet::new();
        for _ in 0..MAX_DEPTH{
            if !seen.insert((id,generation)){return Err(Error::Invalid("stream length cycle"));}
            match self.resolve_inner(id,generation,active)?{
                Object::Ref(i,g)=>{id=i;generation=g;}
                value=>return number(Some(&value),MAX_BYTES,"stream length object"),
            }
        }
        Err(Error::Limit("stream length reference depth"))
    }
}
