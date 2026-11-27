//! GPOS static dependency closure; shared iterative OpenType graph algorithm.
//! Cycles are diagnostic, not runtime execution permission.
use crate::{Error,GposDependency};
pub use crate::{GsubClosureLimits as GposClosureLimits,GsubDependencyClosure as GposDependencyClosure};
pub fn analyze_gpos_dependency_closure(lookup_count:usize,sources:&[u16],edges:&[GposDependency],roots:&[u16],l:GposClosureLimits)->Result<GposDependencyClosure,Error>{
    crate::font_gsub_closure::analyze_layout_dependency_closure(lookup_count,sources,edges.len(),edges.iter().map(|e|(e.site.lookup_index,e.action.lookup_index)),roots,l)
}
