//! MarkToBase format 1: explicit base/mark selection and attachment geometry.
//! No automatic base search, lookup filtering or attachment chain traversal.
use crate::{Error,GlyphCoverage,GposAnchor,GposValueLimits,parse_glyph_coverage};
#[derive(Debug,Clone,Copy)]
pub struct MarkPosLimits{
    pub values:GposValueLimits,pub max_marks:usize,pub max_bases:usize,
    pub max_classes:usize,pub max_cells:usize,pub max_coverage_records:usize,pub max_covered:u32,
}
impl Default for MarkPosLimits{
    fn default()->Self{Self{values:Default::default(),max_marks:65536,max_bases:65536,
        max_classes:65536,max_cells:262144,max_coverage_records:65536,max_covered:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposMarkRecord{pub class:u16,pub anchor:GposAnchor}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MarkToBasePositioning{
    marks:GlyphCoverage,bases:GlyphCoverage,records:Vec<GposMarkRecord>,
    cells:Vec<Option<GposAnchor>>,classes:usize,
}
impl MarkToBasePositioning{
    pub fn dimensions(&self)->(usize,usize){(self.bases.len() as usize,self.classes)}
    pub fn mark(&self,glyph:u16)->Option<&GposMarkRecord>{self.records.get(self.marks.index(glyph)? as usize)}
    pub fn base_anchor(&self,glyph:u16,class:u16)->Option<&GposAnchor>{
        if usize::from(class)>=self.classes{return None;}
        self.cells.get(self.bases.index(glyph)? as usize*self.classes+usize::from(class))?.as_ref()
    }
    /// Return (base anchor, mark anchor), only for an actual class match.
    pub fn pair(&self,base:u16,mark:u16)->Option<(&GposAnchor,&GposAnchor)>{
        let m=self.mark(mark)?;Some((self.base_anchor(base,m.class)?,&m.anchor))
    }
    /// Relative translation of the mark, with the base fixed. Nominal
    /// design units only; supplied origins include prior placement.
    pub fn attach_design(&self,base:u16,mark:u16,origins:[(i32,i32);2])->Result<Option<(i32,i32)>,Error>{
        self.pair(base,mark).map(|(a,b)|attachment((a.x as i32,a.y as i32),(b.x as i32,b.y as i32),origins)).transpose()
    }
    /// Separate pixel-space correction, not scaled design coordinates.
    /// Unresolved contour and variation indices are rejected.
    pub fn attach_pixels(&self,base:u16,mark:u16,origins:[(i32,i32);2],x_ppem:u16,y_ppem:u16)->Result<Option<(i32,i32)>,Error>{
        self.pair(base,mark).map(|(a,b)|attachment(a.pixel_offset(x_ppem,y_ppem)?,b.pixel_offset(x_ppem,y_ppem)?,origins)).transpose()
    }
}
pub(crate) fn attachment(a:(i32,i32),b:(i32,i32),origins:[(i32,i32);2])->Result<(i32,i32),Error>{
    let axis=|o1:i32,o2:i32,x:i32,y:i32|i32::try_from(i64::from(o1)+i64::from(x)-i64::from(o2)-i64::from(y))
        .map_err(|_|Error::Limit("mark attachment overflow"));
    Ok((axis(origins[0].0,origins[1].0,a.0,b.0)?,axis(origins[0].1,origins[1].1,a.1,b.1)?))
}
pub(crate) fn mark_word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("mark offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("mark truncated"))?.try_into().unwrap()))
}
pub(crate) fn mark_anchor(b:&[u8],base:usize,p:usize,end:usize,left:&mut usize,nullable:bool)->Result<Option<GposAnchor>,Error>{
    let rel=usize::from(mark_word(b,p)?);
    if rel==0&&nullable{return Ok(None);}
    let at=base.checked_add(rel).ok_or(Error::Limit("mark anchor offset"))?;
    if rel==0||at<end{return Err(Error::Invalid("mark anchor into records"));}
    crate::font_gpos_anchor::decode_anchor(b,at,left).map(Some)
}
pub(crate) fn decode_marks(b:&[u8],base:usize,coverage:&GlyphCoverage,classes:usize,l:MarkPosLimits,left:&mut usize)->Result<Vec<GposMarkRecord>,Error>{
    let n=usize::from(mark_word(b,base)?);
    if n>l.max_marks{return Err(Error::Limit("mark records"));}
    if n as u32!=coverage.len(){return Err(Error::Invalid("mark coverage count"));}
    let end=base.checked_add(2+4*n).ok_or(Error::Limit("mark records size"))?;
    if end>b.len(){return Err(Error::Invalid("mark records truncated"));}
    let mut records=Vec::new();records.try_reserve_exact(n).map_err(|_|Error::Limit("mark allocation"))?;
    for i in 0..n{
        let p=base+2+4*i;let class=mark_word(b,p)?;
        if usize::from(class)>=classes{return Err(Error::Invalid("mark class"));}
        let anchor=mark_anchor(b,base,p+2,end,left,false)?.ok_or(Error::Invalid("mark missing anchor"))?;
        records.push(GposMarkRecord{class,anchor});
    }
    Ok(records)
}
pub fn parse_mark_to_base(b:&[u8],glyph_count:u32,l:MarkPosLimits)->Result<MarkToBasePositioning,Error>{
    if b.len()>l.values.max_bytes{return Err(Error::Limit("mark bytes"));}
    if mark_word(b,0)?!=1{return Err(Error::Unsupported("mark to base format"));}
    let mc=usize::from(mark_word(b,2)?);let bc=usize::from(mark_word(b,4)?);
    let classes=usize::from(mark_word(b,6)?);
    let ma=usize::from(mark_word(b,8)?);let ba=usize::from(mark_word(b,10)?);
    if classes==0{return Err(Error::Invalid("mark class count"));}
    if classes>l.max_classes{return Err(Error::Limit("mark classes"));}
    if [mc,bc,ma,ba].iter().any(|&p|p<12||p>=b.len()){return Err(Error::Invalid("mark table offset"));}
    let marks=parse_glyph_coverage(&b[mc..],glyph_count,l.max_coverage_records,l.max_covered)?;
    let bases=parse_glyph_coverage(&b[bc..],glyph_count,l.max_coverage_records,l.max_covered)?;
    let n=usize::from(mark_word(b,ba)?);
    if n>l.max_bases{return Err(Error::Limit("base records"));}
    if n as u32!=bases.len(){return Err(Error::Invalid("base coverage count"));}
    let count=n.checked_mul(classes).ok_or(Error::Limit("base cells"))?;
    if count>l.max_cells{return Err(Error::Limit("base cells"));}
    let end=ba.checked_add(2).and_then(|p|count.checked_mul(2).and_then(|s|p.checked_add(s))).ok_or(Error::Limit("base size"))?;
    if end>b.len(){return Err(Error::Invalid("base truncated"));}
    let mut left=l.values.max_device_entries;
    let records=decode_marks(b,ma,&marks,classes,l,&mut left)?;
    let mut cells=Vec::new();cells.try_reserve_exact(count).map_err(|_|Error::Limit("base allocation"))?;
    for i in 0..count{cells.push(mark_anchor(b,ba,ba+2+2*i,end,&mut left,true)?);}
    Ok(MarkToBasePositioning{marks,bases,records,cells,classes})
}
