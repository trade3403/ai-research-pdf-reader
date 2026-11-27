//! Horizontal cursive-only execution with explicit signed nominal advances.
//! Logical order is caller supplied; RTL lookup flag controls attachment root,
//! not bidi reordering. This is a constraint solver, not full script shaping.
use crate::*;
#[derive(Debug,Clone,Copy)]
pub struct GposCursiveRunLimits{
    pub run:GposRunLimits,
    pub attachments:GposAttachmentLimits,
    pub max_connections:usize,
    pub max_span_visits:usize,
}
impl Default for GposCursiveRunLimits{
    fn default()->Self{Self{run:Default::default(),attachments:Default::default(),max_connections:1048576,max_span_visits:16777216}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposCursiveRun{
    pub adjustments:Vec<GposRunAdjustment>,
    pub edges:Vec<Option<GposAttachmentEdge>>,
    pub design_origins:Vec<(i32,i32)>,
    pub pixel_origins:Vec<(i32,i32)>,
}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{*left=left.checked_sub(n).ok_or(Error::Limit("cursive run work"))?;Ok(())}
fn narrow(v:i64)->Result<i32,Error>{i32::try_from(v).map_err(|_|Error::Limit("cursive run coordinate"))}
pub(crate) fn origins(nominal:&[(i32,i32)],out:&[GposRunAdjustment],pixel:bool)->Result<Vec<(i32,i32)>,Error>{
    let mut v=Vec::new();v.try_reserve_exact(out.len()).map_err(|_|Error::Limit("cursive run allocation"))?;
    let(mut x,mut y)=(0i64,0i64);
    for (n,a) in nominal.iter().zip(out){
        v.push((narrow(x)?,narrow(y)?));
        let a=if pixel{a.pixels}else{a.design};
        x+=i64::from(n.0)+i64::from(a.x_advance);y+=i64::from(n.1)+i64::from(a.y_advance);
    }
    Ok(v)
}
pub(crate) fn advance(nominal:&[(i32,i32)],out:&[GposRunAdjustment],a:usize,b:usize,delta:i32,pixel:bool)->Result<i32,Error>{
    // Make the pen interval equal to exit-anchor minus entry-anchor plus
    // the difference of placements. Preserve all intervening glyph advances.
    let mut rest=0i64;
    for i in a+1..b{let q=if pixel{out[i].pixels}else{out[i].design};rest+=i64::from(nominal[i].0)+i64::from(q.x_advance);}
    let p=if pixel{out[a].pixels}else{out[a].design};
    let q=if pixel{out[b].pixels}else{out[b].design};
    narrow(i64::from(delta)+i64::from(p.x_placement)-i64::from(q.x_placement)-rest-i64::from(nominal[a].0))
}
/// Solve horizontal connections in adjacent eligible logical occurrences.
/// Only effective Type 3 roots are accepted (including Type 9 wrappers).
/// Signed nominal advances encode the caller's coordinate convention.
/// Replacing a conflicting parent is deliberately rejected; identical edges
/// are allowed. Final attachment propagation is atomic. Mixed GPOS execution
/// and full OpenType RTL advance redistribution are separate integration work.
pub fn execute_gpos_cursive_horizontal(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],design_advances:&[(i32,i32)],pixel_advances:&[(i32,i32)],mode:GposRunMode,l:GposCursiveRunLimits)->Result<GposCursiveRun,Error>{
    let n=glyphs.len();
    if prior.len()!=n||design_advances.len()!=n||pixel_advances.len()!=n{return Err(Error::Invalid("cursive run length"));}
    if n>l.run.max_glyphs||n>l.attachments.max_glyphs||program.roots().len()>l.run.max_roots{return Err(Error::Limit("cursive run size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){return Err(Error::Invalid("cursive run glyph"));}
    for lookup in program.lookups(){if lookup.effective_kind()!=3{return Err(Error::Unsupported("cursive run kind"));}}
    let mut out=Vec::new();out.try_reserve_exact(n).map_err(|_|Error::Limit("cursive run allocation"))?;out.extend_from_slice(prior);
    let mut edges=Vec::new();edges.try_reserve_exact(n).map_err(|_|Error::Limit("cursive run allocation"))?;edges.resize(n,None);
    let(mut visits,mut attempts,mut connections,mut spans)=(l.run.max_filter_visits,l.run.max_subtable_attempts,l.max_connections,l.max_span_visits);
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("cursive run root"))?;
        charge(&mut visits,n)?;
        let ids=lookup.filter().eligible_indices(glyphs,LookupFilterLimits{max_run_glyphs:l.run.max_glyphs,max_eligible:l.run.max_glyphs})?;
        for pair in ids.windows(2){
            let(a,b)=(pair[0],pair[1]);
            for sub in lookup.subtables(){
                charge(&mut attempts,1)?;
                let GposExtensionTarget::Cursive(p)=sub.target()else{return Err(Error::Invalid("cursive run target"));};
                let Some((exit,entry))=p.pair(glyphs[a],glyphs[b])else{continue;};
                charge(&mut connections,1)?;charge(&mut spans,b-a)?;
                let forward=GposAttachmentEdge::from_anchors(a,exit,entry,mode)?;
                let d=advance(design_advances,&out,a,b,forward.design_delta.0,false)?;
                let px=if matches!(mode,GposRunMode::DevicePixels{..}){Some(advance(pixel_advances,&out,a,b,forward.pixel_delta.0,true)?)}else{None};
                let(child,edge)=if lookup.filter().right_to_left(){(a,GposAttachmentEdge::from_anchors(b,entry,exit,mode)?)}else{(b,forward)};
                if edges[child].is_some_and(|e|e!=edge){return Err(Error::Unsupported("cursive conflicting parent"));}
                edges[child]=Some(edge);
                out[a].design.x_advance=d;
                if let Some(px)=px{out[a].pixels.x_advance=px;}
                break;
            }
        }
    }
    let design_origins=origins(design_advances,&out,false)?;
    let pixel_origins=origins(pixel_advances,&out,true)?;
    // Horizontal placements already satisfy the connection equation. Resolve
    // both axes with zero local residual on the moving side; original child
    // placement is represented in the solved advance interval.
    for (i,e) in edges.iter().enumerate(){if e.is_some(){
        out[i].design.x_placement=0;out[i].design.y_placement=0;
        if matches!(mode,GposRunMode::DevicePixels{..}){out[i].pixels.x_placement=0;out[i].pixels.y_placement=0;}
    }}
    let mut adjusted=resolve_gpos_attachments(&edges,&design_origins,&pixel_origins,&out,l.attachments)?;
    if matches!(mode,GposRunMode::DesignOnly){for (a,p) in adjusted.iter_mut().zip(prior){a.pixels=p.pixels;}}
    Ok(GposCursiveRun{adjustments:adjusted,edges,design_origins,pixel_origins})
}
