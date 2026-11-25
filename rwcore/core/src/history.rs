//! C3.3a: bounded, in-memory, text-only undo/redo with private document ownership.
use crate::{Document,Run,RunPath,Error,TextEdit,EditLimits,EditReport,SelectionEdit,SelectionLimits};
use crate::edit::{stage_text_transaction,commit_staged};
use crate::selection::plan_selection;
use std::collections::VecDeque;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryLimits{
    pub max_entries:usize,
    /// Combined before/after retained text bytes, not allocator capacity.
    pub max_text_bytes:usize,
    pub max_anchor_offsets:usize,
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
struct Entry{changes:Vec<Change>,usage:HistoryUsage}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryReport{pub revision:u64,pub changed_runs:usize,pub evicted_entries:usize}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HistoryEditReport{pub edit:EditReport,pub history:HistoryReport}
/// Owns a document and exposes only shared access. This prevents structural
/// mutations and history reuse on another document through this API.
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
    pub fn apply(&mut self,expected_revision:u64,edits:&[TextEdit],limits:EditLimits)->Result<HistoryEditReport,Error>{
        self.check_revision(expected_revision)?;
        let staged=stage_text_transaction(&self.document,edits,limits)?;
        let edit=EditReport{operations:edits.len(),touched_runs:staged.len()};
        let mut entry=Entry{changes:Vec::new(),usage:HistoryUsage::default()};
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
        // Plan eviction before mutating document or history. Redo is discarded
        // only on a successful new change; a rejected edit preserves both stacks.
        let mut retained=self.usage;
        for e in &self.redo{retained=retained.sub(e.usage);}
        let mut evict=0usize;
        let usage=loop{
            if self.undo.len()-evict<self.limits.max_entries{
                if let Ok(sum)=retained.add(entry.usage){
                    if sum.fits(self.limits){break sum;}
                }
            }
            let e=self.undo.get(evict).ok_or(Error::Limit("history retention budget"))?;
            retained=retained.sub(e.usage);evict+=1;
        };
        commit_staged(&mut self.document,staged);
        self.redo.clear();
        for _ in 0..evict{self.undo.pop_front();}
        self.undo.push_back(entry);
        self.usage=usage;self.revision=revision;
        Ok(HistoryEditReport{edit,history:HistoryReport{revision,changed_runs,evicted_entries:evict}})
    }
    pub fn replace(&mut self,expected_revision:u64,edit:&SelectionEdit,limits:SelectionLimits)->Result<HistoryEditReport,Error>{
        self.check_revision(expected_revision)?;
        let ops=plan_selection(&self.document,edit,limits)?;
        self.apply(expected_revision,&ops,limits.edit)
    }
    fn validate_restore(doc:&Document,e:&Entry,undo:bool)->Result<(),Error>{
        for c in &e.changes{
            let current=if undo{&c.after}else{&c.before};
            if !current.matches(c.path.get(doc)?){return Err(Error::Invalid("history state conflict"));}
        }
        Ok(())
    }
    fn restore(doc:&mut Document,e:&Entry,undo:bool){
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
        let changed_runs=e.changes.len();self.redo.push(e);self.revision=revision;
        Ok(Some(HistoryReport{revision,changed_runs,evicted_entries:0}))
    }
    pub fn redo(&mut self,expected_revision:u64)->Result<Option<HistoryReport>,Error>{
        self.check_revision(expected_revision)?;
        let Some(e)=self.redo.last() else{return Ok(None);};
        Self::validate_restore(&self.document,e,false)?;
        let revision=self.next_revision()?;
        let e=self.redo.pop().expect("validated redo");
        Self::restore(&mut self.document,&e,false);
        let changed_runs=e.changes.len();self.undo.push_back(e);self.revision=revision;
        Ok(Some(HistoryReport{revision,changed_runs,evicted_entries:0}))
    }
}
