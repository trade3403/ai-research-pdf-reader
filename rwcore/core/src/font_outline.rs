//! C4.3a: bounded, unhinted TrueType loca/glyf simple outlines.
use crate::{Error,FontFace,FontFlavor,FontBounds,FontMetricLimits};
use crate::font_metrics::{parse_head,parse_maxp};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct OutlineLimits{
    pub max_glyphs:usize,pub max_glyph_bytes:usize,pub max_contours:usize,
    pub max_points:usize,pub max_instruction_bytes:usize,pub max_path_commands:usize,
}
impl Default for OutlineLimits{
    fn default()->Self{Self{max_glyphs:65535,max_glyph_bytes:2*1024*1024,max_contours:4096,
        max_points:65536,max_instruction_bytes:65535,max_path_commands:131072}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GlyphOutlineKind{Empty,Simple,Composite}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct OutlinePoint{pub x:i16,pub y:i16,pub on_curve:bool}
/// Exact coordinates in half-design-units. Integer design points are even;
/// implied midpoints can be odd, without floating-point rounding.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HalfPoint{pub x2:i32,pub y2:i32}
impl HalfPoint{
    fn from_point(p:OutlinePoint)->Self{Self{x2:i32::from(p.x)*2,y2:i32::from(p.y)*2}}
    fn midpoint(a:OutlinePoint,b:OutlinePoint)->Self{Self{x2:i32::from(a.x)+i32::from(b.x),y2:i32::from(a.y)+i32::from(b.y)}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum OutlineCommand{MoveTo(HalfPoint),LineTo(HalfPoint),QuadTo{control:HalfPoint,end:HalfPoint},Close}
#[derive(Debug)]
pub struct SimpleGlyph<'a>{
    bounds:Option<FontBounds>,points:Vec<OutlinePoint>,ends:Vec<usize>,
    instructions:&'a [u8],overlap:bool,command_limit:usize,
}
impl<'a> SimpleGlyph<'a>{
    pub fn bounds(&self)->Option<FontBounds>{self.bounds}
    pub fn points(&self)->&[OutlinePoint]{&self.points}
    /// Inclusive last point indices, one per contour.
    pub fn contour_ends(&self)->&[usize]{&self.ends}
    pub fn instructions(&self)->&'a [u8]{self.instructions}
    pub fn overlap_simple(&self)->bool{self.overlap}
    /// Declared control-point bounds are advisory. Empty outlines have no
    /// computed point bounds. No curve extrema or hinted ink measurement.
    pub fn point_bounds(&self)->Option<FontBounds>{
        let first=self.points.first()?;
        let mut b=FontBounds{x_min:first.x,y_min:first.y,x_max:first.x,y_max:first.y};
        for p in &self.points{
            b.x_min=b.x_min.min(p.x);b.x_max=b.x_max.max(p.x);
            b.y_min=b.y_min.min(p.y);b.y_max=b.y_max.max(p.y);
        }
        Some(b)
    }
    pub fn path(&self)->Result<Vec<OutlineCommand>,Error>{
        let mut path=Vec::new();
        let mut start=0;
        for &end in &self.ends{
            contour_path(&self.points[start..=end],&mut path,self.command_limit)?;
            start=end+1;
        }
        Ok(path)
    }
}
fn emit(out:&mut Vec<OutlineCommand>,cmd:OutlineCommand,limit:usize)->Result<(),Error>{
    if out.len()>=limit{return Err(Error::Limit("outline path commands"));}
    out.push(cmd);Ok(())
}
fn contour_path(points:&[OutlinePoint],out:&mut Vec<OutlineCommand>,limit:usize)->Result<(),Error>{
    let first=points[0];let last=points[points.len()-1];
    let (origin,body)=if first.on_curve{
        (HalfPoint::from_point(first),&points[1..])
    }else if last.on_curve{
        (HalfPoint::from_point(last),&points[..points.len()-1])
    }else{
        (HalfPoint::midpoint(last,first),points)
    };
    emit(out,OutlineCommand::MoveTo(origin),limit)?;
    let mut pending:Option<OutlinePoint>=None;
    for &p in body{
        if p.on_curve{
            let end=HalfPoint::from_point(p);
            let cmd=match pending.take(){Some(q)=>OutlineCommand::QuadTo{control:HalfPoint::from_point(q),end},None=>OutlineCommand::LineTo(end)};
            emit(out,cmd,limit)?;
        }else{
            if let Some(q)=pending.replace(p){
                emit(out,OutlineCommand::QuadTo{control:HalfPoint::from_point(q),end:HalfPoint::midpoint(q,p)},limit)?;
            }
        }
    }
    if let Some(q)=pending{
        emit(out,OutlineCommand::QuadTo{control:HalfPoint::from_point(q),end:origin},limit)?;
    }
    emit(out,OutlineCommand::Close,limit)
}
fn add(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("outline arithmetic"))}
fn take<'a>(b:&'a [u8],at:&mut usize,n:usize)->Result<&'a [u8],Error>{
    let end=add(*at,n)?;
    let s=b.get(*at..end).ok_or(Error::Invalid("outline truncated bytes"))?;
    *at=end;Ok(s)
}
fn word(b:&[u8],at:&mut usize)->Result<u16,Error>{
    Ok(u16::from_be_bytes(take(b,at,2)?.try_into().expect("two bytes")))
}
fn signed(b:&[u8],at:&mut usize)->Result<i16,Error>{Ok(word(b,at)? as i16)}
fn header(b:&[u8],at:&mut usize)->Result<(i16,FontBounds),Error>{
    let count=signed(b,at)?;
    let bounds=FontBounds{x_min:signed(b,at)?,y_min:signed(b,at)?,x_max:signed(b,at)?,y_max:signed(b,at)?};
    if bounds.x_min>bounds.x_max||bounds.y_min>bounds.y_max{return Err(Error::Invalid("outline reversed bounds"));}
    Ok((count,bounds))
}
fn read_axis(b:&[u8],at:&mut usize,flags:&[u8],short:u8,same:u8)->Result<Vec<i16>,Error>{
    let mut values=Vec::with_capacity(flags.len());
    let mut previous=0i32;
    for &f in flags{
        let delta=if f&short!=0{
            let v=i32::from(take(b,at,1)?[0]);if f&same!=0{v}else{-v}
        }else if f&same!=0{0}else{i32::from(signed(b,at)?)};
        previous=previous.checked_add(delta).ok_or(Error::Limit("outline coordinate arithmetic"))?;
        values.push(i16::try_from(previous).map_err(|_|Error::Invalid("outline coordinate range"))?);
    }
    Ok(values)
}
fn parse_simple(b:&[u8],l:OutlineLimits)->Result<SimpleGlyph<'_>,Error>{
    if b.is_empty(){return Ok(SimpleGlyph{bounds:None,points:Vec::new(),ends:Vec::new(),instructions:b,overlap:false,command_limit:l.max_path_commands});}
    let mut at=0;
    let (count,bounds)=header(b,&mut at)?;
    if count<0{return Err(Error::Unsupported("outline composite glyph"));}
    let n=count as usize;
    if n>l.max_contours{return Err(Error::Limit("outline contours"));}
    let mut ends=Vec::with_capacity(n);
    for _ in 0..n{
        let end=usize::from(word(b,&mut at)?);
        if ends.last().is_some_and(|&p|end<=p){return Err(Error::Invalid("outline contour endpoints"));}
        ends.push(end);
    }
    let points=ends.last().map_or(0,|&e|e+1);
    if points>l.max_points{return Err(Error::Limit("outline points"));}
    // A zero-contour glyph may contain just its header.
    let instruction_len=if n==0&&at==b.len(){0}else{usize::from(word(b,&mut at)?)};
    if instruction_len>l.max_instruction_bytes{return Err(Error::Limit("outline instructions"));}
    let instructions=take(b,&mut at,instruction_len)?;
    let mut flags=Vec::with_capacity(points);
    while flags.len()<points{
        let f=take(b,&mut at,1)?[0];
        if f&0x80!=0{return Err(Error::Invalid("outline reserved flag"));}
        let repeat=if f&8!=0{usize::from(take(b,&mut at,1)?[0])+1}else{1};
        if repeat>points-flags.len(){return Err(Error::Invalid("outline flag repeat"));}
        flags.resize(flags.len()+repeat,f);
    }
    let overlap=flags.first().is_some_and(|f|f&0x40!=0);
    let xs=read_axis(b,&mut at,&flags,2,0x10)?;
    let ys=read_axis(b,&mut at,&flags,4,0x20)?;
    if b[at..].iter().any(|&v|v!=0){return Err(Error::Invalid("outline nonzero trailing data"));}
    let points=xs.into_iter().zip(ys).zip(flags).map(|((x,y),f)|OutlinePoint{x,y,on_curve:f&1!=0}).collect();
    Ok(SimpleGlyph{bounds:Some(bounds),points,ends,instructions,overlap,command_limit:l.max_path_commands})
}
#[derive(Debug)]
pub struct TrueTypeOutlines<'a>{glyf:&'a [u8],offsets:Vec<usize>,limits:OutlineLimits}
impl<'a> TrueTypeOutlines<'a>{
    pub fn glyph_count(&self)->usize{self.offsets.len()-1}
    pub fn glyph_bytes(&self,id:u16)->Result<&'a [u8],Error>{
        let i=usize::from(id);
        if i>=self.glyph_count(){return Err(Error::Invalid("outline glyph index"));}
        let a=self.offsets[i];let z=self.offsets[i+1];
        if z-a>self.limits.max_glyph_bytes{return Err(Error::Limit("outline glyph bytes"));}
        Ok(&self.glyf[a..z])
    }
    pub fn glyph_kind(&self,id:u16)->Result<GlyphOutlineKind,Error>{
        let b=self.glyph_bytes(id)?;
        if b.is_empty(){return Ok(GlyphOutlineKind::Empty);}
        let mut at=0;
        let (n,_)=header(b,&mut at)?;
        Ok(if n<0{GlyphOutlineKind::Composite}else{GlyphOutlineKind::Simple})
    }
    pub fn simple_glyph(&self,id:u16)->Result<SimpleGlyph<'a>,Error>{
        parse_simple(self.glyph_bytes(id)?,self.limits)
    }
}
/// Parse and validate the entire loca index; glyf payloads are decoded lazily.
/// Only raw TrueType default outlines. Composite records are identified but
/// not decoded; hint programs are returned as inert bytes, never executed.
pub fn parse_true_type_outlines<'a>(face:&FontFace<'a>,limits:OutlineLimits)->Result<TrueTypeOutlines<'a>,Error>{
    if face.flavor()!=FontFlavor::TrueType{return Err(Error::Unsupported("outline TrueType only"));}
    let head=parse_head(face.table(*b"head").ok_or(Error::Invalid("outline missing head"))?,face.flavor())?;
    let maxp=parse_maxp(face.table(*b"maxp").ok_or(Error::Invalid("outline missing maxp"))?,
        face.flavor(),FontMetricLimits{max_glyphs:limits.max_glyphs})?;
    let loca=face.table(*b"loca").ok_or(Error::Invalid("outline missing loca"))?;
    let glyf=face.table(*b"glyf").ok_or(Error::Invalid("outline missing glyf"))?;
    let count=usize::from(maxp.glyph_count)+1;
    let stride=if head.loca_format==0{2}else{4};
    if loca.len()!=count*stride{return Err(Error::Invalid("outline loca length"));}
    let mut offsets=Vec::with_capacity(count);
    let mut at=0;
    for _ in 0..count{
        let offset=if stride==2{usize::from(word(loca,&mut at)?)*2}else{
            usize::try_from(u32::from_be_bytes(take(loca,&mut at,4)?.try_into().expect("four bytes")))
                .map_err(|_|Error::Limit("outline loca address"))?
        };
        if offset>glyf.len()||offsets.last().is_some_and(|&p|offset<p){return Err(Error::Invalid("outline loca offsets"));}
        offsets.push(offset);
    }
    if offsets[0]!=0{return Err(Error::Invalid("outline first loca offset"));}
    Ok(TrueTypeOutlines{glyf,offsets,limits})
}
