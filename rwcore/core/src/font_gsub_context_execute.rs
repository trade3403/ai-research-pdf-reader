//! Bounded contextual execution with optional Type 2 expansion and Type 4 contraction.
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
/// Additional limits for nested ligature contraction. Contraction work
/// shares the expansion edit budget; removal and skipped actions are
/// independently reported and bounded.
#[derive(Debug,Clone,Copy)]
pub struct GsubContractionExecLimits{
    pub expansion:GsubExpansionExecLimits,
    pub max_removed:usize,
}
impl Default for GsubContractionExecLimits{
    fn default()->Self{Self{expansion:Default::default(),max_removed:4*1024*1024}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GsubContractionExecResult{
    pub expansion:GsubExpansionExecResult,
    pub ligatures:usize,
    pub removed_glyphs:usize,
    pub skipped_actions:usize,
}
const CONSUMED:usize=usize::MAX;
#[derive(Default)]
struct ContractionWork{ligatures:usize,removed:usize,skipped:usize}
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
struct RootMaskState<'a>{
    masks:&'a crate::ScalarFeatureMasks,
    left:usize,
    skipped:usize,
    strict:bool,
    blocked:usize,
    staged_roots:Option<&'a [crate::GsubPlannedLookup]>,
    root_changes:Vec<usize>,
    trace:bool,
}
struct State<'a>{
    provenance:Option<crate::ProvenanceLedger>,
    choices:&'a [GsubAlternateSelection],
    result:GsubContextExecResult,
    active:Vec<Frame>,
    expansion:Option<GsubExpansionExecLimits>,
    work:ExpansionWork,
    contraction:Option<GsubContractionExecLimits>,
    shrink:ContractionWork,
    limits:GsubContextExecLimits,
}
impl State<'_>{
    // Single and alternate substitutions must update both buffers through
    // the same path. Preserve existing execution counter semantics.
    fn replace_one(&mut self,p:usize,output:u16)->Result<(),Error>{
        if let Some(ledger)=self.provenance.as_mut(){ledger.expand(p,&[output])?;}
        self.result.glyphs[p]=output;
        Ok(())
    }
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
                for at in &mut frame.positions{if *at!=CONSUMED&&*at>p{*at+=delta;}}
            }
        }
        self.matched()?;
        self.work.matches+=1;
        self.result.changes+=usize::from(seq!=[self.result.glyphs[p]]);
        if let Some(ledger)=self.provenance.as_mut(){ledger.expand(p,seq)?;}
        self.result.glyphs.splice(p..p+1,seq.iter().copied());
        Ok(p+seq.len())
    }
    // Positions are sorted matched physical indices, including the first.
    // Compact once; preserve every ignored glyph in its original order.
    fn contract(&mut self,positions:&[usize],output:u16)->Result<usize,Error>{
        let l=self.contraction.ok_or(Error::Unsupported("gsub contextual contraction"))?;
        let p=positions[0];
        let removed=&positions[1..];
        let last=*positions.last().unwrap();
        charge(&mut self.shrink.removed,removed.len(),l.max_removed,"gsub contraction removed")?;
        // Charge one ligature write, the full scanned suffix, and
        // each removal/anchor comparison (conservative linear bound).
        let cap=l.expansion.max_edit_work;
        charge(&mut self.work.edits,self.result.glyphs.len()-p,cap,"gsub contraction edit work")?;
        for frame in &self.active{
            let cost=(frame.positions.len()+2).checked_mul(removed.len())
                .ok_or(Error::Limit("gsub contraction edit work"))?;
            charge(&mut self.work.edits,cost,cap,"gsub contraction edit work")?;
        }
        self.matched()?;
        self.shrink.ligatures+=1;
        self.result.changes+=1;
        for frame in &mut self.active{
            let before=|at:usize|removed.partition_point(|&v|v<at);
            // An active start is normally at or before this edit. If
            // consumed, conservatively associate it with the ligature.
            frame.start=if removed.binary_search(&frame.start).is_ok(){p}
                else{frame.start-before(frame.start)};
            frame.end-=before(frame.end);
            for at in &mut frame.positions{
                if *at==CONSUMED{continue;}
                if removed.binary_search(at).is_ok(){*at=CONSUMED;}
                else{*at-=before(*at);}
            }
        }
        if let Some(ledger)=self.provenance.as_mut(){ledger.contract(positions,output)?;}
        self.result.glyphs[p]=output;
        let mut write=p+1;
        let mut k=0;
        for read in p+1..self.result.glyphs.len(){
            if removed.get(k)==Some(&read){k+=1;}
            else{self.result.glyphs[write]=self.result.glyphs[read];write+=1;}
        }
        self.result.glyphs.truncate(write);
        Ok(last+1-removed.len())
    }
    // Match preceding eligible glyphs nearest-first. Each inspected
    // physical position is charged, including ignored glyphs.
    fn match_back<F:FnMut(usize,u16)->bool>(&mut self,filter:&LookupFilter<'_>,
        start:usize,len:usize,mut matches:F)->Result<bool,Error>{
        let mut p=start;
        for i in 0..len{
            loop{
                if p==0{return Ok(false);}
                p-=1;
                if self.eligible(filter,p)?{break;}
            }
            self.symbol()?;
            if !matches(i,self.result.glyphs[p]){return Ok(false);}
        }
        Ok(true)
    }
    // Lookahead begins after the last matched input glyph, not after
    // the first glyph. No lookahead position becomes an action anchor.
    fn match_ahead<F:FnMut(usize,u16)->bool>(&mut self,filter:&LookupFilter<'_>,
        start:usize,len:usize,mut matches:F)->Result<bool,Error>{
        let mut p=start;
        for i in 0..len{
            while p<self.result.glyphs.len()&&!self.eligible(filter,p)?{p+=1;}
            if p==self.result.glyphs.len(){return Ok(false);}
            self.symbol()?;
            if !matches(i,self.result.glyphs[p]){return Ok(false);}
            p+=1;
        }
        Ok(true)
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
    state:&mut State<'_>,p:usize,masking:&mut Option<&mut RootMaskState<'_>>,bits:u64)->Result<Option<Matched<'a>>,Error>{
    let g=state.result.glyphs[p];
    let filter=lookup.filter();
    match sub{
        ExtensionTarget::Context(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|v==r.remaining[i])?{
                        if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:&r.actions}));}
                    }
                }
            }
        }
        ExtensionTarget::ClassContext(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|c.class_for_match(v)==Some(r.remaining[i]))?{
                        if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:&r.actions}));}
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
                    if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:c.actions()}));}
                }
            }
        }
        ExtensionTarget::Chain(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if !state.match_back(filter,p,r.backtrack.len(),|i,v|v==r.backtrack[i])?{continue;}
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|v==r.remaining[i])?{
                        if state.match_ahead(filter,positions.last().copied().unwrap()+1,
                            r.lookahead.len(),|i,v|v==r.lookahead[i])?{
                            if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:&r.actions}));}
                        }
                    }
                }
            }
        }
        ExtensionTarget::ClassChain(c)=>{
            if let Some(rules)=c.rules(g){
                for r in rules{
                    state.rule()?;
                    if !state.match_back(filter,p,r.backtrack.len(),|i,v|c.backtrack_class(v)==Some(r.backtrack[i]))?{continue;}
                    if let Some(positions)=state.match_tail(filter,p,r.remaining.len(),|i,v|c.input_class(v)==Some(r.remaining[i]))?{
                        if state.match_ahead(filter,positions.last().copied().unwrap()+1,
                            r.lookahead.len(),|i,v|c.lookahead_class(v)==Some(r.lookahead[i]))?{
                            if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:&r.actions}));}
                        }
                    }
                }
            }
        }
        ExtensionTarget::CoverageChain(c)=>{
            state.rule()?;state.symbol()?;
            let input=c.input();
            if input[0].index(g).is_some()&&
                state.match_back(filter,p,c.backtrack().len(),|i,v|c.backtrack()[i].index(v).is_some())?{
                if let Some(positions)=state.match_tail(filter,p,input.len()-1,|i,v|input[i+1].index(v).is_some())?{
                    if state.match_ahead(filter,positions.last().copied().unwrap()+1,
                        c.lookahead().len(),|i,v|c.lookahead()[i].index(v).is_some())?{
                        if common_input_bits(state,&positions,bits,masking)?!=0{return Ok(Some(Matched{positions,actions:c.actions()}));}
                    }
                }
            }
        }
        _=>return Err(Error::Invalid("gsub context target kind")),
    }
    Ok(None)
}
// Return exclusive physical end of a match, or None. Nested callers
// ignore this end; they execute only at the designated action position.
fn apply_at(program:&CompiledGsubProgram<'_>,id:u16,p:usize,state:&mut State<'_>,masking:&mut Option<&mut RootMaskState<'_>>,bits:u64)
    ->Result<Option<usize>,Error>{
    tick(&mut state.result.calls,state.limits.max_calls,"gsub context calls")?;
    if state.active.len()>=state.limits.max_depth.min(GSUB_CONTEXT_MAX_DEPTH){
        return Err(Error::Limit("gsub context depth"));
    }
    if state.active.iter().any(|f|f.id==id&&f.start==p){return Err(Error::Invalid("gsub context active cycle"));}
    state.active.try_reserve(1).map_err(|_|Error::Limit("gsub context stack"))?;
    state.active.push(Frame{id,start:p,positions:Vec::new(),end:p+1});
    state.result.peak_depth=state.result.peak_depth.max(state.active.len());
    let result=apply_inner(program,id,p,state,masking,bits);
    state.active.pop();
    result
}
fn apply_inner(program:&CompiledGsubProgram<'_>,id:u16,p:usize,state:&mut State<'_>,masking:&mut Option<&mut RootMaskState<'_>>,bits:u64)
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
                state.replace_one(p,v)?;
                return Ok(Some(p+1));
            }
        }else if let ExtensionTarget::Reverse(rule)=sub.target(){
            // Type 8 is length-preserving. Its conditions use the same
            // current-run, nearest-first backtrack and forward lookahead
            // semantics as Type 6. Only the input glyph is replaced.
            state.rule()?;state.symbol()?;
            let g=state.result.glyphs[p];
            if let Some(index)=rule.input().index(g){
                if state.match_back(lookup.filter(),p,rule.backtrack().len(),
                    |i,v|rule.backtrack()[i].index(v).is_some())? &&
                    state.match_ahead(lookup.filter(),p+1,rule.lookahead().len(),
                    |i,v|rule.lookahead()[i].index(v).is_some())?{
                    let output=*rule.replacements().get(index as usize)
                        .ok_or(Error::Invalid("gsub reverse replacement index"))?;
                    state.matched()?;
                    state.result.changes+=usize::from(output!=g);
                    state.replace_one(p,output)?;
                    return Ok(Some(p+1));
                }
            }
        }else if let ExtensionTarget::Ligature(rule)=sub.target(){
            let g=state.result.glyphs[p];
            if let Some(rules)=rule.rules(g){
                for rule in rules{
                    state.rule()?;
                    if let Some(positions)=state.match_tail(lookup.filter(),p,rule.remaining.len(),
                        |i,v|v==rule.remaining[i])?{
                        if common_input_bits(state,&positions,bits,masking)?==0{continue;}
                        return state.contract(&positions,rule.output).map(Some);
                    }
                }
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
                state.replace_one(p,v)?;
                return Ok(Some(p+1));
            }
        }else if let Some(m)=context_match(sub.target(),lookup,state,p,masking,bits)?{
            let inherited=common_input_bits(state,&m.positions,bits,masking)?;
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
                if at==CONSUMED{state.shrink.skipped+=1;continue;}
                let live=common_input_bits(state,&[at],inherited,masking)?;
                if live==0{continue;}
                apply_at(program,action.lookup_index,at,state,masking,live)?;
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
    execute_context_inner(program,glyphs,None,limits,None,None,false,false,&mut None,None).map(|r|r.expansion.execution)
}
/// Length-preserving Type 1/3/5 execution. Require exactly one explicit
/// zero-based choice for every reachable effective Type 3 lookup,
/// including dependency-only lookups, sorted by original index.
/// Bounds are checked only on eligible covered glyphs; an invalid
/// choice fails atomically rather than falling through to later subtables.
pub fn execute_gsub_context_with_alternates(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubContextExecLimits)->Result<GsubContextExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits,None,None,false,false,&mut None,None).map(|r|r.expansion.execution)
}
/// Execute Type 1/2/3/5 roots and reachable targets, including Type 7
/// wrappers. Nonempty expansion preserves the first replacement as the
/// original action anchor. Subsequent actions use rebased positions.
/// A root skips its expanded matched span; later roots see all glyphs.
/// No deletion, ligature contraction, cluster or feature-mask propagation.
pub fn execute_gsub_context_with_expansion(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubExpansionExecLimits)->Result<GsubExpansionExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits.context,Some(limits),None,false,false,&mut None,None).map(|r|r.expansion)
}
/// Execute Type 1/2/3/4/5 (direct or wrapped) roots and dependencies.
/// A ligature retains its first anchor and tombstones consumed trailing
/// anchors; actions referencing tombstones count as actions but do not
/// call their targets. Other positions and exclusive boundaries rebase.
/// No cluster/mark component metadata or feature-mask propagation.
pub fn execute_gsub_context_with_contraction(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubContractionExecLimits)->Result<GsubContractionExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits.expansion.context,Some(limits.expansion),Some(limits),false,false,&mut None,None)
}
/// Length-preserving Type 1/3/5/6 roots and reachable dependencies,
/// including Type 7 wrappers. Backtrack is nearest-first; input and
/// lookahead are forward. All three use the current lookup's GDEF
/// eligibility. Only input positions can be action anchors, and a root
/// advances past its input span (not lookahead). All reads see current
/// glyphs. Existing depth/cycle/work limits and explicit alternate
/// choices apply. Other reachable kinds are rejected at preflight.
pub fn execute_gsub_chained_context(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubContextExecLimits)->Result<GsubContextExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits,None,None,true,false,&mut None,None).map(|r|r.expansion.execution)
}
/// Execute effective Type 1 through 6, including Type 7 wrappers.
/// Uses the same input-anchor and exclusive-boundary policy as the
/// contextual contraction API. Backtrack and lookahead are conditions,
/// not action anchors; each new lookup matches the current glyph run.
/// Descendants may inspect or edit beyond an ancestor input span.
/// Growth keeps the first output anchor; ligatures retain the first
/// component and skip actions on consumed trailing components.
/// All existing context, expansion and contraction budgets apply.
/// Type 8, cluster metadata and feature masks remain unsupported.
pub fn execute_gsub_chained_context_with_edits(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubContractionExecLimits)->Result<GsubContractionExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits.expansion.context,Some(limits.expansion),Some(limits),true,false,&mut None,None)
}
/// Stable name for the full-program execution budget. This is the
/// existing contraction/context/expansion budget, not an independent
/// second set of limits.
pub type GsubProgramExecLimits=GsubContractionExecLimits;
/// Stable name for the full-program execution result. All existing
/// counters and nested fields are retained for source compatibility.
pub type GsubProgramExecResult=GsubContractionExecResult;

/// Execute all effective GSUB substitution kinds (1–6 and 8), including
/// Type 7 wrappers. Root order is unchanged, but each effective Type 8
/// root visits current physical positions from end to start; other
/// roots advance forward. Earlier positions see already replaced later
/// glyphs during a reverse pass. A nested Type 8 action is a single
/// position invocation, not a reverse scan of the entire run.
/// All existing context and edit budgets and explicit alternate
/// selections apply. No cluster metadata, feature masks or script stages.
pub fn execute_gsub_program(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    choices:&[GsubAlternateSelection],limits:GsubProgramExecLimits)->Result<GsubProgramExecResult,Error>{
    execute_context_inner(program,glyphs,Some(choices),limits.expansion.context,Some(limits.expansion),Some(limits),true,true,&mut None,None)
}
fn execute_context_inner(program:&CompiledGsubProgram<'_>,glyphs:&[u16],
    selections:Option<&[GsubAlternateSelection]>,limits:GsubContextExecLimits,
    expansion:Option<GsubExpansionExecLimits>,contraction:Option<GsubContractionExecLimits>,chained:bool,reverse:bool,provenance:&mut Option<crate::ProvenanceLedger>,mut masking:Option<&mut RootMaskState<'_>>)->Result<GsubContractionExecResult,Error>{
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
            6 if chained=>{},
            8 if reverse=>{},
            2 if expansion.is_some()=>{},
            4 if contraction.is_some()=>{},
            3 if selections.is_some()=>{
                let choice=iter.next().ok_or(Error::Invalid("gsub context alternate missing"))?;
                if choice.lookup_index!=lookup.index(){return Err(Error::Invalid("gsub context alternate order or lookup"));}
            }
            _=>return Err(Error::Unsupported("gsub context reachable kind")),
        }
    }
    if iter.next().is_some(){return Err(Error::Invalid("gsub context alternate extra"));}
    let mut state=State{provenance:provenance.take(),choices,result:Default::default(),active:Vec::new(),limits,expansion,work:Default::default(),contraction,shrink:Default::default()};
    state.result.glyphs.try_reserve_exact(glyphs.len()).map_err(|_|Error::Limit("gsub context allocation"))?;
    state.result.glyphs.extend_from_slice(glyphs);
    let root_count=masking.as_ref().and_then(|m|m.staged_roots).map_or(program.roots().len(),|r|r.len());
    if root_count>limits.max_roots{return Err(Error::Limit("gsub staged roots"));}
    for root_index in 0..root_count{
        let before_changes=state.result.changes;
        let id=masking.as_ref().and_then(|m|m.staged_roots).map_or_else(||program.roots()[root_index],|r|r[root_index].index);
        let root_mask=if let Some(m)=masking.as_deref_mut(){
            let plan=program.validated_plan().ok_or(Error::Invalid("gsub masks missing plan"))?;
            let features=if let Some(roots)=m.staged_roots{&roots[root_index].features}else{&plan.lookups[root_index].features};
            m.left=m.left.checked_sub(features.len()).ok_or(Error::Limit("gsub mask work"))?;
            m.masks.lookup_mask(features)?
        }else{0};
        let lookup=program.lookup(id).ok_or(Error::Invalid("gsub context missing root"))?;
        if lookup.effective_kind()==8{
            // A reverse lookup can only replace one glyph at a time;
            // the run length stays fixed throughout this root.
            for p in (0..state.result.glyphs.len()).rev(){
                if root_allowed(&state,p,root_mask,masking.as_deref_mut())?{apply_at(program,id,p,&mut state,&mut masking,if root_mask==0{u64::MAX}else{root_mask})?;}
            }
        }else{
            let mut p=0;
            while p<state.result.glyphs.len(){
                p=if root_allowed(&state,p,root_mask,masking.as_deref_mut())?{apply_at(program,id,p,&mut state,&mut masking,if root_mask==0{u64::MAX}else{root_mask})?.unwrap_or(p+1)}else{p+1};
            }
        }
        if let Some(m)=masking.as_deref_mut(){
            if m.trace{m.root_changes.push(state.result.changes-before_changes);}
        }
    }
    *provenance=state.provenance.take();
    Ok(GsubContractionExecResult{expansion:GsubExpansionExecResult{execution:state.result,
        multiple_matches:state.work.matches,inserted_glyphs:state.work.inserted,edit_work:state.work.edits},
        ligatures:state.shrink.ligatures,removed_glyphs:state.shrink.removed,skipped_actions:state.shrink.skipped})
}

/// Atomic provenance-aware execution for effective types 1 and 2
/// (including extension wrappers). Reject other reachable kinds before
/// execution. The existing scheduler and counters remain authoritative.
pub fn execute_gsub_with_provenance(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits)
    ->Result<(GsubProgramExecResult,crate::ProvenanceLedger),Error>{
    if program.lookups().iter().any(|l|!matches!(l.effective_kind(),1|2)){
        return Err(Error::Unsupported("gsub provenance reachable kind"));
    }
    execute_gsub_with_direct_provenance(program,run,&[],limits,source_limits)
}

/// Provenance-aware direct Types 1–4, including extension wrappers.
/// Type 3 requires explicit choices; Type 4 merges only matched physical
/// components, leaving ignored gaps and their sources untouched.
pub fn execute_gsub_with_direct_provenance(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits)
    ->Result<(GsubProgramExecResult,crate::ProvenanceLedger),Error>{
    if program.lookups().iter().any(|l|!matches!(l.effective_kind(),1..=4)){
        return Err(Error::Unsupported("gsub provenance reachable kind"));
    }
    execute_gsub_program_with_provenance(program,run,choices,limits,source_limits)
}

/// Execute all supported GSUB effective kinds with exact scalar source
/// sets. Context conditions do not contribute sources; only actual
/// replacement components do. Nested actions use live physical positions.
/// The same scheduler, counters and limits apply as the untracked API.
pub fn execute_gsub_program_with_provenance(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits)
    ->Result<(GsubProgramExecResult,crate::ProvenanceLedger),Error>{
    if run.glyphs().len()>limits.expansion.context.max_glyphs{
        return Err(Error::Limit("gsub provenance input"));
    }
    let mut provenance=Some(crate::ProvenanceLedger::from_run(run,program.glyph_count(),source_limits)?);
    let mut glyphs=Vec::new();
    glyphs.try_reserve_exact(run.glyphs().len()).map_err(|_|Error::Limit("gsub provenance allocation"))?;
    glyphs.extend(run.glyphs().iter().map(|g|g.glyph_id));
    let result=execute_context_inner(program,&glyphs,Some(choices),limits.expansion.context,
        Some(limits.expansion),Some(limits),true,true,&mut provenance,None)?;
    let ledger=provenance.ok_or(Error::Invalid("gsub provenance missing"))?;
    if !ledger.glyphs().iter().map(|g|g.glyph_id).eq(result.expansion.execution.glyphs.iter().copied()){
        return Err(Error::Invalid("gsub provenance divergence"));
    }
    Ok((result,ledger))
}

/// Opt-in ordered contraction history, with independent trace budgets.
pub fn execute_gsub_program_with_ligature_trace(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,trace_limits:crate::LigatureTraceLimits)
    ->Result<(GsubProgramExecResult,crate::ProvenanceLedger),Error>{
    if run.glyphs().len()>limits.expansion.context.max_glyphs{
        return Err(Error::Limit("gsub provenance input"));
    }
    let mut provenance=Some(crate::ProvenanceLedger::from_run_with_ligature_trace(run,program.glyph_count(),source_limits,trace_limits)?);
    let mut glyphs=Vec::new();
    glyphs.try_reserve_exact(run.glyphs().len()).map_err(|_|Error::Limit("gsub provenance allocation"))?;
    glyphs.extend(run.glyphs().iter().map(|g|g.glyph_id));
    let result=execute_context_inner(program,&glyphs,Some(choices),limits.expansion.context,
        Some(limits.expansion),Some(limits),true,true,&mut provenance,None)?;
    let ledger=provenance.ok_or(Error::Invalid("gsub provenance missing"))?;
    if !ledger.glyphs().iter().map(|g|g.glyph_id).eq(result.expansion.execution.glyphs.iter().copied()){
        return Err(Error::Invalid("gsub provenance divergence"));
    }
    Ok((result,ledger))
}

fn root_allowed(state:&State<'_>,p:usize,root_mask:u64,masking:Option<&mut RootMaskState<'_>>)->Result<bool,Error>{
    let Some(m)=masking else{return Ok(true);};
    let bits=live_bits(state,p,m)?;
    let allowed=bits&root_mask!=0;
    if !allowed{m.skipped+=1;}
    Ok(allowed)
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct MaskedGsubResult{
    pub execution:GsubProgramExecResult,
    pub provenance:crate::ProvenanceLedger,
    pub glyph_masks:Vec<u64>,
    pub skipped_roots:usize,
    pub mask_work:usize,
}
/// Root-anchor mask gating only. Context dependencies and additional
/// ligature components are not individually gated in this API.
/// Shared root features are ORed; each glyph uses intersection of its
/// original scalar masks. The compiled plan must have validated provenance.
pub fn execute_gsub_program_with_masks(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize)
    ->Result<MaskedGsubResult,Error>{
    execute_masked_inner(program,run,choices,masks,limits,source_limits,max_mask_work,false,None,false).map(|r|r.0)
}
/// Strict shared-feature input and nested action policy. Returns the
/// legacy masked result plus number of rejected input/action checks.
pub fn execute_gsub_program_with_strict_masks(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize)
    ->Result<(MaskedGsubResult,usize),Error>{
    execute_masked_inner(program,run,choices,masks,limits,source_limits,max_mask_work,true,None,false).map(|(r,b,_)|(r,b))
}
fn execute_masked_inner(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize,strict:bool,staged_roots:Option<&[crate::GsubPlannedLookup]>,trace:bool)
    ->Result<(MaskedGsubResult,usize,Vec<usize>),Error>{
    let plan=program.validated_plan().ok_or(Error::Invalid("gsub masks missing plan"))?;
    if masks.features()!=plan.features.as_slice()||masks.required_feature()!=plan.required_feature||
        masks.scalar_masks().len()!=run.scalars().len(){
        return Err(Error::Invalid("gsub masks plan or scalar domain"));
    }
    if run.glyphs().len()>limits.expansion.context.max_glyphs{
        return Err(Error::Limit("gsub mask input"));
    }
    let mut provenance=Some(crate::ProvenanceLedger::from_run(run,program.glyph_count(),source_limits)?);
    let mut glyphs=Vec::new();glyphs.try_reserve_exact(run.glyphs().len()).map_err(|_|Error::Limit("gsub mask allocation"))?;
    glyphs.extend(run.glyphs().iter().map(|g|g.glyph_id));
    let mut root_changes=Vec::new();
    if trace{root_changes.try_reserve_exact(staged_roots.map_or(program.roots().len(),|r|r.len())).map_err(|_|Error::Limit("gsub trace allocation"))?;}
    let mut m=RootMaskState{masks,left:max_mask_work,skipped:0,strict,blocked:0,staged_roots,root_changes,trace};
    let execution=execute_context_inner(program,&glyphs,Some(choices),limits.expansion.context,
        Some(limits.expansion),Some(limits),true,true,&mut provenance,Some(&mut m))?;
    let ledger=provenance.ok_or(Error::Invalid("gsub mask missing ledger"))?;
    if !ledger.glyphs().iter().map(|g|g.glyph_id).eq(execution.expansion.execution.glyphs.iter().copied()){
        return Err(Error::Invalid("gsub mask divergence"));
    }
    let glyph_masks=masks.project(&ledger,m.left)?;
    m.left=m.left.checked_sub(ledger.glyphs().len()).and_then(|v|v.checked_sub(ledger.reference_count())).ok_or(Error::Limit("gsub mask work"))?;
    Ok((MaskedGsubResult{execution,provenance:ledger,glyph_masks,skipped_roots:m.skipped,mask_work:max_mask_work-m.left},m.blocked,m.root_changes))
}

fn live_bits(state:&State<'_>,p:usize,m:&mut RootMaskState<'_>)->Result<u64,Error>{
    let ledger=state.provenance.as_ref().ok_or(Error::Invalid("gsub mask missing provenance"))?;
    let g=ledger.glyphs().get(p).ok_or(Error::Invalid("gsub mask position"))?;
    m.left=m.left.checked_sub(1).and_then(|v|v.checked_sub(g.sources().len())).ok_or(Error::Limit("gsub mask work"))?;
    let mut bits=u64::MAX;
    for &i in g.sources(){bits&=*m.masks.scalar_masks().get(i).ok_or(Error::Invalid("gsub mask scalar"))?;}
    Ok(bits)
}
// For strict mode, at least one common contributing feature must remain
// enabled across every matched input. Mask-disabled glyphs are barriers,
// not GDEF-ignored gaps. Backtrack/lookahead are conditions only.
fn common_input_bits(state:&State<'_>,positions:&[usize],mut bits:u64,
    masking:&mut Option<&mut RootMaskState<'_>>)->Result<u64,Error>{
    let Some(m)=masking.as_deref_mut() else{return Ok(bits);};
    if !m.strict{return Ok(bits);}
    for &p in positions{
        bits&=live_bits(state,p,m)?;
        if bits==0{m.blocked=m.blocked.checked_add(1).ok_or(Error::Limit("gsub mask blocked"))?;break;}
    }
    Ok(bits)
}

/// Build and execute an explicit stage partition in one atomic run.
/// All existing execution, source and mask budgets are cumulative.
/// A shared root may run in multiple stages with distinct contributor
/// masks; the physical glyph buffer and provenance ledger never reset.
/// Alternate choices remain one per reachable lookup, across stages.
pub fn execute_staged_gsub(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    specs:&[crate::ShapeStageSpec],stage_limits:crate::ShapeStageLimits,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize)
    ->Result<(crate::ShapeStageSchedule,MaskedGsubResult,usize),Error>{
    execute_staged_inner(program,run,choices,masks,specs,stage_limits,limits,source_limits,max_mask_work,false)
        .map(|(s,r,b,_)|(s,r,b))
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ShapeStageTrace{
    pub stage_index:usize,
    pub root_count:usize,
    /// Number of changing substitution operations, not number of glyphs.
    pub changes:usize,
}
/// Same atomic execution with bounded stage-level change diagnostics.
/// Trace storage is limited by validated stage and root caps.
pub fn execute_staged_gsub_traced(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    specs:&[crate::ShapeStageSpec],stage_limits:crate::ShapeStageLimits,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize)
    ->Result<(crate::ShapeStageSchedule,MaskedGsubResult,usize,Vec<ShapeStageTrace>),Error>{
    execute_staged_inner(program,run,choices,masks,specs,stage_limits,limits,source_limits,max_mask_work,true)
}
fn execute_staged_inner(program:&CompiledGsubProgram<'_>,run:&crate::SourceRun,
    choices:&[GsubAlternateSelection],masks:&crate::ScalarFeatureMasks,
    specs:&[crate::ShapeStageSpec],stage_limits:crate::ShapeStageLimits,
    limits:GsubProgramExecLimits,source_limits:crate::ProvenanceLimits,max_mask_work:usize,trace:bool)
    ->Result<(crate::ShapeStageSchedule,MaskedGsubResult,usize,Vec<ShapeStageTrace>),Error>{
    let schedule=crate::build_shape_stage_schedule(program,specs,stage_limits)?;
    if schedule.root_count()>limits.expansion.context.max_roots{return Err(Error::Limit("gsub staged roots"));}
    let mut roots=Vec::new();
    roots.try_reserve_exact(schedule.root_count()).map_err(|_|Error::Limit("gsub staged allocation"))?;
    for stage in schedule.stages(){
        for root in stage.lookups(){
            let mut features=Vec::new();
            features.try_reserve_exact(root.features.len()).map_err(|_|Error::Limit("gsub staged allocation"))?;
            features.extend_from_slice(&root.features);
            roots.push(crate::GsubPlannedLookup{index:root.index,features});
        }
    }
    let (result,blocked,counts)=execute_masked_inner(program,run,choices,masks,limits,source_limits,max_mask_work,true,Some(&roots),trace)?;
    let mut traces=Vec::new();
    if trace{
        traces.try_reserve_exact(schedule.stages().len()).map_err(|_|Error::Limit("gsub trace allocation"))?;
        let mut at=0;
        for (stage_index,stage) in schedule.stages().iter().enumerate(){
            let n=stage.lookups().len();
            let end=at+n;
            let changes=counts.get(at..end).ok_or(Error::Invalid("gsub trace count"))?.iter().sum();
            traces.push(ShapeStageTrace{stage_index,root_count:n,changes});
            at=end;
        }
        if at!=counts.len(){return Err(Error::Invalid("gsub trace length"));}
    }
    Ok((schedule,result,blocked,traces))
}
