//! Shared OpenType LookupFlag eligibility policy.
//! Logical glyph order is unchanged; no substitution or positioning is executed.
use crate::{Error,Gdef,GdefGlyphClass,GsubLookup,MarkGlyphSets};

pub const LOOKUP_RIGHT_TO_LEFT:u16=0x0001;
pub const LOOKUP_IGNORE_BASE:u16=0x0002;
pub const LOOKUP_IGNORE_LIGATURE:u16=0x0004;
pub const LOOKUP_IGNORE_MARK:u16=0x0008;
pub const LOOKUP_USE_MARK_SET:u16=0x0010;
pub const LOOKUP_MARK_ATTACHMENT_MASK:u16=0xff00;

#[derive(Debug,Clone,Copy)]
pub struct LookupFilterLimits{
    pub max_run_glyphs:usize,
    pub max_eligible:usize,
}
impl Default for LookupFilterLimits{
    fn default()->Self{Self{max_run_glyphs:262144,max_eligible:262144}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GlyphEligibility{
    Eligible,
    IgnoreBase,
    IgnoreLigature,
    IgnoreMark,
    OutsideMarkSet,
    WrongMarkAttachmentClass,
}
impl GlyphEligibility{
    pub fn is_eligible(self)->bool{self==Self::Eligible}
}
/// Validated policy borrowed from the owning font. In the absence of a
/// GlyphClassDef, glyphs remain unclassified; no Unicode-based inference
/// is attempted. A declared mark set must always resolve, even if marks
/// will be ignored. RightToLeft is retained as metadata, not a scan order.
#[derive(Debug,Clone,Copy)]
pub struct LookupFilter<'a>{
    flags:u16,
    glyph_count:u32,
    gdef:Option<&'a Gdef>,
    mark_set:Option<(&'a MarkGlyphSets,usize)>,
}
impl<'a> LookupFilter<'a>{
    pub fn new(flags:u16,mark_filtering_set:Option<u16>,glyph_count:u32,gdef:Option<&'a Gdef>)->Result<Self,Error>{
        if glyph_count==0||glyph_count>65536{return Err(Error::Invalid("lookup filter glyph domain"));}
        if flags&0x00e0!=0{return Err(Error::Invalid("lookup reserved flags"));}
        if gdef.is_some_and(|g|g.glyph_count()!=glyph_count){return Err(Error::Invalid("lookup GDEF glyph domain"));}
        let use_set=flags&LOOKUP_USE_MARK_SET!=0;
        if use_set!=mark_filtering_set.is_some(){return Err(Error::Invalid("lookup mark set flag mismatch"));}
        let mark_set=if let Some(index)=mark_filtering_set{
            let sets=gdef.and_then(Gdef::mark_sets).ok_or(Error::Invalid("lookup mark sets missing"))?;
            let index=usize::from(index);
            if sets.coverage(index).is_none(){return Err(Error::Invalid("lookup mark set index"));}
            Some((sets,index))
        }else{None};
        Ok(Self{flags,glyph_count,gdef,mark_set})
    }
    pub fn from_gsub_lookup(lookup:&GsubLookup,glyph_count:u32,gdef:Option<&'a Gdef>)->Result<Self,Error>{
        if !(1..=8).contains(&lookup.kind){return Err(Error::Invalid("lookup kind"));}
        Self::new(lookup.flags,lookup.mark_filtering_set,glyph_count,gdef)
    }
    /// GPOS has its own lookup kind domain (1..=9), including Type 9 extension.
    /// LookupFlag semantics and GDEF validation are shared with GSUB.
    pub fn from_gpos_lookup(lookup:&crate::GposLookup,glyph_count:u32,gdef:Option<&'a Gdef>)->Result<Self,Error>{
        if !(1..=9).contains(&lookup.kind){return Err(Error::Invalid("lookup kind"));}
        Self::new(lookup.flags,lookup.mark_filtering_set,glyph_count,gdef)
    }
    pub fn flags(&self)->u16{self.flags}
    pub fn right_to_left(&self)->bool{self.flags&LOOKUP_RIGHT_TO_LEFT!=0}
    pub fn mark_attachment_type(&self)->u16{(self.flags&LOOKUP_MARK_ATTACHMENT_MASK)>>8}
    pub fn mark_filtering_set(&self)->Option<usize>{self.mark_set.map(|(_,i)|i)}
    pub fn eligibility(&self,glyph:u16)->Result<GlyphEligibility,Error>{
        use GdefGlyphClass::*;
        use GlyphEligibility::*;
        if u32::from(glyph)>=self.glyph_count{return Err(Error::Invalid("lookup filter glyph"));}
        let class=self.gdef.and_then(|g|g.glyph_class(glyph)).unwrap_or(Unclassified);
        Ok(match class{
            Base if self.flags&LOOKUP_IGNORE_BASE!=0=>IgnoreBase,
            Ligature if self.flags&LOOKUP_IGNORE_LIGATURE!=0=>IgnoreLigature,
            Mark=>{
                if self.flags&LOOKUP_IGNORE_MARK!=0{IgnoreMark}
                else if let Some((sets,index))=self.mark_set{
                    if sets.contains(index,glyph).ok_or(Error::Invalid("lookup mark set query"))?{Eligible}else{OutsideMarkSet}
                }else{
                    let attachment=self.mark_attachment_type();
                    let actual=self.gdef.and_then(|g|g.mark_attachment_class(glyph)).unwrap_or(0);
                    if attachment!=0&&actual!=attachment{WrongMarkAttachmentClass}else{Eligible}
                }
            }
            _=>Eligible,
        })
    }
    /// Produce ascending original indices, preserving skipped glyphs and
    /// logical order. The caller can use these indices to map matches back
    /// to the original glyph stream. No mutation and no partial result on error.
    pub fn eligible_indices(&self,glyphs:&[u16],l:LookupFilterLimits)->Result<Vec<usize>,Error>{
        if glyphs.len()>l.max_run_glyphs{return Err(Error::Limit("lookup filter run length"));}
        if glyphs.iter().any(|&g|u32::from(g)>=self.glyph_count){return Err(Error::Invalid("lookup filter glyph"));}
        let mut result=Vec::new();
        for (i,&glyph) in glyphs.iter().enumerate(){
            if self.eligibility(glyph)?.is_eligible(){
                if result.len()>=l.max_eligible{return Err(Error::Limit("lookup filter eligible count"));}
                result.try_reserve(1).map_err(|_|Error::Limit("lookup filter allocation"))?;
                result.push(i);
            }
        }
        Ok(result)
    }
}
