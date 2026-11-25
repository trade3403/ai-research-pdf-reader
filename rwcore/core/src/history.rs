//! C3.3c: bounded in-memory text and structural undo/redo with private ownership.
use crate::{Document,Run,RunPath,Error,TextEdit,EditLimits,EditReport,SelectionEdit,SelectionLimits};
use crate::edit::{stage_text_transaction,commit_staged};
use crate::selection::plan_selection;
use std::collections::VecDeque;
use crate::{Block,CellStep,DocumentSection,Paragraph,ParagraphPath,StructureLimits,StructureTransaction,StructureReport};
use crate::structure::{stage_structure_transaction,commit_structure,siblings,siblings_mut};
use std::ops::Range;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryLimits{
    pub max_entries:usize,
    /// Combined before/after retained text bytes, not allocator capacity.
    pub max_text_bytes:usize,
    pub max_anchor_offsets:usize,
    /// Text run changes plus structural paragraph/run/section snapshot slots.
    pub max_run_changes:usize,
}
impl Default for HistoryLimits{
    fn default()->Self{Self{max_entries:128,max_text_bytes:32*1024*1024,
        max_anchor_offsets:262144,max_run_changes:16384}}
}
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct HistoryUsage{pub text_bytes:usize,pub anchor_offsets:usize,pub run_changes:usize}
impl HistoryUsage{
    fn add(self,b:Self)->Result<Self,Error>{
        let add=|a:usize,b:usize|a.checked_add(b).ok_or(Error::Limit("history usage arithmetic"));
        Ok(Self{text_bytes:add(self.text_bytes,b.text_bytes)?,
            anchor_offsets:add(self.anchor_offsets,b.anchor_offsets)?,run_changes:add(self.run_changes,b.run_changes)?})
    }
    fn sub(self,b:Self)->Self{
        Self{text_bytes:self.text_bytes-b.text_bytes,anchor_offsets:self.anchor_offsets-b.anchor_offsets,
            run_changes:self.run_changes-b.run_changes}
    }
    fn fits(self,l:HistoryLimits)->bool{
        self.text_bytes<=l.max_text_bytes&&self.anchor_offsets<=l.max_anchor_offsets&&self.run_changes<=l.max_run_changes
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
struct RunState{text:String,inline:Vec<usize>,mixed:Vec<usize>}
impl RunState{
    fn capture(r:&Run)->Self{
        Self{text:r.text.clone(),inline:r.pictures.iter().map(|p|p.text_offset).collect(),
            mixed:r.drawing_order.iter().map(|p|p.text_offset).collect()}
    }
    fn matches(&self,r:&Run)->bool{
        self.text==r.text&&self.inline.iter().copied().eq(r.pictures.iter().map(|p|p.text_offset))
            &&self.mixed.iter().copied().eq(r.drawing_order.iter().map(|p|p.text_offset))
    }
    fn install(&self,r:&mut Run){
        r.text.clone_from(&self.text);
        for (p,o) in r.pictures.iter_mut().zip(&self.inline){p.text_offset=*o;}
        for (p,o) in r.drawing_order.iter_mut().zip(&self.mixed){p.text_offset=*o;}
    }
}
#[derive(Debug)]
struct Change{path:RunPath,before:RunState,after:RunState}
#[derive(Debug)]
struct StructureState{blocks:Vec<Block>,sections:Option<Vec<DocumentSection>>}
#[derive(Debug)]
struct StructureChange{cells:Vec<CellStep>,start:usize,before:StructureState,after:StructureState}
impl StructureChange{
    fn states(&self,undo:bool)->(&StructureState,&StructureState){
        if undo{(&self.after,&self.before)}else{(&self.before,&self.after)}
    }
    fn range(&self,state:&StructureState)->Result<Range<usize>,Error>{
        Ok(self.start..self.start.checked_add(state.blocks.len()).ok_or(Error::Limit("history structure range"))?)
    }
    fn validate(&self,doc:&Document,undo:bool)->Result<(),Error>{
        let (current,_)=self.states(undo);
        let blocks=siblings(doc,&self.cells)?;
        if blocks.get(self.range(current)?)!=Some(current.blocks.as_slice()){
            return Err(Error::Invalid("history structure conflict"));
        }
        if let Some(s)=&current.sections{
            if &doc.sections!=s{return Err(Error::Invalid("history section conflict"));}
        }
        Ok(())
    }
    fn restore(&self,doc:&mut Document,undo:bool){
        let (current,target)=self.states(undo);
        siblings_mut(doc,&self.cells).expect("validated history sibling path")
            .splice(self.range(current).expect("validated history range"),target.blocks.iter().cloned());
        if let Some(s)=&target.sections{doc.sections.clone_from(s);}
    }
    fn changed_runs(&self)->usize{
        let count=|s:&StructureState|s.blocks.iter().map(|b|match b{Block::Paragraph(p)=>p.runs.len(),_=>0}).sum::<usize>();
        count(&self.before).max(count(&self.after))
    }
}
fn structure_usage(blocks:&[Block],sections:usize)->Result<HistoryUsage,Error>{
    let mut usage=HistoryUsage{run_changes:sections,..Default::default()};
    for b in blocks{
        let Block::Paragraph(p)=b else{return Err(Error::Invalid("history nonparagraph"));};
        usage=usage.add(HistoryUsage{run_changes:1,..Default::default()})?;
        for r in &p.runs{
            let offsets=r.pictures.len().checked_add(r.drawing_order.len()).ok_or(Error::Limit("history anchor offsets"))?;
            usage=usage.add(HistoryUsage{text_bytes:r.text.len(),anchor_offsets:offsets,run_changes:1})?;
        }
    }
    Ok(usage)
}
#[derive(Debug)]
struct Entry{changes:Vec<Change>,structure:Option<StructureChange>,usage:HistoryUsage}
impl Entry{
    fn changed_runs(&self)->usize{
        self.structure.as_ref().map_or(self.changes.len(),StructureChange::changed_runs)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryReport{pub revision:u64,pub changed_runs:usize,pub evicted_entries:usize}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryEditReport{pub edit:EditReport,pub history:HistoryReport}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct StructureHistoryReport{pub structure:StructureReport,pub history:HistoryReport}
/// Owns a document and exposes only shared access. Structural mutations
/// are recorded in the same LIFO history; old paths require a fresh revision.
/// Each successful non-noop action advances a session-local revision.
/// No persistent journal, crash recovery, grouping or cross-session identity.
#[derive(Debug)]
pub struct DocumentEditor{
    document:Document,limits:HistoryLimits,revision:u64,
    undo:VecDeque<Entry>,redo:Vec<Entry>,usage:HistoryUsage,
}
impl DocumentEditor{
    pub fn new(document:Document,limits:HistoryLimits)->Self{
        Self{document,limits,revision:0,undo:VecDeque::new(),redo:Vec::new(),usage:HistoryUsage::default()}
    }
    pub fn document(&self)->&Document{&self.document}
    pub fn into_document(self)->Document{self.document}
    pub fn revision(&self)->u64{self.revision}
    pub fn undo_len(&self)->usize{self.undo.len()}
    pub fn redo_len(&self)->usize{self.redo.len()}
    pub fn usage(&self)->HistoryUsage{self.usage}
    fn check_revision(&self,expected:u64)->Result<(),Error>{
        if expected!=self.revision{return Err(Error::Invalid("stale editor revision"));}
        Ok(())
    }
    fn next_revision(&self)->Result<u64,Error>{
        self.revision.checked_add(1).ok_or(Error::Limit("editor revision"))
    }
    /// Resolve an index path only against the revision that produced it.
    /// A new revision requires callers to obtain fresh paths; no auto-remap.
    pub fn run_at(&self,expected_revision:u64,path:&RunPath)->Result<&Run,Error>{
        self.check_revision(expected_revision)?;path.get(&self.document)
    }
    pub fn paragraph_at(&self,expected_revision:u64,path:&ParagraphPath)->Result<&Paragraph,Error>{
        self.check_revision(expected_revision)?;path.get(&self.document)
    }
    fn plan_retention(&self,entry:HistoryUsage)->Result<(usize,HistoryUsage),Error>{
        if self.limits.max_entries==0||!entry.fits(self.limits){return Err(Error::Limit("history entry budget"));}
        let mut retained=self.usage;
        for e in &self.redo{retained=retained.sub(e.usage);}
        let mut evict=0usize;
        loop{
            if self.undo.len()-evict<self.limits.max_entries{
                if let Ok(sum)=retained.add(entry){
                    if sum.fits(self.limits){return Ok((evict,sum));}
                }
            }
            let e=self.undo.get(evict).ok_or(Error::Limit("history retention budget"))?;
            retained=retained.sub(e.usage);evict+=1;
        }
    }
    fn record(&mut self,entry:Entry,revision:u64,evict:usize,usage:HistoryUsage){
        self.redo.clear();
        for _ in 0..evict{self.undo.pop_front();}
        self.undo.push_back(entry);
        self.usage=usage;self.revision=revision;
    }
    pub fn apply(&mut self,expected_revision:u64,edits:&[TextEdit],limits:EditLimits)->Result<HistoryEditReport,Error>{
        self.check_revision(expected_revision)?;
        let staged=stage_text_transaction(&self.document,edits,limits)?;
        let edit=EditReport{operations:edits.len(),touched_runs:staged.len()};
        let mut entry=Entry{changes:Vec::new(),structure:None,usage:HistoryUsage::default()};
        for (path,after) in &staged{
            let before=path.get(&self.document)?;
            if before==after{continue;}
            let offsets=before.pictures.len().checked_add(before.drawing_order.len())
                .and_then(|n|n.checked_add(after.pictures.len())).and_then(|n|n.checked_add(after.drawing_order.len()))
                .ok_or(Error::Limit("history anchor offsets"))?;
            entry.usage=entry.usage.add(HistoryUsage{text_bytes:before.text.len().checked_add(after.text.len())
                .ok_or(Error::Limit("history text bytes"))?,anchor_offsets:offsets,run_changes:1})?;
            if self.limits.max_entries==0||!entry.usage.fits(self.limits){
                return Err(Error::Limit("history entry budget"));
            }
            entry.changes.push(Change{path:path.clone(),before:RunState::capture(before),after:RunState::capture(after)});
        }
        let changed_runs=entry.changes.len();
        if changed_runs==0{return Ok(HistoryEditReport{edit,history:HistoryReport{revision:self.revision,changed_runs:0,evicted_entries:0}});}
        let revision=self.next_revision()?;
        let (evict,usage)=self.plan_retention(entry.usage)?;
        commit_staged(&mut self.document,staged);
        self.record(entry,revision,evict,usage);
        Ok(HistoryEditReport{edit,history:HistoryReport{revision,changed_runs,evicted_entries:evict}})
    }
    pub fn replace(&mut self,expected_revision:u64,edit:&SelectionEdit,limits:SelectionLimits)->Result<HistoryEditReport,Error>{
        self.check_revision(expected_revision)?;
        let ops=plan_selection(&self.document,edit,limits)?;
        self.apply(expected_revision,&ops,limits.edit)
    }
    /// Structural operations and text operations share one history order.
    /// Capture only the edited paragraph slice and affected section metadata,
    /// not other document blocks or media bytes.
    pub fn structure(&mut self,expected_revision:u64,transaction:&StructureTransaction,limits:StructureLimits)->Result<StructureHistoryReport,Error>{
        self.check_revision(expected_revision)?;
        let plan=stage_structure_transaction(&self.document,transaction,limits)?;
        let report=plan.report;
        let before=&siblings(&self.document,&plan.cells)?[plan.range.clone()];
        let section_changed=plan.sections.as_ref().is_some_and(|s|s!=&self.document.sections);
        if before==plan.blocks.as_slice()&&!section_changed{
            return Ok(StructureHistoryReport{structure:report,history:HistoryReport{revision:self.revision,changed_runs:0,evicted_entries:0}});
        }
        // Both snapshots are accounted before cloning for history.
        let old_sections=if section_changed{self.document.sections.len()}else{0};
        let new_sections=if section_changed{plan.sections.as_ref().map_or(0,Vec::len)}else{0};
        let charge=structure_usage(before,old_sections)?.add(structure_usage(&plan.blocks,new_sections)?)?;
        let revision=self.next_revision()?;
        let (evict,usage)=self.plan_retention(charge)?;
        let change=StructureChange{cells:plan.cells.clone(),start:plan.range.start,
            before:StructureState{blocks:before.to_vec(),sections:if section_changed{Some(self.document.sections.clone())}else{None}},
            after:StructureState{blocks:plan.blocks.clone(),sections:if section_changed{plan.sections.clone()}else{None}}};
        let changed_runs=change.changed_runs();
        let entry=Entry{changes:Vec::new(),structure:Some(change),usage:charge};
        commit_structure(&mut self.document,plan);
        self.record(entry,revision,evict,usage);
        Ok(StructureHistoryReport{structure:report,history:HistoryReport{revision,changed_runs,evicted_entries:evict}})
    }
    fn validate_restore(doc:&Document,e:&Entry,undo:bool)->Result<(),Error>{
        if let Some(s)=&e.structure{return s.validate(doc,undo);}
        for c in &e.changes{
            let current=if undo{&c.after}else{&c.before};
            if !current.matches(c.path.get(doc)?){return Err(Error::Invalid("history state conflict"));}
        }
        Ok(())
    }
    fn restore(doc:&mut Document,e:&Entry,undo:bool){
        if let Some(s)=&e.structure{s.restore(doc,undo);return;}
        for c in &e.changes{
            let target=if undo{&c.before}else{&c.after};
            target.install(c.path.get_mut(doc).expect("validated history path"));
        }
    }
    pub fn undo(&mut self,expected_revision:u64)->Result<Option<HistoryReport>,Error>{
        self.check_revision(expected_revision)?;
        let Some(e)=self.undo.back() else{return Ok(None);};
        Self::validate_restore(&self.document,e,true)?;
        let revision=self.next_revision()?;
        let e=self.undo.pop_back().expect("validated undo");
        Self::restore(&mut self.document,&e,true);
        let changed_runs=e.changed_runs();self.redo.push(e);self.revision=revision;
        Ok(Some(HistoryReport{revision,changed_runs,evicted_entries:0}))
    }
    pub fn redo(&mut self,expected_revision:u64)->Result<Option<HistoryReport>,Error>{
        self.check_revision(expected_revision)?;
        let Some(e)=self.redo.last() else{return Ok(None);};
        Self::validate_restore(&self.document,e,false)?;
        let revision=self.next_revision()?;
        let e=self.redo.pop().expect("validated redo");
        Self::restore(&mut self.document,&e,false);
        let changed_runs=e.changed_runs();self.undo.push_back(e);self.revision=revision;
        Ok(Some(HistoryReport{revision,changed_runs,evicted_entries:0}))
    }
}
