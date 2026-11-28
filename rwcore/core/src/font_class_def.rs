//! Shared bounded OpenType ClassDef format 1/2 decoder.
use crate::Error;
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GlyphClassDef{ranges:Vec<(u16,u16,u16)>,glyph_count:u32}
impl GlyphClassDef{
    /// Unassigned in-domain glyphs belong to class zero. Outside face: None.
    pub fn class(&self,id:u16)->Option<u16>{
        if u32::from(id)>=self.glyph_count{return None;}
        let i=self.ranges.partition_point(|r|r.1<id);
        Some(self.ranges.get(i).filter(|r|r.0<=id).map_or(0,|r|r.2))
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let a=b.get(p..p.checked_add(2).ok_or(Error::Limit("class offset"))?).ok_or(Error::Invalid("class truncated"))?;
    Ok(u16::from_be_bytes([a[0],a[1]]))
}
/// Class limit is exclusive, including implicit class zero. Trailing bytes allowed.
pub fn parse_glyph_class_def(b:&[u8],glyph_count:u32,class_limit:u32,max_records:usize,max_assigned:u32)->Result<GlyphClassDef,Error>{
    if glyph_count==0||glyph_count>65536||class_limit==0||class_limit>65536{return Err(Error::Invalid("class domain"));}
    let format=word(b,0)?;
    let (n,start,stride,base)=match format{
        1=>(usize::from(word(b,4)?),u32::from(word(b,2)?),2,6),
        2=>(usize::from(word(b,2)?),0,6,4),
        _=>return Err(Error::Unsupported("class format")),
    };
    if n>max_records{return Err(Error::Limit("class records"));}
    if base+n*stride>b.len(){return Err(Error::Invalid("class truncated records"));}
    if format==1&&start+n as u32>glyph_count{return Err(Error::Invalid("class glyph range"));}
    let mut ranges:Vec<(u16,u16,u16)>=Vec::with_capacity(n);
    let mut assigned=0u32;
    for i in 0..n{
        let p=base+i*stride;
        let (a,z,c)=if format==1{let id=(start+i as u32) as u16;(id,id,word(b,p)?)}else{(word(b,p)?,word(b,p+2)?,word(b,p+4)?)};
        if a>z||u32::from(z)>=glyph_count||u32::from(c)>=class_limit||ranges.last().is_some_and(|r|r.1>=a){
            return Err(Error::Invalid("class range or value"));
        }
        assigned+=u32::from(z)-u32::from(a)+1;
        if assigned>max_assigned{return Err(Error::Limit("class assigned glyphs"));}
        ranges.push((a,z,c));
    }
    Ok(GlyphClassDef{ranges,glyph_count})
}
