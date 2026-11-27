//! GPOS lookup type 7 format 1, glyph-based positioning context.
//! OpenType uses the same sequence-rule grammar as GSUB type 5.
//! This API matches a caller-supplied contiguous sequence and returns ordered
//! positioning lookup references; it does not execute lookups or filter glyphs.
use crate::{Error,ContextLimits,ContextRule,ContextMatch,ContextSubstitution,parse_context_substitution};
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ContextPositioning { rules:ContextSubstitution }
impl ContextPositioning {
    pub fn len(&self)->usize{self.rules.len()}
    pub fn is_empty(&self)->bool{self.rules.is_empty()}
    pub fn rule_sets(&self)->&[Option<Vec<ContextRule>>]{self.rules.rule_sets()}
    pub fn rules(&self,glyph:u16)->Option<&[ContextRule]>{self.rules.rules(glyph)}
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{self.rules.match_prefix(glyphs)}
    /// Match at an explicit glyph position, without modifying the input.
    pub fn match_at(&self,glyphs:&[u16],start:usize)->Option<ContextMatch<'_>>{
        self.match_prefix(glyphs.get(start..)?)
    }
}
/// lookup_count is the number of lookups in the owning GPOS LookupList,
/// not the GSUB lookup count. The caller must identify lookup type 7.
pub fn parse_context_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ContextPositioning,Error>{
    Ok(ContextPositioning{rules:parse_context_substitution(b,glyph_count,lookup_count,l)?})
}

use crate::{ClassContextSubstitution,parse_class_context_substitution};
/// Type 7 format 2: covered initial glyph, then class-based sequence rules.
/// Class zero includes in-domain glyphs not assigned an explicit class.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassContextPositioning { rules:ClassContextSubstitution }
impl ClassContextPositioning {
    pub fn rule_sets(&self)->&[Option<Vec<ContextRule>>]{self.rules.rule_sets()}
    pub fn rules(&self,glyph:u16)->Option<&[ContextRule]>{self.rules.rules(glyph)}
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{self.rules.match_prefix(glyphs)}
    pub fn match_at(&self,glyphs:&[u16],start:usize)->Option<ContextMatch<'_>>{
        self.match_prefix(glyphs.get(start..)?)
    }
}
/// Parse a caller-identified GPOS type 7 format 2 payload. Lookup references
/// are checked against the owning GPOS LookupList; execution is separate.
pub fn parse_class_context_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ClassContextPositioning,Error>{
    Ok(ClassContextPositioning{rules:parse_class_context_substitution(b,glyph_count,lookup_count,l)?})
}

use crate::{ContextAction,GlyphCoverage,CoverageContextSubstitution,parse_coverage_context_substitution};
/// Type 7 format 3: one independent glyph coverage per input position.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct CoverageContextPositioning { rules:CoverageContextSubstitution }
impl CoverageContextPositioning {
    pub fn input_len(&self)->usize{self.rules.input_len()}
    pub fn coverages(&self)->&[GlyphCoverage]{self.rules.coverages()}
    pub fn actions(&self)->&[ContextAction]{self.rules.actions()}
    pub fn match_prefix(&self,glyphs:&[u16])->Option<ContextMatch<'_>>{self.rules.match_prefix(glyphs)}
    pub fn match_at(&self,glyphs:&[u16],start:usize)->Option<ContextMatch<'_>>{
        self.match_prefix(glyphs.get(start..)?)
    }
}
/// Parse caller-identified GPOS type 7 format 3. Shared coverage offsets
/// consume cumulative budgets per reference; lookup indices refer to GPOS.
pub fn parse_coverage_context_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<CoverageContextPositioning,Error>{
    Ok(CoverageContextPositioning{rules:parse_coverage_context_substitution(b,glyph_count,lookup_count,l)?})
}
