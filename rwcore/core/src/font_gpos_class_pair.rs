//! PairPos format 2, class-indexed positioning matrix.
use crate::{Error,GlyphCoverage,GlyphClassDef,GposValueRecord,GposAdjustment,PairPosLimits,
    parse_glyph_coverage,parse_glyph_class_def,gpos_value_record_size};
#[derive(Debug,Clone,Copy)]
pub struct ClassPairPosLimits{
    pub pair:PairPosLimits,pub max_classes:usize,pub max_class_records:usize,pub max_class_assigned:u32,
}
impl Default for ClassPairPosLimits{
    fn default()->Self{Self{pair:Default::default(),max_classes:65536,max_class_records:65536,max_class_assigned:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassPairValue{pub first_value:GposValueRecord,pub second_value:GposValueRecord}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassPairPositioning{
    coverage:GlyphCoverage,first:GlyphClassDef,second:GlyphClassDef,
    rows:usize,cols:usize,values:Vec<ClassPairValue>,second_format:u16,
}
impl ClassPairPositioning{
    pub fn dimensions(&self)->(usize,usize){(self.rows,self.cols)}
    pub fn second_format(&self)->u16{self.second_format}
    pub fn classes(&self,first:u16,second:u16)->Option<(u16,u16)>{
        self.coverage.index(first)?;
        Some((self.first.class(first)?,self.second.class(second)?))
    }
    pub fn pair(&self,first:u16,second:u16)->Option<&ClassPairValue>{
        let (a,b)=self.classes(first,second)?;
        self.values.get(usize::from(a)*self.cols+usize::from(b))
    }
    pub fn apply_design(&self,first:u16,second:u16,prior:[GposAdjustment;2])->Result<Option<[GposAdjustment;2]>,Error>{
        self.pair(first,second).map(|v|Ok([prior[0].checked_add(v.first_value.design_adjustment())?,
            prior[1].checked_add(v.second_value.design_adjustment())?])).transpose()
    }
    pub fn apply(&self,first:u16,second:u16,design:[GposAdjustment;2],pixels:[GposAdjustment;2],x_ppem:u16,y_ppem:u16)
        ->Result<Option<([GposAdjustment;2],[GposAdjustment;2])>,Error>{
        self.pair(first,second).map(|v|Ok((
            [design[0].checked_add(v.first_value.design_adjustment())?,design[1].checked_add(v.second_value.design_adjustment())?],
            [pixels[0].checked_add(v.first_value.pixel_adjustment(x_ppem,y_ppem)?)?,
             pixels[1].checked_add(v.second_value.pixel_adjustment(x_ppem,y_ppem)?)?]
        ))).transpose()
    }
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("class pair offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("class pair truncated"))?.try_into().unwrap()))
}
/// Decode the dense class matrix, even when both value formats are zero.
/// Device offsets are relative to the positioning subtable. Both class
/// definitions have independent record/assigned caps; matrix cells and
/// Device entries have cumulative caps.
pub fn parse_pair_positioning_format2(b:&[u8],glyph_count:u32,l:ClassPairPosLimits)->Result<ClassPairPositioning,Error>{
    if b.len()>l.pair.values.max_bytes{return Err(Error::Limit("class pair bytes"));}
    if word(b,0)?!=2{return Err(Error::Unsupported("class pair format"));}
    let cov=usize::from(word(b,2)?);let f1=word(b,4)?;let f2=word(b,6)?;
    let c1=usize::from(word(b,8)?);let c2=usize::from(word(b,10)?);
    let rows=usize::from(word(b,12)?);let cols=usize::from(word(b,14)?);
    if rows==0||cols==0{return Err(Error::Invalid("class pair dimensions"));}
    if rows>l.max_classes||cols>l.max_classes{return Err(Error::Limit("class pair classes"));}
    let count=rows.checked_mul(cols).ok_or(Error::Limit("class pair cells"))?;
    if count>l.pair.max_pairs{return Err(Error::Limit("class pair cells"));}
    let stride=gpos_value_record_size(f1)?+gpos_value_record_size(f2)?;
    let end=16usize.checked_add(count.checked_mul(stride).ok_or(Error::Limit("class pair size"))?).ok_or(Error::Limit("class pair size"))?;
    if end>b.len()||[cov,c1,c2].iter().any(|&p|p<end||p>=b.len()){
        return Err(Error::Invalid("class pair inline range"));
    }
    let coverage=parse_glyph_coverage(&b[cov..],glyph_count,l.pair.max_coverage_records,l.pair.max_covered)?;
    let first=parse_glyph_class_def(&b[c1..],glyph_count,rows as u32,l.max_class_records,l.max_class_assigned)?;
    let second=parse_glyph_class_def(&b[c2..],glyph_count,cols as u32,l.max_class_records,l.max_class_assigned)?;
    let mut values=Vec::new();values.try_reserve_exact(count).map_err(|_|Error::Limit("class pair allocation"))?;
    let mut at=16;let mut left=l.pair.values.max_device_entries;
    for _ in 0..count{
        let (first_value,next)=crate::font_gpos_value::decode_value_record(b,at,0,end,f1,&mut left)?;
        let (second_value,next)=crate::font_gpos_value::decode_value_record(b,next,0,end,f2,&mut left)?;
        values.push(ClassPairValue{first_value,second_value});at=next;
    }
    Ok(ClassPairPositioning{coverage,first,second,rows,cols,values,second_format:f2})
}
