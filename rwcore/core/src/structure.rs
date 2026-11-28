//! C3.3b: atomic structural edits in a contiguous paragraph-only sibling range.
use crate::{Block,CellStep,Document,DocumentSection,Error,Paragraph,Run,RunDrawingKind,RunPoint,SectionEnd,text_byte_offset};
use std::ops::Range;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum SplitAnchors{Left,Right}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum SplitPoint{
    /// Boundary before run i, including the end boundary.
    BetweenRuns(usize),
    InRun{point:RunPoint,anchors:SplitAnchors},
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum JoinFormatting{RequireEqual,KeepLeft,KeepRight}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum StructureOp{
    Split{paragraph:usize,at:SplitPoint},
    JoinNext{paragraph:usize,formatting:JoinFormatting},
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct StructureTransaction{
    pub cells:Vec<CellStep>,
    /// Absolute half-open sibling block range, all existing paragraphs.
    pub range:Range<usize>,
    /// Indices are relative to the staged range after preceding operations.
    pub operations:Vec<StructureOp>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct StructureLimits{
    pub max_operations:usize,pub max_paragraphs:usize,pub max_runs:usize,
    pub max_text_bytes:usize,pub max_drawings:usize,pub max_sibling_blocks:usize,pub max_sections:usize,
}
impl Default for StructureLimits{
    fn default()->Self{Self{max_operations:128,max_paragraphs:4096,max_runs:32768,
        max_text_bytes:32*1024*1024,max_drawings:32768,max_sibling_blocks:200000,max_sections:4096}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct StructureReport{pub operations:usize,pub before_paragraphs:usize,pub after_paragraphs:usize}
pub(crate) fn siblings<'a>(doc:&'a Document,cells:&[CellStep])->Result<&'a Vec<Block>,Error>{
    if cells.len()>16{return Err(Error::Limit("structure table depth"));}
    let mut blocks=&doc.blocks;
    for s in cells{
        let Some(Block::Table(t))=blocks.get(s.block) else{return Err(Error::Invalid("structure table path"));};
        blocks=&t.rows.get(s.row).and_then(|r|r.cells.get(s.cell)).ok_or(Error::Invalid("structure cell path"))?.blocks;
    }
    Ok(blocks)
}
pub(crate) fn siblings_mut<'a>(doc:&'a mut Document,cells:&[CellStep])->Option<&'a mut Vec<Block>>{
    let mut blocks=&mut doc.blocks;
    for s in cells{
        let Block::Table(t)=blocks.get_mut(s.block)? else{return None;};
        blocks=&mut t.rows.get_mut(s.row)?.cells.get_mut(s.cell)?.blocks;
    }
    Some(blocks)
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("structure arithmetic"))}
fn validate_sections(doc:&Document,limits:StructureLimits)->Result<(),Error>{
    if doc.sections.len()>limits.max_sections{return Err(Error::Limit("structure sections"));}
    if doc.sections.is_empty(){return Ok(());}
    let mut cursor=0;
    for (i,s) in doc.sections.iter().enumerate(){
        if s.blocks.start!=cursor||s.blocks.end<cursor||s.blocks.end>doc.blocks.len(){
            return Err(Error::Invalid("structure section ranges"));
        }
        if s.end==SectionEnd::Paragraph{
            if s.blocks.is_empty()||!matches!(doc.blocks.get(s.blocks.end-1),Some(Block::Paragraph(_))){
                return Err(Error::Invalid("structure section terminal paragraph"));
            }
        }else if i+1!=doc.sections.len(){return Err(Error::Invalid("structure nonfinal body section"));}
        cursor=s.blocks.end;
    }
    if cursor!=doc.blocks.len(){return Err(Error::Invalid("structure section coverage"));}
    Ok(())
}
fn section_owner(doc:&Document,t:&StructureTransaction)->Result<Option<usize>,Error>{
    if !t.cells.is_empty()||doc.sections.is_empty(){return Ok(None);}
    doc.sections.iter().position(|s|s.blocks.start<=t.range.start&&s.blocks.end>=t.range.end)
        .map(Some).ok_or(Error::Unsupported("structure crosses section boundary"))
}
fn shift(v:usize,before:usize,after:usize)->Result<usize,Error>{
    if after>=before{add(v,after-before)}else{v.checked_sub(before-after).ok_or(Error::Invalid("structure section shift"))}
}
fn adjusted_sections(doc:&Document,owner:Option<usize>,before:usize,after:usize)->Result<Option<Vec<DocumentSection>>,Error>{
    let Some(owner)=owner else{return Ok(None);};
    let mut sections=doc.sections.clone();
    sections[owner].blocks.end=shift(sections[owner].blocks.end,before,after)?;
    for s in &mut sections[owner+1..]{
        s.blocks.start=shift(s.blocks.start,before,after)?;
        s.blocks.end=shift(s.blocks.end,before,after)?;
    }
    Ok(Some(sections))
}
fn check_paragraphs<'a>(paragraphs:impl Iterator<Item=&'a Paragraph>,l:StructureLimits)->Result<(),Error>{
    let(mut n,mut runs,mut bytes,mut drawings)=(0,0,0,0);
    for p in paragraphs{
        n=add(n,1)?;runs=add(runs,p.runs.len())?;
        if n>l.max_paragraphs||runs>l.max_runs{return Err(Error::Limit("structure paragraphs or runs"));}
        for r in &p.runs{
            bytes=add(bytes,r.text.len())?;
            drawings=add(drawings,add(r.pictures.len(),r.drawing_order.len())?)?;
            if bytes>l.max_text_bytes||drawings>l.max_drawings{return Err(Error::Limit("structure text or drawing budget"));}
            r.drawings()?;
        }
    }
    Ok(())
}
fn split_run(r:&mut Run,byte:usize,side:SplitAnchors)->Result<Run,Error>{
    let left=|offset:usize|offset<byte||(offset==byte&&side==SplitAnchors::Left);
    // Validated ordered indices: inline prefix and mixed prefix agree.
    let inline=r.pictures.iter().take_while(|p|left(p.text_offset)).count();
    let mixed=r.drawing_order.iter().take_while(|p|left(p.text_offset)).count();
    let mut right=Run{text:r.text.split_off(byte),properties:r.properties.clone(),
        pictures:r.pictures.split_off(inline),drawing_order:r.drawing_order.split_off(mixed)};
    for p in &mut right.pictures{p.text_offset-=byte;}
    for p in &mut right.drawing_order{
        p.text_offset-=byte;
        if let RunDrawingKind::Inline(i)=&mut p.kind{*i-=inline;}
    }
    r.drawings()?;right.drawings()?;
    Ok(right)
}
fn split(p:&mut Paragraph,point:SplitPoint)->Result<Paragraph,Error>{
    let runs=match point{
        SplitPoint::BetweenRuns(i)=>{
            if i>p.runs.len(){return Err(Error::Invalid("structure run boundary"));}
            p.runs.split_off(i)
        },
        SplitPoint::InRun{point,anchors}=>{
            let r=p.runs.get_mut(point.run).ok_or(Error::Invalid("structure run index"))?;
            let byte=text_byte_offset(&r.text,point.offset,point.unit)?;
            let right=split_run(r,byte,anchors)?;
            let mut rest=p.runs.split_off(point.run+1);
            rest.insert(0,right);rest
        }
    };
    Ok(Paragraph{runs,properties:p.properties.clone()})
}
/// Atomic paragraph splits and adjacent joins in a single existing sibling
/// range. No table/section crossing, no implicit image deletion or run merge.
/// Split copies paragraph formatting; a terminating section stays on the
/// right paragraph via the updated section range. Join policy is explicit.
/// The staged range is the only cloned content, plus section metadata.
/// Limits count logical items/text, not exact heap bytes. Structural edits
/// invalidate index-based paths; use DocumentEditor::structure for history.
pub fn apply_structure_transaction(doc:&mut Document,t:&StructureTransaction,l:StructureLimits)->Result<StructureReport,Error>{
    let plan=stage_structure_transaction(doc,t,l)?;
    let report=plan.report;
    commit_structure(doc,plan);
    Ok(report)
}
pub(crate) struct StructurePlan{
    pub(crate) cells:Vec<CellStep>,
    pub(crate) range:Range<usize>,
    pub(crate) blocks:Vec<Block>,
    pub(crate) sections:Option<Vec<DocumentSection>>,
    pub(crate) report:StructureReport,
}
pub(crate) fn stage_structure_transaction(doc:&Document,t:&StructureTransaction,l:StructureLimits)->Result<StructurePlan,Error>{
    if t.operations.len()>l.max_operations{return Err(Error::Limit("structure operations"));}
    validate_sections(doc,l)?;
    let blocks=siblings(doc,&t.cells)?;
    if blocks.len()>l.max_sibling_blocks{return Err(Error::Limit("structure sibling blocks"));}
    if t.range.is_empty()||t.range.end>blocks.len(){return Err(Error::Invalid("structure paragraph range"));}
    let before=t.range.len();
    let owner=section_owner(doc,t)?;
    let mut borrowed=Vec::new();
    if before>l.max_paragraphs{return Err(Error::Limit("structure paragraphs"));}
    for b in &blocks[t.range.clone()]{
        let Block::Paragraph(p)=b else{return Err(Error::Unsupported("structure range includes nonparagraph"));};
        borrowed.push(p);
    }
    check_paragraphs(borrowed.iter().copied(),l)?;
    let mut staged:Vec<Paragraph>=borrowed.into_iter().cloned().collect();
    for op in &t.operations{
        match *op{
            StructureOp::Split{paragraph,at}=>{
                if staged.len()>=l.max_paragraphs{return Err(Error::Limit("structure paragraphs"));}
                if matches!(at,SplitPoint::InRun{..}){
                    let n=staged.iter().try_fold(0usize,|n,p|add(n,p.runs.len()))?;
                    if n>=l.max_runs{return Err(Error::Limit("structure runs"));}
                }
                let p=staged.get_mut(paragraph).ok_or(Error::Invalid("structure paragraph index"))?;
                let right=split(p,at)?;
                staged.insert(paragraph+1,right);
            },
            StructureOp::JoinNext{paragraph,formatting}=>{
                let next=add(paragraph,1)?;
                if next>=staged.len(){return Err(Error::Invalid("structure join index"));}
                if formatting==JoinFormatting::RequireEqual&&staged[paragraph].properties!=staged[next].properties{
                    return Err(Error::Invalid("structure paragraph formatting mismatch"));
                }
                let mut right=staged.remove(next);
                let left=&mut staged[paragraph];
                if formatting==JoinFormatting::KeepRight{left.properties=right.properties;}
                left.runs.append(&mut right.runs);
            }
        }
        check_paragraphs(staged.iter(),l)?;
        let total=add(blocks.len()-before,staged.len())?;
        if total>l.max_sibling_blocks{return Err(Error::Limit("structure sibling blocks"));}
    }
    let after=staged.len();
    let sections=adjusted_sections(doc,owner,before,after)?;
    Ok(StructurePlan{cells:t.cells.clone(),range:t.range.clone(),blocks:staged.into_iter().map(Block::Paragraph).collect(),
        sections,report:StructureReport{operations:t.operations.len(),before_paragraphs:before,after_paragraphs:after}})
}
pub(crate) fn commit_structure(doc:&mut Document,plan:StructurePlan){
    // The path was validated against the unchanged tree.
    siblings_mut(doc,&plan.cells).expect("validated structure path")
        .splice(plan.range,plan.blocks);
    if let Some(s)=plan.sections{doc.sections=s;}
}
