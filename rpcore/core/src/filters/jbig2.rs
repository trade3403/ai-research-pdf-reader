//! Bounded native JBIG2 segment inspection (ITU-T T.88, sections 7.2, 7.4 and 8).
//! Sequential standalone files and headerless embedded streams only.
//! Payloads remain opaque except for page information. This is not a bitmap
//! decoder; embedded references may require a separately supplied globals stream.
pub mod qm;
pub mod integer;
pub mod bitmap;
pub mod generic;
pub mod huffman;
pub mod dictionary;
pub mod refinement;
pub mod text_region;
pub mod pattern;
pub mod halftone;
use crate::{Error,Result,MAX_BYTES};
use std::{collections::{BTreeMap,BTreeSet},ops::Range};

const SIGNATURE:&[u8;8]=b"\x97JB2\r\n\x1a\n";
const MAX_SEGMENTS:usize=65536;
const MAX_REFS:usize=4096;
const MAX_TOTAL_REFS:usize=262144;
const MAX_PAGES:usize=4096;
const MAX_SIDE:u32=1_048_576;
const MAX_PIXELS:u64=100_000_000;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Container{SequentialFile,Embedded}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Segment{
    pub number:u32,
    pub kind:u8,
    pub deferred_non_retain:bool,
    pub retention:Vec<u8>,
    pub referred:Vec<u32>,
    pub page:u32,
    pub header:Range<usize>,
    pub payload:Range<usize>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PageInfo{
    pub page:u32,
    pub width:u32,
    pub height:Option<u32>,
    pub x_resolution:u32,
    pub y_resolution:u32,
    pub flags:u8,
    pub striped:bool,
    pub max_stripe_size:u16,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Inspection{
    pub container:Container,
    pub declared_pages:Option<u32>,
    pub segments:Vec<Segment>,
    pub pages:Vec<PageInfo>,
    /// References not present in this stream. Embedded data may depend on
    /// PDF JBIG2Globals; no claim of resolution or payload validity is made.
    pub external_references:Vec<u32>,
}
struct Cursor<'a>{data:&'a[u8],pos:usize}
impl<'a> Cursor<'a>{
    fn take(&mut self,n:usize)->Result<&'a[u8]>{
        let end=self.pos.checked_add(n).ok_or(Error::Limit("JBIG2 range"))?;
        let b=self.data.get(self.pos..end).ok_or(Error::Invalid("JBIG2 truncated field"))?;
        self.pos=end;Ok(b)
    }
    fn byte(&mut self)->Result<u8>{Ok(self.take(1)?[0])}
    fn u16(&mut self)->Result<u16>{
        let b=self.take(2)?;Ok(u16::from_be_bytes([b[0],b[1]]))
    }
    fn u32(&mut self)->Result<u32>{
        let b=self.take(4)?;Ok(u32::from_be_bytes([b[0],b[1],b[2],b[3]]))
    }
}
fn segment(c:&mut Cursor<'_>,total_refs:&mut usize)->Result<Segment>{
    let start=c.pos;
    let number=c.u32()?;
    if number==u32::MAX{return Err(Error::Invalid("JBIG2 reserved segment number"));}
    let flags=c.byte()?;
    let first=c.byte()?;
    let (count,retention)=match first>>5{
        0..=4=>(usize::from(first>>5),vec![first&31]),
        7=>{
            let b=c.take(3)?;
            let count=u32::from_be_bytes([first&31,b[0],b[1],b[2]]) as usize;
            if count<5{return Err(Error::Invalid("JBIG2 extended reference count"));}
            if count>MAX_REFS{return Err(Error::Limit("JBIG2 references"));}
            (count,c.take((count+1).div_ceil(8))?.to_vec())
        }
        _=>return Err(Error::Invalid("JBIG2 reserved reference count")),
    };
    *total_refs=total_refs.checked_add(count).ok_or(Error::Limit("JBIG2 total references"))?;
    if *total_refs>MAX_TOTAL_REFS{return Err(Error::Limit("JBIG2 total references"));}
    let mut referred=Vec::with_capacity(count);
    for _ in 0..count{
        let n=if number<=256{u32::from(c.byte()?)}else if number<=65536{u32::from(c.u16()?)}else{c.u32()?};
        if n>=number{return Err(Error::Invalid("JBIG2 forward/self reference"));}
        referred.push(n);
    }
    let page=if flags&64!=0{c.u32()?}else{u32::from(c.byte()?)};
    let len=c.u32()?;
    if len==u32::MAX{return Err(Error::Unsupported("JBIG2 indeterminate segment length"));}
    let end=c.pos;
    c.take(len as usize)?;
    Ok(Segment{number,kind:flags&63,deferred_non_retain:flags&128!=0,
        retention,referred,page,header:start..end,payload:end..c.pos})
}
fn page_info(s:&Segment,data:&[u8])->Result<PageInfo>{
    if s.page==0||!s.referred.is_empty()||s.payload.len()!=19{
        return Err(Error::Invalid("JBIG2 page information layout"));
    }
    let mut c=Cursor{data:&data[s.payload.clone()],pos:0};
    let width=c.u32()?;let h=c.u32()?;
    let x_resolution=c.u32()?;let y_resolution=c.u32()?;
    let flags=c.byte()?;let stripes=c.u16()?;
    let striped=stripes&0x8000!=0;
    let height=if h==u32::MAX{
        if !striped{return Err(Error::Invalid("JBIG2 unknown unstriped height"));}
        None
    }else{Some(h)};
    if width==0||height==Some(0){return Err(Error::Invalid("JBIG2 page dimensions"));}
    if width>MAX_SIDE||height.is_some_and(|v|v>MAX_SIDE||u64::from(width)*u64::from(v)>MAX_PIXELS){
        return Err(Error::Limit("JBIG2 page dimensions"));
    }
    Ok(PageInfo{page:s.page,width,height,x_resolution,y_resolution,flags,striped,max_stripe_size:stripes&0x7fff})
}
/// Inspect bounded sequential JBIG2. Embedded data may omit EOF and have
/// unresolved globals; standalone files require EOF and resolved references.
/// This deliberately does not validate region bodies, retention semantics,
/// page composition or arithmetic coding.
pub fn inspect(data:&[u8])->Result<Inspection>{
    if data.len()>MAX_BYTES{return Err(Error::Limit("JBIG2 input bytes"));}
    let mut c=Cursor{data,pos:0};
    let (container,declared_pages)=if data.starts_with(SIGNATURE){
        c.take(8)?;
        let flags=c.byte()?;
        if flags&0xfc!=0{return Err(Error::Unsupported("JBIG2 file flags"));}
        if flags&1==0{return Err(Error::Unsupported("JBIG2 random access file"));}
        let count=if flags&2!=0{None}else{Some(c.u32()?)};
        if count.is_some_and(|n|n as usize>MAX_PAGES){return Err(Error::Limit("JBIG2 page count"));}
        (Container::SequentialFile,count)
    }else{(Container::Embedded,None)};
    let mut segments=Vec::new();
    let mut pages=BTreeMap::new();
    let mut seen=BTreeSet::new();
    let mut external=BTreeSet::new();
    let mut total_refs=0;
    let mut eof=false;
    while c.pos<data.len(){
        if segments.len()>=MAX_SEGMENTS{return Err(Error::Limit("JBIG2 segment count"));}
        let s=segment(&mut c,&mut total_refs)?;
        if !seen.insert(s.number){return Err(Error::Invalid("JBIG2 duplicate segment number"));}
        for &r in &s.referred{if !seen.contains(&r){external.insert(r);}}
        match s.kind{
            48=>{
                let p=page_info(&s,data)?;
                if pages.len()>=MAX_PAGES{return Err(Error::Limit("JBIG2 pages"));}
                if pages.insert(p.page,p).is_some(){return Err(Error::Invalid("JBIG2 duplicate page information"));}
            }
            49|51=>{
                if !s.payload.is_empty()||!s.referred.is_empty()||
                    (s.kind==49&&s.page==0)||(s.kind==51&&s.page!=0){
                    return Err(Error::Invalid("JBIG2 end segment layout"));
                }
                if s.kind==51{
                    if c.pos!=data.len(){return Err(Error::Invalid("JBIG2 data after EOF"));}
                    eof=true;
                }
            }
            _=>{} // Other segment kinds are opaque, not supported decode claims.
        }
        segments.push(s);
    }
    if segments.is_empty(){return Err(Error::Invalid("JBIG2 empty stream"));}
    if container==Container::SequentialFile{
        if !eof{return Err(Error::Invalid("JBIG2 missing EOF"));}
        if !external.is_empty(){return Err(Error::Invalid("JBIG2 unresolved file reference"));}
        if declared_pages.is_some_and(|n|n as usize!=pages.len()){
            return Err(Error::Invalid("JBIG2 declared page count mismatch"));
        }
    }
    Ok(Inspection{container,declared_pages,segments,pages:pages.into_values().collect(),
        external_references:external.into_iter().collect()})
}


/// Original stream and index of a segment. Byte ranges in Segment remain
/// relative to that stream; globals and page bytes are never concatenated.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum SegmentSource{Globals,Page}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SegmentLocation{pub source:SegmentSource,pub index:usize}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ResolvedSegment{
    pub number:u32,
    pub location:SegmentLocation,
    pub dependencies:Vec<SegmentLocation>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PdfInspection{
    pub globals:Option<Inspection>,
    pub page_stream:Inspection,
    pub page:PageInfo,
    pub graph:Vec<ResolvedSegment>,
    pub explicit_end_of_page:bool,
}
impl PdfInspection{
    pub fn segment(&self,location:SegmentLocation)->Option<&Segment>{
        match location.source{
            SegmentSource::Globals=>self.globals.as_ref()?.segments.get(location.index),
            SegmentSource::Page=>self.page_stream.segments.get(location.index),
        }
    }
}
fn reusable_global(kind:u8)->bool{matches!(kind,0|16|52|53|62)}
fn page_kind(kind:u8)->bool{
    matches!(kind,0|4|6|7|16|20|22|23|36|38|39|40|42|43|48|49|50|51|52|53|62)
}

/// Structural dependency graph for a single, known-height, unstriped PDF
/// JBIG2 image. Globals are parsed first, in a separate byte namespace;
/// segment identifiers must be unique across both streams. Missing references
/// cannot be satisfied by future segments. Retention bits are preserved but
/// not executed, and opaque segment payload semantics are not validated.
/// An omitted End-of-Page/End-of-File is permitted for embedded streams.
pub fn inspect_pdf_streams(globals:&[u8],data:&[u8])->Result<PdfInspection>{
    if globals.len().checked_add(data.len()).ok_or(Error::Limit("JBIG2 combined input"))?>MAX_BYTES{
        return Err(Error::Limit("JBIG2 combined input"));
    }
    let global=if globals.is_empty(){None}else{Some(inspect(globals)?)};
    let local=inspect(data)?;
    if local.container!=Container::Embedded||
        global.as_ref().is_some_and(|g|g.container!=Container::Embedded){
        return Err(Error::Unsupported("JBIG2 PDF requires embedded streams"));
    }
    if local.pages.len()!=1{return Err(Error::Invalid("JBIG2 PDF single page required"));}
    let page=local.pages[0].clone();
    if page.striped||page.height.is_none(){return Err(Error::Unsupported("JBIG2 PDF striped page"));}
    let global_segments=global.as_ref().map_or(&[][..],|g|g.segments.as_slice());
    let count=global_segments.len().checked_add(local.segments.len()).ok_or(Error::Limit("JBIG2 combined segments"))?;
    if count>MAX_SEGMENTS{return Err(Error::Limit("JBIG2 combined segments"));}
    let mut refs=0usize;
    let mut seen:BTreeMap<u32,(SegmentLocation,u32,u8)>=BTreeMap::new();
    let mut graph=Vec::with_capacity(count);
    let mut opened=false;
    let mut closed=false;
    for (source,segments) in [(SegmentSource::Globals,global_segments),(SegmentSource::Page,local.segments.as_slice())]{
        for (index,s) in segments.iter().enumerate(){
            let location=SegmentLocation{source,index};
            if seen.contains_key(&s.number){return Err(Error::Invalid("JBIG2 cross-stream duplicate segment"));}
            if source==SegmentSource::Globals{
                if s.page!=0||!reusable_global(s.kind){return Err(Error::Unsupported("JBIG2 PDF globals segment kind/page"));}
            }else{
                if !page_kind(s.kind){return Err(Error::Unsupported("JBIG2 PDF segment kind"));}
                if s.page==0{
                    if s.kind!=51&&!reusable_global(s.kind){return Err(Error::Unsupported("JBIG2 PDF page-zero segment"));}
                }else{
                    if s.page!=page.page{return Err(Error::Invalid("JBIG2 PDF foreign page"));}
                    if closed{return Err(Error::Invalid("JBIG2 PDF segment after page end"));}
                    if s.kind==48{
                        if opened{return Err(Error::Invalid("JBIG2 PDF duplicate page start"));}
                        opened=true;
                    }else if !opened{
                        return Err(Error::Invalid("JBIG2 PDF segment before page info"));
                    }
                    if s.kind==50{return Err(Error::Unsupported("JBIG2 PDF stripe end"));}
                    if s.kind==49{closed=true;}
                }
                if s.kind==51&&!opened{return Err(Error::Invalid("JBIG2 PDF EOF before page"));}
            }
            refs=refs.checked_add(s.referred.len()).ok_or(Error::Limit("JBIG2 combined references"))?;
            if refs>MAX_TOTAL_REFS{return Err(Error::Limit("JBIG2 combined references"));}
            let mut dependencies=Vec::with_capacity(s.referred.len());
            for r in &s.referred{
                let &(target,target_page,target_kind)=seen.get(r).ok_or(Error::Invalid("JBIG2 PDF unresolved reference"))?;
                if target_page!=0&&target_page!=s.page{
                    return Err(Error::Invalid("JBIG2 PDF reference page mismatch"));
                }
                if matches!(target_kind,48|49|50|51){
                    return Err(Error::Invalid("JBIG2 PDF reference to page control"));
                }
                dependencies.push(target);
            }
            seen.insert(s.number,(location,s.page,s.kind));
            graph.push(ResolvedSegment{number:s.number,location,dependencies});
        }
    }
    if !opened{return Err(Error::Invalid("JBIG2 PDF missing page start"));}
    Ok(PdfInspection{globals:global,page_stream:local,page,graph,explicit_end_of_page:closed})
}

#[cfg(test)]
mod tests{
    use super::*;
    fn seg(number:u32,kind:u8,page:u32,refs:&[u32],payload:&[u8])->Vec<u8>{
        let mut v=number.to_be_bytes().to_vec();
        v.push(kind|if page>255{64}else{0});
        if refs.len()<=4{v.push((refs.len()as u8)<<5);}
        else{
            v.extend((0xe0000000u32|refs.len()as u32).to_be_bytes());
            v.resize(v.len()+(refs.len()+1).div_ceil(8),0);
        }
        for &r in refs{
            if number<=256{v.push(r as u8);}
            else if number<=65536{v.extend((r as u16).to_be_bytes());}
            else{v.extend(r.to_be_bytes());}
        }
        if page>255{v.extend(page.to_be_bytes());}else{v.push(page as u8);}
        v.extend((payload.len()as u32).to_be_bytes());v.extend(payload);v
    }
    fn info(w:u32,h:u32,stripe:u16)->Vec<u8>{
        let mut p=Vec::new();for n in [w,h,300,300]{p.extend(n.to_be_bytes());}
        p.push(0);p.extend(stripe.to_be_bytes());p
    }
    fn file()->Vec<u8>{
        let mut v=SIGNATURE.to_vec();v.push(1);v.extend(1u32.to_be_bytes());
        v.extend(seg(1,48,1,&[],&info(13,7,0)));
        v.extend(seg(2,49,1,&[],&[]));v.extend(seg(3,51,0,&[],&[]));v
    }
    #[test]fn jbig2_sequential_file(){
        let v=file();let a=inspect(&v).unwrap();
        assert_eq!(a.container,Container::SequentialFile);assert_eq!(a.declared_pages,Some(1));
        assert_eq!(a.segments.len(),3);assert_eq!((a.pages[0].width,a.pages[0].height),(13,Some(7)));
        assert_eq!(&v[a.segments[0].payload.clone()],info(13,7,0).as_slice());
        assert_eq!(a.segments[0].header,13..24);
    }
    #[test]fn jbig2_embedded_globals_and_widths(){
        let mut v=seg(256,0,0,&[255],&[1,2]);
        v.extend(seg(257,0,0,&[256],&[]));
        v.extend(seg(65536,0,0,&[257],&[]));
        v.extend(seg(65537,38,300,&[65536],&[7]));
        let a=inspect(&v).unwrap();assert_eq!(a.container,Container::Embedded);
        assert_eq!(a.external_references,vec![255]);assert_eq!(a.segments[3].page,300);
        assert_eq!(a.segments[3].referred,vec![65536]);
    }
    #[test]fn jbig2_extended_reference_retention(){
        let mut v=seg(20,0,0,&[1,2,3,4,5,6,7,8],&[9]);
        // Eight references plus current segment occupy two retention bytes.
        v[9]=0xa5;v[10]=1;v[4]|=128;
        let a=inspect(&v).unwrap();let s=&a.segments[0];
        assert!(s.deferred_non_retain);assert_eq!(s.retention,vec![0xa5,1]);
        assert_eq!(s.referred,(1..=8).collect::<Vec<u32>>());
        assert_eq!(&v[s.payload.clone()],&[9]);
    }
    #[test]fn jbig2_striped_unknown_height(){
        let a=inspect(&seg(1,48,1,&[],&info(80,u32::MAX,0x800a))).unwrap();
        assert_eq!(a.pages[0].height,None);assert!(a.pages[0].striped);
        assert_eq!(a.pages[0].max_stripe_size,10);
        assert!(inspect(&seg(1,48,1,&[],&info(80,u32::MAX,0))).is_err());
    }
    #[test]fn jbig2_truncation_and_file_flags(){
        let v=file();
        for end in 0..v.len(){assert!(inspect(&v[..end]).is_err(),"prefix {end}");}
        let mut b=v.clone();b[8]=0;assert!(matches!(inspect(&b),Err(Error::Unsupported(_))));
        let mut b=v.clone();b[12]=2;assert!(inspect(&b).is_err());
        let mut b=v.clone();b.push(0);assert!(inspect(&b).is_err());
        let mut b=v.clone();b[8]=3;b.drain(9..13);assert_eq!(inspect(&b).unwrap().declared_pages,None);
    }
    #[test]fn jbig2_invalid_references_and_limits(){
        assert!(inspect(&seg(4,0,0,&[4],&[])).is_err());
        let mut b=seg(4,0,0,&[],&[]);b.extend(b.clone());assert!(inspect(&b).is_err());
        let mut b=seg(4,0,0,&[],&[]);b[5]=0xa0;assert!(inspect(&b).is_err());
        let mut b=seg(4,0,0,&[],&[]);b[7..11].fill(255);
        assert!(matches!(inspect(&b),Err(Error::Unsupported(_))));
        let b=[0,0,0,9,0,0xe0,0,0x10,1];assert!(matches!(inspect(&b),Err(Error::Limit(_))));
        assert!(matches!(inspect(&seg(1,48,1,&[],&info(1_048_577,1,0))),Err(Error::Limit(_))));
    }
    #[test]fn jbig2_inspection_not_generic_decode(){
        let mut dict=BTreeMap::new();
        dict.insert(b"Filter".to_vec(),crate::Object::Name(b"JBIG2Decode".to_vec()));
        let stream=crate::Object::Stream{dict,data:seg(1,48,1,&[],&info(13,7,0))};
        assert!(matches!(super::super::decode(&stream,1024),Err(Error::Unsupported("stream filter"))));
    }

    #[test]fn jbig2_pdf_globals_graph(){
        let mut g=seg(1,0,0,&[],&[10]);
        g.extend(seg(2,0,0,&[1],&[20,21]));
        let mut p=seg(3,48,1,&[],&info(16,8,0));
        p.extend(seg(4,0,1,&[2],&[30]));
        p.extend(seg(5,6,1,&[1,4],&[40]));
        let a=inspect_pdf_streams(&g,&p).unwrap();
        assert_eq!((a.page.width,a.page.height),(16,Some(8)));
        assert!(!a.explicit_end_of_page);
        let g1=SegmentLocation{source:SegmentSource::Globals,index:0};
        let g2=SegmentLocation{source:SegmentSource::Globals,index:1};
        let p1=SegmentLocation{source:SegmentSource::Page,index:1};
        assert_eq!(a.graph[1].dependencies,vec![g1]);
        assert_eq!(a.graph[3].dependencies,vec![g2]);
        assert_eq!(a.graph[4].dependencies,vec![g1,p1]);
        assert_eq!(&g[a.segment(g2).unwrap().payload.clone()],&[20,21]);
        assert_eq!(&p[a.segment(p1).unwrap().payload.clone()],&[30]);
        assert!(a.segment(SegmentLocation{source:SegmentSource::Page,index:99}).is_none());
    }
    #[test]fn jbig2_pdf_globals_missing_duplicate(){
        let mut p=seg(2,48,1,&[],&info(16,8,0));
        p.extend(seg(3,6,1,&[1],&[]));
        assert!(inspect_pdf_streams(&[],&p).is_err());
        assert!(inspect_pdf_streams(&seg(2,0,0,&[],&[]),&p).is_err());
        assert!(inspect_pdf_streams(&seg(1,0,0,&[],&[]),&p).is_ok());
        let mut g=seg(2,0,0,&[1],&[]);
        g.extend(seg(1,0,0,&[],&[]));
        assert!(inspect_pdf_streams(&g,&p).is_err());
    }
    #[test]fn jbig2_pdf_globals_scope(){
        let p=seg(8,48,1,&[],&info(8,8,0));
        for g in [seg(1,48,1,&[],&info(8,8,0)),seg(1,0,2,&[],&[]),
            seg(1,51,0,&[],&[]),seg(1,38,0,&[],&[]),file()]{
            assert!(inspect_pdf_streams(&g,&p).is_err());
        }
        assert!(inspect_pdf_streams(&[],&file()).is_err());
    }
    #[test]fn jbig2_pdf_page_lifecycle(){
        let mut p=seg(1,48,300,&[],&info(13,7,0));
        p.extend(seg(2,38,300,&[],&[1]));
        p.extend(seg(3,49,300,&[],&[]));
        p.extend(seg(4,51,0,&[],&[]));
        let a=inspect_pdf_streams(&[],&p).unwrap();
        assert_eq!(a.page.page,300);assert!(a.explicit_end_of_page);
        let mut p=seg(1,38,1,&[],&[]);
        p.extend(seg(2,48,1,&[],&info(13,7,0)));
        assert!(inspect_pdf_streams(&[],&p).is_err());
        let mut p=seg(1,48,1,&[],&info(13,7,0));
        p.extend(seg(2,49,1,&[],&[]));p.extend(seg(3,38,1,&[],&[]));
        assert!(inspect_pdf_streams(&[],&p).is_err());
    }
    #[test]fn jbig2_pdf_page_dependency_scope(){
        let mut p=seg(1,48,1,&[],&info(8,8,0));
        p.extend(seg(2,0,1,&[],&[]));
        let mut foreign=p.clone();foreign.extend(seg(3,6,2,&[2],&[]));
        assert!(inspect_pdf_streams(&[],&foreign).is_err());
        let mut global=p.clone();global.extend(seg(3,0,0,&[2],&[]));
        assert!(inspect_pdf_streams(&[],&global).is_err());
        let mut control=p.clone();control.extend(seg(3,6,1,&[1],&[]));
        assert!(inspect_pdf_streams(&[],&control).is_err());
        let mut good=p;good.extend(seg(3,6,1,&[2],&[]));
        assert!(inspect_pdf_streams(&[],&good).is_ok());
    }
    #[test]fn jbig2_pdf_unsupported_lifecycle(){
        for (h,stripes) in [(8,0x8004),(u32::MAX,0x8004)]{
            assert!(matches!(inspect_pdf_streams(&[],&seg(1,48,1,&[],&info(8,h,stripes))),Err(Error::Unsupported(_))));
        }
        let mut p=seg(1,48,1,&[],&info(8,8,0));
        p.extend(seg(2,50,1,&[],&4u32.to_be_bytes()));
        assert!(inspect_pdf_streams(&[],&p).is_err());
        let mut p=seg(1,48,1,&[],&info(8,8,0));
        p.extend(seg(2,48,2,&[],&info(8,8,0)));
        assert!(inspect_pdf_streams(&[],&p).is_err());
        assert!(inspect_pdf_streams(&[],&seg(1,0,0,&[],&[])).is_err());
    }
    #[test]fn jbig2_pdf_combined_segment_limit(){
        let mut g=Vec::new();
        for n in 1..=MAX_SEGMENTS as u32{g.extend(seg(n,0,0,&[],&[]));}
        let p=seg(MAX_SEGMENTS as u32+1,48,1,&[],&info(8,8,0));
        assert!(matches!(inspect_pdf_streams(&g,&p),Err(Error::Limit(_))));
    }
}
