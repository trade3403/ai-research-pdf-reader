//! GPOS Type 9 Format 1 checked extension resolution and direct dispatch.
//! No recursive wrappers or execution; target slices are borrowed suffixes.
use crate::*;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ExtensionPositioning<'a>{kind:u16,offset:usize,target:&'a [u8]}
impl<'a> ExtensionPositioning<'a>{
    pub fn kind(&self)->u16{self.kind}
    pub fn offset(&self)->usize{self.offset}
    pub fn target(&self)->&'a [u8]{self.target}
}
pub fn parse_extension_positioning(b:&[u8],max_bytes:usize)->Result<ExtensionPositioning<'_>,Error>{
    if b.len()>max_bytes{return Err(Error::Limit("gpos extension bytes"));}
    let h=b.get(..8).ok_or(Error::Invalid("gpos extension header"))?;
    if u16::from_be_bytes([h[0],h[1]])!=1{return Err(Error::Unsupported("gpos extension format"));}
    let kind=u16::from_be_bytes([h[2],h[3]]);
    if !(1..=8).contains(&kind){return Err(Error::Invalid("gpos extension kind"));}
    let offset=usize::try_from(u32::from_be_bytes([h[4],h[5],h[6],h[7]])).map_err(|_|Error::Limit("gpos extension offset"))?;
    if offset<8{return Err(Error::Invalid("gpos extension header overlap"));}
    let target=b.get(offset..).filter(|s|s.len()>=2).ok_or(Error::Invalid("gpos extension target"))?;
    Ok(ExtensionPositioning{kind,offset,target})
}
/// Preserve wrapper order and require homogeneous effective lookup type.
pub fn resolve_gpos_extension_lookup<'a>(b:&'a [u8],lookup:&GposLookup,max_bytes:usize,max_subtables:usize)->Result<Vec<ExtensionPositioning<'a>>,Error>{
    if lookup.kind!=9||lookup.subtables.is_empty(){return Err(Error::Invalid("gpos extension lookup"));}
    if b.len()>max_bytes||lookup.subtables.len()>max_subtables{return Err(Error::Limit("gpos extension lookup budget"));}
    let mut out=Vec::new();out.try_reserve_exact(lookup.subtables.len()).map_err(|_|Error::Limit("gpos extension allocation"))?;
    let mut kind=None;
    for &at in &lookup.subtables{
        let ext=parse_extension_positioning(b.get(at..).ok_or(Error::Invalid("gpos extension wrapper"))?,max_bytes)?;
        if kind.is_some_and(|k|k!=ext.kind()){return Err(Error::Invalid("gpos extension mixed kinds"));}
        kind=Some(ext.kind());out.push(ext);
    }Ok(out)
}
#[derive(Debug,Clone,Copy)]
pub struct GposExtensionLimits{
    pub max_bytes:usize,pub single:SinglePosLimits,pub pair:PairPosLimits,pub class_pair:ClassPairPosLimits,
    pub cursive:CursivePosLimits,pub mark:MarkPosLimits,pub ligature:MarkLigatureLimits,pub mark_mark:MarkToMarkLimits,pub context:ContextLimits,
}
impl Default for GposExtensionLimits{
    fn default()->Self{Self{max_bytes:32*1024*1024,single:Default::default(),pair:Default::default(),class_pair:Default::default(),cursive:Default::default(),mark:Default::default(),ligature:Default::default(),mark_mark:Default::default(),context:Default::default()}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum GposExtensionTarget{
    Single(SinglePositioning),Pair(PairPositioning),ClassPair(ClassPairPositioning),Cursive(CursivePositioning),
    MarkBase(MarkToBasePositioning),MarkLigature(MarkToLigaturePositioning),MarkMark(MarkToMarkPositioning),
    Context(ContextPositioning),ClassContext(ClassContextPositioning),CoverageContext(CoverageContextPositioning),
    Chain(ChainContextPositioning),ClassChain(ClassChainContextPositioning),CoverageChain(CoverageChainContextPositioning),
}
impl GposExtensionTarget{
    pub fn kind(&self)->u16{use GposExtensionTarget::*;match self{
        Single(_)=>1,Pair(_)|ClassPair(_)=>2,Cursive(_)=>3,MarkBase(_)=>4,MarkLigature(_)=>5,MarkMark(_)=>6,
        Context(_)|ClassContext(_)|CoverageContext(_)=>7,Chain(_)|ClassChain(_)|CoverageChain(_)=>8,
    }}
}
/// Direct parser for types 1..8; byte limit applies to the supplied suffix,
/// not a certified subtable extent. Child-specific budgets remain in force.
pub fn parse_direct_gpos_target(kind:u16,b:&[u8],glyph_count:u32,lookup_count:usize,l:GposExtensionLimits)->Result<GposExtensionTarget,Error>{
    if b.len()>l.max_bytes{return Err(Error::Limit("gpos target bytes"));}
    let h=b.get(..2).ok_or(Error::Invalid("gpos target header"))?;
    let format=u16::from_be_bytes([h[0],h[1]]);
    use GposExtensionTarget::*;
    Ok(match kind{
        1=>Single(parse_single_positioning(b,glyph_count,l.single)?),
        2=>match format{1=>Pair(parse_pair_positioning_format1(b,glyph_count,l.pair)?),2=>ClassPair(parse_pair_positioning_format2(b,glyph_count,l.class_pair)?),_=>return Err(Error::Unsupported("gpos pair format"))},
        3=>Cursive(parse_cursive_positioning(b,glyph_count,l.cursive)?),
        4=>MarkBase(parse_mark_to_base(b,glyph_count,l.mark)?),
        5=>MarkLigature(parse_mark_to_ligature(b,glyph_count,l.ligature)?),
        6=>MarkMark(parse_mark_to_mark(b,glyph_count,l.mark_mark)?),
        7=>match format{1=>Context(parse_context_positioning(b,glyph_count,lookup_count,l.context)?),2=>ClassContext(parse_class_context_positioning(b,glyph_count,lookup_count,l.context)?),3=>CoverageContext(parse_coverage_context_positioning(b,glyph_count,lookup_count,l.context)?),_=>return Err(Error::Unsupported("gpos context format"))},
        8=>match format{1=>Chain(parse_chain_context_positioning(b,glyph_count,lookup_count,l.context)?),2=>ClassChain(parse_class_chain_context_positioning(b,glyph_count,lookup_count,l.context)?),3=>CoverageChain(parse_coverage_chain_context_positioning(b,glyph_count,lookup_count,l.context)?),_=>return Err(Error::Unsupported("gpos chain format"))},
        _=>return Err(Error::Unsupported("gpos target kind")),
    })
}
pub fn parse_gpos_extension_target(ext:ExtensionPositioning<'_>,glyph_count:u32,lookup_count:usize,l:GposExtensionLimits)->Result<GposExtensionTarget,Error>{
    parse_direct_gpos_target(ext.kind(),ext.target(),glyph_count,lookup_count,l)
}
pub fn parse_extension_target_positioning(b:&[u8],glyph_count:u32,lookup_count:usize,l:GposExtensionLimits)->Result<GposExtensionTarget,Error>{
    parse_gpos_extension_target(parse_extension_positioning(b,l.max_bytes)?,glyph_count,lookup_count,l)
}
