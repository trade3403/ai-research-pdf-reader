//! GPOS type 6, MarkToMark format 1. Mark1 moves relative to fixed Mark2.
//! The caller selects both glyph occurrences; no automatic search or attachment chain.
//! The binary matrix layout is identical to MarkToBase format 1, so parsing
//! and checked attachment math deliberately share that implementation.
use crate::{Error,GposAnchor,GposMarkRecord,MarkPosLimits,MarkToBasePositioning};
use crate::font_gpos_mark_base::decode_mark_matrix;

/// Independent semantic names for the two coverage roles.
#[derive(Debug,Clone,Copy)]
pub struct MarkToMarkLimits {
    pub matrix: MarkPosLimits,
}
impl Default for MarkToMarkLimits {
    fn default()->Self{Self{matrix:Default::default()}}
}
/// Limits: matrix.max_marks bounds Mark1; matrix.max_bases bounds Mark2;
/// matrix.max_cells bounds Mark2Count * MarkClassCount. Device budget is
/// cumulative across both arrays, including repeated anchor references.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MarkToMarkPositioning {
    matrix: MarkToBasePositioning,
}
impl MarkToMarkPositioning {
    pub fn dimensions(&self)->(usize,usize){self.matrix.dimensions()}
    pub fn mark1(&self,glyph:u16)->Option<&GposMarkRecord>{self.matrix.mark(glyph)}
    pub fn mark2_anchor(&self,glyph:u16,class:u16)->Option<&GposAnchor>{self.matrix.base_anchor(glyph,class)}
    /// Return (fixed Mark2 anchor, moving Mark1 anchor).
    pub fn pair(&self,mark2:u16,mark1:u16)->Option<(&GposAnchor,&GposAnchor)>{
        self.matrix.pair(mark2,mark1)
    }
    /// Origins are ordered [fixed Mark2, moving Mark1], in design units.
    /// Return the relative translation, not an absolute position.
    pub fn attach_design(&self,mark2:u16,mark1:u16,origins:[(i32,i32);2])->Result<Option<(i32,i32)>,Error>{
        self.matrix.attach_design(mark2,mark1,origins)
    }
    /// Separate pixel correction. Unresolved contour/variation anchors
    /// return Unsupported. No scaling or rounding of design coordinates.
    pub fn attach_pixels(&self,mark2:u16,mark1:u16,origins:[(i32,i32);2],x_ppem:u16,y_ppem:u16)->Result<Option<(i32,i32)>,Error>{
        self.matrix.attach_pixels(mark2,mark1,origins,x_ppem,y_ppem)
    }
}
/// Parse the format 1 payload of a caller-identified GPOS type 6 subtable.
/// Type 4 and 6 have identical payload grammar, so the caller must establish
/// the lookup type externally. Coverage roles need not be disjoint.
pub fn parse_mark_to_mark(b:&[u8],glyph_count:u32,l:MarkToMarkLimits)->Result<MarkToMarkPositioning,Error>{
    Ok(MarkToMarkPositioning{matrix:decode_mark_matrix(b,glyph_count,l.matrix,"mark to mark format")?})
}
