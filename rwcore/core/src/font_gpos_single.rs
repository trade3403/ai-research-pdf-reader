//! GPOS SinglePos formats 1 and 2. Standalone matching and atomic
//! adjustment, without lookup flags, script scheduling or variation evaluation.
use crate::{Error,GlyphCoverage,GposValueRecord,GposValueLimits,GposAdjustment,
    parse_glyph_coverage,gpos_value_record_size};
#[derive(Debug,Clone,Copy)]
pub struct SinglePosLimits{
    pub values:GposValueLimits,pub max_records:usize,pub max_coverage_records:usize,pub max_covered:u32,
}
impl Default for SinglePosLimits{
    fn default()->Self{Self{values:Default::default(),max_records:65536,max_coverage_records:65536,max_covered:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SinglePositioning{
    coverage:GlyphCoverage,values:Vec<GposValueRecord>,shared:bool,
}
impl SinglePositioning{
    pub fn len(&self)->u32{self.coverage.len()}
    pub fn is_empty(&self)->bool{self.coverage.is_empty()}
    pub fn is_shared(&self)->bool{self.shared}
    pub fn value(&self,glyph:u16)->Option<&GposValueRecord>{
        let i=self.coverage.index(glyph)?;
        self.values.get(if self.shared{0}else{i as usize})
    }
    /// Nominal design-unit adjustment, explicitly ignoring Device data.
    /// Returns None for a glyph outside coverage.
    pub fn apply_design(&self,glyph:u16,prior:GposAdjustment)->Result<Option<GposAdjustment>,Error>{
        self.value(glyph).map(|v|prior.checked_add(v.design_adjustment())).transpose()
    }
    /// Separate pixel and design adjustments; unresolved variation
    /// indices are rejected. Inputs are values, so errors are atomic.
    pub fn apply(&self,glyph:u16,design:GposAdjustment,pixels:GposAdjustment,x_ppem:u16,y_ppem:u16)
        ->Result<Option<(GposAdjustment,GposAdjustment)>,Error>{
        self.value(glyph).map(|v|Ok((design.checked_add(v.design_adjustment())?,
            pixels.checked_add(v.pixel_adjustment(x_ppem,y_ppem)?)?))).transpose()
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("single pos offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("single pos truncated"))?.try_into().unwrap()))
}
/// Decode a SinglePos subtable at byte zero. Coverage and Device data
/// may follow inline records in either order. All Device entries across
/// all value records share one cumulative budget.
pub fn parse_single_positioning(b:&[u8],glyph_count:u32,l:SinglePosLimits)->Result<SinglePositioning,Error>{
    if b.len()>l.values.max_bytes{return Err(Error::Limit("single pos bytes"));}
    let format=word(b,0)?;let cov_at=usize::from(word(b,2)?);
    let value_format=word(b,4)?;let stride=gpos_value_record_size(value_format)?;
    let (shared,header,count)=match format{
        1=>(true,6usize,1usize),
        2=>(false,8usize,usize::from(word(b,6)?)),
        _=>return Err(Error::Unsupported("single pos format")),
    };
    if count>l.max_records{return Err(Error::Limit("single pos records"));}
    let end=header.checked_add(count.checked_mul(stride).ok_or(Error::Limit("single pos size"))?).ok_or(Error::Limit("single pos size"))?;
    if end>b.len()||cov_at<end{return Err(Error::Invalid("single pos inline range"));}
    let coverage=parse_glyph_coverage(b.get(cov_at..).ok_or(Error::Invalid("single pos coverage offset"))?,
        glyph_count,l.max_coverage_records,l.max_covered)?;
    if !shared&&coverage.len()!=count as u32{return Err(Error::Invalid("single pos coverage count"));}
    let mut values=Vec::new();values.try_reserve_exact(count).map_err(|_|Error::Limit("single pos allocation"))?;
    let mut left=l.values.max_device_entries;
    let mut at=header;
    for _ in 0..count{
        let (v,next)=crate::font_gpos_value::decode_value_record(b,at,0,end,value_format,&mut left)?;
        values.push(v);at=next;
    }
    Ok(SinglePositioning{coverage,values,shared})
}
