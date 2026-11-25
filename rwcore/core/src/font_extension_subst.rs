//! GSUB Type 7 Format 1: checked extension target resolution and parsing.
//! Does not execute lookups or interpret lookup flags.
use crate::*;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ExtensionSubstitution<'a>{
    kind:u16,
    offset:usize,
    target:&'a [u8],
}
impl<'a> ExtensionSubstitution<'a>{
    pub fn kind(&self)->u16{self.kind}
    pub fn offset(&self)->usize{self.offset}
    pub fn target(&self)->&'a [u8]{self.target}
}
/// The offset is relative to the beginning of this wrapper, not the GSUB
/// table. The returned target is a borrowed suffix, not a certified extent.
/// Type 8 is a valid target type but has no target parser in this batch.
pub fn parse_extension_substitution(b:&[u8],max_bytes:usize)->Result<ExtensionSubstitution<'_>,Error>{
    if b.len()>max_bytes{return Err(Error::Limit("extension bytes"));}
    let h=b.get(..8).ok_or(Error::Invalid("extension header"))?;
    let format=u16::from_be_bytes([h[0],h[1]]);
    if format!=1{return Err(Error::Unsupported("extension format"));}
    let kind=u16::from_be_bytes([h[2],h[3]]);
    if !matches!(kind,1..=6|8){return Err(Error::Invalid("extension target kind"));}
    let offset=usize::try_from(u32::from_be_bytes([h[4],h[5],h[6],h[7]])).map_err(|_|Error::Limit("extension offset"))?;
    if offset<8{return Err(Error::Invalid("extension offset into header"));}
    let target=b.get(offset..).filter(|s|s.len()>=2).ok_or(Error::Invalid("extension target truncated"))?;
    Ok(ExtensionSubstitution{kind,offset,target})
}
/// Resolve all wrappers of one parsed Type 7 lookup, preserving order.
/// Every wrapper must declare the same target lookup type. Shared targets
/// are allowed; resolution borrows slices and does not decode them here.
/// max_bytes is the parent GSUB byte bound, max_subtables the wrapper count.
pub fn resolve_extension_lookup<'a>(gsub:&'a [u8],lookup:&GsubLookup,max_bytes:usize,max_subtables:usize)->Result<Vec<ExtensionSubstitution<'a>>,Error>{
    if lookup.kind!=7{return Err(Error::Invalid("not extension lookup"));}
    if gsub.len()>max_bytes||lookup.subtables.len()>max_subtables{return Err(Error::Limit("extension lookup budget"));}
    if lookup.subtables.is_empty(){return Err(Error::Invalid("extension empty lookup"));}
    let mut out=Vec::with_capacity(lookup.subtables.len());
    let mut kind=None;
    for &at in &lookup.subtables{
        let b=gsub.get(at..).ok_or(Error::Invalid("extension wrapper offset"))?;
        let ext=parse_extension_substitution(b,max_bytes)?;
        if kind.is_some_and(|k|k!=ext.kind()){return Err(Error::Invalid("extension mixed target kinds"));}
        kind=Some(ext.kind());
        out.push(ext);
    }
    Ok(out)
}
#[derive(Debug,Clone,Copy)]
pub struct ExtensionLimits{
    pub max_bytes:usize,
    pub single:SingleSubstLimits,
    pub multiple:MultipleSubstLimits,
    pub alternate:AlternateSubstLimits,
    pub ligature:LigatureLimits,
    pub context:ContextLimits,
}
impl Default for ExtensionLimits{
    fn default()->Self{Self{max_bytes:32*1024*1024,single:Default::default(),multiple:Default::default(),alternate:Default::default(),ligature:Default::default(),context:Default::default()}}
}
/// Owned, parsed direct target. Context variants only match and return
/// actions, as in their standalone APIs.
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum ExtensionTarget{
    Single(SingleSubstitution),
    Multiple(MultipleSubstitution),
    Alternate(AlternateSubstitution),
    Ligature(LigatureSubstitution),
    Context(ContextSubstitution),
    ClassContext(ClassContextSubstitution),
    CoverageContext(CoverageContextSubstitution),
    Chain(ChainContextSubstitution),
    ClassChain(ClassChainContextSubstitution),
    CoverageChain(CoverageChainContextSubstitution),
}
impl ExtensionTarget{
    pub fn kind(&self)->u16{
        match self{
            Self::Single(_)=>1,Self::Multiple(_)=>2,Self::Alternate(_)=>3,Self::Ligature(_)=>4,
            Self::Context(_)|Self::ClassContext(_)|Self::CoverageContext(_)=>5,
            Self::Chain(_)|Self::ClassChain(_)|Self::CoverageChain(_)=>6,
        }
    }
}
/// Decode one resolved target using the existing autonomous parser limits.
/// No recursive Type 7 dispatch. Lookup indices are checked against the
/// caller-supplied owning lookup count. No cumulative multi-target budget.
pub fn parse_extension_target(ext:ExtensionSubstitution<'_>,glyph_count:u32,lookup_count:usize,l:ExtensionLimits)->Result<ExtensionTarget,Error>{
    let b=ext.target();
    if b.len()>l.max_bytes{return Err(Error::Limit("extension target bytes"));}
    let format=u16::from_be_bytes([b[0],b[1]]);
    use ExtensionTarget::*;
    Ok(match ext.kind(){
        1=>Single(parse_single_substitution(b,glyph_count,l.single)?),
        2=>Multiple(parse_multiple_substitution(b,glyph_count,l.multiple)?),
        3=>Alternate(parse_alternate_substitution(b,glyph_count,l.alternate)?),
        4=>Ligature(parse_ligature_substitution(b,glyph_count,l.ligature)?),
        5=>match format{
            1=>Context(parse_context_substitution(b,glyph_count,lookup_count,l.context)?),
            2=>ClassContext(parse_class_context_substitution(b,glyph_count,lookup_count,l.context)?),
            3=>CoverageContext(parse_coverage_context_substitution(b,glyph_count,lookup_count,l.context)?),
            _=>return Err(Error::Unsupported("extension context format")),
        },
        6=>match format{
            1=>Chain(parse_chain_context_substitution(b,glyph_count,lookup_count,l.context)?),
            2=>ClassChain(parse_class_chain_context_substitution(b,glyph_count,lookup_count,l.context)?),
            3=>CoverageChain(parse_coverage_chain_context_substitution(b,glyph_count,lookup_count,l.context)?),
            _=>return Err(Error::Unsupported("extension chain format")),
        },
        _=>return Err(Error::Unsupported("extension target parser")),
    })
}
pub fn parse_extension_target_substitution(b:&[u8],glyph_count:u32,lookup_count:usize,l:ExtensionLimits)->Result<ExtensionTarget,Error>{
    parse_extension_target(parse_extension_substitution(b,l.max_bytes)?,glyph_count,lookup_count,l)
}
