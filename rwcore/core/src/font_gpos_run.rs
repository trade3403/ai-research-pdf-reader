//! Bounded GPOS execution, preserving original glyph indices.
//! Legacy restricted entry points and a shared horizontal mixed runner.
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
#[derive(Clone,Copy)]
struct CursiveContribution{
    end:usize,child:usize,owner:usize,design:i64,pixels:i64,
}
struct DirectionalLedger{
    // Active non-overlapping half-open spans, keyed by first logical index.
    spans:std::collections::BTreeMap<usize,CursiveContribution>,
    child_start:Vec<Option<usize>>,
}
impl DirectionalLedger{
    fn remove(&mut self,start:usize,out:&mut [GposRunAdjustment],edges:&mut [Option<GposAttachmentEdge>],children:&mut [bool],updates:&mut usize)->Result<(),Error>{
        let c=*self.spans.get(&start).ok_or(Error::Invalid("cursive ledger missing span"))?;
        charge(updates,1)?;
        let d=i32::try_from(i64::from(out[c.owner].design.x_advance)-c.design).map_err(|_|Error::Limit("cursive ledger restore"))?;
        let px=i32::try_from(i64::from(out[c.owner].pixels.x_advance)-c.pixels).map_err(|_|Error::Limit("cursive ledger restore"))?;
        out[c.owner].design.x_advance=d;
        out[c.owner].pixels.x_advance=px;
        self.spans.remove(&start);
        self.child_start[c.child]=None;
        children[c.child]=false;
        edges[c.child]=None;
        // Keep subsequent local placement adjustments, not a stale resolved
        // world placement. The next attachment may reset its moving child.
        Ok(())
    }
    fn overlaps(&self,a:usize,b:usize)->bool{
        self.spans.range(..=a).next_back().is_some_and(|(_,c)|c.end>a)
            ||self.spans.range(a..b).next().is_some()
    }
}
struct MixedCursive<'g>{
    design_advances:&'g [(i32,i32)],
    pixel_advances:&'g [(i32,i32)],
    children:Vec<bool>,
    flow:Option<GposHorizontalFlow>,
    ledger:Option<DirectionalLedger>,
    connections:usize,
    spans:usize,
}
struct ContextRunner<'p,'g,'d>{
    program:&'p CompiledGposProgram<'d>,glyphs:&'g [u16],out:Vec<GposRunAdjustment>,mode:GposRunMode,
    limits:GposContextRunLimits,visits:usize,attempts:usize,calls:usize,actions:usize,work:usize,
    active:Vec<(u16,usize)>,
    attachment_edges:Option<Vec<Option<GposAttachmentEdge>>>,
    parent_visits:usize,
    attachment_updates:usize,
    ligatures:Option<&'g [GposLigatureSource]>,
    cursive:Option<MixedCursive<'g>>,
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
    fn attach(&mut self,child:usize,parent:usize,a:&GposAnchor,b:&GposAnchor)->Result<(),Error>{
        charge(&mut self.attachment_updates,1)?;
        let edge=GposAttachmentEdge::from_anchors(parent,a,b,self.mode)?;
        let edges=self.attachment_edges.as_mut().ok_or(Error::Unsupported("gpos attachment mode"))?;
        if let Some(state)=self.cursive.as_mut(){
            if state.children[child]{
                let ledger=state.ledger.as_mut().ok_or(Error::Unsupported("mixed cursive parent replacement"))?;
                let start=ledger.child_start[child].ok_or(Error::Invalid("cursive ledger child"))?;
                ledger.remove(start,&mut self.out,edges,&mut state.children,&mut self.attachment_updates)?;
            }
        }
        edges[child]=Some(edge);
        // Attachment replaces prior placement; subsequent adjustments are residuals.
        self.out[child].design.x_placement=0;self.out[child].design.y_placement=0;
        self.out[child].pixels.x_placement=0;self.out[child].pixels.y_placement=0;
        Ok(())
    }
    fn connect_cursive(&mut self,a:usize,b:usize,exit:&GposAnchor,entry:&GposAnchor,rtl:bool)->Result<(),Error>{
        use crate::font_gpos_cursive_run::{advance,directional_advance};
        let state=self.cursive.as_mut().ok_or(Error::Unsupported("mixed cursive mode"))?;
        let edges=self.attachment_edges.as_mut().ok_or(Error::Invalid("mixed cursive edges"))?;
        let forward=GposAttachmentEdge::from_anchors(a,exit,entry,self.mode)?;
        let (child,edge)=if rtl{(a,GposAttachmentEdge::from_anchors(b,entry,exit,self.mode)?)}else{(b,forward)};
        charge(&mut state.connections,1)?;
        charge(&mut state.spans,b-a)?;
        charge(&mut self.attachment_updates,1)?;
        if let Some(ledger)=state.ledger.as_mut(){
            // Last successful attachment wins for the moving child. An exact
            // interval can also reverse its fixed end without leaving two edges.
            if let Some(start)=ledger.child_start[child]{
                ledger.remove(start,&mut self.out,edges,&mut state.children,&mut self.attachment_updates)?;
            }
            if ledger.spans.get(&a).is_some_and(|c|c.end==b){
                ledger.remove(a,&mut self.out,edges,&mut state.children,&mut self.attachment_updates)?;
            }
            if ledger.overlaps(a,b){
                return Err(Error::Unsupported("mixed overlapping cursive intervals"));
            }
        }else{
            // Legacy signed-advance API retains the previous conflict policy.
            if [a,b].into_iter().any(|i|edges[i].is_some()&&!state.children[i]){
                return Err(Error::Unsupported("mixed mark cursive endpoint"));
            }
            if edges[child].is_some(){return Err(Error::Unsupported("mixed cursive repeated parent"));}
        }
        let (owner,d)=if let Some(flow)=state.flow{
            directional_advance(state.design_advances,&self.out,a,b,child,forward.design_delta.0,false,flow)?
        }else{(a,advance(state.design_advances,&self.out,a,b,forward.design_delta.0,false)?)};
        let px=if matches!(self.mode,GposRunMode::DevicePixels{..}){
            Some(if let Some(flow)=state.flow{
                directional_advance(state.pixel_advances,&self.out,a,b,child,forward.pixel_delta.0,true,flow)?.1
            }else{advance(state.pixel_advances,&self.out,a,b,forward.pixel_delta.0,true)?})
        }else{None};
        if let Some(ledger)=state.ledger.as_mut(){
            let contribution=CursiveContribution{end:b,child,owner,
                design:i64::from(d)-i64::from(self.out[owner].design.x_advance),
                pixels:px.map_or(0,|v|i64::from(v)-i64::from(self.out[owner].pixels.x_advance))};
            ledger.spans.insert(a,contribution);
            ledger.child_start[child]=Some(a);
        }
        edges[child]=Some(edge);
        state.children[child]=true;
        self.out[owner].design.x_advance=d;
        self.out[child].design.x_placement=0;
        self.out[child].design.y_placement=0;
        if let Some(px)=px{
            self.out[owner].pixels.x_advance=px;
            self.out[child].pixels.x_placement=0;
            self.out[child].pixels.y_placement=0;
        }
        Ok(())
    }
    fn mark_parent(&mut self,lookup:&CompiledGposLookup<'_>,ids:&[usize],pos:usize,mark_to_mark:bool)->Result<Option<usize>,Error>{
        for &i in ids[..pos].iter().rev(){
            charge(&mut self.parent_visits,1)?;
            let class=lookup.filter().glyph_class(self.glyphs[i]).ok_or(Error::Invalid("gpos parent glyph"))?;
            if !mark_to_mark&&class==GdefGlyphClass::Mark{continue;}
            // Do not jump over an eligible incompatible non-mark/mark.
            if mark_to_mark&&matches!(class,GdefGlyphClass::Base|GdefGlyphClass::Ligature|GdefGlyphClass::Component){return Ok(None);}
            return Ok(Some(i));
        }
        Ok(None)
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
                Cursive(p)=>{
                    if self.cursive.is_none(){return Err(Error::Unsupported("mixed cursive mode"));}
                    let Some(&b)=ids.get(pos+1)else{return Ok(1);};
                    if let Some((exit,entry))=p.pair(self.glyphs[a],self.glyphs[b]){
                        self.connect_cursive(a,b,exit,entry,lookup.filter().right_to_left())?;
                        return Ok(1);
                    }
                    continue;
                }
                MarkBase(p)=>{
                    if self.attachment_edges.is_none(){return Err(Error::Unsupported("gpos attachment mode"));}
                    if p.mark(stream[pos]).is_none(){continue;}
                    if let Some(parent)=self.mark_parent(lookup,ids,pos,false)?{
                        if let Some((pa,ca))=p.pair(self.glyphs[parent],stream[pos]){self.attach(a,parent,pa,ca)?;return Ok(1);}
                    }continue;
                }
                MarkLigature(p)=>{
                    if self.attachment_edges.is_none(){return Err(Error::Unsupported("gpos attachment mode"));}
                    if p.mark(stream[pos]).is_none(){continue;}
                    if let Some(parent)=self.mark_parent(lookup,ids,pos,false)?{
                        if let Some(count)=p.component_count(self.glyphs[parent]){
                            let sources=self.ligatures.ok_or(Error::Unsupported("gpos ligature provenance"))?;
                            let component=select_gpos_ligature_component(sources[parent],sources[a],count)?;
                            if let Some((pa,ca))=p.pair(self.glyphs[parent],component,stream[pos]){self.attach(a,parent,pa,ca)?;return Ok(1);}
                        }
                    }continue;
                }
                MarkMark(p)=>{
                    if self.attachment_edges.is_none(){return Err(Error::Unsupported("gpos attachment mode"));}
                    if p.mark1(stream[pos]).is_none(){continue;}
                    if let Some(parent)=self.mark_parent(lookup,ids,pos,true)?{
                        if self.ligatures.is_some_and(|sources|!gpos_mark_sources_compatible(sources[parent],sources[a])){continue;}
                        if let Some((pa,ca))=p.pair(self.glyphs[parent],stream[pos]){self.attach(a,parent,pa,ca)?;return Ok(1);}
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
    let mut runner=ContextRunner{program,glyphs,out,mode,limits:l,visits:l.run.max_filter_visits,attempts:l.run.max_subtable_attempts,calls:l.max_calls,actions:l.max_actions,work:l.max_context_work,active:Vec::new(),attachment_edges:None,parent_visits:0,attachment_updates:0,ligatures:None,cursive:None};
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("gpos context root"))?;
        let (ids,stream)=runner.filtered(lookup)?;
        let mut pos=0;while pos<ids.len(){pos+=runner.at(lookup,&ids,&stream,pos)?;}
    }
    Ok(runner.out)
}

#[derive(Debug,Clone,Copy)]
pub struct GposMarkRunLimits{
    pub context:GposContextRunLimits,
    pub max_parent_visits:usize,
    pub max_attachment_updates:usize,
}
impl Default for GposMarkRunLimits{
    fn default()->Self{Self{context:Default::default(),max_parent_visits:16777216,max_attachment_updates:1048576}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposMarkRun{
    pub residuals:Vec<GposRunAdjustment>,
    pub edges:Vec<Option<GposAttachmentEdge>>,
}
impl GposMarkRun{
    /// Origins must include final advance changes and exclude placement.
    /// In DesignOnly mode callers should supply zero pixel origins and residuals.
    pub fn resolve(&self,design_origins:&[(i32,i32)],pixel_origins:&[(i32,i32)],limits:GposAttachmentLimits)->Result<Vec<GposRunAdjustment>,Error>{
        resolve_gpos_attachments(&self.edges,design_origins,pixel_origins,&self.residuals,limits)
    }
}
/// Execute effective types 1,2,4,6,7,8 in logical order, including nested
/// mark actions. MarkToBase skips classified marks; MarkToMark stops at an
/// eligible non-mark. With no GDEF class, nearest eligible occurrence is
/// checked against coverage (no speculative backward coverage search).
/// Ligature component provenance, cursive advances, and mark ligature-id
/// compatibility are not handled here. Last successful attachment replaces
/// earlier edge and placement; later adjustments are local residuals.
pub fn execute_gpos_marks(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],mode:GposRunMode,limits:GposMarkRunLimits)->Result<GposMarkRun,Error>{
    execute_gpos_marks_inner(program,glyphs,prior,mode,limits,None)
}
/// Explicit caller-provided source mapping. Ligature identity is occurrence-specific,
/// not a glyph ID. Components are zero-based. Unmapped marks use the last component.
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub enum GposLigatureSource{
    #[default] Unmapped,
    Ligature{id:u32,components:usize},
    Mark{id:u32,component:usize},
}
pub fn select_gpos_ligature_component(parent:GposLigatureSource,mark:GposLigatureSource,count:usize)->Result<usize,Error>{
    if count==0{return Err(Error::Invalid("gpos ligature empty components"));}
    if let GposLigatureSource::Ligature{id,components}=parent{
        if id==0||components!=count{return Err(Error::Invalid("gpos ligature component count"));}
        if let GposLigatureSource::Mark{id:mid,component}=mark{
            if mid==id{
                if component>=count{return Err(Error::Invalid("gpos ligature component index"));}
                return Ok(component);
            }
        }
    }
    Ok(count-1)
}
pub fn gpos_mark_sources_compatible(parent:GposLigatureSource,child:GposLigatureSource)->bool{
    match (parent,child){
        (GposLigatureSource::Unmapped,GposLigatureSource::Unmapped)=>true,
        (GposLigatureSource::Mark{id:a,component:x},GposLigatureSource::Mark{id:b,component:y})=>a==b&&x==y,
        _=>false,
    }
}
/// Validate unique ligature occurrence IDs and all mapped marks before execution.
/// A source can precede or follow its mark in logical order. No source inference.
pub fn execute_gpos_ligature_marks(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],sources:&[GposLigatureSource],mode:GposRunMode,limits:GposMarkRunLimits)->Result<GposMarkRun,Error>{
    validate_ligature_sources(sources,glyphs.len(),limits.context.run.max_glyphs)?;
    execute_gpos_marks_inner(program,glyphs,prior,mode,limits,Some(sources))
}
fn validate_ligature_sources(sources:&[GposLigatureSource],n:usize,max:usize)->Result<(),Error>{
    if sources.len()!=n{return Err(Error::Invalid("gpos ligature source length"));}
    if sources.len()>max{return Err(Error::Limit("gpos ligature sources"));}
    let mut map=std::collections::BTreeMap::new();
    for &source in sources{
        if let GposLigatureSource::Ligature{id,components}=source{
            if id==0||components==0||components>65535||map.insert(id,components).is_some(){return Err(Error::Invalid("gpos ligature identity"));}
        }
    }
    for &source in sources{
        if let GposLigatureSource::Mark{id,component}=source{
            if !map.get(&id).is_some_and(|&n|component<n){return Err(Error::Invalid("gpos ligature mark source"));}
        }
    }
    Ok(())
}
fn execute_gpos_marks_inner(program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],mode:GposRunMode,limits:GposMarkRunLimits,ligatures:Option<&[GposLigatureSource]>)->Result<GposMarkRun,Error>{
    let l=limits.context;
    if glyphs.len()!=prior.len(){return Err(Error::Invalid("gpos run length mismatch"));}
    if glyphs.len()>l.run.max_glyphs||program.roots().len()>l.run.max_roots{return Err(Error::Limit("gpos run size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){return Err(Error::Invalid("gpos run glyph domain"));}
    for lookup in program.lookups(){if !matches!(lookup.effective_kind(),1|2|4|6|7|8)&&!(lookup.effective_kind()==5&&ligatures.is_some()){return Err(Error::Unsupported("gpos mark run kind"));}}
    let mut out=Vec::new();out.try_reserve_exact(prior.len()).map_err(|_|Error::Limit("gpos mark allocation"))?;out.extend_from_slice(prior);
    let mut edges=Vec::new();edges.try_reserve_exact(prior.len()).map_err(|_|Error::Limit("gpos mark allocation"))?;edges.resize(prior.len(),None);
    let mut runner=ContextRunner{program,glyphs,out,mode,limits:l,visits:l.run.max_filter_visits,attempts:l.run.max_subtable_attempts,calls:l.max_calls,actions:l.max_actions,work:l.max_context_work,active:Vec::new(),attachment_edges:Some(edges),parent_visits:limits.max_parent_visits,attachment_updates:limits.max_attachment_updates,ligatures,cursive:None};
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("gpos mark root"))?;
        let (ids,stream)=runner.filtered(lookup)?;
        let mut pos=0;while pos<ids.len(){pos+=runner.at(lookup,&ids,&stream,pos)?;}
    }
    Ok(GposMarkRun{residuals:runner.out,edges:runner.attachment_edges.ok_or(Error::Invalid("gpos mark edges"))?})
}

 
#[derive(Debug,Clone,Copy)]
pub struct GposMixedRunLimits{
    pub marks:GposMarkRunLimits,
    pub attachments:GposAttachmentLimits,
    pub max_connections:usize,
    pub max_span_visits:usize,
}
impl Default for GposMixedRunLimits{
    fn default()->Self{Self{marks:Default::default(),attachments:Default::default(),max_connections:1048576,max_span_visits:16777216}}
}
/// Bounded horizontal mixed execution. All effective GPOS types share one
/// ordered root/nested scheduler, residual state, attachment forest and budgets.
/// Caller-supplied logical order and signed advances use the existing horizontal
/// constraint convention; RightToLeft controls the fixed end, not text direction.
/// Repeated cursive child edges and replacement of a cursive parent are rejected.
/// Mark edges on cursive endpoints are conservatively rejected. This is not full
/// RTL redistribution, chain reversal or OpenType compatibility.
/// DesignOnly ignores pixel origins (returned as zero) and preserves prior pixels.
pub fn execute_gpos_mixed_horizontal(
    program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],
    design_advances:&[(i32,i32)],pixel_advances:&[(i32,i32)],sources:&[GposLigatureSource],
    mode:GposRunMode,limits:GposMixedRunLimits,
)->Result<GposCursiveRun,Error>{
    execute_gpos_mixed_inner(program,glyphs,prior,design_advances,pixel_advances,sources,mode,limits,None)
}

/// Explicit physical horizontal flow. The glyph stream remains in logical
/// order, even in RTL; there is no bidi analysis or reordering here.
/// Both nominal arrays hold physical widths (signed values allowed for
/// corrections). Device pixels are independent, and are ignored in DesignOnly.
/// Cursive changes the leading interval owner for LTR and the trailing owner
/// for RTL. Anchor fixed-end selection is separately controlled by the lookup.
/// The directional path tracks each active cursive width contribution. A new
/// attachment can replace the moving child; exact-pair reversal removes the old
/// edge and contribution. Other overlapping live intervals are rejected.
/// Final forest validation rejects cycles. General OpenType chain reversal,
/// simultaneous overlapping constraints and script shaping remain separate.
pub fn execute_gpos_mixed_directional(
    program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],
    design_advances:&[(i32,i32)],pixel_advances:&[(i32,i32)],sources:&[GposLigatureSource],
    mode:GposRunMode,flow:GposHorizontalFlow,limits:GposMixedRunLimits,
)->Result<GposCursiveRun,Error>{
    execute_gpos_mixed_inner(program,glyphs,prior,design_advances,pixel_advances,sources,mode,limits,Some(flow))
}
fn execute_gpos_mixed_inner(
    program:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],
    design_advances:&[(i32,i32)],pixel_advances:&[(i32,i32)],sources:&[GposLigatureSource],
    mode:GposRunMode,limits:GposMixedRunLimits,flow:Option<GposHorizontalFlow>,
)->Result<GposCursiveRun,Error>{
    use crate::font_gpos_cursive_run::{origins,directional_origins};
    let n=glyphs.len();
    let l=limits.marks.context;
    if prior.len()!=n||design_advances.len()!=n||pixel_advances.len()!=n{return Err(Error::Invalid("mixed gpos length"));}
    if n>l.run.max_glyphs||n>limits.attachments.max_glyphs||program.roots().len()>l.run.max_roots{return Err(Error::Limit("mixed gpos size"));}
    if glyphs.iter().any(|&g|u32::from(g)>=program.glyph_count()){return Err(Error::Invalid("mixed gpos glyph"));}
    validate_ligature_sources(sources,n,l.run.max_glyphs)?;
    for lookup in program.lookups(){if !(1..=8).contains(&lookup.effective_kind()){return Err(Error::Unsupported("mixed gpos kind"));}}
    fn filled<T:Clone>(n:usize,value:T)->Result<Vec<T>,Error>{
        let mut v=Vec::new();v.try_reserve_exact(n).map_err(|_|Error::Limit("mixed gpos allocation"))?;v.resize(n,value);Ok(v)
    }
    let mut out=filled(n,GposRunAdjustment::default())?;
    out.copy_from_slice(prior);
    if matches!(mode,GposRunMode::DesignOnly){for a in &mut out{a.pixels=Default::default();}}
    let mut runner=ContextRunner{program,glyphs,out,mode,limits:l,visits:l.run.max_filter_visits,
        attempts:l.run.max_subtable_attempts,calls:l.max_calls,actions:l.max_actions,work:l.max_context_work,
        active:Vec::new(),attachment_edges:Some(filled(n,None)?),parent_visits:limits.marks.max_parent_visits,
        attachment_updates:limits.marks.max_attachment_updates,ligatures:Some(sources),
        cursive:Some(MixedCursive{design_advances,pixel_advances,children:filled(n,false)?,flow,
            ledger:if flow.is_some(){Some(DirectionalLedger{spans:Default::default(),child_start:filled(n,None)?})}else{None},
            connections:limits.max_connections,spans:limits.max_span_visits})};
    for &root in program.roots(){
        let lookup=program.lookup(root).ok_or(Error::Invalid("mixed gpos root"))?;
        let (ids,stream)=runner.filtered(lookup)?;
        let mut pos=0;while pos<ids.len(){pos+=runner.at(lookup,&ids,&stream,pos)?;}
    }
    let edges=runner.attachment_edges.ok_or(Error::Invalid("mixed gpos edges"))?;
    let pen=|nominal:&[(i32,i32)],pixel:bool|match flow{
        Some(flow)=>directional_origins(nominal,&runner.out,pixel,flow),
        None=>origins(nominal,&runner.out,pixel),
    };
    let design_origins=pen(design_advances,false)?;
    let pixel_origins=if matches!(mode,GposRunMode::DesignOnly){filled(n,(0,0))?}
        else{pen(pixel_advances,true)?};
    let mut adjustments=resolve_gpos_attachments(&edges,&design_origins,&pixel_origins,&runner.out,limits.attachments)?;
    if matches!(mode,GposRunMode::DesignOnly){for (a,p) in adjustments.iter_mut().zip(prior){a.pixels=p.pixels;}}
    Ok(GposCursiveRun{adjustments,edges,design_origins,pixel_origins})
}
