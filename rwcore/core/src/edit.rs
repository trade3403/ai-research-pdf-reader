//! C3.1: atomic body-run text transactions; no structural edits or history.
use crate::{Block,Document,Error,Run};
use std::collections::BTreeMap;
const MAX_DEPTH:usize=16;

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct CellStep{pub block:usize,pub row:usize,pub cell:usize}
/// Stable for text-only transactions, not across structural changes.
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord)]
pub struct RunPath{pub cells:Vec<CellStep>,pub paragraph:usize,pub run:usize}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TextUnit{Utf8,UnicodeScalar,Utf16}
/// Convert a boundary from any supported unit to a UTF-8 byte offset.
/// No grapheme segmentation; UTF-16 offsets inside surrogate pairs fail.
pub fn text_byte_offset(text:&str,offset:usize,unit:TextUnit)->Result<usize,Error>{
    if unit==TextUnit::Utf8{
        return if text.is_char_boundary(offset){Ok(offset)}else{Err(Error::Invalid("edit UTF-8 boundary"))};
    }
    let mut n=0usize;
    for (byte,ch) in text.char_indices(){
        if n==offset{return Ok(byte);}
        n+=if unit==TextUnit::Utf16{ch.len_utf16()}else{1};
        if n>offset{return Err(Error::Invalid("edit text boundary"));}
    }
    if n==offset{Ok(text.len())}else{Err(Error::Invalid("edit text offset"))}
}
pub fn text_offset_from_byte(text:&str,byte:usize,unit:TextUnit)->Result<usize,Error>{
    let b=text_byte_offset(text,byte,TextUnit::Utf8)?;
    Ok(match unit{TextUnit::Utf8=>b,TextUnit::UnicodeScalar=>text[..b].chars().count(),
        TextUnit::Utf16=>text[..b].encode_utf16().count()})
}
impl RunPath{
    pub fn get<'a>(&self,doc:&'a Document)->Result<&'a Run,Error>{
        if self.cells.len()>MAX_DEPTH{return Err(Error::Limit("edit table depth"));}
        let mut blocks=&doc.blocks;
        for s in &self.cells{
            let Some(Block::Table(t))=blocks.get(s.block) else{return Err(Error::Invalid("edit table path"));};
            blocks=&t.rows.get(s.row).and_then(|r|r.cells.get(s.cell))
                .ok_or(Error::Invalid("edit cell path"))?.blocks;
        }
        let Some(Block::Paragraph(p))=blocks.get(self.paragraph) else{return Err(Error::Invalid("edit paragraph path"));};
        p.runs.get(self.run).ok_or(Error::Invalid("edit run path"))
    }
    pub(crate) fn get_mut<'a>(&self,doc:&'a mut Document)->Option<&'a mut Run>{
        let mut blocks=&mut doc.blocks;
        for s in &self.cells{
            let Block::Table(t)=blocks.get_mut(s.block)? else{return None;};
            blocks=&mut t.rows.get_mut(s.row)?.cells.get_mut(s.cell)?.blocks;
        }
        let Block::Paragraph(p)=blocks.get_mut(self.paragraph)? else{return None;};
        p.runs.get_mut(self.run)
    }
}
/// Insertion at an anchor: Before moves the anchor right; After leaves it.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InsertAffinity{Before,After}
/// For a nonempty replaced range, anchors strictly inside the range are
/// rejected or collapsed to the start, ahead of the replacement text.
/// Boundary anchors are preserved: start stays, end follows replacement.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InteriorAnchorPolicy{Reject,PreserveAtStart}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TextEdit{
    pub path:RunPath,pub start:usize,pub end:usize,pub unit:TextUnit,
    /// Exact expected removed text, checked against the staged run.
    pub expected:String,pub replacement:String,
    pub insertion:InsertAffinity,pub interior:InteriorAnchorPolicy,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct EditLimits{
    pub max_operations:usize,pub max_run_bytes:usize,pub max_staged_bytes:usize,
    pub max_drawings_per_run:usize,pub max_input_bytes:usize,
}
impl Default for EditLimits{
    fn default()->Self{Self{max_operations:1024,max_run_bytes:8*1024*1024,
        max_staged_bytes:32*1024*1024,max_drawings_per_run:16384,max_input_bytes:16*1024*1024}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct EditReport{pub operations:usize,pub touched_runs:usize}
fn check_run(run:&Run,limits:EditLimits)->Result<(),Error>{
    if run.text.len()>limits.max_run_bytes{return Err(Error::Limit("edit run bytes"));}
    if run.pictures.len()>limits.max_drawings_per_run||run.drawing_order.len()>limits.max_drawings_per_run{
        return Err(Error::Limit("edit drawing count"));
    }
    run.drawings()?;Ok(())
}
fn xml_text(s:&str)->bool{
    s.chars().all(|c|matches!(c,'\t'|'\n'|'\r')||((c as u32)>=0x20&&!matches!(c,'\u{fffe}'|'\u{ffff}')))
}
fn remap(p:usize,a:usize,b:usize,n:usize,aff:InsertAffinity,policy:InteriorAnchorPolicy)->Result<usize,Error>{
    if a==b{
        if p>a||(p==a&&aff==InsertAffinity::Before){return p.checked_add(n).ok_or(Error::Limit("edit anchor arithmetic"));}
        return Ok(p);
    }
    if p<=a{return Ok(p);}
    if p<b{
        return match policy{InteriorAnchorPolicy::Reject=>Err(Error::Invalid("edit crosses picture anchor")),
            InteriorAnchorPolicy::PreserveAtStart=>Ok(a)};
    }
    (p-b).checked_add(a).and_then(|v|v.checked_add(n)).ok_or(Error::Limit("edit anchor arithmetic"))
}
fn edit_run(run:&mut Run,edit:&TextEdit,limits:EditLimits)->Result<(),Error>{
    let a=text_byte_offset(&run.text,edit.start,edit.unit)?;
    let b=text_byte_offset(&run.text,edit.end,edit.unit)?;
    if a>b{return Err(Error::Invalid("reversed edit range"));}
    if run.text[a..b]!=edit.expected{return Err(Error::Invalid("stale edit text"));}
    if !xml_text(&edit.replacement){return Err(Error::Invalid("invalid XML edit character"));}
    let n=edit.replacement.len();
    let len=(run.text.len()-(b-a)).checked_add(n).ok_or(Error::Limit("edit run bytes"))?;
    if len>limits.max_run_bytes{return Err(Error::Limit("edit run bytes"));}
    if edit.expected==edit.replacement{return Ok(());}
    for p in &mut run.pictures{
        p.text_offset=remap(p.text_offset,a,b,n,edit.insertion,edit.interior)?;
    }
    for p in &mut run.drawing_order{
        p.text_offset=remap(p.text_offset,a,b,n,edit.insertion,edit.interior)?;
    }
    run.text.replace_range(a..b,&edit.replacement);
    check_run(run,limits)
}
/// Atomic multi-run text edits. Coordinates and expected strings are relative
/// to the result of preceding operations. Only touched runs are cloned; no
/// whole-document clone. The byte budgets count text, not metadata heap size.
/// Body and nested body tables only: stories/fields, structure, image deletion,
/// undo history, concurrency versions and grapheme semantics are later work.
pub fn apply_text_transaction(doc:&mut Document,edits:&[TextEdit],limits:EditLimits)->Result<EditReport,Error>{
    let staged=stage_text_transaction(doc,edits,limits)?;
    let touched_runs=staged.len();
    commit_staged(doc,staged);
    Ok(EditReport{operations:edits.len(),touched_runs})
}
pub(crate) fn stage_text_transaction(doc:&Document,edits:&[TextEdit],limits:EditLimits)->Result<BTreeMap<RunPath,Run>,Error>{
    if edits.len()>limits.max_operations{return Err(Error::Limit("edit operations"));}
    let mut input=0usize;
    let mut staged_bytes=0usize;
    let mut staged:BTreeMap<RunPath,Run>=BTreeMap::new();
    for e in edits{
        input=input.checked_add(e.expected.len()).and_then(|v|v.checked_add(e.replacement.len()))
            .ok_or(Error::Limit("edit input bytes"))?;
        if input>limits.max_input_bytes{return Err(Error::Limit("edit input bytes"));}
        if !staged.contains_key(&e.path){
            let r=e.path.get(doc)?;
            check_run(r,limits)?;
            staged_bytes=staged_bytes.checked_add(r.text.len()).ok_or(Error::Limit("edit staged bytes"))?;
            if staged_bytes>limits.max_staged_bytes{return Err(Error::Limit("edit staged bytes"));}
            staged.insert(e.path.clone(),r.clone());
        }
        let r=staged.get_mut(&e.path).expect("staged run");
        let previous=r.text.len();
        // Check aggregate budget before allocating replacement text.
        let a=text_byte_offset(&r.text,e.start,e.unit)?;
        let b=text_byte_offset(&r.text,e.end,e.unit)?;
        if a>b{return Err(Error::Invalid("reversed edit range"));}
        let next=(previous-(b-a)).checked_add(e.replacement.len()).ok_or(Error::Limit("edit staged bytes"))?;
        let total=(staged_bytes-previous).checked_add(next).ok_or(Error::Limit("edit staged bytes"))?;
        if total>limits.max_staged_bytes{return Err(Error::Limit("edit staged bytes"));}
        edit_run(r,e,limits)?;
        staged_bytes=total;
    }
    Ok(staged)
}
pub(crate) fn commit_staged(doc:&mut Document,staged:BTreeMap<RunPath,Run>){
    // All paths were resolved against the same unchanged tree.
    for (path,r) in staged{
        *path.get_mut(doc).expect("validated text-only path")=r;
    }
}
