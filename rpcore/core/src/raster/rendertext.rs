//! First native font renderer: horizontal embedded CIDFontType2 only.
use super::{Surface,Matrix,FillRule,Color,Stroke,Error,Result,flatten,mask,rgb,stroke_paths};
use crate::{Document,PageInfo,Object,content::Operation,fonts::{Font,FontKind,WritingMode,truetype::{TrueType,MAX_FONT_BYTES}}};
use std::{collections::BTreeMap,sync::Arc};
const MAX_FONT_LOADS:usize=16;
const MAX_FONT_TOTAL:usize=32*1024*1024;
struct Face{font:Font,tt:TrueType}
#[derive(Clone)]
struct State{face:Option<Arc<Face>>,size:f64,scale:f64,tc:f64,tw:f64,leading:f64,rise:f64,mode:u8}
impl Default for State{fn default()->Self{Self{face:None,size:0.,scale:1.,tc:0.,tw:0.,leading:0.,rise:0.,mode:0}}}
pub(super) struct TextRenderer{state:State,stack:Vec<State>,tm:Matrix,line:Matrix,in_text:bool,fonts:BTreeMap<Vec<u8>,Arc<Face>>,font_bytes:usize,glyphs:usize}
fn arity(a:&[Object],n:usize)->Result<()>{if a.len()==n{Ok(())}else{Err(Error::Invalid("render text operand count"))}}
fn num(v:&Object)->Result<f64>{if let Object::Number(n)=v{if n.is_finite()&&n.abs()<=1e12{return Ok(*n);}}Err(Error::Invalid("render text number"))}
fn nums<const N:usize>(a:&[Object])->Result<[f64;N]>{arity(a,N)?;let mut o=[0.;N];for(i,v)in a.iter().enumerate(){o[i]=num(v)?;}Ok(o)}
fn string(v:&Object)->Result<&[u8]>{if let Object::String(s)=v{Ok(s)}else{Err(Error::Invalid("render text string"))}}
impl TextRenderer{
    pub(super) fn new()->Self{Self{state:State::default(),stack:Vec::new(),tm:Matrix::IDENTITY,line:Matrix::IDENTITY,in_text:false,fonts:BTreeMap::new(),font_bytes:0,glyphs:0}}
    pub(super) fn save(&mut self){self.stack.push(self.state.clone());}
    pub(super) fn restore(&mut self)->Result<()>{self.state=self.stack.pop().ok_or(Error::Invalid("text graphics stack"))?;Ok(())}
    fn load(&mut self,doc:&Document<'_>,page:&PageInfo,n:&[u8])->Result<Arc<Face>>{
        if let Some(f)=self.fonts.get(n){return Ok(Arc::clone(f));}
        if self.fonts.len()>=MAX_FONT_LOADS{return Err(Error::Limit("render font count"));}
        let resources=crate::dict(doc.dereference(page.resources.as_ref().ok_or(Error::Invalid("page font Resources"))?)?)?;
        let fonts=crate::dict(doc.dereference(resources.get(b"Font".as_slice()).ok_or(Error::Invalid("page Font resources"))?)?)?;
        let entry=fonts.get(n).ok_or(Error::Invalid("render font resource"))?;
        let font=Font::load(doc,entry)?;
        if font.kind!=FontKind::Type0Cid2||font.writing_mode!=WritingMode::Horizontal{return Err(Error::Unsupported("render font subtype or writing mode"));}
        let d=crate::dict(doc.dereference(entry)?)?;
        let Object::Array(a)=doc.dereference(d.get(b"DescendantFonts".as_slice()).ok_or(Error::Invalid("DescendantFonts"))?)? else{return Err(Error::Invalid("DescendantFonts"));};
        let cid=crate::dict(doc.dereference(a.first().ok_or(Error::Invalid("DescendantFonts"))?)?)?;
        let desc=crate::dict(doc.dereference(cid.get(b"FontDescriptor".as_slice()).ok_or(Error::Unsupported("missing embedded font descriptor"))?)?)?;
        let file=doc.dereference(desc.get(b"FontFile2".as_slice()).ok_or(Error::Unsupported("missing FontFile2"))?)?;
        let data=crate::filters::decode(&file,MAX_FONT_BYTES.min(MAX_FONT_TOTAL-self.font_bytes))?;
        self.font_bytes+=data.len();
        let f=Arc::new(Face{font,tt:TrueType::parse(data)?});
        self.fonts.insert(n.to_vec(),Arc::clone(&f));Ok(f)
    }
    fn next_line(&mut self,x:f64,y:f64)->Result<()>{self.line=self.line.compose(Matrix::translation(x,y))?;self.tm=self.line;Ok(())}
    fn adjust(&mut self,n:f64)->Result<()>{self.tm=self.tm.compose(Matrix::translation(-n/1000.*self.state.size*self.state.scale,0.))?;Ok(())}
    fn show(&mut self,s:&mut Surface,data:&[u8],view:Matrix,ctm:Matrix,fill:Color,stroke:&Stroke)->Result<()>{
        let f=Arc::clone(self.state.face.as_ref().ok_or(Error::Invalid("no current render font"))?);
        for glyph in f.font.decode(data)?{
            self.glyphs+=1;if self.glyphs>100_000{return Err(Error::Limit("render glyph count"));}
            let gid=glyph.glyph_id.ok_or(Error::Unsupported("missing render glyph ID"))?;
            let width=glyph.width.ok_or(Error::Unsupported("missing render glyph width"))?;
            if self.state.mode!=3{
                let unit=self.state.size/f64::from(f.tt.units_per_em());
                let m=view.compose(ctm)?.compose(self.tm)?.compose(Matrix([unit*self.state.scale,0.,0.,unit,0.,self.state.rise]))?;
                let outline=f.tt.outline(gid)?;
                let paths=flatten(&outline,m,&mut s.budget)?;
                if matches!(self.state.mode,0|2){let mask=mask(&paths,FillRule::NonZero,s.w,s.h,&mut s.budget)?;s.paint(&mask,rgb(fill))?;}
                if matches!(self.state.mode,1|2){let paths=stroke_paths(&paths,stroke,view,&mut s.budget)?;let mask=mask(&paths,FillRule::NonZero,s.w,s.h,&mut s.budget)?;s.paint(&mask,rgb(stroke.color))?;}
            }
            let word=if glyph.code==b" "{self.state.tw}else{0.};
            let advance=(width/1000.*self.state.size+self.state.tc+word)*self.state.scale;
            self.tm=self.tm.compose(Matrix::translation(advance,0.))?;
        }Ok(())
    }
    pub(super) fn execute(&mut self,doc:&Document<'_>,page:&PageInfo,s:&mut Surface,op:&Operation,view:Matrix,ctm:Matrix,fill:Color,stroke:&Stroke)->Result<()>{
        let a=&op.operands;
        match op.operator.as_slice(){
            b"BT"=>{arity(a,0)?;if self.in_text{return Err(Error::Invalid("nested render BT"));}self.in_text=true;self.tm=Matrix::IDENTITY;self.line=Matrix::IDENTITY;}
            b"ET"=>{arity(a,0)?;if !self.in_text{return Err(Error::Invalid("render ET"));}self.in_text=false;}
            b"Tf"=>{arity(a,2)?;let Object::Name(n)=&a[0] else{return Err(Error::Invalid("render font name"));};self.state.size=num(&a[1])?;self.state.face=Some(self.load(doc,page,n)?);}
            b"Tc"=>self.state.tc=nums::<1>(a)?[0],b"Tw"=>self.state.tw=nums::<1>(a)?[0],b"Tz"=>self.state.scale=nums::<1>(a)?[0]/100.,
            b"TL"=>self.state.leading=nums::<1>(a)?[0],b"Ts"=>self.state.rise=nums::<1>(a)?[0],
            b"Tr"=>{let v=nums::<1>(a)?[0];if !(0.0..=3.0).contains(&v)||v.fract()!=0.{return Err(Error::Unsupported("text render mode"));}self.state.mode=v as u8;}
            b"Tm"=>{self.tm=Matrix(nums(a)?);self.line=self.tm;}
            b"Td"=>{let [x,y]=nums(a)?;self.next_line(x,y)?;}
            b"TD"=>{let [x,y]=nums(a)?;self.state.leading=-y;self.next_line(x,y)?;}
            b"T*"=>{arity(a,0)?;self.next_line(0.,-self.state.leading)?;}
            b"Tj"=>{arity(a,1)?;if !self.in_text{return Err(Error::Invalid("render text outside BT"));}self.show(s,string(&a[0])?,view,ctm,fill,stroke)?;}
            b"TJ"=>{arity(a,1)?;if !self.in_text{return Err(Error::Invalid("render text outside BT"));}let Object::Array(items)=&a[0] else{return Err(Error::Invalid("render TJ"));};for v in items{match v{Object::String(b)=>self.show(s,b,view,ctm,fill,stroke)?,Object::Number(n)=>self.adjust(*n)?,_=>return Err(Error::Invalid("render TJ item"))}}}
            b"'"=>{arity(a,1)?;if !self.in_text{return Err(Error::Invalid("render text outside BT"));}self.next_line(0.,-self.state.leading)?;self.show(s,string(&a[0])?,view,ctm,fill,stroke)?;}
            b"\""=>{arity(a,3)?;if !self.in_text{return Err(Error::Invalid("render text outside BT"));}self.state.tw=num(&a[0])?;self.state.tc=num(&a[1])?;self.next_line(0.,-self.state.leading)?;self.show(s,string(&a[2])?,view,ctm,fill,stroke)?;}
            _=>return Err(Error::Unsupported("render text operator")),
        }Ok(())
    }
}
