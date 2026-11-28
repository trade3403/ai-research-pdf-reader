//! Bounded OpenType Coverage table decoder shared by future GSUB and GPOS.
use crate::Error;
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GlyphCoverage { ranges: Vec<(u16,u16,u32)>, count:u32 }
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("coverage offset"))?;
    let s=b.get(p..end).ok_or(Error::Invalid("coverage truncated"))?;
    Ok(u16::from_be_bytes([s[0],s[1]]))
}
impl GlyphCoverage {
    pub fn len(&self)->u32{self.count}
    pub fn is_empty(&self)->bool{self.count==0}
    pub fn index(&self,id:u16)->Option<u32>{
        let i=self.ranges.partition_point(|r|r.1<id);
        let &(a,b,start)=self.ranges.get(i)?;
        if id<a||id>b{return None;}
        Some(start+u32::from(id-a))
    }
}
/// Parse coverage at byte zero. Trailing parent-table bytes are permitted.
/// glyph_count is the exclusive glyph-ID bound from the owning face.
/// max_records bounds stored records; max_covered bounds expanded coverage.
pub fn parse_glyph_coverage(bytes:&[u8],glyph_count:u32,max_records:usize,max_covered:u32)->Result<GlyphCoverage,Error>{
    if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("coverage glyph count"));}
    let format=word(bytes,0)?;
    let stride=match format{1=>2,2=>6,_=>return Err(Error::Unsupported("coverage format"))};
    let n=usize::from(word(bytes,2)?);
    if n>max_records{return Err(Error::Limit("coverage records"));}
    let end=4+n*stride;
    if bytes.len()<end{return Err(Error::Invalid("coverage truncated records"));}
    let mut ranges:Vec<(u16,u16,u32)>=Vec::with_capacity(n);
    let mut count=0u32;
    for i in 0..n{
        let p=4+i*stride;
        let a=word(bytes,p)?;
        let b=if format==1{a}else{word(bytes,p+2)?};
        if a>b||u32::from(b)>=glyph_count||ranges.last().is_some_and(|r|r.1>=a){
            return Err(Error::Invalid("coverage glyph order or range"));
        }
        if format==2&&u32::from(word(bytes,p+4)?)!=count{return Err(Error::Invalid("coverage index sequence"));}
        let start=count;
        count+=u32::from(b)-u32::from(a)+1;
        if count>max_covered{return Err(Error::Limit("coverage glyphs"));}
        ranges.push((a,b,start));
    }
    Ok(GlyphCoverage{ranges,count})
}
