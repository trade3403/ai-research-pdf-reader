//! MarkToLigature format 1, explicit zero-based component selection.
use crate::{Error,GlyphCoverage,GposAnchor,GposMarkRecord,MarkPosLimits,parse_glyph_coverage};
use crate::font_gpos_mark_base::{attachment,mark_word,mark_anchor,decode_marks};

#[derive(Debug,Clone,Copy)]
pub struct MarkLigatureLimits {
    pub marks: MarkPosLimits,
    pub max_components: usize,
}
impl Default for MarkLigatureLimits {
    fn default()->Self { Self{marks:Default::default(),max_components:65536} }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MarkToLigaturePositioning {
    marks:GlyphCoverage, ligatures:GlyphCoverage, records:Vec<GposMarkRecord>,
    // (first cell, component count), in coverage order.
    spans:Vec<(usize,usize)>, cells:Vec<Option<GposAnchor>>, classes:usize,
}
impl MarkToLigaturePositioning {
    pub fn mark(&self,glyph:u16)->Option<&GposMarkRecord>{
        self.records.get(self.marks.index(glyph)? as usize)
    }
    pub fn component_count(&self,glyph:u16)->Option<usize>{
        Some(self.spans.get(self.ligatures.index(glyph)? as usize)?.1)
    }
    pub fn component_anchor(&self,glyph:u16,component:usize,class:u16)->Option<&GposAnchor>{
        let &(start,n)=self.spans.get(self.ligatures.index(glyph)? as usize)?;
        if component>=n || usize::from(class)>=self.classes{return None;}
        self.cells.get(start.checked_add(component.checked_mul(self.classes)?)?.checked_add(usize::from(class))?)?.as_ref()
    }
    pub fn pair(&self,ligature:u16,component:usize,mark:u16)->Option<(&GposAnchor,&GposAnchor)>{
        let m=self.mark(mark)?;
        Some((self.component_anchor(ligature,component,m.class)?,&m.anchor))
    }
    pub fn attach_design(&self,ligature:u16,component:usize,mark:u16,origins:[(i32,i32);2])->Result<Option<(i32,i32)>,Error>{
        self.pair(ligature,component,mark).map(|(a,b)|attachment((a.x as i32,a.y as i32),(b.x as i32,b.y as i32),origins)).transpose()
    }
    pub fn attach_pixels(&self,ligature:u16,component:usize,mark:u16,origins:[(i32,i32);2],x_ppem:u16,y_ppem:u16)->Result<Option<(i32,i32)>,Error>{
        self.pair(ligature,component,mark).map(|(a,b)|attachment(a.pixel_offset(x_ppem,y_ppem)?,b.pixel_offset(x_ppem,y_ppem)?,origins)).transpose()
    }
}
fn end_of(base:usize, count:usize, stride:usize)->Result<usize,Error>{
    count.checked_mul(stride).and_then(|n|n.checked_add(2)).and_then(|n|base.checked_add(n)).ok_or(Error::Limit("ligature size"))
}
pub fn parse_mark_to_ligature(b:&[u8],glyph_count:u32,limits:MarkLigatureLimits)->Result<MarkToLigaturePositioning,Error>{
    let l=limits.marks;
    if b.len()>l.values.max_bytes{return Err(Error::Limit("ligature bytes"));}
    if mark_word(b,0)?!=1{return Err(Error::Unsupported("mark to ligature format"));}
    let mc=usize::from(mark_word(b,2)?);let lc=usize::from(mark_word(b,4)?);
    let classes=usize::from(mark_word(b,6)?);
    let ma=usize::from(mark_word(b,8)?);let la=usize::from(mark_word(b,10)?);
    if classes==0{return Err(Error::Invalid("ligature class count"));}
    if classes>l.max_classes{return Err(Error::Limit("ligature classes"));}
    if [mc,lc,ma,la].iter().any(|&p|p<12||p>=b.len()){return Err(Error::Invalid("ligature table offset"));}
    let marks=parse_glyph_coverage(&b[mc..],glyph_count,l.max_coverage_records,l.max_covered)?;
    let ligatures=parse_glyph_coverage(&b[lc..],glyph_count,l.max_coverage_records,l.max_covered)?;
    let n=usize::from(mark_word(b,la)?);
    if n>l.max_bases{return Err(Error::Limit("ligature records"));}
    if n as u32!=ligatures.len(){return Err(Error::Invalid("ligature coverage count"));}
    let array_end=end_of(la,n,2)?;
    if array_end>b.len(){return Err(Error::Invalid("ligature array truncated"));}
    let mut left=l.values.max_device_entries;
    let records=decode_marks(b,ma,&marks,classes,l,&mut left)?;
    let mut spans=Vec::new();
    spans.try_reserve_exact(n).map_err(|_|Error::Limit("ligature allocation"))?;
    let mut cells=Vec::new();
    let mut components=0usize;
    for i in 0..n {
        let rel=usize::from(mark_word(b,la+2+2*i)?);
        let at=la.checked_add(rel).ok_or(Error::Limit("ligature offset"))?;
        if at<array_end{return Err(Error::Invalid("ligature attach into array"));}
        let c=usize::from(mark_word(b,at)?);
        if c==0{return Err(Error::Invalid("ligature component count"));}
        components=components.checked_add(c).ok_or(Error::Limit("ligature components"))?;
        if components>limits.max_components{return Err(Error::Limit("ligature components"));}
        let count=c.checked_mul(classes).ok_or(Error::Limit("ligature cells"))?;
        let total=cells.len().checked_add(count).ok_or(Error::Limit("ligature cells"))?;
        if total>l.max_cells{return Err(Error::Limit("ligature cells"));}
        let end=end_of(at,count,2)?;
        if end>b.len(){return Err(Error::Invalid("ligature components truncated"));}
        spans.push((cells.len(),c));
        cells.try_reserve(count).map_err(|_|Error::Limit("ligature allocation"))?;
        for j in 0..count {cells.push(mark_anchor(b,at,at+2+2*j,end,&mut left,true)?);}
    }
    Ok(MarkToLigaturePositioning{marks,ligatures,records,spans,cells,classes})
}
