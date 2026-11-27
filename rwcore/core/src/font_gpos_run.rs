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

#[derive(Debug,Clone,Copy)]
pub struct GposContextRunLimits{
    pub run:GposRunLimits,
    pub max_depth:usize,
    pub max_calls:usize,
    pub max_actions:usize,
    /// Conservative count of candidate rule positions inspected, before matching.
    pub max_context_work:usize,
}
impl Default for GposContextRunLimits{
    fn default()->Self{Self{run:Default::default(),max_depth:32,max_calls:1048576,max_actions:1048576,max_context_work:16777216}}
}
struct ContextRunner<'p,'g,'d>{
    program:&'p CompiledGposProgram<'d>,glyphs:&'g [u16],out:Vec<GposRunAdjustment>,mode:GposRunMode,
    limits:GposContextRunLimits,visits:usize,attempts:usize,calls:usize,actions:usize,work:usize,
    active:Vec<(u16,usize)>,
}
impl<'p,'g,'d> ContextRunner<'p,'g,'d>{
    fn filtered(&mut self,lookup:&CompiledGposLookup<'_>)->Result<(Vec<usize>,Vec<u16>),Error>{
        charge(&mut self.visits,self.glyphs.len())?;
        let ids=lookup.filter().eligible_indices(self.glyphs,LookupFilterLimits{max_run_glyphs:self.limits.run.max_glyphs,max_eligible:self.limits.run.max_glyphs})?;
        let mut stream=Vec::new();stream.try_reserve_exact(ids.len()).map_err(|_|Error::Limit("gpos context allocation"))?;
        stream.extend(ids.iter().map(|&i|self.glyphs[i]));Ok((ids,stream))
    }
    fn nested(&mut self,id:u16,original:usize)->Result<(),Error>{
        let program=self.program;
        let lookup=program.lookup(id).ok_or(Error::Invalid("gpos nested lookup"))?;
        let (ids,stream)=self.filtered(lookup)?;
        if let Ok(pos)=ids.binary_search(&original){self.at(lookup,&ids,&stream,pos)?;}
        Ok(())
    }
    fn at(&mut self,lookup:&CompiledGposLookup<'_>,ids:&[usize],stream:&[u16],pos:usize)->Result<usize,Error>{
        charge(&mut self.calls,1)?;
        // Hard stack cap even when caller supplies an excessive max_depth.
        if self.active.len()>=self.limits.max_depth.min(64){return Err(Error::Limit("gpos context depth"));}
        let key=(lookup.index(),ids[pos]);
        if self.active.contains(&key){return Err(Error::Invalid("gpos context cycle"));}
        self.active.try_reserve(1).map_err(|_|Error::Limit("gpos context allocation"))?;
        self.active.push(key);
        let result=self.apply_at(lookup,ids,stream,pos);
        self.active.pop();result
    }
    fn apply_at(&mut self,lookup:&CompiledGposLookup<'_>,ids:&[usize],stream:&[u16],pos:usize)->Result<usize,Error>{
        let a=ids[pos];
        for sub in lookup.subtables(){
            charge(&mut self.attempts,1)?;
            use GposExtensionTarget::*;
            let matched=match sub.target(){
                Single(p)=>{
                    let got=match self.mode{
                        GposRunMode::DesignOnly=>p.apply_design(stream[pos],self.out[a].design)?.map(|design|GposRunAdjustment{design,pixels:self.out[a].pixels}),
                        GposRunMode::DevicePixels{x_ppem,y_ppem}=>p.apply(stream[pos],self.out[a].design,self.out[a].pixels,x_ppem,y_ppem)?.map(|(design,pixels)|GposRunAdjustment{design,pixels}),
                    };
                    if let Some(v)=got{self.out[a]=v;return Ok(1);}continue;
                }
                Pair(_)|ClassPair(_)=>{
                    let Some(&b)=ids.get(pos+1)else{return Ok(1);};
                    let d=[self.out[a].design,self.out[b].design];let px=[self.out[a].pixels,self.out[b].pixels];
                    macro_rules! pair{($p:expr)=>{match self.mode{
                        GposRunMode::DesignOnly=>$p.apply_design(self.glyphs[a],self.glyphs[b],d)?.map(|d|(d,px)),
                        GposRunMode::DevicePixels{x_ppem,y_ppem}=>$p.apply(self.glyphs[a],self.glyphs[b],d,px,x_ppem,y_ppem)?,
                    }}}
                    let (got,second)=match sub.target(){Pair(p)=>(pair!(p),p.second_format()),ClassPair(p)=>(pair!(p),p.second_format()),_=>unreachable!()};
                    if let Some((d,px))=got{
                        self.out[a]=GposRunAdjustment{design:d[0],pixels:px[0]};self.out[b]=GposRunAdjustment{design:d[1],pixels:px[1]};
                        return Ok(if second==0{1}else{2});
                    }continue;
                }
                Context(p)=>{if let Some(rs)=p.rules(stream[pos]){for r in rs{charge(&mut self.work,1+r.remaining.len())?;}}p.match_at(stream,pos)}
                ClassContext(p)=>{if let Some(rs)=p.rules(stream[pos]){for r in rs{charge(&mut self.work,1+r.remaining.len())?;}}p.match_at(stream,pos)}
                CoverageContext(p)=>{charge(&mut self.work,p.input_len())?;p.match_at(stream,pos)}
                Chain(p)=>{if let Some(rs)=p.rules(stream[pos]){for r in rs{charge(&mut self.work,1+r.remaining.len()+r.backtrack.len()+r.lookahead.len())?;}}p.match_at(stream,pos)}
                ClassChain(p)=>{if let Some(rs)=p.rules(stream[pos]){for r in rs{charge(&mut self.work,1+r.remaining.len()+r.backtrack.len()+r.lookahead.len())?;}}p.match_at(stream,pos)}
                CoverageChain(p)=>{charge(&mut self.work,p.input_len()+p.backtrack().len()+p.lookahead().len())?;p.match_at(stream,pos)}
                _=>return Err(Error::Unsupported("gpos context run target")),
            };
            if let Some(m)=matched{
                charge(&mut self.actions,m.actions.len())?;
                for action in m.actions{
                    let offset=usize::from(action.sequence_index);
                    if offset>=m.input_len{return Err(Error::Invalid("gpos context sequence index"));}
                    let original=*ids.get(pos+offset).ok_or(Error::Invalid("gpos context input"))?;
                    self.nested(action.lookup_index,original)?;
                }
                return Ok(m.input_len);
            }
        }
        Ok(1)
    }
}
/// Execute types 1, 2, 7 and 8 including extension wrappers. Context actions
/// use parent-filtered input indices; child lookups apply their own filters
/// at the same original glyph. A skipped child target is a no-op, not retargeted.
/// The immutable glyph stream makes action indices stable. Cycles are rejected
/// only when the same lookup/original-index pair is active. All failures atomic.
pub fn execute_gpos_context(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],mode:GposRunMode,l:GposContextRunLimits)->Result<Vec<GposRunAdjustment>,Error>{
    if glyphs.len()!=prior.len(){return Err(Error::Invalid("gpos run length mismatch"));}
    if glyphs.len()>l.run.max_glyphs||program.roots().len()>l.run.max_roots{return Err(Error::Limit("gpos run size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){return Err(Error::Invalid("gpos run glyph domain"));}
    for lookup in program.lookups(){if !matches!(lookup.effective_kind(),1|2|7|8){return Err(Error::Unsupported("gpos context run kind"));}}
    let mut out=Vec::new();out.try_reserve_exact(prior.len()).map_err(|_|Error::Limit("gpos context allocation"))?;out.extend_from_slice(prior);
    let mut runner=ContextRunner{program,glyphs,out,mode,limits:l,visits:l.run.max_filter_visits,attempts:l.run.max_subtable_attempts,calls:l.max_calls,actions:l.max_actions,work:l.max_context_work,active:Vec::new()};
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("gpos context root"))?;
        let (ids,stream)=runner.filtered(lookup)?;
        let mut pos=0;while pos<ids.len(){pos+=runner.at(lookup,&ids,&stream,pos)?;}
    }
    Ok(runner.out)
}
