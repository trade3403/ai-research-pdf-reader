//! C3.2: paragraph text index, directional selections and atomic cross-run edits.
use crate::{Block,CellStep,Document,EditLimits,EditReport,Error,InsertAffinity,
    InteriorAnchorPolicy,Paragraph,RunPath,TextEdit,TextUnit,apply_text_transaction,
    text_byte_offset,text_offset_from_byte};

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ParagraphPath{pub cells:Vec<CellStep>,pub paragraph:usize}
impl ParagraphPath{
    pub fn get<'a>(&self,doc:&'a Document)->Result<&'a Paragraph,Error>{
        if self.cells.len()>16{return Err(Error::Limit("selection table depth"));}
        let mut blocks=&doc.blocks;
        for s in &self.cells{
            let Some(Block::Table(t))=blocks.get(s.block) else{return Err(Error::Invalid("selection table path"));};
            blocks=&t.rows.get(s.row).and_then(|r|r.cells.get(s.cell))
                .ok_or(Error::Invalid("selection cell path"))?.blocks;
        }
        let Some(Block::Paragraph(p))=blocks.get(self.paragraph) else{return Err(Error::Invalid("selection paragraph path"));};
        Ok(p)
    }
    fn run(&self,run:usize)->RunPath{
        RunPath{cells:self.cells.clone(),paragraph:self.paragraph,run}
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SelectionLimits{pub max_runs:usize,pub max_paragraph_bytes:usize,pub edit:EditLimits}
impl Default for SelectionLimits{
    fn default()->Self{Self{max_runs:16384,max_paragraph_bytes:16*1024*1024,edit:EditLimits::default()}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RunBoundary{First,Last}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RunPoint{pub run:usize,pub offset:usize,pub unit:TextUnit}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct BytePoint{pub run:usize,pub byte:usize}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Selection{pub anchor:RunPoint,pub focus:RunPoint}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ResolvedSelection{pub start:BytePoint,pub end:BytePoint,pub reversed:bool}
#[derive(Debug,Clone,Copy,Default)]
struct Counts{bytes:usize,scalars:usize,utf16:usize}
impl Counts{
    fn in_unit(self,u:TextUnit)->usize{
        match u{TextUnit::Utf8=>self.bytes,TextUnit::UnicodeScalar=>self.scalars,TextUnit::Utf16=>self.utf16}
    }
    fn add(self,s:&str)->Result<Self,Error>{
        let add=|a:usize,b:usize|a.checked_add(b).ok_or(Error::Limit("selection index arithmetic"));
        Ok(Self{bytes:add(self.bytes,s.len())?,scalars:add(self.scalars,s.chars().count())?,
            utf16:add(self.utf16,s.encode_utf16().count())?})
    }
}
/// Borrowed snapshot: cannot survive a mutable edit of its paragraph.
/// Prefix counts include empty runs; images consume no text units.
pub struct ParagraphIndex<'a>{paragraph:&'a Paragraph,prefix:Vec<Counts>,limits:SelectionLimits}
impl<'a> ParagraphIndex<'a>{
    pub fn new(paragraph:&'a Paragraph,limits:SelectionLimits)->Result<Self,Error>{
        if paragraph.runs.len()>limits.max_runs{return Err(Error::Limit("selection runs"));}
        let mut prefix=vec![Counts::default()];
        let mut total=Counts::default();
        for r in &paragraph.runs{
            if r.text.len()>limits.edit.max_run_bytes{return Err(Error::Limit("edit run bytes"));}
            total=total.add(&r.text)?;
            if total.bytes>limits.max_paragraph_bytes{return Err(Error::Limit("selection paragraph bytes"));}
            prefix.push(total);
        }
        Ok(Self{paragraph,prefix,limits})
    }
    pub fn len(&self,unit:TextUnit)->usize{self.prefix.last().unwrap().in_unit(unit)}
    pub fn resolve_point(&self,p:RunPoint)->Result<BytePoint,Error>{
        let r=self.paragraph.runs.get(p.run).ok_or(Error::Invalid("selection run index"))?;
        Ok(BytePoint{run:p.run,byte:text_byte_offset(&r.text,p.offset,p.unit)?})
    }
    /// Paragraph-wide position to run-local UTF-8 byte point. At a shared
    /// boundary (including empty runs), choose the first or last matching run.
    /// A paragraph without any runs has no editable run point.
    pub fn locate(&self,offset:usize,unit:TextUnit,boundary:RunBoundary)->Result<BytePoint,Error>{
        if offset>self.len(unit){return Err(Error::Invalid("selection paragraph offset"));}
        let mut found=None;
        for (i,r) in self.paragraph.runs.iter().enumerate(){
            let a=self.prefix[i].in_unit(unit);let b=self.prefix[i+1].in_unit(unit);
            if a<=offset&&offset<=b{
                let p=BytePoint{run:i,byte:text_byte_offset(&r.text,offset-a,unit)?};
                if boundary==RunBoundary::First{return Ok(p);}
                found=Some(p);
            }
        }
        found.ok_or(Error::Invalid("selection empty paragraph"))
    }
    pub fn global_offset(&self,p:RunPoint,unit:TextUnit)->Result<usize,Error>{
        let b=self.resolve_point(p)?;
        self.prefix[b.run].in_unit(unit).checked_add(text_offset_from_byte(
            &self.paragraph.runs[b.run].text,b.byte,unit)?).ok_or(Error::Limit("selection index arithmetic"))
    }
    pub fn resolve(&self,s:Selection)->Result<ResolvedSelection,Error>{
        let a=self.resolve_point(s.anchor)?;let b=self.resolve_point(s.focus)?;
        Ok(if a<=b{ResolvedSelection{start:a,end:b,reversed:false}}
            else{ResolvedSelection{start:b,end:a,reversed:true}})
    }
    fn slices(&self,s:ResolvedSelection)->impl Iterator<Item=(usize,usize,usize,&str)>+'_{
        (s.start.run..=s.end.run).map(move |i|{
            let text=&self.paragraph.runs[i].text;
            let a=if i==s.start.run{s.start.byte}else{0};
            let b=if i==s.end.run{s.end.byte}else{text.len()};
            (i,a,b,&text[a..b])
        })
    }
    pub fn selected_text(&self,s:Selection)->Result<String,Error>{
        let s=self.resolve(s)?;
        let mut out=String::new();
        for (_,_,_,t) in self.slices(s){out.push_str(t);}
        Ok(out)
    }
    fn validate_drawings(&self,s:ResolvedSelection,reject:bool)->Result<(),Error>{
        for (i,_,_,_) in self.slices(s){
            let r=&self.paragraph.runs[i];
            if r.pictures.len()>self.limits.edit.max_drawings_per_run||r.drawing_order.len()>self.limits.edit.max_drawings_per_run{
                return Err(Error::Limit("edit drawing count"));
            }
            for a in r.drawings()?{
                let p=BytePoint{run:i,byte:a.text_offset};
                if reject&&s.start<p&&p<s.end{return Err(Error::Invalid("selection crosses picture anchor"));}
            }
        }
        Ok(())
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SelectionEdit{
    pub paragraph:ParagraphPath,pub selection:Selection,
    pub expected:String,pub replacement:String,
    pub insertion:InsertAffinity,pub interior:InteriorAnchorPolicy,
}
/// Replace one contiguous text selection within an existing paragraph.
/// Replacement inherits start-run formatting. Other runs stay in place,
/// even when emptied; no images are deleted or moved between runs.
/// PreserveAtStart is local to each affected run, not the global selection.
/// Reject checks strictly interior anchors in ordered (run,byte) space,
/// including zero-length and shared-boundary runs. Endpoints are explicit.
/// All generated edits commit atomically through C3.1.
pub fn replace_selection(doc:&mut Document,e:&SelectionEdit,limits:SelectionLimits)->Result<EditReport,Error>{
    let edits=plan_selection(doc,e,limits)?;
    apply_text_transaction(doc,&edits,limits.edit)
}
pub(crate) fn plan_selection(doc:&Document,e:&SelectionEdit,limits:SelectionLimits)->Result<Vec<TextEdit>,Error>{
    let input=e.expected.len().checked_add(e.replacement.len()).ok_or(Error::Limit("edit input bytes"))?;
    if input>limits.edit.max_input_bytes{return Err(Error::Limit("edit input bytes"));}
    let edits={
        let index=ParagraphIndex::new(e.paragraph.get(doc)?,limits)?;
        let s=index.resolve(e.selection)?;
        let mut expected=e.expected.as_str();
        for (_,_,_,t) in index.slices(s){
            expected=expected.strip_prefix(t).ok_or(Error::Invalid("stale selection text"))?;
        }
        if !expected.is_empty(){return Err(Error::Invalid("stale selection text"));}
        let changed=e.expected!=e.replacement;
        index.validate_drawings(s,changed&&e.interior==InteriorAnchorPolicy::Reject)?;
        if !changed{return Ok(Vec::new());}
        let final_bytes=(index.len(TextUnit::Utf8)-e.expected.len()).checked_add(e.replacement.len())
            .ok_or(Error::Limit("selection paragraph bytes"))?;
        if final_bytes>limits.max_paragraph_bytes{return Err(Error::Limit("selection paragraph bytes"));}
        let mut ops=Vec::new();
        for (i,a,b,t) in index.slices(s){
            let replacement=if i==s.start.run{e.replacement.as_str()}else{""};
            if t==replacement{continue;}
            if ops.len()>=limits.edit.max_operations{return Err(Error::Limit("edit operations"));}
            ops.push(TextEdit{path:e.paragraph.run(i),start:a,end:b,unit:TextUnit::Utf8,
                expected:t.to_owned(),replacement:replacement.to_owned(),insertion:e.insertion,interior:e.interior});
        }
        ops
    };
    Ok(edits)
}
