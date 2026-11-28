//! C4.1: bounded SFNT / TTC directory parsing, independent of font engines.
use crate::Error;
use std::ops::Range;

pub type FontTag=[u8;4];
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FontFlavor{TrueType,OpenTypeCff}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FontContainerKind{Single,CollectionV1,CollectionV2}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontLimits{
    pub max_bytes:usize,pub max_faces:usize,pub max_tables_per_face:usize,pub max_total_tables:usize,
}
impl Default for FontLimits{
    fn default()->Self{Self{max_bytes:128*1024*1024,max_faces:64,max_tables_per_face:256,max_total_tables:4096}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontTableRecord{
    pub tag:FontTag,pub checksum:u32,pub offset:usize,pub length:usize,
}
#[derive(Debug)]
pub struct FontFace<'a>{
    bytes:&'a [u8],offset:usize,flavor:FontFlavor,tables:Vec<FontTableRecord>,search_fields_valid:bool,
}
impl<'a> FontFace<'a>{
    pub fn offset(&self)->usize{self.offset}
    pub fn flavor(&self)->FontFlavor{self.flavor}
    pub fn tables(&self)->&[FontTableRecord]{&self.tables}
    /// Search parameters are advisory; lookups use our independently sorted index.
    pub fn search_fields_valid(&self)->bool{self.search_fields_valid}
    pub fn record(&self,tag:FontTag)->Option<&FontTableRecord>{
        self.tables.binary_search_by_key(&tag,|r|r.tag).ok().map(|i|&self.tables[i])
    }
    pub fn table(&self,tag:FontTag)->Option<&'a [u8]>{
        let r=self.record(tag)?;
        self.bytes.get(r.offset..r.offset+r.length)
    }
    /// Table checksums use big-endian u32 words, zero-padding the final word.
    /// The head checksum excludes checkSumAdjustment (bytes 8..12).
    pub fn verify_table_checksum(&self,tag:FontTag)->Result<bool,Error>{
        let r=self.record(tag).ok_or(Error::Invalid("font missing table"))?;
        let bytes=self.table(tag).ok_or(Error::Invalid("font table range"))?;
        if tag==*b"head"&&bytes.len()<12{return Err(Error::Invalid("font head checksum field"));}
        let mut sum=0u32;
        for (i,chunk) in bytes.chunks(4).enumerate(){
            if tag==*b"head"&&i==2{continue;}
            let mut word=[0;4];word[..chunk.len()].copy_from_slice(chunk);
            sum=sum.wrapping_add(u32::from_be_bytes(word));
        }
        Ok(sum==r.checksum)
    }
}
#[derive(Debug)]
pub struct FontContainer<'a>{
    kind:FontContainerKind,faces:Vec<FontFace<'a>>,signature:Option<&'a [u8]>,
}
impl<'a> FontContainer<'a>{
    pub fn kind(&self)->FontContainerKind{self.kind}
    pub fn faces(&self)->&[FontFace<'a>]{&self.faces}
    pub fn face(&self,index:usize)->Option<&FontFace<'a>>{self.faces.get(index)}
    /// Raw TTC v2 DSIG payload only; no trust or cryptographic verification.
    pub fn collection_signature(&self)->Option<&'a [u8]>{self.signature}
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("font offset arithmetic"))}
fn mul(a:usize,b:usize)->Result<usize,Error>{a.checked_mul(b).ok_or(Error::Limit("font size arithmetic"))}
fn range(bytes:&[u8],at:usize,n:usize)->Result<Range<usize>,Error>{
    let end=add(at,n)?;
    if end>bytes.len(){return Err(Error::Invalid("font truncated range"));}
    Ok(at..end)
}
fn word(bytes:&[u8],at:usize)->Result<u16,Error>{
    let r=range(bytes,at,2)?;
    Ok(u16::from_be_bytes(bytes[r].try_into().expect("two bytes")))
}
fn dword(bytes:&[u8],at:usize)->Result<u32,Error>{
    let r=range(bytes,at,4)?;
    Ok(u32::from_be_bytes(bytes[r].try_into().expect("four bytes")))
}
fn usize32(bytes:&[u8],at:usize)->Result<usize,Error>{
    usize::try_from(dword(bytes,at)?).map_err(|_|Error::Limit("font address width"))
}
fn tag(bytes:&[u8],at:usize)->Result<FontTag,Error>{
    let r=range(bytes,at,4)?;
    Ok(bytes[r].try_into().expect("four bytes"))
}
fn flavor(bytes:&[u8],at:usize)->Result<FontFlavor,Error>{
    match tag(bytes,at)?{
        [0,1,0,0]=>Ok(FontFlavor::TrueType),
        [b'O',b'T',b'T',b'O']=>Ok(FontFlavor::OpenTypeCff),
        _=>Err(Error::Unsupported("font SFNT flavor")),
    }
}
fn aligned(at:usize)->Result<(),Error>{
    if at%4!=0{return Err(Error::Invalid("font unaligned offset"));}
    Ok(())
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum RegionKind{Metadata,Table(FontTag,u32)}
#[derive(Debug,Clone)]
struct Region{range:Range<usize>,kind:RegionKind}
fn push_region(regions:&mut Vec<Region>,r:Range<usize>,kind:RegionKind){
    if !r.is_empty(){regions.push(Region{range:r,kind});}
}
fn validate_regions(regions:&mut [Region])->Result<(),Error>{
    regions.sort_unstable_by_key(|r|(r.range.start,r.range.end));
    for pair in regions.windows(2){
        let a=&pair[0];let b=&pair[1];
        if a.range.end>b.range.start{
            // Exact shared TTC table data is allowed. Partial aliasing, table
            // versus directory overlap, and inconsistent shared tags/checksums fail.
            if a.range!=b.range||a.kind!=b.kind||!matches!(a.kind,RegionKind::Table(..)){
                return Err(Error::Invalid("font overlapping regions"));
            }
        }
    }
    Ok(())
}
fn parse_face<'a>(bytes:&'a [u8],at:usize,limits:FontLimits,total:&mut usize,regions:&mut Vec<Region>)->Result<FontFace<'a>,Error>{
    aligned(at)?;
    range(bytes,at,12)?;
    let flavor=flavor(bytes,at)?;
    let count=usize::from(word(bytes,add(at,4)?)?);
    if count==0{return Err(Error::Invalid("font empty table directory"));}
    if count>limits.max_tables_per_face||count>4095{return Err(Error::Limit("font tables per face"));}
    *total=add(*total,count)?;
    if *total>limits.max_total_tables{return Err(Error::Limit("font total tables"));}
    let dir=range(bytes,at,add(12,mul(count,16)?)?)?;
    push_region(regions,dir,RegionKind::Metadata);
    let pow=1usize<<(usize::BITS-1-count.leading_zeros());
    let expected_search=pow*16;
    let expected_selector=pow.trailing_zeros() as usize;
    let search_fields_valid=usize::from(word(bytes,at+6)?)==expected_search
        &&usize::from(word(bytes,at+8)?)==expected_selector
        &&usize::from(word(bytes,at+10)?)==count*16-expected_search;
    let mut tables=Vec::with_capacity(count);
    for i in 0..count{
        let p=at+12+i*16;
        let t=tag(bytes,p)?;
        if !t.iter().all(|b|(0x20..=0x7e).contains(b)){return Err(Error::Invalid("font table tag"));}
        let checksum=dword(bytes,p+4)?;
        let offset=usize32(bytes,p+8)?;
        let length=usize32(bytes,p+12)?;
        aligned(offset)?;
        let r=range(bytes,offset,length)?;
        push_region(regions,r,RegionKind::Table(t,checksum));
        tables.push(FontTableRecord{tag:t,checksum,offset,length});
    }
    tables.sort_unstable_by_key(|r|r.tag);
    if tables.windows(2).any(|p|p[0].tag==p[1].tag){return Err(Error::Invalid("font duplicate table tag"));}
    Ok(FontFace{bytes,offset:at,flavor,tables,search_fields_valid})
}
/// Parse a borrowed raw SFNT file or TTC/OTC collection. Table offsets are
/// absolute from the container start (also in TTC). No decompression,
/// outlines, glyph mapping, required-table validation or font execution.
/// Table checksums are opt-in; malformed byte ranges always fail.
pub fn parse_font(bytes:&[u8],limits:FontLimits)->Result<FontContainer<'_>,Error>{
    if bytes.len()>limits.max_bytes{return Err(Error::Limit("font file bytes"));}
    let mut regions=Vec::new();
    let mut signature=None;
    let (kind,offsets)=if tag(bytes,0)?==*b"ttcf"{
        let kind=match dword(bytes,4)?{
            0x00010000=>FontContainerKind::CollectionV1,
            0x00020000=>FontContainerKind::CollectionV2,
            _=>return Err(Error::Unsupported("font collection version")),
        };
        let count=usize32(bytes,8)?;
        if count==0{return Err(Error::Invalid("font empty collection"));}
        if count>limits.max_faces{return Err(Error::Limit("font collection faces"));}
        let end=add(12,mul(count,4)?)?;
        let header_end=add(end,if kind==FontContainerKind::CollectionV2{12}else{0})?;
        push_region(&mut regions,range(bytes,0,header_end)?,RegionKind::Metadata);
        if kind==FontContainerKind::CollectionV2{
            let t=tag(bytes,end)?;let len=usize32(bytes,end+4)?;let at=usize32(bytes,end+8)?;
            if t==[0;4]&&len==0&&at==0{}else{
                if t!=*b"DSIG"||len==0{return Err(Error::Invalid("font collection signature fields"));}
                aligned(at)?;
                let r=range(bytes,at,len)?;
                signature=Some(&bytes[r.clone()]);
                push_region(&mut regions,r,RegionKind::Metadata);
            }
        }
        let mut offsets=Vec::with_capacity(count);
        for i in 0..count{offsets.push(usize32(bytes,12+i*4)?);}
        (kind,offsets)
    }else{
        if limits.max_faces==0{return Err(Error::Limit("font collection faces"));}
        (FontContainerKind::Single,vec![0])
    };
    let mut total=0;
    let mut faces=Vec::with_capacity(offsets.len());
    for at in offsets{faces.push(parse_face(bytes,at,limits,&mut total,&mut regions)?);}
    validate_regions(&mut regions)?;
    Ok(FontContainer{kind,faces,signature})
}
