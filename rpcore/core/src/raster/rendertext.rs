//! Native horizontal TrueType, CFF1 and traditional Type1 font rendering.
use super::{Path,StandardFontSet,Surface,Matrix,FillRule,Color,Stroke,Error,Result,flatten,mask,rgb,stroke_paths};
use crate::{Document,PageInfo,Object,content::Operation,fonts::{standard::StandardFont,Font,FontKind,WritingMode,truetype::{TrueType,CmapKind,MAX_FONT_BYTES},cff::Cff,type1::{Type1,standard_name}}};
use std::{collections::BTreeMap,sync::Arc};
const MAX_FONT_LOADS:usize=16;
const MAX_FONT_TOTAL:usize=32*1024*1024;
enum Outline{Tt(TrueType),SimpleTt(TrueType,[Option<u16>;256]),Cff(Cff,[Option<u16>;256]),Type1(Type1,BTreeMap<u8,Vec<u8>>,bool),ReplacementType1(Type1)}
struct Face{font:Font,outline:Outline}
fn differences(doc:&Document<'_>,d:&BTreeMap<Vec<u8>,Object>)->Result<BTreeMap<u8,Vec<u8>>>{
    let Object::Dict(enc)=doc.dereference(d.get(b"Encoding".as_slice()).ok_or(Error::Invalid("missing Encoding dictionary"))?)? else{return Err(Error::Invalid("Encoding dictionary"));};
    let Object::Array(a)=doc.dereference(enc.get(b"Differences".as_slice()).ok_or(Error::Unsupported("CFF Differences required"))?)? else{return Err(Error::Invalid("CFF Differences"));};
    if a.len()>1024{return Err(Error::Limit("font Differences"));}
    let mut out=BTreeMap::new();let mut code=None;
    for v in a{match v{
        Object::Number(n) if (0.0..=255.).contains(&n)&&n.fract()==0.=>code=Some(n as usize),
        Object::Name(n)=>{let i=code.ok_or(Error::Invalid("CFF Differences code"))?;if i>255{return Err(Error::Invalid("CFF Differences range"));}out.insert(i as u8,n);code=Some(i+1);}
        _=>return Err(Error::Invalid("CFF Differences item")),
    }}Ok(out)
}
fn cff_mapping(doc:&Document<'_>,d:&BTreeMap<Vec<u8>,Object>,c:&Cff)->Result<[Option<u16>;256]>{
    let enc=d.get(b"Encoding".as_slice()).map(|v|doc.dereference(v)).transpose()?;
    let (base,diff)=match enc{
        None=>(None,BTreeMap::new()),
        Some(Object::Name(n))=>(Some(n),BTreeMap::new()),
        Some(Object::Dict(e))=>{
            let base=match e.get(b"BaseEncoding".as_slice()).map(|v|doc.dereference(v)).transpose()?{
                None=>None,Some(Object::Name(n))=>Some(n),_=>return Err(Error::Invalid("CFF BaseEncoding")),
            };
            let diff=if e.contains_key(b"Differences".as_slice()){differences(doc,d)?}else{BTreeMap::new()};
            (base,diff)
        }
        _=>return Err(Error::Invalid("CFF PDF Encoding")),
    };
    if base.as_ref().is_some_and(|n|!matches!(n.as_slice(),b"StandardEncoding"|b"WinAnsiEncoding"|b"MacRomanEncoding"|b"MacExpertEncoding")){
        return Err(Error::Unsupported("CFF PDF BaseEncoding"));
    }
    let mut map=[None;256];
    for code in 0..=255u8{
        map[code as usize]=if let Some(name)=diff.get(&code){
            c.glyph_for_name(name)
        }else if let Some(base)=&base{
            c.glyph_for_encoding(base,code)
        }else{c.glyph_for_code(code)};
    }
    Ok(map)
}
fn simple_tt_mapping(doc:&Document<'_>,d:&BTreeMap<Vec<u8>,Object>,desc:&BTreeMap<Vec<u8>,Object>,font:&Font,tt:&TrueType)->Result<[Option<u16>;256]>{
    let flags=match desc.get(b"Flags".as_slice()).map(|v|doc.dereference(v)).transpose()?{
        Some(Object::Number(n)) if n>=0.&&n<=u32::MAX as f64&&n.fract()==0.=>n as u32,
        _=>return Err(Error::Invalid("TrueType FontDescriptor Flags")),
    };
    let symbolic=flags&4!=0;
    if symbolic&&flags&32!=0{return Err(Error::Invalid("conflicting TrueType font flags"));}
    let enc=d.get(b"Encoding".as_slice()).map(|v|doc.dereference(v)).transpose()?;
    let (base,has_diff)=match enc{
        None=>(None,false),
        Some(Object::Name(n))=>(Some(n),false),
        Some(Object::Dict(e))=>{
            let base=match e.get(b"BaseEncoding".as_slice()).map(|v|doc.dereference(v)).transpose()?{
                None=>None,Some(Object::Name(n))=>Some(n),_=>return Err(Error::Invalid("TrueType BaseEncoding")),
            };(base,e.contains_key(b"Differences".as_slice()))
        }
        _=>return Err(Error::Invalid("TrueType Encoding")),
    };
    if base.as_ref().is_some_and(|b|!matches!(b.as_slice(),b"WinAnsiEncoding"|b"MacRomanEncoding"|b"StandardEncoding"|b"MacExpertEncoding")){
        return Err(Error::Unsupported("TrueType PDF encoding"));
    }
    let cm=tt.character_map()?;let mut out=[None;256];
    // Windows Symbol uses a single consistent high byte in its private-use
    // range, or a direct low-byte mapping. Ambiguous matches remain unmapped.
    let symbol_only=symbolic&&base.is_none()&&!has_diff;
    let mut prefix=None;
    if symbol_only{
        for p in [0,0xf000,0xf100,0xf200]{
            if (0..=255).any(|c|cm.glyph(CmapKind::Symbol,p+c).is_some()){
                if prefix.is_some(){return Err(Error::Unsupported("ambiguous TrueType symbol cmap"));}
                prefix=Some(p);
            }
        }
    }
    for code in 0..=255u8{
        out[code as usize]=if symbol_only{
            if let Some(p)=prefix{cm.glyph(CmapKind::Symbol,p+u32::from(code))}
            else{cm.glyph(CmapKind::MacRoman,u32::from(code))}
        }else{
            let name=font.simple_glyph_name(code);
            // Adobe glyph names map to scalar characters; ligature names
            // are special because the extraction interface expands them.
            let scalar=name.and_then(|n|match n{
                b"fi"=>Some(0xfb01),b"fl"=>Some(0xfb02),b"ff"=>Some(0xfb00),b"ffi"=>Some(0xfb03),b"ffl"=>Some(0xfb04),
                _=>StandardFont::Helvetica.unicode(n).or_else(||font.simple_encoding_unicode(code)).and_then(|s|{let mut c=s.chars();let first=c.next()?;c.next().is_none().then_some(first as u32)}),
            });
            scalar.and_then(|c|cm.glyph(CmapKind::Unicode,c)).or_else(||{
                let name=name?;
                (0..=255u8).find(|&c|crate::fonts::standard::encoded_name(b"MacRomanEncoding",c)==Some(name)).and_then(|c|cm.glyph(CmapKind::MacRoman,u32::from(c)))
            })
        };
    }
    Ok(out)
}
fn type1_mapping(doc:&Document<'_>,d:&BTreeMap<Vec<u8>,Object>)->Result<(BTreeMap<u8,Vec<u8>>,bool)>{
    match d.get(b"Encoding".as_slice()).map(|v|doc.dereference(v)).transpose()?{
        None=>Ok((BTreeMap::new(),false)),
        Some(Object::Name(n)) if n==b"StandardEncoding"=>Ok((BTreeMap::new(),true)),
        Some(Object::Dict(enc))=>{
            let standard=match enc.get(b"BaseEncoding".as_slice()){
                None=>false,Some(Object::Name(n)) if n==b"StandardEncoding"=>true,
                _=>return Err(Error::Unsupported("Type1 BaseEncoding")),
            };
            let names=if enc.contains_key(b"Differences".as_slice()){differences(doc,d)?}else{BTreeMap::new()};
            Ok((names,standard))
        }
        _=>Err(Error::Unsupported("Type1 PDF Encoding")),
    }
}
#[derive(Clone)]
struct State{face:Option<Arc<Face>>,size:f64,scale:f64,tc:f64,tw:f64,leading:f64,rise:f64,mode:u8}
impl Default for State{fn default()->Self{Self{face:None,size:0.,scale:1.,tc:0.,tw:0.,leading:0.,rise:0.,mode:0}}}
pub(super) struct TextRenderer{state:State,stack:Vec<State>,tm:Matrix,line:Matrix,in_text:bool,fonts:BTreeMap<(Option<(u32,u16)>,Vec<u8>),Arc<Face>>,indirect_fonts:BTreeMap<(u32,u16),Arc<Face>>,font_bytes:usize,glyphs:usize,substituted:Vec<StandardFont>,outlines:BTreeMap<(usize,Vec<u8>),(Arc<Path>,Matrix)>,outline_segments:usize}
fn arity(a:&[Object],n:usize)->Result<()>{if a.len()==n{Ok(())}else{Err(Error::Invalid("render text operand count"))}}
fn num(v:&Object)->Result<f64>{if let Object::Number(n)=v{if n.is_finite()&&n.abs()<=1e12{return Ok(*n);}}Err(Error::Invalid("render text number"))}
fn nums<const N:usize>(a:&[Object])->Result<[f64;N]>{arity(a,N)?;let mut o=[0.;N];for(i,v)in a.iter().enumerate(){o[i]=num(v)?;}Ok(o)}
fn string(v:&Object)->Result<&[u8]>{if let Object::String(s)=v{Ok(s)}else{Err(Error::Invalid("render text string"))}}
impl TextRenderer{
    pub(super) fn new()->Self{Self{state:State::default(),stack:Vec::new(),tm:Matrix::IDENTITY,line:Matrix::IDENTITY,in_text:false,fonts:BTreeMap::new(),indirect_fonts:BTreeMap::new(),font_bytes:0,glyphs:0,substituted:Vec::new(),outlines:BTreeMap::new(),outline_segments:0}}
    pub(super) fn save(&mut self){self.stack.push(self.state.clone());}
    pub(super) fn restore(&mut self)->Result<()>{self.state=self.stack.pop().ok_or(Error::Invalid("text graphics stack"))?;Ok(())}
    pub(super) fn substituted_fonts(&self)->Vec<StandardFont>{self.substituted.clone()}
    fn load(&mut self,doc:&Document<'_>,page:&PageInfo,n:&[u8],substitutes:Option<&StandardFontSet>,owner:Option<(u32,u16)>)->Result<Arc<Face>>{
        let cache_key=(owner,n.to_vec());
        if let Some(f)=self.fonts.get(&cache_key){return Ok(Arc::clone(f));}
        if self.fonts.len()>=1024{return Err(Error::Limit("render font aliases"));}
        let resources=if let Some((num,gen))=owner{
            let Object::Stream{dict,..}=doc.resolve(num,gen)? else{return Err(Error::Invalid("Form font resource owner"));};
            doc.dereference(dict.get(b"Resources".as_slice()).ok_or(Error::Invalid("Form font Resources"))?)?
        }else{doc.dereference(page.resources.as_ref().ok_or(Error::Invalid("page font Resources"))?)?};
        let resources=crate::dict(resources)?;
        let fonts=crate::dict(doc.dereference(resources.get(b"Font".as_slice()).ok_or(Error::Invalid("Font resources"))?)?)?;
        let entry=fonts.get(n).ok_or(Error::Invalid("render font resource"))?;
        let id=if let Object::Ref(num,gen)=entry{Some((*num,*gen))}else{None};
        if let Some(f)=id.and_then(|id|self.indirect_fonts.get(&id)).cloned(){
            self.fonts.insert(cache_key,Arc::clone(&f));return Ok(f);
        }
        // Bound unique loaded faces independently of aliases to the same indirect font.
        if self.indirect_fonts.len()+self.fonts.values().filter(|f|!self.indirect_fonts.values().any(|v|Arc::ptr_eq(f,v))).count()>=MAX_FONT_LOADS{return Err(Error::Limit("render font count"));}
        let font=Font::load(doc,entry)?;
        if font.writing_mode!=WritingMode::Horizontal{return Err(Error::Unsupported("vertical font rasterization"));}
        let d=crate::dict(doc.dereference(entry)?)?;
        if let Some(face)=font.standard_face(){
            if let Some(program)=substitutes.and_then(|s|s.get(face)){
                if program.len()>MAX_FONT_TOTAL-self.font_bytes{return Err(Error::Limit("render replacement font bytes"));}
                let outline=if program.starts_with(&[1,0]){
                    let c=Cff::parse(program.to_vec())?;
                    if c.is_cid(){return Err(Error::Unsupported("CID replacement font"));}
                    let map=std::array::from_fn(|i|font.simple_glyph_name(i as u8).and_then(|n|c.glyph_for_name(n)));
                    Outline::Cff(c,map)
                }else if program.starts_with(&[0x80,1]){
                    Outline::ReplacementType1(Type1::parse_pfb(program)?)
                }else{
                    Outline::ReplacementType1(Type1::parse_pfa(program)?)
                };
                self.font_bytes+=program.len();
                if !self.substituted.contains(&face){self.substituted.push(face);}
                let f=Arc::new(Face{font,outline});if let Some(id)=id{self.indirect_fonts.insert(id,Arc::clone(&f));}self.fonts.insert(cache_key,Arc::clone(&f));return Ok(f);
            }
        }
        let cid=if matches!(font.kind,FontKind::Type0Cid0|FontKind::Type0Cid2){
            let Object::Array(a)=doc.dereference(d.get(b"DescendantFonts".as_slice()).ok_or(Error::Invalid("DescendantFonts"))?)? else{return Err(Error::Invalid("DescendantFonts"));};
            crate::dict(doc.dereference(a.first().ok_or(Error::Invalid("DescendantFonts"))?)?)?
        }else{d.clone()};
        let desc=crate::dict(doc.dereference(cid.get(b"FontDescriptor".as_slice()).ok_or(Error::Unsupported("missing embedded font descriptor"))?)?)?;
        let traditional=font.kind==FontKind::Type1&&desc.contains_key(b"FontFile".as_slice());
        let key=if traditional{b"FontFile".as_slice()}else if matches!(font.kind,FontKind::Type0Cid2|FontKind::TrueType)&&desc.contains_key(b"FontFile2".as_slice()){b"FontFile2".as_slice()}else{b"FontFile3".as_slice()};
        let file=doc.dereference(desc.get(key).ok_or(Error::Unsupported("missing supported embedded font file"))?)?;
        let mut opentype=false;
        if key==b"FontFile3"{
            let Object::Stream{dict,..}=&file else{return Err(Error::Invalid("font program stream"));};
            let expected=if font.kind==FontKind::Type0Cid0{b"CIDFontType0C".as_slice()}else{b"Type1C".as_slice()};
            match dict.get(b"Subtype".as_slice()).map(|v|doc.dereference(v)).transpose()?{
                Some(Object::Name(n)) if n==b"OpenType"=>opentype=true,
                Some(Object::Name(n)) if !matches!(font.kind,FontKind::Type0Cid2|FontKind::TrueType)&&n==expected=>{},
                _=>return Err(Error::Unsupported("FontFile3 subtype")),
            }
        }
        let data=crate::filters::decode(&file,MAX_FONT_BYTES.min(MAX_FONT_TOTAL-self.font_bytes))?;
        self.font_bytes+=data.len();
        let outline=if traditional{
            let Object::Stream{dict,..}=&file else{return Err(Error::Invalid("Type1 FontFile stream"));};
            let length=|key:&[u8]|->Result<usize>{
                let n=doc.dereference(dict.get(key).ok_or(Error::Invalid("Type1 FontFile length"))?)?;
                if let Object::Number(n)=n{if n>=0.&&n<=MAX_FONT_BYTES as f64&&n.fract()==0.{return Ok(n as usize);}}
                Err(Error::Invalid("Type1 FontFile length"))
            };
            let t=Type1::parse_pdf(&data,length(b"Length1")?,length(b"Length2")?,length(b"Length3")?)?;
            let (names,standard)=type1_mapping(doc,&d)?;
            Outline::Type1(t,names,standard)
         }else if font.kind==FontKind::Type0Cid2{Outline::Tt(TrueType::parse(data)?)}else if font.kind==FontKind::TrueType{
            let tt=TrueType::parse(data)?;let map=simple_tt_mapping(doc,&d,&desc,&font,&tt)?;Outline::SimpleTt(tt,map)
        }else{
            let c=if opentype{Cff::parse_opentype(&data)?}else{Cff::parse(data)?};
            if c.is_cid()!=(font.kind==FontKind::Type0Cid0){return Err(Error::Invalid("CFF CID subtype mismatch"));}
            let names=if c.is_cid(){[None;256]}else{cff_mapping(doc,&d,&c)?};
            Outline::Cff(c,names)
        };
        let f=Arc::new(Face{font,outline});
        if let Some(id)=id{self.indirect_fonts.insert(id,Arc::clone(&f));}self.fonts.insert(cache_key,Arc::clone(&f));Ok(f)
    }
    fn next_line(&mut self,x:f64,y:f64)->Result<()>{self.line=self.line.compose(Matrix::translation(x,y))?;self.tm=self.line;Ok(())}
    fn adjust(&mut self,n:f64)->Result<()>{self.tm=self.tm.compose(Matrix::translation(-n/1000.*self.state.size*self.state.scale,0.))?;Ok(())}
    fn show(&mut self,s:&mut Surface,data:&[u8],view:Matrix,ctm:Matrix,fill:Color,stroke:&Stroke)->Result<()>{
        let f=Arc::clone(self.state.face.as_ref().ok_or(Error::Invalid("no current render font"))?);
        for glyph in f.font.decode(data)?{
            self.glyphs+=1;if self.glyphs>100_000{return Err(Error::Limit("render glyph count"));}
            let width=glyph.width.ok_or(Error::Unsupported("missing render glyph width"))?;
            if self.state.mode!=3{
                // Face objects live in the page font cache for the entire render.
                // The Arc address and PDF code identify a stable outline; CTM,
                // color, opacity and clipping remain per-instance.
                let key=(Arc::as_ptr(&f) as usize,glyph.code.clone());
                let (outline,fm)=if let Some((p,m))=self.outlines.get(&key){(Arc::clone(p),*m)}else{
                    let (p,m)=match &f.outline{
                    Outline::Tt(tt)=>{let gid=glyph.glyph_id.ok_or(Error::Unsupported("missing render glyph ID"))?;let u=1./f64::from(tt.units_per_em());(tt.outline(gid)?,Matrix([u,0.,0.,u,0.,0.]))}
                    Outline::SimpleTt(tt,map)=>{let gid=map[glyph.code[0] as usize].ok_or(Error::Unsupported("missing simple TrueType glyph mapping"))?;let u=1./f64::from(tt.units_per_em());(tt.outline(gid)?,Matrix([u,0.,0.,u,0.,0.]))}
                    Outline::ReplacementType1(t)=>{
                        let name=f.font.simple_glyph_name(glyph.code[0]).ok_or(Error::Unsupported("missing replacement glyph name"))?;
                        (t.outline(name)?,t.font_matrix())
                    }
                    Outline::Type1(t,names,standard)=>{
                        let code=glyph.code[0];
                        let name=names.get(&code).map(Vec::as_slice).or_else(||if *standard{standard_name(code)}else{t.glyph_name(code)}).ok_or(Error::Unsupported("missing Type1 encoding"))?;
                        (t.outline(name)?,t.font_matrix())
                    }
                    Outline::Cff(c,names)=>{
                        let gid=if c.is_cid(){glyph.cid.and_then(|id|c.glyph_for_cid(id))}else{names[glyph.code[0] as usize]}.ok_or(Error::Unsupported("missing CFF glyph mapping"))?;
                        (c.outline(gid)?,c.font_matrix())
                    }
                    };
                    let p=Arc::new(p);
                    if self.outlines.len()<4096&&p.len()<=200_000-self.outline_segments{
                        self.outline_segments+=p.len();
                        self.outlines.insert(key,(Arc::clone(&p),m));
                    }
                    (p,m)
                };
                let unit=self.state.size;
                let m=view.compose(ctm)?.compose(self.tm)?.compose(Matrix([unit*self.state.scale,0.,0.,unit,0.,self.state.rise]))?.compose(fm)?;
                let paths=flatten(&outline,m,&mut s.budget)?;
                if matches!(self.state.mode,0|2){let mask=mask(&paths,FillRule::NonZero,s.w,s.h,&s.clip,&mut s.budget)?;s.paint(&mask,rgb(fill),s.fill_alpha)?;}
                if matches!(self.state.mode,1|2){let paths=stroke_paths(&paths,stroke,view,&mut s.budget)?;let mask=mask(&paths,FillRule::NonZero,s.w,s.h,&s.clip,&mut s.budget)?;s.paint(&mask,rgb(stroke.color),s.stroke_alpha)?;}
            }
            let word=if glyph.code==b" "{self.state.tw}else{0.};
            let advance=(width/1000.*self.state.size+self.state.tc+word)*self.state.scale;
            self.tm=self.tm.compose(Matrix::translation(advance,0.))?;
        }Ok(())
    }
    pub(super) fn execute(&mut self,doc:&Document<'_>,page:&PageInfo,s:&mut Surface,op:&Operation,view:Matrix,ctm:Matrix,fill:Color,stroke:&Stroke,substitutes:Option<&StandardFontSet>,owner:Option<(u32,u16)>)->Result<()>{
        let a=&op.operands;
        match op.operator.as_slice(){
            b"BT"=>{arity(a,0)?;if self.in_text{return Err(Error::Invalid("nested render BT"));}self.in_text=true;self.tm=Matrix::IDENTITY;self.line=Matrix::IDENTITY;}
            b"ET"=>{arity(a,0)?;if !self.in_text{return Err(Error::Invalid("render ET"));}self.in_text=false;}
            b"Tf"=>{arity(a,2)?;let Object::Name(n)=&a[0] else{return Err(Error::Invalid("render font name"));};self.state.size=num(&a[1])?;self.state.face=Some(self.load(doc,page,n,substitutes,owner)?);}
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
