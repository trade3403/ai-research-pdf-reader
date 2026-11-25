//! C4.3b: bounded unhinted TrueType composite assembly in signed 16.16 units.
use crate::{Error,TrueTypeOutlines,GlyphOutlineKind,FontBounds};
pub const COMPOSITE_ONE:i64=65536;
const COORD_LIMIT:i64=1<<46;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ComponentPlacement{Offset{x:i16,y:i16},Points{parent:u16,child:u16}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ComponentMatrix{pub xx:i16,pub xy:i16,pub yx:i16,pub yy:i16}
impl Default for ComponentMatrix{fn default()->Self{Self{xx:16384,xy:0,yx:0,yy:16384}}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GlyphComponent{
    pub glyph_id:u16,pub placement:ComponentPlacement,pub matrix:ComponentMatrix,
    pub scaled_offset:bool,pub round_to_grid:bool,pub use_my_metrics:bool,pub overlap:bool,
}
#[derive(Debug)]
pub struct CompositeGlyph<'a>{pub bounds:FontBounds,pub components:Vec<GlyphComponent>,pub instructions:&'a [u8]}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct CompositeLimits{
    pub max_depth:usize,pub max_components:usize,pub max_visits:usize,
    pub max_points:usize,pub max_contours:usize,pub max_instruction_bytes:usize,
}
impl Default for CompositeLimits{
    fn default()->Self{Self{max_depth:16,max_components:1024,max_visits:4096,
        max_points:65536,max_contours:4096,max_instruction_bytes:65535}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FixedOutlinePoint{pub x:i64,pub y:i64,pub on_curve:bool}
#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct AssembledGlyph{
    pub points:Vec<FixedOutlinePoint>,pub contour_ends:Vec<usize>,
    /// Effective unhinted metrics source ID (not transformed advance metrics).
    pub metrics_glyph:u16,pub overlap:bool,
}
fn take<'a>(b:&'a [u8],at:&mut usize,n:usize)->Result<&'a [u8],Error>{
    let end=at.checked_add(n).ok_or(Error::Limit("composite offset"))?;
    let s=b.get(*at..end).ok_or(Error::Invalid("composite truncated bytes"))?;
    *at=end;Ok(s)
}
fn word(b:&[u8],at:&mut usize)->Result<u16,Error>{Ok(u16::from_be_bytes(take(b,at,2)?.try_into().expect("two bytes")))}
fn signed(b:&[u8],at:&mut usize)->Result<i16,Error>{Ok(word(b,at)? as i16)}
/// Parse components only; no hint execution or recursion here.
pub fn parse_composite_glyph<'a>(font:&TrueTypeOutlines<'a>,id:u16,l:CompositeLimits)->Result<CompositeGlyph<'a>,Error>{
    let b=font.glyph_bytes(id)?;let mut at=0;
    if signed(b,&mut at)? >=0{return Err(Error::Unsupported("composite glyph required"));}
    let bounds=FontBounds{x_min:signed(b,&mut at)?,y_min:signed(b,&mut at)?,x_max:signed(b,&mut at)?,y_max:signed(b,&mut at)?};
    if bounds.x_min>bounds.x_max||bounds.y_min>bounds.y_max{return Err(Error::Invalid("composite reversed bounds"));}
    let mut components=Vec::new();let mut has_instructions=false;let mut metrics=false;
    loop{
        if components.len()>=l.max_components{return Err(Error::Limit("composite components"));}
        let flags=word(b,&mut at)?;let glyph_id=word(b,&mut at)?;
        if flags&!0x1fef!=0{return Err(Error::Invalid("composite reserved flags"));}
        if usize::from(glyph_id)>=font.glyph_count(){return Err(Error::Invalid("composite glyph reference"));}
        let words=flags&1!=0;let xy=flags&2!=0;
        if components.is_empty()&&!xy{return Err(Error::Invalid("composite first point attachment"));}
        if flags&0x1800==0x1800{return Err(Error::Invalid("composite conflicting offset flags"));}
        let placement=if xy{
            let (x,y)=if words{(signed(b,&mut at)?,signed(b,&mut at)?)}else{
                let a=take(b,&mut at,2)?;(i16::from(a[0] as i8),i16::from(a[1] as i8))
            };
            ComponentPlacement::Offset{x,y}
        }else{
            let (parent,child)=if words{(word(b,&mut at)?,word(b,&mut at)?)}else{
                let a=take(b,&mut at,2)?;(u16::from(a[0]),u16::from(a[1]))
            };
            ComponentPlacement::Points{parent,child}
        };
        let mut matrix=ComponentMatrix::default();
        match flags&0xc8{
            0=>{},
            8=>{let s=signed(b,&mut at)?;matrix.xx=s;matrix.yy=s;},
            0x40=>{matrix.xx=signed(b,&mut at)?;matrix.yy=signed(b,&mut at)?;},
            0x80=>{matrix.xx=signed(b,&mut at)?;matrix.yx=signed(b,&mut at)?;
                matrix.xy=signed(b,&mut at)?;matrix.yy=signed(b,&mut at)?;},
            _=>return Err(Error::Invalid("composite conflicting transform flags")),
        }
        let use_my_metrics=flags&0x200!=0;
        if use_my_metrics&&metrics{return Err(Error::Invalid("composite multiple metric sources"));}
        metrics|=use_my_metrics;has_instructions|=flags&0x100!=0;
        components.push(GlyphComponent{glyph_id,placement,matrix,scaled_offset:flags&0x800!=0,
            round_to_grid:flags&4!=0,use_my_metrics,overlap:flags&0x400!=0});
        if flags&0x20==0{break;}
    }
    let instructions=if has_instructions{
        let n=usize::from(word(b,&mut at)?);
        if n>l.max_instruction_bytes{return Err(Error::Limit("composite instructions"));}
        take(b,&mut at,n)?
    }else{&b[at..at]};
    if b[at..].iter().any(|&v|v!=0){return Err(Error::Invalid("composite trailing data"));}
    Ok(CompositeGlyph{bounds,components,instructions})
}
fn coord(v:i128)->Result<i64,Error>{
    if v.abs()>i128::from(COORD_LIMIT){return Err(Error::Limit("composite coordinate"));}
    Ok(v as i64)
}
fn rounded(n:i128)->Result<i64,Error>{
    let q=n/16384;let r=n%16384;
    coord(q+if r.abs()*2>=16384{n.signum()}else{0})
}
fn transform(m:ComponentMatrix,x:i64,y:i64)->Result<(i64,i64),Error>{
    Ok((rounded(i128::from(m.xx)*i128::from(x)+i128::from(m.xy)*i128::from(y))?,
        rounded(i128::from(m.yx)*i128::from(x)+i128::from(m.yy)*i128::from(y))?))
}
fn plus(a:usize,b:usize)->Result<usize,Error>{a.checked_add(b).ok_or(Error::Limit("composite count"))}
struct Work{visits:usize,components:usize,points:usize,contours:usize}
fn charge(w:&mut Work,points:usize,contours:usize,l:CompositeLimits)->Result<(),Error>{
    w.points=plus(w.points,points)?;w.contours=plus(w.contours,contours)?;
    if w.points>l.max_points||w.contours>l.max_contours{return Err(Error::Limit("composite cumulative geometry"));}
    Ok(())
}
fn assemble(font:&TrueTypeOutlines<'_>,id:u16,l:CompositeLimits,stack:&mut Vec<u16>,w:&mut Work)->Result<AssembledGlyph,Error>{
    if stack.len()>l.max_depth.min(64){return Err(Error::Limit("composite recursion depth"));}
    if stack.contains(&id){return Err(Error::Invalid("composite cycle"));}
    w.visits=plus(w.visits,1)?;
    if w.visits>l.max_visits{return Err(Error::Limit("composite visits"));}
    if font.glyph_kind(id)?!=GlyphOutlineKind::Composite{
        let g=font.simple_glyph(id)?;
        charge(w,g.points().len(),g.contour_ends().len(),l)?;
        return Ok(AssembledGlyph{points:g.points().iter().map(|p|FixedOutlinePoint{x:i64::from(p.x)*COMPOSITE_ONE,
            y:i64::from(p.y)*COMPOSITE_ONE,on_curve:p.on_curve}).collect(),contour_ends:g.contour_ends().to_vec(),
            metrics_glyph:id,overlap:g.overlap_simple()});
    }
    let g=parse_composite_glyph(font,id,l)?;
    w.components=plus(w.components,g.components.len())?;
    if w.components>l.max_components{return Err(Error::Limit("composite cumulative components"));}
    stack.push(id);
    let mut out=AssembledGlyph{metrics_glyph:id,..Default::default()};
    for c in g.components{
        let mut child=assemble(font,c.glyph_id,l,stack,w)?;
        for p in &mut child.points{(p.x,p.y)=transform(c.matrix,p.x,p.y)?;}
        let (dx,dy)=match c.placement{
            ComponentPlacement::Offset{x,y}=>{
                let (x,y)=(i64::from(x)*COMPOSITE_ONE,i64::from(y)*COMPOSITE_ONE);
                if c.scaled_offset{transform(c.matrix,x,y)?}else{(x,y)}
            },
            ComponentPlacement::Points{parent,child:child_id}=>{
                let a=out.points.get(usize::from(parent)).ok_or(Error::Unsupported("composite parent attachment point or phantom"))?;
                let b=child.points.get(usize::from(child_id)).ok_or(Error::Unsupported("composite child attachment point or phantom"))?;
                (coord(i128::from(a.x)-i128::from(b.x))?,coord(i128::from(a.y)-i128::from(b.y))?)
            },
        };
        let base=out.points.len();
        if plus(base,child.points.len())?>l.max_points||plus(out.contour_ends.len(),child.contour_ends.len())?>l.max_contours{
            return Err(Error::Limit("composite assembled geometry"));
        }
        // Grid rounding is device-dependent and intentionally not applied.
        for p in &mut child.points{p.x=coord(i128::from(p.x)+i128::from(dx))?;p.y=coord(i128::from(p.y)+i128::from(dy))?;}
        for end in child.contour_ends{out.contour_ends.push(plus(base,end)?);}
        out.points.extend(child.points);
        out.overlap|=c.overlap||child.overlap;
        if c.use_my_metrics{out.metrics_glyph=child.metrics_glyph;}
    }
    stack.pop();
    Ok(out)
}
/// Assemble all real contour points, maintaining source order. Each transform
/// is rounded to 16.16, ties away from zero. Unspecified offsets use unscaled
/// semantics. No phantom points, variation deltas, hinting or pixel rounding.
/// Limits on geometry count cumulative leaf decoding, and final geometry.
pub fn assemble_true_type_glyph(font:&TrueTypeOutlines<'_>,id:u16,l:CompositeLimits)->Result<AssembledGlyph,Error>{
    assemble(font,id,l,&mut Vec::new(),&mut Work{visits:0,components:0,points:0,contours:0})
}
