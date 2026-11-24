//! Horizontal PDF text interpreter. Coordinates are in unrotated PDF user space.
//! Selection quads are advance/em approximations, not glyph ink bounds.
use crate::{content::Operation,fonts::{Font,WritingMode},Document,Error,Object,PageInfo,Result};
use std::{collections::BTreeMap,sync::Arc};
const MAX_GLYPHS:usize=100_000;
const MAX_FONTS:usize=16;
const MAX_STACK:usize=64;
const MAX_TEXT_BYTES:usize=16*1024*1024;
const MAX_COORD:f64=1e12;
pub type Point=[f64;2];
pub type Quad=[Point;4];

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Matrix(pub [f64;6]);
impl Matrix{
    pub const IDENTITY:Self=Self([1.,0.,0.,1.,0.,0.]);
    pub fn translation(x:f64,y:f64)->Self{Self([1.,0.,0.,1.,x,y])}
    /// Compose self after rhs: self(rhs(point)).
    pub fn compose(self,rhs:Self)->Result<Self>{
        let [a,b,c,d,e,f]=self.0;let [g,h,i,j,k,l]=rhs.0;
        let m=Self([a*g+c*h,b*g+d*h,a*i+c*j,b*i+d*j,a*k+c*l+e,b*k+d*l+f]);
        for v in m.0{bounded(v)?;}
        Ok(m)
    }
    pub fn point(self,p:Point)->Result<Point>{
        let [a,b,c,d,e,f]=self.0;
        Ok([bounded(a*p[0]+c*p[1]+e)?,bounded(b*p[0]+d*p[1]+f)?])
    }
}
fn bounded(v:f64)->Result<f64>{
    if v.is_finite()&&v.abs()<=MAX_COORD{Ok(v)}else{Err(Error::Limit("text coordinate"))}
}
#[derive(Debug,Clone,PartialEq)]
pub struct TextGlyph{
    pub code:Vec<u8>,
    pub unicode:Option<String>,
    pub font:Vec<u8>,
    pub cid:Option<u32>,
    pub glyph_id:Option<u16>,
    pub origin:Point,
    pub advance_end:Point,
    /// Approximate em-height selection quad, before CropBox/Rotate.
    pub quad:Quad,
    pub operation_index:usize,
    pub rendering_mode:u8,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum TextWarning{ExternalObjectNotInterpreted,GraphicsStateNotInterpreted,ActualTextNotInterpreted,UnknownOperator}
#[derive(Debug,Clone,Default,PartialEq)]
pub struct PageText{
    pub glyphs:Vec<TextGlyph>,
    pub warnings:Vec<TextWarning>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TextMatch{
    /// Half-open range in content-order glyphs; ligatures may cover multiple characters.
    pub glyph_start:usize,
    pub glyph_end:usize,
}
#[derive(Debug,Clone)]
struct State{
    ctm:Matrix,font:Option<Vec<u8>>,size:f64,char_space:f64,word_space:f64,
    scale:f64,leading:f64,rise:f64,mode:u8,
}
impl Default for State{
    fn default()->Self{Self{ctm:Matrix::IDENTITY,font:None,size:0.,char_space:0.,word_space:0.,scale:1.,leading:0.,rise:0.,mode:0}}
}
fn arity(a:&[Object],n:usize)->Result<()>{
    if a.len()==n{Ok(())}else{Err(Error::Invalid("text operand count"))}
}
fn num(o:&Object)->Result<f64>{
    if let Object::Number(v)=o{bounded(*v)}else{Err(Error::Invalid("text number"))}
}
fn nums<const N:usize>(a:&[Object])->Result<[f64;N]>{
    arity(a,N)?;let mut out=[0.;N];
    for (i,v) in a.iter().enumerate(){out[i]=num(v)?;}
    Ok(out)
}
fn string(o:&Object)->Result<&[u8]>{
    if let Object::String(v)=o{Ok(v)}else{Err(Error::Invalid("text string"))}
}
struct Interpreter<'d,'a>{
    doc:&'d Document<'a>,page:&'d PageInfo,
    state:State,stack:Vec<State>,fonts:BTreeMap<Vec<u8>,Arc<Font>>,
    tm:Matrix,line:Matrix,in_text:bool,out:PageText,unicode_bytes:usize,
}
impl<'d,'a> Interpreter<'d,'a>{
    fn new(doc:&'d Document<'a>,page:&'d PageInfo)->Self{
        Self{doc,page,state:State::default(),stack:Vec::new(),fonts:BTreeMap::new(),
            tm:Matrix::IDENTITY,line:Matrix::IDENTITY,in_text:false,out:PageText::default(),unicode_bytes:0}
    }
    fn require_text(&self)->Result<()>{
        if self.in_text{Ok(())}else{Err(Error::Invalid("text operator outside BT/ET"))}
    }
    fn warning(&mut self,w:TextWarning){if !self.out.warnings.contains(&w){self.out.warnings.push(w);}}
    fn move_line(&mut self,x:f64,y:f64)->Result<()>{
        self.line=self.line.compose(Matrix::translation(x,y))?;
        self.tm=self.line;Ok(())
    }
    fn next_line(&mut self)->Result<()>{self.move_line(0.,-self.state.leading)}
    fn font(&mut self)->Result<Arc<Font>>{
        let n=self.state.font.as_ref().ok_or(Error::Invalid("no current text font"))?;
        if let Some(f)=self.fonts.get(n){return Ok(Arc::clone(f));}
        let f=Arc::new(self.doc.page_font(self.page,n)?);
        if self.fonts.len()>=MAX_FONTS{self.fonts.clear();}
        self.fonts.insert(n.clone(),Arc::clone(&f));
        Ok(f)
    }
    fn show(&mut self,data:&[u8],operation_index:usize)->Result<()>{
        self.require_text()?;
        let f=self.font()?;
        if f.writing_mode!=WritingMode::Horizontal{return Err(Error::Unsupported("vertical text positioning"));}
        let font_name=self.state.font.clone().ok_or(Error::Invalid("no current text font"))?;
        for g in f.decode(data)?{
            if self.out.glyphs.len()>=MAX_GLYPHS{return Err(Error::Limit("page text glyphs"));}
            let width=g.width.ok_or(Error::Unsupported("missing glyph width"))?;
            self.unicode_bytes=self.unicode_bytes.checked_add(g.unicode.as_ref().map_or(3,String::len)).ok_or(Error::Limit("text bytes"))?;
            if self.unicode_bytes>MAX_TEXT_BYTES{return Err(Error::Limit("text bytes"));}
            // Word spacing applies to a single-byte character code 32, not Unicode spaces.
            let word=if g.code.as_slice()==b" "{self.state.word_space}else{0.};
            let ink=bounded(width/1000.*self.state.size*self.state.scale)?;
            let advance=bounded((width/1000.*self.state.size+self.state.char_space+word)*self.state.scale)?;
            let m=self.state.ctm.compose(self.tm)?;
            let y=self.state.rise;
            let h=self.state.size;
            let origin=m.point([0.,y])?;
            let advance_end=m.point([advance,y])?;
            let quad=[origin,m.point([ink,y])?,m.point([ink,y+h])?,m.point([0.,y+h])?];
            self.out.glyphs.push(TextGlyph{code:g.code,unicode:g.unicode,font:font_name.clone(),cid:g.cid,glyph_id:g.glyph_id,origin,advance_end,quad,operation_index,rendering_mode:self.state.mode});
            self.tm=self.tm.compose(Matrix::translation(advance,0.))?;
        }
        Ok(())
    }
    fn adjust(&mut self,v:f64)->Result<()>{
        self.require_text()?;
        if self.font()?.writing_mode!=WritingMode::Horizontal{return Err(Error::Unsupported("vertical text positioning"));}
        let dx=bounded(-v/1000.*self.state.size*self.state.scale)?;
        self.tm=self.tm.compose(Matrix::translation(dx,0.))?;
        Ok(())
    }
    fn execute(&mut self,op:&Operation,i:usize)->Result<()>{
        let a=op.operands.as_slice();
        match op.operator.as_slice(){
            b"q"=>{arity(a,0)?;if self.stack.len()>=MAX_STACK{return Err(Error::Limit("graphics state stack"));}self.stack.push(self.state.clone());}
            b"Q"=>{arity(a,0)?;self.state=self.stack.pop().ok_or(Error::Invalid("graphics state underflow"))?;}
            b"cm"=>{self.state.ctm=self.state.ctm.compose(Matrix(nums(a)?))?;}
            b"BT"=>{arity(a,0)?;if self.in_text{return Err(Error::Invalid("nested BT"));}self.in_text=true;self.tm=Matrix::IDENTITY;self.line=Matrix::IDENTITY;}
            b"ET"=>{arity(a,0)?;self.require_text()?;self.in_text=false;}
            b"Tf"=>{
                arity(a,2)?;
                let Object::Name(n)=&a[0] else{return Err(Error::Invalid("font resource name"));};
                if n.len()>256{return Err(Error::Limit("font resource name"));}
                self.state.font=Some(n.clone());self.state.size=num(&a[1])?;
            }
            b"Tc"=>{self.state.char_space=nums::<1>(a)?[0];}
            b"Tw"=>{self.state.word_space=nums::<1>(a)?[0];}
            b"Tz"=>{self.state.scale=nums::<1>(a)?[0]/100.;}
            b"TL"=>{self.state.leading=nums::<1>(a)?[0];}
            b"Ts"=>{self.state.rise=nums::<1>(a)?[0];}
            b"Tr"=>{let v=nums::<1>(a)?[0];if !(0.0..=7.0).contains(&v)||v.fract()!=0.{return Err(Error::Invalid("text rendering mode"));}self.state.mode=v as u8;}
            b"Tm"=>{self.require_text()?;self.tm=Matrix(nums(a)?);self.line=self.tm;}
            b"Td"=>{self.require_text()?;let [x,y]=nums(a)?;self.move_line(x,y)?;}
            b"TD"=>{self.require_text()?;let [x,y]=nums(a)?;self.state.leading=-y;self.move_line(x,y)?;}
            b"T*"=>{self.require_text()?;arity(a,0)?;self.next_line()?;}
            b"Tj"=>{arity(a,1)?;self.show(string(&a[0])?,i)?;}
            b"TJ"=>{
                self.require_text()?;arity(a,1)?;
                let Object::Array(items)=&a[0] else{return Err(Error::Invalid("TJ array"));};
                for v in items{match v{Object::String(s)=>self.show(s,i)?,Object::Number(n)=>self.adjust(*n)?,_=>return Err(Error::Invalid("TJ item"))}}
            }
            b"'"=>{self.require_text()?;arity(a,1)?;self.next_line()?;self.show(string(&a[0])?,i)?;}
            b"\""=>{self.require_text()?;arity(a,3)?;self.state.word_space=num(&a[0])?;self.state.char_space=num(&a[1])?;self.next_line()?;self.show(string(&a[2])?,i)?;}
            // These operators can affect extracted text. Do not silently claim complete extraction.
            b"Do"=>self.warning(TextWarning::ExternalObjectNotInterpreted),
            b"gs"=>{
                arity(a,1)?;
                let Object::Name(n)=&a[0] else{return Err(Error::Invalid("ExtGState name"));};
                let resources=crate::dict(self.doc.dereference(self.page.resources.as_ref().ok_or(Error::Invalid("ExtGState Resources"))?)?)?;
                let states=crate::dict(self.doc.dereference(resources.get(b"ExtGState".as_slice()).ok_or(Error::Invalid("ExtGState dictionary"))?)?)?;
                let state=crate::dict(self.doc.dereference(states.get(n).ok_or(Error::Invalid("ExtGState name"))?)?)?;
                // Font is the only ExtGState entry that changes text positioning
                // directly. Preserve explicit failure rather than using a stale font.
                if state.contains_key(b"Font".as_slice()){return Err(Error::Unsupported("ExtGState text font"));}
                self.warning(TextWarning::GraphicsStateNotInterpreted);
            }
            b"BDC"=>{self.warning(TextWarning::ActualTextNotInterpreted);}
            b"BMC"|b"EMC"|b"MP"|b"DP"|b"BX"|b"EX"|
            b"m"|b"l"|b"c"|b"v"|b"y"|b"h"|b"re"|b"S"|b"s"|b"f"|b"F"|b"f*"|b"B"|b"B*"|b"b"|b"b*"|b"n"|
            b"W"|b"W*"|b"w"|b"J"|b"j"|b"M"|b"d"|b"ri"|b"i"|b"CS"|b"cs"|b"SC"|b"SCN"|b"sc"|b"scn"|b"G"|b"g"|b"RG"|b"rg"|b"K"|b"k"|b"sh"=>{},
            _=>return Err(Error::Unsupported("unknown page operator")),
        }
        Ok(())
    }
}
impl Document<'_>{
    /// Strict horizontal text extraction from page content (not Form XObjects).
    pub fn page_text(&self,page:&PageInfo,content_limit:usize)->Result<PageText>{
        let ops=self.page_operations(page,content_limit)?;
        let mut state=Interpreter::new(self,page);
        for (i,op) in ops.iter().enumerate(){state.execute(op,i)?;}
        if state.in_text||!state.stack.is_empty(){return Err(Error::Invalid("unbalanced page text or graphics state"));}
        Ok(state.out)
    }
}
impl PageText{
    /// Content order only, no invented spaces, line breaks or reading order.
    pub fn plain_text(&self)->String{
        let mut out=String::new();
        for g in &self.glyphs{out.push_str(g.unicode.as_deref().unwrap_or("\u{fffd}"));}
        out
    }
    /// Exact, case-sensitive Unicode substring search. Match ranges are glyph indices.
    /// An unknown character is a separator, never a wildcard or a fabricated match.
    pub fn search(&self,query:&str)->Result<Vec<TextMatch>>{
        if query.is_empty(){return Err(Error::Invalid("empty text query"));}
        if query.len()>MAX_TEXT_BYTES{return Err(Error::Limit("text query"));}
        let mut out=Vec::new();let mut text=String::new();let mut spans=Vec::new();
        fn scan(text:&str,spans:&[(usize,usize)],query:&str,out:&mut Vec<TextMatch>)->Result<()>{
            for (start,_) in text.match_indices(query){
                let end=start+query.len()-1;
                let a=spans.partition_point(|(offset,_)|*offset<=start);
                let b=spans.partition_point(|(offset,_)|*offset<=end);
                if a==0||b==0{return Err(Error::Invalid("text search span"));}
                if out.len()>=MAX_GLYPHS{return Err(Error::Limit("text matches"));}
                let found=TextMatch{glyph_start:spans[a-1].1,glyph_end:spans[b-1].1+1};
                if out.last()!=Some(&found){out.push(found);}
            }
            Ok(())
        }
        for (i,g) in self.glyphs.iter().enumerate(){
            match &g.unicode{
                Some(s) if !s.is_empty()=>{spans.push((text.len(),i));text.push_str(s);}
                _=>{scan(&text,&spans,query,&mut out)?;text.clear();spans.clear();}
            }
        }
        scan(&text,&spans,query,&mut out)?;
        Ok(out)
    }
    /// Return glyph indices whose approximate quad AABB intersects the user-space rectangle.
    pub fn select(&self,rect:[f64;4])->Result<Vec<usize>>{
        for v in rect{bounded(v)?;}
        if rect[0]>rect[2]||rect[1]>rect[3]{return Err(Error::Invalid("text selection rectangle"));}
        let mut out=Vec::new();
        for (i,g) in self.glyphs.iter().enumerate(){
            let mut b=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];
            for [x,y] in g.quad{b[0]=b[0].min(x);b[1]=b[1].min(y);b[2]=b[2].max(x);b[3]=b[3].max(y);}
            if b[0]<=rect[2]&&b[2]>=rect[0]&&b[1]<=rect[3]&&b[3]>=rect[1]{out.push(i);}
        }
        Ok(out)
    }
}
