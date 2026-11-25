//! C4.2b: native Unicode cmap 4/12 mapping, checked against maxp glyph count.
use crate::{Error,FontFace,FontMetricLimits};
use crate::font_metrics::parse_maxp;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CmapFormat{Format4,Format12}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct CmapSource{pub platform:u16,pub encoding:u16,pub format:CmapFormat}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct CmapLimits{
    pub max_records:usize,pub max_segments:usize,pub max_groups:usize,pub max_validation_codes:usize,
}
impl Default for CmapLimits{
    fn default()->Self{Self{max_records:256,max_segments:8192,max_groups:65536,max_validation_codes:65536}}
}
#[derive(Debug)]
struct Segment{start:u16,end:u16,delta:u16,glyph_start:Option<usize>}
impl Segment{
    fn glyph(&self,b:&[u8],c:u16)->Result<u16,Error>{
        if let Some(at)=self.glyph_start{
            let v=u16_at(b,add(at,mul(usize::from(c-self.start),2)?)?)?;
            Ok(if v==0{0}else{v.wrapping_add(self.delta)})
        }else{Ok(c.wrapping_add(self.delta))}
    }
}
#[derive(Debug)]
struct Group{start:u32,end:u32,glyph:u32}
#[derive(Debug)]
enum Mapping<'a>{Bmp{bytes:&'a [u8],segments:Vec<Segment>,search_valid:bool},Full(Vec<Group>)}
#[derive(Debug)]
pub struct UnicodeCmap<'a>{source:CmapSource,glyph_count:u16,mapping:Mapping<'a>}
impl UnicodeCmap<'_>{
    pub fn source(&self)->CmapSource{self.source}
    pub fn glyph_count(&self)->u16{self.glyph_count}
    /// Search fields are advisory, not used for binary search.
    pub fn format4_search_fields_valid(&self)->Option<bool>{
        match &self.mapping{Mapping::Bmp{search_valid,..}=>Some(*search_valid),_=>None}
    }
    /// Missing characters (including explicit glyph 0) return None.
    /// Invalid Unicode scalar values return an error rather than a glyph.
    pub fn glyph_id(&self,code:u32)->Result<Option<u16>,Error>{
        if char::from_u32(code).is_none(){return Err(Error::Invalid("cmap Unicode scalar"));}
        let id=match &self.mapping{
            Mapping::Bmp{bytes,segments,..}=>{
                if code>0xffff{return Ok(None);}
                let c=code as u16;
                let i=segments.partition_point(|s|s.end<c);
                match segments.get(i){Some(s) if s.start<=c=>s.glyph(bytes,c)?,_=>0}
            },
            Mapping::Full(groups)=>{
                let i=groups.partition_point(|g|g.end<code);
                match groups.get(i){Some(g) if g.start<=code=>(g.glyph+code-g.start) as u16,_=>0}
            },
        };
        if id>=self.glyph_count{return Err(Error::Invalid("cmap glyph out of range"));}
        Ok(if id==0{None}else{Some(id)})
    }
    pub fn glyph_for_char(&self,c:char)->Result<Option<u16>,Error>{self.glyph_id(c as u32)}
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("cmap arithmetic"))}
fn mul(a:usize,b:usize)->Result<usize,Error>{a.checked_mul(b).ok_or(Error::Limit("cmap arithmetic"))}
fn slice(b:&[u8],at:usize,n:usize)->Result<&[u8],Error>{
    b.get(at..add(at,n)?).ok_or(Error::Invalid("cmap truncated range"))
}
fn u16_at(b:&[u8],at:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(slice(b,at,2)?.try_into().expect("two bytes")))}
fn u32_at(b:&[u8],at:usize)->Result<u32,Error>{Ok(u32::from_be_bytes(slice(b,at,4)?.try_into().expect("four bytes")))}
fn usize_at(b:&[u8],at:usize)->Result<usize,Error>{usize::try_from(u32_at(b,at)?).map_err(|_|Error::Limit("cmap address width"))}
fn parse4(b:&[u8],glyph_count:u16,l:CmapLimits)->Result<Mapping<'_>,Error>{
    let b=slice(b,0,usize::from(u16_at(b,2)?))?;
    if b.len()<24||b.len()%2!=0||u16_at(b,4)?!=0{return Err(Error::Invalid("cmap format4 header"));}
    let doubled=usize::from(u16_at(b,6)?);
    if doubled==0||doubled%2!=0{return Err(Error::Invalid("cmap segment count"));}
    let n=doubled/2;
    if n>l.max_segments{return Err(Error::Limit("cmap segments"));}
    let starts=add(16,mul(n,2)?)?;
    let deltas=add(starts,mul(n,2)?)?;
    let offsets=add(deltas,mul(n,2)?)?;
    let glyph_array=add(offsets,mul(n,2)?)?;
    slice(b,0,glyph_array)?;
    if u16_at(b,14+n*2)?!=0{return Err(Error::Invalid("cmap reserved pad"));}
    let pow=1usize<<(usize::BITS-1-n.leading_zeros());
    let search_valid=usize::from(u16_at(b,8)?)==pow*2
        &&usize::from(u16_at(b,10)?)==pow.trailing_zeros() as usize
        &&usize::from(u16_at(b,12)?)==doubled-pow*2;
    let mut segments=Vec::with_capacity(n);
    let mut prev=None;
    let mut work=0usize;
    for i in 0..n{
        let end=u16_at(b,14+i*2)?;
        let start=u16_at(b,starts+i*2)?;
        if start>end||prev.is_some_and(|p|start<=p){return Err(Error::Invalid("cmap overlapping or reversed segments"));}
        prev=Some(end);
        let delta=u16_at(b,deltas+i*2)?;
        let offset_word=offsets+i*2;
        let offset=usize::from(u16_at(b,offset_word)?);
        let span=usize::from(end-start)+1;
        work=add(work,span)?;
        if work>l.max_validation_codes{return Err(Error::Limit("cmap format4 validation codes"));}
        let glyph_start=if offset==0{None}else{
            if offset%2!=0{return Err(Error::Invalid("cmap odd glyph offset"));}
            let at=add(offset_word,offset)?;
            if at<glyph_array{return Err(Error::Invalid("cmap glyph offset before array"));}
            slice(b,at,mul(span,2)?)?;
            Some(at)
        };
        let s=Segment{start,end,delta,glyph_start};
        for c in start..=end{
            if s.glyph(b,c)? >= glyph_count{return Err(Error::Invalid("cmap glyph out of range"));}
        }
        segments.push(s);
    }
    if prev!=Some(0xffff){return Err(Error::Invalid("cmap missing terminal segment"));}
    Ok(Mapping::Bmp{bytes:b,segments,search_valid})
}
fn parse12(b:&[u8],glyph_count:u16,l:CmapLimits)->Result<Mapping<'_>,Error>{
    let b=slice(b,0,usize_at(b,4)?)?;
    if b.len()<16||u16_at(b,2)?!=0||u32_at(b,8)?!=0{return Err(Error::Invalid("cmap format12 header"));}
    let n=usize_at(b,12)?;
    if n>l.max_groups{return Err(Error::Limit("cmap groups"));}
    if add(16,mul(n,12)?)?!=b.len(){return Err(Error::Invalid("cmap format12 length"));}
    let mut groups=Vec::with_capacity(n);
    let mut prev=None;
    for i in 0..n{
        let at=16+i*12;
        let start=u32_at(b,at)?;let end=u32_at(b,at+4)?;let glyph=u32_at(b,at+8)?;
        if start>end||end>0x10ffff||(start<=0xdfff&&end>=0xd800)||prev.is_some_and(|p|start<=p){
            return Err(Error::Invalid("cmap invalid Unicode group"));
        }
        let last=glyph.checked_add(end-start).ok_or(Error::Invalid("cmap glyph range overflow"))?;
        if last>=u32::from(glyph_count){return Err(Error::Invalid("cmap glyph out of range"));}
        groups.push(Group{start,end,glyph});prev=Some(end);
    }
    Ok(Mapping::Full(groups))
}
/// Deterministic preference: full Unicode 12 over BMP 4; within format,
/// Windows Unicode then supported Unicode platform encodings. Only the
/// selected subtable is semantically validated; all record offsets checked.
/// Malformed preferred subtable is an error, not a silent BMP fallback.
/// No symbol remapping, variation selectors, or character-to-glyph shaping.
fn preference(platform:u16,encoding:u16,format:u16)->Option<(u8,CmapFormat)>{
    match (platform,encoding,format){
        (3,10,12)=>Some((7,CmapFormat::Format12)),
        (0,4,12)=>Some((6,CmapFormat::Format12)),
        (3,1,4)=>Some((5,CmapFormat::Format4)),
        (0,3,4)=>Some((4,CmapFormat::Format4)),
        (0,2,4)=>Some((3,CmapFormat::Format4)),
        (0,1,4)=>Some((2,CmapFormat::Format4)),
        (0,0,4)=>Some((1,CmapFormat::Format4)),
        _=>None,
    }
}
pub fn parse_unicode_cmap<'a>(face:&FontFace<'a>,l:CmapLimits)->Result<UnicodeCmap<'a>,Error>{
    let maxp=face.table(*b"maxp").ok_or(Error::Invalid("cmap missing maxp"))?;
    let glyph_count=parse_maxp(maxp,face.flavor(),FontMetricLimits::default())?.glyph_count;
    let b=face.table(*b"cmap").ok_or(Error::Invalid("cmap missing table"))?;
    if u16_at(b,0)?!=0{return Err(Error::Unsupported("cmap version"));}
    let n=usize::from(u16_at(b,2)?);
    if n>l.max_records{return Err(Error::Limit("cmap records"));}
    let header=add(4,mul(n,8)?)?;
    slice(b,0,header)?;
    let mut best:Option<(u8,CmapSource,usize)>=None;
    // Same supported encoding with different offsets is ambiguous even
    // if a higher-ranked record exists. Track bounded supported ranks.
    let mut seen=[None;8];
    for i in 0..n{
        let p=4+i*8;
        let platform=u16_at(b,p)?;let encoding=u16_at(b,p+2)?;
        let at=usize_at(b,p+4)?;
        if at<header{return Err(Error::Invalid("cmap offset into records"));}
        let format=u16_at(b,at)?;
        if let Some((rank,format))=preference(platform,encoding,format){
            let slot=&mut seen[usize::from(rank)];
            if slot.is_some_and(|old|old!=at){return Err(Error::Invalid("cmap ambiguous encoding"));}
            *slot=Some(at);
            if best.as_ref().map_or(true,|(old,_,_)|rank>*old){
                best=Some((rank,CmapSource{platform,encoding,format},at));
            }
        }
    }
    let (_,source,at)=best.ok_or(Error::Unsupported("cmap Unicode format 4 or 12"))?;
    let mapping=match source.format{
        CmapFormat::Format4=>parse4(&b[at..],glyph_count,l)?,
        CmapFormat::Format12=>parse12(&b[at..],glyph_count,l)?,
    };
    Ok(UnicodeCmap{source,glyph_count,mapping})
}
