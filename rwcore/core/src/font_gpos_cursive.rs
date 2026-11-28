//! Standalone CursivePos format 1 anchor matching and connection geometry.
//! Caller supplies origins in the same coordinate system; no bidi,
//! lookup filtering, advance redistribution or attachment-chain traversal.
use crate::{Error,GlyphCoverage,GposAnchor,GposValueLimits,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct CursivePosLimits{
    pub values:GposValueLimits,pub max_records:usize,pub max_coverage_records:usize,pub max_covered:u32,
}
impl Default for CursivePosLimits{
    fn default()->Self{Self{values:Default::default(),max_records:65536,max_coverage_records:65536,max_covered:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CursiveAnchors{pub entry:Option<GposAnchor>,pub exit:Option<GposAnchor>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CursivePositioning{coverage:GlyphCoverage,records:Vec<CursiveAnchors>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CursiveFixedEnd{Exit,Entry}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct CursiveConnection{pub exit_shift:(i32,i32),pub entry_shift:(i32,i32)}
fn connection(exit:(i32,i32),entry:(i32,i32),origins:[(i32,i32);2],fixed:CursiveFixedEnd)->Result<CursiveConnection,Error>{
    // i64 intermediate prevents false overflow when large coordinates cancel.
    let dx=i64::from(origins[0].0)+i64::from(exit.0)-i64::from(origins[1].0)-i64::from(entry.0);
    let dy=i64::from(origins[0].1)+i64::from(exit.1)-i64::from(origins[1].1)-i64::from(entry.1);
    let cast=|v:i64|i32::try_from(v).map_err(|_|Error::Limit("cursive connection overflow"));
    match fixed{
        CursiveFixedEnd::Exit=>Ok(CursiveConnection{exit_shift:(0,0),entry_shift:(cast(dx)?,cast(dy)?)}),
        CursiveFixedEnd::Entry=>Ok(CursiveConnection{exit_shift:(cast(-dx)?,cast(-dy)?),entry_shift:(0,0)}),
    }
}
impl CursivePositioning{
    pub fn len(&self)->usize{self.records.len()}
    pub fn is_empty(&self)->bool{self.records.is_empty()}
    pub fn anchors(&self,glyph:u16)->Option<&CursiveAnchors>{self.records.get(self.coverage.index(glyph)? as usize)}
    pub fn pair(&self,exit:u16,entry:u16)->Option<(&GposAnchor,&GposAnchor)>{
        Some((self.anchors(exit)?.exit.as_ref()?,self.anchors(entry)?.entry.as_ref()?))
    }
    /// Nominal design units only. Outline and device corrections ignored.
    pub fn connect_design(&self,exit:u16,entry:u16,origins:[(i32,i32);2],fixed:CursiveFixedEnd)
        ->Result<Option<CursiveConnection>,Error>{
        self.pair(exit,entry).map(|(a,b)|connection((i32::from(a.x),i32::from(a.y)),(i32::from(b.x),i32::from(b.y)),origins,fixed)).transpose()
    }
    /// Pixel corrections only, using pixel-space origin corrections.
    /// Unresolved contour points and variation indices are rejected.
    pub fn connect_pixels(&self,exit:u16,entry:u16,origins:[(i32,i32);2],fixed:CursiveFixedEnd,x_ppem:u16,y_ppem:u16)
        ->Result<Option<CursiveConnection>,Error>{
        self.pair(exit,entry).map(|(a,b)|connection(a.pixel_offset(x_ppem,y_ppem)?,b.pixel_offset(x_ppem,y_ppem)?,origins,fixed)).transpose()
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("cursive offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("cursive truncated"))?.try_into().unwrap()))
}
pub fn parse_cursive_positioning(b:&[u8],glyph_count:u32,l:CursivePosLimits)->Result<CursivePositioning,Error>{
    if b.len()>l.values.max_bytes{return Err(Error::Limit("cursive bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("cursive format"));}
    let cov=usize::from(word(b,2)?);let n=usize::from(word(b,4)?);
    if n>l.max_records{return Err(Error::Limit("cursive records"));}
    let end=6+4*n;
    if end>b.len()||cov<end{return Err(Error::Invalid("cursive inline range"));}
    let coverage=parse_glyph_coverage(b.get(cov..).ok_or(Error::Invalid("cursive coverage offset"))?,glyph_count,l.max_coverage_records,l.max_covered)?;
    if coverage.len()!=n as u32{return Err(Error::Invalid("cursive coverage count"));}
    let mut records=Vec::new();records.try_reserve_exact(n).map_err(|_|Error::Limit("cursive allocation"))?;
    let mut left=l.values.max_device_entries;
    let mut anchor=|p:usize|{
        let at=usize::from(word(b,p)?);
        if at==0{return Ok(None);}
        if at<end{return Err(Error::Invalid("cursive anchor into records"));}
        crate::font_gpos_anchor::decode_anchor(b,at,&mut left).map(Some)
    };
    for i in 0..n{records.push(CursiveAnchors{entry:anchor(6+4*i)?,exit:anchor(8+4*i)?});}
    Ok(CursivePositioning{coverage,records})
}
