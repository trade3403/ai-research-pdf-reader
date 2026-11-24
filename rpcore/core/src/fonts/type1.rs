//! Bounded Type 1 font reader for conventional Adobe-style font programs.
//! Only data declarations are read; no PostScript interpreter is executed.
use crate::{graphics::{Path,Segment},text::{Matrix,Point},Error,Result};
use std::collections::BTreeMap;
const MAX_BYTES:usize=16*1024*1024;
const MAX_ENTRIES:usize=65535;
const MAX_TOKENS:usize=1_000_000;
const MAX_OPS:usize=100_000;
const MAX_DEPTH:usize=16;
const MAX_STACK:usize=24;
fn decrypt(data:&[u8],mut key:u16)->Vec<u8>{
    data.iter().map(|&c|{let p=c^(key>>8) as u8;key=(u16::from(c).wrapping_add(key)).wrapping_mul(52845).wrapping_add(22719);p}).collect()
}
fn white(b:u8)->bool{matches!(b,0|9|10|12|13|32)}
fn delimiter(b:u8)->bool{white(b)||matches!(b,b'('|b')'|b'<'|b'>'|b'['|b']'|b'{'|b'}'|b'/'|b'%')}
struct Scan<'a>{d:&'a[u8],p:usize,count:usize}
impl<'a> Scan<'a>{
    fn new(d:&'a[u8])->Self{Self{d,p:0,count:0}}
    fn next(&mut self)->Result<Option<&'a[u8]>>{
        loop{
            while self.d.get(self.p).is_some_and(|&b|white(b)){self.p+=1;}
            if self.d.get(self.p)==Some(&b'%'){while self.d.get(self.p).is_some_and(|&b|b!=b'\n'&&b!=b'\r'){self.p+=1;}}else{break;}
        }
        let Some(&b)=self.d.get(self.p)else{return Ok(None)};
        self.count+=1;if self.count>MAX_TOKENS{return Err(Error::Limit("Type1 tokens"));}
        let start=self.p;self.p+=1;
        match b{
            b'('=>{
                let mut depth=1usize;
                while depth>0{
                    let c=*self.d.get(self.p).ok_or(Error::Invalid("Type1 string"))?;self.p+=1;
                    match c{b'\\'=>{if self.p>=self.d.len(){return Err(Error::Invalid("Type1 escape"));}self.p+=1;},b'('=>{depth+=1;if depth>64{return Err(Error::Limit("Type1 string depth"));}},b')'=>depth-=1,_=>{}}
                }
            }
            b'<' if self.d.get(self.p)!=Some(&b'<')=>{
                while self.d.get(self.p).is_some_and(|&v|v!=b'>'){self.p+=1;}
                if self.p==self.d.len(){return Err(Error::Invalid("Type1 hex string"));}self.p+=1;
            }
            b'<'|b'>' if self.d.get(self.p)==Some(&b)=>self.p+=1,
            b'/' =>{while self.d.get(self.p).is_some_and(|&v|!delimiter(v)){self.p+=1;}}
            _ if !delimiter(b)=>{while self.d.get(self.p).is_some_and(|&v|!delimiter(v)){self.p+=1;}}
            _=>{}
        }
        if self.p-start>MAX_BYTES{return Err(Error::Limit("Type1 token"));}
        Ok(Some(&self.d[start..self.p]))
    }
    fn required(&mut self)->Result<&'a[u8]>{self.next()?.ok_or(Error::Invalid("truncated Type1 declaration"))}
    fn binary(&mut self,n:usize)->Result<&'a[u8]>{
        let b=*self.d.get(self.p).ok_or(Error::Invalid("Type1 RD separator"))?;
        if !white(b){return Err(Error::Invalid("Type1 RD separator"));}
        self.p+=1;
        // RD consumes exactly one separator byte, not arbitrary whitespace in binary.
        let end=self.p.checked_add(n).ok_or(Error::Limit("Type1 RD length"))?;
        let v=self.d.get(self.p..end).ok_or(Error::Invalid("truncated Type1 RD data"))?;self.p=end;Ok(v)
    }
}
fn integer(t:&[u8])->Result<i32>{std::str::from_utf8(t).ok().and_then(|s|s.parse().ok()).ok_or(Error::Invalid("Type1 integer"))}
fn count(t:&[u8],max:usize)->Result<usize>{let n=usize::try_from(integer(t)?).map_err(|_|Error::Invalid("Type1 count"))?;if n>max{return Err(Error::Limit("Type1 count"));}Ok(n)}
fn real(t:&[u8])->Result<f64>{std::str::from_utf8(t).ok().and_then(|s|s.parse::<f64>().ok()).filter(|v|v.is_finite()&&v.abs()<=1e6).ok_or(Error::Invalid("Type1 FontMatrix"))}
fn hex(data:&[u8])->Result<Vec<u8>>{
    let mut out=Vec::new();let mut high=None;
    for &b in data{if white(b){continue;}let n=match b{b'0'..=b'9'=>b-b'0',b'a'..=b'f'=>b-b'a'+10,b'A'..=b'F'=>b-b'A'+10,_=>return Err(Error::Invalid("Type1 eexec hex"))};
        if let Some(h)=high.take(){out.push(h*16+n);}else{high=Some(n);}
    }
    if high.is_some(){return Err(Error::Invalid("odd Type1 eexec hex"));}Ok(out)
}
fn standard(code:u8)->Option<&'static [u8]>{
    // Adobe StandardEncoding (not Unicode/WinAnsi).
    const ASCII:&str="space exclam quotedbl numbersign dollar percent ampersand quoteright parenleft parenright asterisk plus comma hyphen period slash zero one two three four five six seven eight nine colon semicolon less equal greater question at A B C D E F G H I J K L M N O P Q R S T U V W X Y Z bracketleft backslash bracketright asciicircum underscore quoteleft a b c d e f g h i j k l m n o p q r s t u v w x y z braceleft bar braceright asciitilde";
    if (32..=126).contains(&code){return ASCII.split_ascii_whitespace().nth(usize::from(code-32)).map(str::as_bytes);}
    Some(match code{
        161=>b"exclamdown",162=>b"cent",163=>b"sterling",164=>b"fraction",165=>b"yen",166=>b"florin",167=>b"section",168=>b"currency",169=>b"quotesingle",170=>b"quotedblleft",171=>b"guillemotleft",172=>b"guilsinglleft",173=>b"guilsinglright",174=>b"fi",175=>b"fl",177=>b"endash",178=>b"dagger",179=>b"daggerdbl",180=>b"periodcentered",182=>b"paragraph",183=>b"bullet",184=>b"quotesinglbase",185=>b"quotedblbase",186=>b"quotedblright",187=>b"guillemotright",188=>b"ellipsis",189=>b"perthousand",191=>b"questiondown",193=>b"grave",194=>b"acute",195=>b"circumflex",196=>b"tilde",197=>b"macron",198=>b"breve",199=>b"dotaccent",200=>b"dieresis",202=>b"ring",203=>b"cedilla",205=>b"hungarumlaut",206=>b"ogonek",207=>b"caron",208=>b"emdash",225=>b"AE",227=>b"ordfeminine",232=>b"Lslash",233=>b"Oslash",234=>b"OE",235=>b"ordmasculine",241=>b"ae",245=>b"dotlessi",248=>b"lslash",249=>b"oslash",250=>b"oe",251=>b"germandbls",_=>return None,
    })
}
pub fn standard_name(code:u8)->Option<&'static [u8]>{standard(code)}
#[derive(Debug)]
pub struct Type1{matrix:Matrix,encoding:BTreeMap<u8,Vec<u8>>,glyphs:BTreeMap<Vec<u8>,Vec<u8>>,subrs:BTreeMap<usize,Vec<u8>>}
fn program(data:&[u8],len_iv:i32)->Result<Vec<u8>>{
    if len_iv==-1{return Ok(data.to_vec());}
    let n=usize::try_from(len_iv).map_err(|_|Error::Invalid("Type1 lenIV"))?;
    if n>32||n>data.len(){return Err(Error::Invalid("Type1 lenIV"));}
    let b=decrypt(data,4330);Ok(b[n..].to_vec())
}
impl Type1{
    /// Parse PDF Type1 FontFile (lengths are decoded stream byte counts).
    /// ASCII-hex eexec is also accepted when the supplied encrypted section is hex.
    pub fn parse_pdf(data:&[u8],length1:usize,length2:usize,length3:usize)->Result<Self>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("Type1 font bytes"));}
        let end=length1.checked_add(length2).ok_or(Error::Limit("Type1 lengths"))?;
        if length1==0||length2==0||end.checked_add(length3)!=Some(data.len()){return Err(Error::Invalid("Type1 FontFile lengths"));}
        let enc=&data[length1..end];
        // PDF normally embeds binary eexec. The ASCII-hex variant is detected by
        // four leading non-whitespace hex digits (the Adobe convention).
        let ascii=enc.iter().copied().filter(|&b|!white(b)).take(4).collect::<Vec<_>>();
        if ascii.len()==4&&ascii.iter().all(u8::is_ascii_hexdigit){Self::from_sections(&data[..length1],&hex(enc)?)}else{Self::from_sections(&data[..length1],enc)}
    }
    /// PFB container: ASCII cleartext, binary eexec blocks, ASCII trailer, EOF.
    pub fn parse_pfb(data:&[u8])->Result<Self>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("Type1 font bytes"));}
        let(mut p,mut stage)=(0usize,0u8);let mut clear=Vec::new();let mut enc=Vec::new();let mut blocks=0usize;
        loop{
            blocks+=1;if blocks>1024{return Err(Error::Limit("PFB blocks"));}
            let head=data.get(p..p+2).ok_or(Error::Invalid("PFB header"))?;p+=2;
            if head[0]!=0x80{return Err(Error::Invalid("PFB marker"));}
            if head[1]==3{if p!=data.len()||enc.is_empty(){return Err(Error::Invalid("PFB EOF"));}break;}
            let n=u32::from_le_bytes(data.get(p..p+4).ok_or(Error::Invalid("PFB length"))?.try_into().unwrap()) as usize;p+=4;
            let end=p.checked_add(n).ok_or(Error::Limit("PFB length"))?;
            let b=data.get(p..end).ok_or(Error::Invalid("PFB segment"))?;p=end;
            match(head[1],stage){(1,0)=>clear.extend_from_slice(b),(2,0|1)=>{stage=1;enc.extend_from_slice(b);},(1,1|2)=>stage=2,_=>return Err(Error::Invalid("PFB segment order"))}
        }
        Self::from_sections(&clear,&enc)
    }
    /// PFA with ASCII-hex eexec and the conventional cleartomark trailer.
    pub fn parse_pfa(data:&[u8])->Result<Self>{
        if data.len()>MAX_BYTES{return Err(Error::Limit("Type1 font bytes"));}
        let mut s=Scan::new(data);
        while let Some(t)=s.next()?{if t==b"eexec"{
            let start=s.p;let tail=&data[start..];let end=tail.windows(11).rposition(|w|w==b"cleartomark").ok_or(Error::Invalid("PFA cleartomark"))?;
            return Self::from_sections(&data[..start],&hex(&tail[..end])?);
        }}
        Err(Error::Invalid("PFA eexec"))
    }
    fn from_sections(clear:&[u8],encrypted:&[u8])->Result<Self>{
        if !clear.starts_with(b"%!")||encrypted.len()<4{return Err(Error::Invalid("Type1 header"));}
        let mut s=Scan::new(clear);let mut matrix=None;let mut encoding=BTreeMap::new();let mut eexec=false;
        let mut in_encoding=false;
        while let Some(t)=s.next()?{
            match t{
                b"/FontMatrix"=>{
                    if !matches!(s.required()?,b"["|b"{"){return Err(Error::Invalid("Type1 FontMatrix"));}let mut m=[0.;6];
                    for v in &mut m{*v=real(s.required()?)?;}if !matches!(s.required()?,b"]"|b"}"){return Err(Error::Invalid("Type1 FontMatrix end"));}matrix=Some(Matrix(m));
                }
                b"/Encoding"=>{
                    in_encoding=true;let v=s.required()?;
                    if v==b"StandardEncoding"{for c in 0..=255u8{if let Some(n)=standard(c){encoding.insert(c,n.to_vec());}}}
                    else if v!=b"256"{return Err(Error::Unsupported("Type1 Encoding declaration"));}
                }
                b"dup" if in_encoding=>{
                    // Conventional "dup code /name put"; other dup idioms rejected.
                    let code=s.required()?;let name=s.required()?;let put=s.required()?;
                    let n=count(code,255)?;
                    if !name.starts_with(b"/")||name.len()<2||name.len()>256||put!=b"put"{return Err(Error::Unsupported("Type1 Encoding entry"));}
                    encoding.insert(n as u8,name[1..].to_vec());
                }
                b"def" if in_encoding=>in_encoding=false,
                b"eexec"=>{eexec=true;break;}
                _=>{}
            }
        }
        if !eexec{return Err(Error::Invalid("Type1 missing eexec"));}
        let matrix=matrix.ok_or(Error::Invalid("Type1 missing FontMatrix"))?;
        let plain=decrypt(encrypted,55665);let mut s=Scan::new(&plain[4..]);
        let mut len_iv=4;let mut subrs=BTreeMap::new();let mut glyphs=BTreeMap::new();let mut sub_count=None;let mut glyph_count=None;let mut chars=false;
        let mut prev:Vec<&[u8]>=Vec::new();
        while let Some(t)=s.next()?{
            match t{
                b"/lenIV"=>{if !subrs.is_empty()||!glyphs.is_empty(){return Err(Error::Invalid("late Type1 lenIV"));}len_iv=integer(s.required()?)?;if !(-1..=32).contains(&len_iv){return Err(Error::Invalid("Type1 lenIV"));}prev.clear();}
                b"/Subrs" if glyph_count.is_none()=>{if sub_count.is_some(){return Err(Error::Invalid("duplicate Type1 Subrs"));}sub_count=Some(count(s.required()?,MAX_ENTRIES)?);if s.required()?!=b"array"{return Err(Error::Unsupported("Type1 Subrs declaration"));}prev.clear();}
                b"/CharStrings"=>{if glyph_count.is_some(){return Err(Error::Invalid("duplicate CharStrings"));}let n=count(s.required()?,MAX_ENTRIES)?;if n==0||s.required()?!=b"dict"{return Err(Error::Invalid("Type1 CharStrings declaration"));}glyph_count=Some(n);prev.clear();}
                b"begin" if glyph_count.is_some()&&!chars=>{chars=true;prev.clear();}
                b"end" if chars=>break,
                b"RD"|b"-|"=>{
                    if chars{
                        if prev.len()<2{return Err(Error::Invalid("Type1 glyph RD"));}
                        let name=prev[prev.len()-2];if !name.starts_with(b"/")||name.len()<2||name.len()>256{return Err(Error::Invalid("Type1 glyph name"));}
                        let n=count(prev[prev.len()-1],MAX_BYTES)?;
                        if glyphs.len()>=glyph_count.unwrap_or(0){return Err(Error::Invalid("Type1 CharStrings count"));}
                        let bytes=program(s.binary(n)?,len_iv)?;
                        if glyphs.insert(name[1..].to_vec(),bytes).is_some(){return Err(Error::Invalid("duplicate Type1 glyph"));}
                    }else if let Some(total)=sub_count{
                        if prev.len()<3||prev[prev.len()-3]!=b"dup"{return Err(Error::Invalid("Type1 Subrs RD"));}
                        let index=count(prev[prev.len()-2],MAX_ENTRIES)?;let n=count(prev[prev.len()-1],MAX_BYTES)?;
                        if index>=total{return Err(Error::Invalid("Type1 Subrs index"));}
                        let bytes=program(s.binary(n)?,len_iv)?;
                        if subrs.insert(index,bytes).is_some(){return Err(Error::Invalid("duplicate Type1 Subr"));}
                    }else{ return Err(Error::Unsupported("Type1 binary declaration")); }
                    prev.clear();
                }
                b"closefile"=>break,
                _=>{if prev.len()==3{prev.remove(0);}prev.push(t);}
            }
        }
        if glyphs.is_empty()||!chars{return Err(Error::Invalid("missing Type1 CharStrings"));}
        Ok(Self{matrix,encoding,glyphs,subrs})
    }
    pub fn font_matrix(&self)->Matrix{self.matrix}
    pub fn glyph_count(&self)->usize{self.glyphs.len()}
    pub fn glyph_name(&self,c:u8)->Option<&[u8]>{self.encoding.get(&c).map(Vec::as_slice)}
    pub fn outline(&self,name:&[u8])->Result<Path>{
        let bytes=self.glyphs.get(name).ok_or(Error::Unsupported("missing Type1 glyph"))?;
        let mut vm=Vm{font:self,stack:Vec::new(),path:Vec::new(),p:[0.,0.],start:None,metrics:false,ops:0,flex:None,returns:Vec::new()};
        if vm.run(bytes,0)?!=Flow::End||!vm.metrics||vm.flex.is_some()||!vm.returns.is_empty(){return Err(Error::Invalid("Type1 glyph termination or metrics"));}
        Ok(vm.path)
    }
}
#[derive(PartialEq,Eq)]
enum Flow{End,Return}
struct Vm<'a>{font:&'a Type1,stack:Vec<f64>,path:Path,p:Point,start:Option<Point>,metrics:bool,ops:usize,flex:Option<Vec<Point>>,returns:Vec<f64>}
impl Vm<'_>{
    fn push(&mut self,v:f64)->Result<()>{if self.stack.len()>=MAX_STACK{return Err(Error::Limit("Type1 operand stack"));}if !v.is_finite()||v.abs()>1e9{return Err(Error::Limit("Type1 operand"));}self.stack.push(v);Ok(())}
    fn args<const N:usize>(&mut self)->Result<[f64;N]>{if self.stack.len()!=N{return Err(Error::Invalid("Type1 operand count"));}Ok(std::mem::take(&mut self.stack).try_into().unwrap())}
    fn rel(&mut self,x:f64,y:f64)->Result<Point>{let p=[self.p[0]+x,self.p[1]+y];if p.iter().any(|v|!v.is_finite()||v.abs()>1e9){return Err(Error::Limit("Type1 coordinates"));}self.p=p;Ok(p)}
    fn close(&mut self){if self.start.take().is_some(){self.path.push(Segment::Close);}}
    fn mov(&mut self,x:f64,y:f64)->Result<()>{if !self.metrics{return Err(Error::Invalid("Type1 missing metrics"));}if self.flex.is_some(){self.rel(x,y)?;return Ok(());}self.close();let p=self.rel(x,y)?;self.path.push(Segment::Move(p));self.start=Some(p);Ok(())}
    fn line(&mut self,x:f64,y:f64)->Result<()>{if self.flex.is_some(){return Err(Error::Invalid("Type1 line during flex"));}if self.start.is_none(){return Err(Error::Invalid("Type1 line without moveto"));}let p=self.rel(x,y)?;self.path.push(Segment::Line(p));Ok(())}
    fn curve(&mut self,a:[f64;6])->Result<()>{if self.flex.is_some(){return Err(Error::Invalid("Type1 curve during flex"));}if self.start.is_none(){return Err(Error::Invalid("Type1 curve without moveto"));}let p=self.rel(a[0],a[1])?;let q=self.rel(a[2],a[3])?;let r=self.rel(a[4],a[5])?;self.path.push(Segment::Cubic(p,q,r));Ok(())}
    fn other(&mut self)->Result<()>{
        let id=self.stack.pop().ok_or(Error::Invalid("Type1 OtherSubr number"))?;
        let n=self.stack.pop().ok_or(Error::Invalid("Type1 OtherSubr count"))?;
        if n<0.||n.fract()!=0.||n>self.stack.len() as f64{return Err(Error::Invalid("Type1 OtherSubr count"));}
        let a=self.stack.split_off(self.stack.len()-n as usize);
        if !self.returns.is_empty(){return Err(Error::Invalid("unconsumed Type1 OtherSubr results"));}
        match (id,a.len()){
            (1.,0)=>{
                if self.flex.is_some()||self.start.is_none(){return Err(Error::Invalid("Type1 flex start"));}
                self.flex=Some(Vec::with_capacity(7));
            }
            (2.,0)=>{
                let points=self.flex.as_mut().ok_or(Error::Invalid("Type1 flex point"))?;
                if points.len()>=7{return Err(Error::Invalid("Type1 flex point count"));}
                points.push(self.p);
            }
            (0.,3)=>{
                let points=self.flex.take().ok_or(Error::Invalid("Type1 flex end"))?;
                if points.len()!=7{return Err(Error::Invalid("Type1 flex point count"));}
                // First recorded point is the reference point; the following six
                // are the two cubic control/control/end triples. Hint depth is
                // intentionally not executed. The supplied endpoint is returned
                // to CharString via two pop operators followed by setcurrentpoint.
                self.path.push(Segment::Cubic(points[1],points[2],points[3]));
                self.path.push(Segment::Cubic(points[4],points[5],points[6]));
                self.p=points[6];
                self.returns.extend([a[2],a[1]]);
            }
            (3.,1)=>{
                // Standard hint replacement returns the requested Subr number.
                // Hint instructions are ignored, but its CharString still runs.
                self.returns.push(a[0]);
            }
            _=>return Err(Error::Unsupported("Type1 OtherSubrs/flex")),
        }Ok(())
    }
    fn run(&mut self,d:&[u8],depth:usize)->Result<Flow>{
        if depth>MAX_DEPTH{return Err(Error::Limit("Type1 subroutine depth"));}
        let mut i=0usize;
        fn byte(d:&[u8],i:&mut usize)->Result<u8>{let b=*d.get(*i).ok_or(Error::Invalid("truncated Type1 CharString"))?;*i+=1;Ok(b)}
        while i<d.len(){
            self.ops+=1;if self.ops>MAX_OPS{return Err(Error::Limit("Type1 CharString operations"));}
            let b=byte(d,&mut i)?;
            if b>=32{
                let n=match b{32..=246=>f64::from(b)-139.,247..=250=>(f64::from(b)-247.)*256.+f64::from(byte(d,&mut i)?)+108.,251..=254=>-(f64::from(b)-251.)*256.-f64::from(byte(d,&mut i)?)-108.,255=>{let mut a=[0;4];for v in &mut a{*v=byte(d,&mut i)?;}f64::from(i32::from_be_bytes(a))},_=>unreachable!()};
                self.push(n)?;continue;
            }
            match b{
                1|3=>{let _: [f64;2]=self.args()?;}
                4=>{let [y]=self.args()?;self.mov(0.,y)?;}
                5=>{let [x,y]=self.args()?;self.line(x,y)?;}
                6=>{let [x]=self.args()?;self.line(x,0.)?;}
                7=>{let [y]=self.args()?;self.line(0.,y)?;}
                8=>{let a=self.args()?;self.curve(a)?;}
                9=>{let _: [f64;0]=self.args()?;if self.start.is_none()||self.flex.is_some(){return Err(Error::Invalid("Type1 closepath"));}self.close();}
                10=>{
                    let n=self.stack.pop().ok_or(Error::Invalid("Type1 callsubr operand"))?;
                    if n<0.||n>MAX_ENTRIES as f64||n.fract()!=0.{return Err(Error::Invalid("Type1 callsubr index"));}
                    let font=self.font;let sub=font.subrs.get(&(n as usize)).ok_or(Error::Invalid("missing Type1 Subr"))?;
                    if self.run(sub,depth+1)?==Flow::End{return Ok(Flow::End);}
                }
                11=>{if depth==0{return Err(Error::Invalid("Type1 top-level return"));}return Ok(Flow::Return);}
                13=>{let [x,_w]=self.args()?;if self.metrics{return Err(Error::Invalid("duplicate Type1 hsbw"));}self.p=[x,0.];self.metrics=true;}
                14=>{let _: [f64;0]=self.args()?;self.close();return Ok(Flow::End);}
                21=>{let [x,y]=self.args()?;self.mov(x,y)?;}
                22=>{let [x]=self.args()?;self.mov(x,0.)?;}
                30=>{let [a,b,c,d]=self.args()?;self.curve([0.,a,b,c,d,0.])?;}
                31=>{let [a,b,c,d]=self.args()?;self.curve([a,0.,b,c,0.,d])?;}
                12=>match byte(d,&mut i)?{
                    0=>{let _: [f64;0]=self.args()?;} // dotsection hint
                    1|2=>{let _: [f64;6]=self.args()?;} // stem3 hints
                    7=>{let [x,y,_wx,_wy]=self.args()?;if self.metrics{return Err(Error::Invalid("duplicate Type1 sbw"));}self.p=[x,y];self.metrics=true;}
                    12=>{let b=self.stack.pop().ok_or(Error::Invalid("Type1 div"))?;let a=self.stack.pop().ok_or(Error::Invalid("Type1 div"))?;if b==0.{return Err(Error::Invalid("Type1 div by zero"));}self.push(a/b)?;}
                    6=>return Err(Error::Unsupported("Type1 seac")),
                    16=>self.other()?,
                    17=>{let v=self.returns.pop().ok_or(Error::Invalid("Type1 OtherSubr pop"))?;self.push(v)?;}
                    33=>{let [x,y]=self.args()?;if self.flex.is_some(){return Err(Error::Invalid("Type1 setcurrentpoint during flex"));}self.p=[x,y];}
                    _=>return Err(Error::Unsupported("Type1 escaped operator")),
                }
                _=>return Err(Error::Unsupported("Type1 operator")),
            }
        }
        Err(Error::Invalid("truncated Type1 CharString"))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn op(v:&mut Vec<u8>,a:&[i32],o:&[u8]){
        for &n in a{if (-107..=107).contains(&n){v.push((n+139)as u8);}else{v.push(255);v.extend_from_slice(&n.to_be_bytes());}}
        v.extend_from_slice(o);
    }
    fn font(g:Vec<u8>)->Type1{
        Type1{matrix:Matrix([0.001,0.,0.,0.001,0.,0.]),encoding:BTreeMap::new(),
            glyphs:BTreeMap::from([(b"A".to_vec(),g)]),subrs:BTreeMap::from([(4,vec![139,159,3,11])])}
    }
    #[test]fn standard_hint_replacement(){
        let mut g=Vec::new();op(&mut g,&[0,650],&[13]);op(&mut g,&[4,1,3],&[12,16]);
        g.extend([12,17,10]);op(&mut g,&[10,20],&[21]);op(&mut g,&[50,0],&[5]);g.push(14);
        assert_eq!(font(g).outline(b"A").unwrap(),vec![Segment::Move([10.,20.]),Segment::Line([60.,20.]),Segment::Close]);
    }
    #[test]fn flex_two_cubics(){
        let mut g=Vec::new();op(&mut g,&[0,650],&[13]);op(&mut g,&[10,20],&[21]);op(&mut g,&[0,1],&[12,16]);
        for (x,y) in [(0,0),(10,0),(10,10),(10,10),(10,0),(10,-10),(10,-10)]{
            op(&mut g,&[x,y],&[21]);op(&mut g,&[0,2],&[12,16]);
        }
        op(&mut g,&[50,70,20,3,0],&[12,16]);g.extend([12,17,12,17,12,33]);op(&mut g,&[10,0],&[5]);g.push(14);
        assert_eq!(font(g).outline(b"A").unwrap(),vec![Segment::Move([10.,20.]),Segment::Cubic([20.,20.],[30.,30.],[40.,40.]),Segment::Cubic([50.,40.],[60.,30.],[70.,20.]),Segment::Line([80.,20.]),Segment::Close]);
    }
    #[test]fn unsupported_and_malformed_othersubrs(){
        for (n,id) in [(0,2),(0,9),(1,3)]{let mut g=Vec::new();op(&mut g,&[0,650],&[13]);op(&mut g,&[n,id],&[12,16]);g.push(14);assert!(font(g).outline(b"A").is_err());}
    }
}
