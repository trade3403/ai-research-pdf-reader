//! GPOS PairPos format 1. Explicit glyph pairs, no lookup scheduling.
use crate::{Error,GlyphCoverage,GposValueRecord,GposValueLimits,GposAdjustment,parse_glyph_coverage,gpos_value_record_size};
#[derive(Debug,Clone,Copy)]
pub struct PairPosLimits{
    pub values:GposValueLimits,pub max_sets:usize,pub max_pairs:usize,
    pub max_coverage_records:usize,pub max_covered:u32,
}
impl Default for PairPosLimits{
    fn default()->Self{Self{values:Default::default(),max_sets:65536,max_pairs:262144,max_coverage_records:65536,max_covered:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PairValue{pub second:u16,pub first_value:GposValueRecord,pub second_value:GposValueRecord}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PairPositioning{
    coverage:GlyphCoverage,sets:Vec<Vec<PairValue>>,second_format:u16,
}
impl PairPositioning{
    pub fn len(&self)->usize{self.sets.len()}
    pub fn is_empty(&self)->bool{self.sets.is_empty()}
    pub fn second_format(&self)->u16{self.second_format}
    pub fn pair(&self,first:u16,second:u16)->Option<&PairValue>{
        let set=self.sets.get(self.coverage.index(first)? as usize)?;
        set.binary_search_by_key(&second,|v|v.second).ok().map(|i|&set[i])
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
    let end=p.checked_add(2).ok_or(Error::Limit("pair pos offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("pair pos truncated"))?.try_into().unwrap()))
}
/// Device offsets are relative to the enclosing PairSet. Shared offsets
/// are decoded under the same cumulative pair and device budgets.
pub fn parse_pair_positioning_format1(b:&[u8],glyph_count:u32,l:PairPosLimits)->Result<PairPositioning,Error>{
    if b.len()>l.values.max_bytes{return Err(Error::Limit("pair pos bytes"));}
    if word(b,0)?!=1{return Err(Error::Unsupported("pair pos format"));}
    let cov=usize::from(word(b,2)?);let f1=word(b,4)?;let f2=word(b,6)?;
    let stride=2+gpos_value_record_size(f1)?+gpos_value_record_size(f2)?;
    let n=usize::from(word(b,8)?);
    if n>l.max_sets{return Err(Error::Limit("pair pos sets"));}
    let header=10+2*n;
    if header>b.len()||cov<header{return Err(Error::Invalid("pair pos header range"));}
    let coverage=parse_glyph_coverage(b.get(cov..).ok_or(Error::Invalid("pair pos coverage offset"))?,
        glyph_count,l.max_coverage_records,l.max_covered)?;
    if coverage.len()!=n as u32{return Err(Error::Invalid("pair pos coverage count"));}
    let mut sets=Vec::new();sets.try_reserve_exact(n).map_err(|_|Error::Limit("pair pos allocation"))?;
    let mut pair_left=l.max_pairs;let mut device_left=l.values.max_device_entries;
    for i in 0..n{
        let base=usize::from(word(b,10+2*i)?);
        if base<header{return Err(Error::Invalid("pair pos set offset"));}
        let count=usize::from(word(b,base)?);
        pair_left=pair_left.checked_sub(count).ok_or(Error::Limit("pair pos pairs"))?;
        let end=base.checked_add(2).and_then(|v|count.checked_mul(stride).and_then(|s|v.checked_add(s))).ok_or(Error::Limit("pair pos size"))?;
        if end>b.len(){return Err(Error::Invalid("pair pos truncated records"));}
        let mut set:Vec<PairValue>=Vec::new();set.try_reserve_exact(count).map_err(|_|Error::Limit("pair pos allocation"))?;
        let mut at=base+2;
        for _ in 0..count{
            let second=word(b,at)?;
            if u32::from(second)>=glyph_count||set.last().is_some_and(|v|v.second>=second){
                return Err(Error::Invalid("pair pos second glyph order or domain"));
            }
            let (first_value,next)=crate::font_gpos_value::decode_value_record(b,at+2,base,end,f1,&mut device_left)?;
            let (second_value,next)=crate::font_gpos_value::decode_value_record(b,next,base,end,f2,&mut device_left)?;
            set.push(PairValue{second,first_value,second_value});at=next;
        }
        sets.push(set);
    }
    Ok(PairPositioning{coverage,sets,second_format:f2})
}
