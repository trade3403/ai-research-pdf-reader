//! GPOS 1.0/1.1 directory and child headers. OpenType Layout common
//! structures share the GSUB decoder; GPOS lookup kinds are 1 through 9.
//! Positioning subtables, variation bodies and GDEF references remain opaque.
use crate::Error;
pub use crate::font_gsub::{GsubRecord as GposRecord,GsubDirectory as GposDirectory,
    GsubLimits as GposLimits,GsubLangSys as GposLangSys,GsubScript as GposScript,
    GsubFeature as GposFeature,GsubLookup as GposLookup,GsubLayout as GposLayout};
/// Offsets are absolute within the supplied GPOS table.
pub fn parse_gpos_directory(b:&[u8],limits:GposLimits)->Result<GposDirectory,Error>{
    crate::font_gsub::parse_layout_directory(b,limits)
}
/// Validate script/language/feature indices, lookup headers, flags,
/// subtable offsets and cumulative record limits. Shared decoder error
/// strings retain their historical GSUB prefix for compatibility.
pub fn parse_gpos_layout(b:&[u8],limits:GposLimits)->Result<GposLayout,Error>{
    crate::font_gsub::parse_layout_children(b,limits,9)
}
