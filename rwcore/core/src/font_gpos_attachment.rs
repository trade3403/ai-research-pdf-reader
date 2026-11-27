//! Explicit attachment graph for positioned glyph occurrences.
//! Parent/child anchor deltas are independent of pen origins; final placement
//! is resolved after advances are known. No automatic GPOS parent search.
use crate::{Error,GposAnchor,GposRunAdjustment,GposRunMode};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GposAttachmentEdge{
    pub parent:usize,
    /// Parent anchor minus child anchor, in design units.
    pub design_delta:(i32,i32),
    /// Separate device correction, never scaled design coordinates.
    pub pixel_delta:(i32,i32),
}
#[derive(Debug,Clone,Copy)]
pub struct GposAttachmentLimits{pub max_glyphs:usize,pub max_edges:usize,pub max_depth:usize}
impl Default for GposAttachmentLimits{
    fn default()->Self{Self{max_glyphs:262144,max_edges:262144,max_depth:262144}}
}
impl GposAttachmentEdge{
    pub fn from_anchors(parent:usize,a:&GposAnchor,b:&GposAnchor,mode:GposRunMode)->Result<Self,Error>{
        let design_delta=(i32::from(a.x)-i32::from(b.x),i32::from(a.y)-i32::from(b.y));
        let pixel_delta=match mode{
            GposRunMode::DesignOnly=>(0,0),
            GposRunMode::DevicePixels{x_ppem,y_ppem}=>{
                let a=a.pixel_offset(x_ppem,y_ppem)?;let b=b.pixel_offset(x_ppem,y_ppem)?;
                (a.0.checked_sub(b.0).ok_or(Error::Limit("gpos attachment delta"))?,
                 a.1.checked_sub(b.1).ok_or(Error::Limit("gpos attachment delta"))?)
            }
        };Ok(Self{parent,design_delta,pixel_delta})
    }
}
fn filled<T:Clone>(n:usize,value:T)->Result<Vec<T>,Error>{
    let mut v=Vec::new();v.try_reserve_exact(n).map_err(|_|Error::Limit("gpos attachment allocation"))?;v.resize(n,value);Ok(v)
}
fn axis(parent_origin:i32,parent_place:i32,delta:i32,child_origin:i32,child_place:i32)->Result<i32,Error>{
    i32::try_from(i64::from(parent_origin)+i64::from(parent_place)+i64::from(delta)-i64::from(child_origin)+i64::from(child_place))
        .map_err(|_|Error::Limit("gpos attachment placement"))
}
/// Resolve a forest of explicit attachment edges. Each child slot has at most
/// one parent. Origins are final pen positions (including advance adjustments)
/// but exclude placement. For attached children, prior placement is a local
/// residual relative to their attachment, not a pre-existing world position.
/// Unattached roots retain prior placement. Advances never change here.
/// The caller decides replacement policy and converts placement at attachment
/// time into residuals. No partial output on invalid graph or overflow.
pub fn resolve_gpos_attachments(edges:&[Option<GposAttachmentEdge>],design_origins:&[(i32,i32)],pixel_origins:&[(i32,i32)],prior:&[GposRunAdjustment],l:GposAttachmentLimits)->Result<Vec<GposRunAdjustment>,Error>{
    let n=prior.len();
    if edges.len()!=n||design_origins.len()!=n||pixel_origins.len()!=n{return Err(Error::Invalid("gpos attachment length"));}
    if n>l.max_glyphs{return Err(Error::Limit("gpos attachment glyphs"));}
    let mut count=0usize;
    for (i,e) in edges.iter().enumerate(){if let Some(e)=e{
        if e.parent>=n||e.parent==i{return Err(Error::Invalid("gpos attachment parent"));}
        count+=1;if count>l.max_edges{return Err(Error::Limit("gpos attachment edges"));}
    }}
    let mut out=Vec::new();out.try_reserve_exact(n).map_err(|_|Error::Limit("gpos attachment allocation"))?;out.extend_from_slice(prior);
    let mut color=filled(n,0u8)?;
    let mut depth=filled(n,0usize)?;
    let mut stack=Vec::new();stack.try_reserve_exact(n.min(l.max_depth)).map_err(|_|Error::Limit("gpos attachment allocation"))?;
    for start in 0..n{
        if color[start]==2{continue;}
        let mut i=start;
        loop{
            if color[i]==2{break;}
            if color[i]==1{return Err(Error::Invalid("gpos attachment cycle"));}
            if stack.len()>=l.max_depth{return Err(Error::Limit("gpos attachment depth"));}
            color[i]=1;stack.push(i);
            if let Some(e)=edges[i]{i=e.parent;}else{break;}
        }
        while let Some(i)=stack.pop(){
            if let Some(e)=edges[i]{
                let p=e.parent;
                depth[i]=depth[p].checked_add(1).ok_or(Error::Limit("gpos attachment depth"))?;
                if depth[i]>=l.max_depth{return Err(Error::Limit("gpos attachment depth"));}
                let a=design_origins[p];let b=design_origins[i];let d=out[p].design;let local=prior[i].design;
                out[i].design.x_placement=axis(a.0,d.x_placement,e.design_delta.0,b.0,local.x_placement)?;
                out[i].design.y_placement=axis(a.1,d.y_placement,e.design_delta.1,b.1,local.y_placement)?;
                let a=pixel_origins[p];let b=pixel_origins[i];let d=out[p].pixels;let local=prior[i].pixels;
                out[i].pixels.x_placement=axis(a.0,d.x_placement,e.pixel_delta.0,b.0,local.x_placement)?;
                out[i].pixels.y_placement=axis(a.1,d.y_placement,e.pixel_delta.1,b.1,local.y_placement)?;
            }
            color[i]=2;
        }
    }
    Ok(out)
}
