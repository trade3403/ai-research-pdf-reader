//! Bounded GPOS single/pair root execution, preserving original glyph indices.
//! No contextual or attachment execution yet. All results are owned and atomic.
use crate::*;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GposRunMode{DesignOnly,DevicePixels{x_ppem:u16,y_ppem:u16}}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub struct GposRunAdjustment{pub design:GposAdjustment,pub pixels:GposAdjustment}
#[derive(Debug,Clone,Copy)]
pub struct GposRunLimits{pub max_glyphs:usize,pub max_roots:usize,pub max_filter_visits:usize,pub max_subtable_attempts:usize}
impl Default for GposRunLimits{fn default()->Self{Self{max_glyphs:262144,max_roots:16384,max_filter_visits:16777216,max_subtable_attempts:16777216}}}
fn charge(left:&mut usize,n:usize)->Result<(),Error>{*left=left.checked_sub(n).ok_or(Error::Limit("gpos run work"))?;Ok(())}
/// Apply only effective lookup types 1 and 2. Reject unsupported root types
/// even on empty runs. First matching subtable wins, including zero values.
/// PairPos with absent ValueRecord2 revisits the second eligible glyph;
/// otherwise both eligible glyphs are consumed. Skipped glyphs are untouched.
/// DesignOnly intentionally ignores Device and VariationIndex values.
pub fn execute_gpos_single_pair(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],mode:GposRunMode,l:GposRunLimits)->Result<Vec<GposRunAdjustment>,Error>{
    if glyphs.len()!=prior.len(){return Err(Error::Invalid("gpos run length mismatch"));}
    if glyphs.len()>l.max_glyphs||program.roots().len()>l.max_roots{return Err(Error::Limit("gpos run size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){return Err(Error::Invalid("gpos run glyph domain"));}
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("gpos run root"))?;
        if !matches!(lookup.effective_kind(),1|2){return Err(Error::Unsupported("gpos run root kind"));}
    }
    let mut out=Vec::new();out.try_reserve_exact(prior.len()).map_err(|_|Error::Limit("gpos run allocation"))?;out.extend_from_slice(prior);
    let(mut visits,mut attempts)=(l.max_filter_visits,l.max_subtable_attempts);
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("gpos run root"))?;
        charge(&mut visits,glyphs.len())?;
        let ids=lookup.filter().eligible_indices(glyphs,LookupFilterLimits{max_run_glyphs:l.max_glyphs,max_eligible:l.max_glyphs})?;
        let mut i=0;
        while i<ids.len(){
            let a=ids[i];let mut step=1;
            for sub in lookup.subtables(){
                charge(&mut attempts,1)?;
                match sub.target(){
                    GposExtensionTarget::Single(p)=>{
                        let got=match mode{
                            GposRunMode::DesignOnly=>p.apply_design(glyphs[a],out[a].design)?.map(|design|GposRunAdjustment{design,pixels:out[a].pixels}),
                            GposRunMode::DevicePixels{x_ppem,y_ppem}=>p.apply(glyphs[a],out[a].design,out[a].pixels,x_ppem,y_ppem)?.map(|(design,pixels)|GposRunAdjustment{design,pixels}),
                        };
                        if let Some(v)=got{out[a]=v;break;}
                    }
                    GposExtensionTarget::Pair(_)|GposExtensionTarget::ClassPair(_)=>{
                        let Some(&b)=ids.get(i+1)else{break;};
                        let d=[out[a].design,out[b].design];let px=[out[a].pixels,out[b].pixels];
                        macro_rules! apply_pair{($p:expr)=>{match mode{
                            GposRunMode::DesignOnly=>$p.apply_design(glyphs[a],glyphs[b],d)?.map(|d|(d,px)),
                            GposRunMode::DevicePixels{x_ppem,y_ppem}=>$p.apply(glyphs[a],glyphs[b],d,px,x_ppem,y_ppem)?,
                        }}}
                        let (got,second)=match sub.target(){
                            GposExtensionTarget::Pair(p)=>(apply_pair!(p),p.second_format()),
                            GposExtensionTarget::ClassPair(p)=>(apply_pair!(p),p.second_format()),
                            _=>return Err(Error::Invalid("gpos run pair target")),
                        };
                        if let Some((d,px))=got{
                            out[a]=GposRunAdjustment{design:d[0],pixels:px[0]};out[b]=GposRunAdjustment{design:d[1],pixels:px[1]};
                            if second!=0{step=2;}break;
                        }
                    }
                    _=>return Err(Error::Unsupported("gpos run target")),
                }
            }
            i+=step;
        }
    }
    Ok(out)
}
