//! GPOS type 8 format 1, glyph-based chained contextual positioning.
//! Binary rule grammar is shared with GSUB type 6; lookup indices refer
//! to the owning GPOS LookupList. Matching does not execute positioning.
use crate::{ChainContextRule,ChainContextSubstitution,ContextLimits,ContextMatch,Error,parse_chain_context_substitution};
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ChainContextPositioning { rules:ChainContextSubstitution }
impl ChainContextPositioning {
    pub fn len(&self)->usize{self.rules.len()}
    pub fn is_empty(&self)->bool{self.rules.is_empty()}
    pub fn rule_sets(&self)->&[Option<Vec<ChainContextRule>>]{self.rules.rule_sets()}
    pub fn rules(&self,glyph:u16)->Option<&[ChainContextRule]>{self.rules.rules(glyph)}
    /// Position is the first input glyph. Backtrack is closest-first,
    /// lookahead starts after the entire input. First matching rule wins,
    /// including empty-action matches. Caller handles GDEF filtering.
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<ContextMatch<'_>>{
        self.rules.match_at(glyphs,position)
    }
}
/// Parse a caller-identified type 8 format 1 subtable. Resource limits
/// charge all three segments cumulatively, including shared rule references.
pub fn parse_chain_context_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ChainContextPositioning,Error>{
    Ok(ChainContextPositioning{rules:parse_chain_context_substitution(b,glyph_count,lookup_count,l)?})
}

use crate::ClassChainContextSubstitution;
use crate::font_chain_context_subst::decode_class_chain_context;
/// Type 8 format 2: independent class definitions for backtrack, input
/// and lookahead. Trailing input classes need not have initial rule sets.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ClassChainContextPositioning { rules:ClassChainContextSubstitution }
impl ClassChainContextPositioning {
    pub fn len(&self)->usize{self.rules.len()}
    pub fn is_empty(&self)->bool{self.rules.is_empty()}
    pub fn rule_sets(&self)->&[Option<Vec<ChainContextRule>>]{self.rules.rule_sets()}
    pub fn rules(&self,glyph:u16)->Option<&[ChainContextRule]>{self.rules.rules(glyph)}
    pub fn match_at(&self,glyphs:&[u16],position:usize)->Option<ContextMatch<'_>>{
        self.rules.match_at(glyphs,position)
    }
}
/// Parse a caller-identified GPOS type 8 format 2 payload, including
/// trailing-only input classes; nested lookup execution is separate.
pub fn parse_class_chain_context_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:ContextLimits)->Result<ClassChainContextPositioning,Error>{
    Ok(ClassChainContextPositioning{rules:decode_class_chain_context(b,glyph_count,lookup_count,l,true)?})
}
