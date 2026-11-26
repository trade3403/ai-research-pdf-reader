//! Bounded contextual execution with optional Type 2 expansion.
//! Explicit roots; dependency-only lookups run only at action positions.
use crate::{CompiledGsubProgram,CompiledGsubLookup,ContextAction,Error,ExtensionTarget,LookupFilter,GsubAlternateSelection};

/// Hard stack ceiling even when a caller requests a larger limit.
pub const GSUB_CONTEXT_MAX_DEPTH:usize=32;
#[derive(Debug,Clone,Copy)]
pub struct GsubContextExecLimits{
    pub max_glyphs:usize,pub max_roots:usize,pub max_lookups:usize,
    pub max_depth:usize,pub max_calls:usize,pub max_probes:usize,
    pub max_filter_checks:usize,pub max_rules:usize,pub max_symbols:usize,
    pub max_actions:usize,pub max_matches:usize,
}
impl Default for GsubContextExecLimits{
    fn default()->Self{Self{max_glyphs:262144,max_roots:16384,max_lookups:16384,
        max_depth:16,max_calls:16*1024*1024,max_probes:32*1024*1024,
        max_filter_checks:32*1024*1024,max_rules:16*1024*1024,
        max_symbols:32*1024*1024,max_actions:16*1024*1024,max_matches:16*1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct GsubContextExecResult{
    pub glyphs:Vec<u16>,pub calls:usize,pub probes:usize,pub filter_checks:usize,
    pub rules:usize,pub symbols:usize,pub actions:usize,pub matches:usize,
    pub changes:usize,pub peak_depth:usize,
}
fn tick(n:&mut usize,cap:usize,label:&'static str)->Result<(),Error>{
    if *n>=cap{return Err(Error::Limit(label));}*n+=1;Ok(())
}
/// Extra limits for variable-length Type 2 execution. Edit work charges
/// replacement writes, moved suffix glyphs and frame/anchor inspections.
#[derive(Debug,Clone,Copy)]
pub struct GsubExpansionExecLimits{
    pub context:GsubContextExecLimits,
    pub max_inserted:usize,
    pub max_edit_work:usize,
}
impl Default for GsubExpansionExecLimits{
    fn default()->Self{Self{context:Default::default(),max_inserted:4*1024*1024,
        max_edit_work:32*1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubExpansionExecResult{
    pub execution:GsubContextExecResult,
    pub multiple_matches:usize,
    pub inserted_glyphs:usize,
    pub edit_work:usize,
}
#[derive(Default)]
struct ExpansionWork{matches:usize,inserted:usize,edits:usize}
struct Frame{
    id:u16,start:usize,positions:Vec<usize>,end:usize,
}
fn charge(n:&mut usize,amount:usize,cap:usize,label:&'static str)->Result<(),Error>{
    let total=n.checked_add(amount).ok_or(Error::Limit(label))?;
    if total>cap{return Err(Error::Limit(label));}
    *n=total;Ok(())
}
struct State<'a>{
    choices:&'a [GsubAlternateSelection],
    result:GsubContextExecResult,
    active:Vec<Frame>,
    expansion:Option<GsubExpansionExecLimits>,
    work:ExpansionWork,
    limits:GsubContextExecLimits,
}
impl State<'_>{
    fn expand(&mut self,p:usize,seq:&[u16])->Result<usize,Error>{
        let l=self.expansion.ok_or(Error::Unsupported("gsub contextual expansion"))?;
        let delta=seq.len().checked_sub(1).ok_or(Error::Invalid("gsub empty expansion"))?;
        let new_len=self.result.glyphs.len().checked_add(delta).ok_or(Error::Limit("gsub expansion length"))?;
        if new_len>l.context.max_glyphs{return Err(Error::Limit("gsub expansion length"));}
        charge(&mut self.work.inserted,delta,l.max_inserted,"gsub expansion inserted")?;
        charge(&mut self.work.edits,seq.len(),l.max_edit_work,"gsub expansion edit work")?;
        if delta>0{
            charge(&mut self.work.edits,self.result.glyphs.len()-p-1,l.max_edit_work,"gsub expansion edit work")?;
            for frame in &self.active{
                charge(&mut self.work.edits,frame.positions.len()+2,l.max_edit_work,"gsub expansion edit work")?;
            }
            self.result.glyphs.try_reserve(delta).map_err(|_|Error::Limit("gsub expansion allocation"))?;
            // Earlier/equal anchors remain attached to the first replacement
            // glyph; later anchors move with their original glyph. Boundaries
            // are exclusive, so expansion at the last matched glyph extends
            // that frame, while expansion outside its span does not.
            for frame in &mut self.active{
                if frame.start>p{frame.start+=delta;}
                if frame.end>p{frame.end+=delta;}
                for at in &mut frame.positions{if *at>p{*at+=delta;}}
            }
        }
        self.matched()?;
        self.work.matches+=1;
        self.result.changes+=usize::from(seq!=[self.result.glyphs[p]]);
        self.result.glyphs.splice(p..p+1,seq.iter().copied());
        Ok(p+seq.len())
    }
    fn eligible(&mut self,filter:&LookupFilter<'_>,p:usize)->Result<bool,Error>{
        tick(&mut self.result.filter_checks,self.limits.max_filter_checks,"gsub context filters")?;
        let g=*self.result.glyphs.get(p).ok_or(Error::Invalid("gsub context position"))?;
        Ok(filter.eligibility(g)?.is_eligible())
    }
    fn symbol(&mut self)->Result<(),Error>{
        tick(&mut self.result.symbols,self.limits.max_symbols,"gsub context symbols")
    }
    fn rule(&mut self)->Result<(),Error>{
        tick(&mut self.result.rules,self.limits.max_rules,"gsub context rules")
    }
    fn matched(&mut self)->Result<(),Error>{
        tick(&mut self.result.matches,self.limits.max_matches,"gsub context matches")
    }
    // First position is already covered/eligible. Tail indices refer to
    // the original physical buffer, and ignored glyphs never enter them.
    fn match_tail<F>(&mut self,filter:&LookupFilter<'_>,start:usize,len:usize,
        mut matches:F)->Result<Option<Vec<usize>>,Error>
        where F:FnMut(usize,u16)->bool{
        let mut positions=Vec::new();
        positions.try_reserve(1).map_err(|_|Error::Limit("gsub context positions"))?;
        positions.push(start);
        let mut p=start+1;
        for i in 0..len{
            while p<self.result.glyphs.len()&&!self.eligible(filter,p)?{p+=1;}
            if p==self.result.glyphs.len(){return Ok(None);}
            self.symbol()?;
            if !matches(i,self.result.glyphs[p]){return Ok(None);}
            positions.try_reserve(1).map_err(|_|Error::Limit("gsub context positions"))?;
            positions.push(p);p+=1;
        }
        Ok(Some(positions))
    }
}
struct Matched<'a>{positions:Vec<usize>,actions:&'a [ContextAction]}
fn context_match<'a>(sub:&'a ExtensionTarget,lookup:&CompiledGsubLookup<'_>,
    state:&mut State<'_>,p:usize)->Result<Option<Matched<'a>>,Error>{
    let g=state.result.glyphs[p];
    let filter=lookup.filter();
    match sub{
        ExtensionTarget::Context(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|v==r.remaining[i])?{
                        return Ok(Some(Matched{positions,actions:&r.actions}));
                    }
                }
            }
        }
        ExtensionTarget::ClassContext(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|c.class_for_match(v)==Some(r.remaining[i]))?{
                        return Ok(Some(Matched{positions,actions:&r.actions}));
                    }
                }
            }
        }
        ExtensionTarget::CoverageContext(c)=>{
            state.rule()?;
            state.symbol()?;
            let cov=c.coverages();
            if cov[0].index(g).is_some(){
                if let Some(positions)=state.match_tail(filter,p,cov.len()-1,|i,v|cov[i+1].index(v).is_some())?{
                    return Ok(Some(Matched{positions,actions:c.actions()}));
                }
            }
        }
        _=>return Err(Error::Invalid("gsub context target kind")),
    }
    Ok(None)
}
// Return exclusive physical end of a match, or None. Nested callers
// ignore this end; they execute only at the designated action position.
fn apply_at(program:&CompiledGsubProgram<'_>,id:u16,p:usize,state:&mut State<'_>)
    ->Result<Option<usize>,Error>{
    tick(&mut state.result.calls,state.limits.max_calls,"gsub context calls")?;
    if state.active.len()>=state.limits.max_depth.min(GSUB_CONTEXT_MAX_DEPTH){
        return Err(Error::Limit("gsub context depth"));
    }
    if state.active.iter().any(|f|f.id==id&&f.start==p){return Err(Error::Invalid("gsub context active cycle"));}
    state.active.try_reserve(1).map_err(|_|Error::Limit("gsub context stack"))?;
    state.active.push(Frame{id,start:p,positions:Vec::new(),end:p+1});
    state.result.peak_depth=state.result.peak_depth.max(state.active.len());
    let result=apply_inner(program,id,p,state);
    state.active.pop();
    result
}
fn apply_inner(program:&CompiledGsubProgram<'_>,id:u16,p:usize,state:&mut State<'_>)
    ->Result<Option<usize>,Error>{
    let lookup=program.lookup(id).ok_or(Error::Invalid("gsub context missing lookup"))?;
    if !state.eligible(lookup.filter(),p)?{return Ok(None);}
    for sub in lookup.subtables(){
        tick(&mut state.result.probes,state.limits.max_probes,"gsub context probes")?;
        if let ExtensionTarget::Single(rule)=sub.target(){
            let g=state.result.glyphs[p];
            if let Some(v)=rule.substitute(g){
                state.matched()?;
                state.result.changes+=usize::from(g!=v);
                state.result.glyphs[p]=v;
                return Ok(Some(p+1));
            }
        }else if let ExtensionTarget::Multiple(rule)=sub.target(){
            let g=state.result.glyphs[p];
            if let Some(seq)=rule.substitute(g){return state.expand(p,seq).map(Some);}
        }else if let ExtensionTarget::Alternate(rule)=sub.target(){
            let index=state.choices.binary_search_by_key(&id,|c|c.lookup_index)
                .map_err(|_|Error::Invalid("gsub context alternate missing"))?;
            let g=state.result.glyphs[p];
            if let Some(v)=rule.select(g,state.choices[index].candidate_index)?{
                state.matched()?;
                state.result.changes+=usize::from(g!=v);
                state.result.glyphs[p]=v;
                return Ok(Some(p+1));
            }
        }else if let Some(m)=context_match(sub.target(),lookup,state,p)?{
            state.matched()?;
            // Active frames own their captured positions. All expanding
            // descendants rebase them before the next action is dispatched.
            let frame=state.active.len()-1;
            state.active[frame].end=m.positions.last().copied().unwrap()+1;
            state.active[frame].positions=m.positions;
            for action in m.actions{
                tick(&mut state.result.actions,state.limits.max_actions,"gsub context actions")?;
                let at=*state.active[frame].positions.get(action.sequence_index as usize)
                    .ok_or(Error::Invalid("gsub context action position"))?;
                apply_at(program,action.lookup_index,at,state)?;
            }
            return Ok(Some(state.active[frame].end));
        }
    }
    Ok(None)
}
/// Execute explicit Type 1/5 roots and their reachable Type 1/5
/// dependencies (including Type 7 wrappers). Other reachable kinds fail
/// preflight even for empty input. Context matching uses its own GDEF
/// filter; actions run at captured physical positions in stored order.
/// A matched root advances beyond the matched span, including empty
/// actions. Runtime cycles on an active (lookup,position) fail closed;
/// static cycles are permitted when they do not recur at runtime.
/// All limits are cumulative except depth and input/program size.
/// No output is returned on error; no variable-length or cluster edits.
pub fn execute_gsub_context(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    limits:GsubContextExecLimits)->Result<GsubContextExecResult,Error>{
    execute_context_inner(program,glyphs,None,limits,None).map(|r|r.execution)
}
/// Length-preserving Type 1/3/5 execution. Require exactly one explicit
/// zero-based choice for every reachable effective Type 3 lookup,
/// including dependency-only lookups, sorted by original index.
/// Bounds are checked only on eligible covered glyphs; an invalid
/// choice fails atomically rather than falling through to later subtables.
pub fn execute_gsub_context_with_alternates(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubContextExecLimits)->Result<GsubContextExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits,None).map(|r|r.execution)
}
/// Execute Type 1/2/3/5 roots and reachable targets, including Type 7
/// wrappers. Nonempty expansion preserves the first replacement as the
/// original action anchor. Subsequent actions use rebased positions.
/// A root skips its expanded matched span; later roots see all glyphs.
/// No deletion, ligature contraction, cluster or feature-mask propagation.
pub fn execute_gsub_context_with_expansion(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubExpansionExecLimits)->Result<GsubExpansionExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits.context,Some(limits))
}
fn execute_context_inner(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    selections:Option<&[GsubAlternateSelection]>,limits:GsubContextExecLimits,
    expansion:Option<GsubExpansionExecLimits>)->Result<GsubExpansionExecResult,Error>{
    if glyphs.len()>limits.max_glyphs||program.roots().len()>limits.max_roots
        ||program.lookups().len()>limits.max_lookups{return Err(Error::Limit("gsub context size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){
        return Err(Error::Invalid("gsub context glyph domain"));
    }
    let choices=selections.unwrap_or(&[]);
    if choices.len()>limits.max_lookups{return Err(Error::Limit("gsub context alternate choices"));}
    let mut iter=choices.iter();
    for lookup in program.lookups(){
        match lookup.effective_kind(){
            1|5=>{},
            2 if expansion.is_some()=>{},
            3 if selections.is_some()=>{
                let choice=iter.next().ok_or(Error::Invalid("gsub context alternate missing"))?;
                if choice.lookup_index!=lookup.index(){return Err(Error::Invalid("gsub context alternate order or lookup"));}
            }
            _=>return Err(Error::Unsupported("gsub context reachable kind")),
        }
    }
    if iter.next().is_some(){return Err(Error::Invalid("gsub context alternate extra"));}
    let mut state=State{choices,result:Default::default(),active:Vec::new(),limits,expansion,work:Default::default()};
    state.result.glyphs.try_reserve_exact(glyphs.len()).map_err(|_|Error::Limit("gsub context allocation"))?;
    state.result.glyphs.extend_from_slice(glyphs);
    for &id in program.roots(){
        let mut p=0;
        while p<state.result.glyphs.len(){
            p=apply_at(program,id,p,&mut state)?.unwrap_or(p+1);
        }
    }
    Ok(GsubExpansionExecResult{execution:state.result,multiple_matches:state.work.matches,
        inserted_glyphs:state.work.inserted,edit_work:state.work.edits})
}
