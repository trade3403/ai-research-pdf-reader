//! C4.3c: unified unhinted quadratic paths and conservative exact-rational bounds.
use crate::{Error,AssembledGlyph,FixedOutlinePoint,TrueTypeOutlines,CompositeLimits,assemble_true_type_glyph};
/// Path coordinates use 17 fractional bits, preserving midpoints of 16.16
/// assembled points without further rounding.
pub const PATH_ONE:i64=131072;
const POINT_LIMIT:i64=1<<46;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PathLimits{pub max_points:usize,pub max_contours:usize,pub max_commands:usize}
impl Default for PathLimits{
    fn default()->Self{Self{max_points:65536,max_contours:4096,max_commands:131072}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PathPoint{pub x:i64,pub y:i64}
impl PathPoint{
    fn real(p:FixedOutlinePoint)->Self{Self{x:p.x*2,y:p.y*2}}
    fn midpoint(a:FixedOutlinePoint,b:FixedOutlinePoint)->Self{Self{x:a.x+b.x,y:a.y+b.y}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GlyphPathCommand{MoveTo(PathPoint),LineTo(PathPoint),QuadTo{control:PathPoint,end:PathPoint},Close}
/// Bounds in path units, exact when extrema are integral and otherwise
/// conservatively rounded outward by less than one path unit per side.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GlyphPathBounds{pub x_min:i64,pub y_min:i64,pub x_max:i64,pub y_max:i64}
impl GlyphPathBounds{
    fn point(p:PathPoint)->Self{Self{x_min:p.x,y_min:p.y,x_max:p.x,y_max:p.y}}
    fn include(&mut self,p:PathPoint){
        self.x_min=self.x_min.min(p.x);self.y_min=self.y_min.min(p.y);
        self.x_max=self.x_max.max(p.x);self.y_max=self.y_max.max(p.y);
    }
    fn quadratic(&mut self,a:PathPoint,b:PathPoint,c:PathPoint)->Result<(),Error>{
        self.include(a);self.include(c);
        if let Some((lo,hi))=axis_extremum(a.x,b.x,c.x)?{
            self.x_min=self.x_min.min(lo);self.x_max=self.x_max.max(hi);
        }
        if let Some((lo,hi))=axis_extremum(a.y,b.y,c.y)?{
            self.y_min=self.y_min.min(lo);self.y_max=self.y_max.max(hi);
        }
        Ok(())
    }
}
/// Evaluate the only possible interior extremum of a quadratic exactly:
/// t=(a-b)/(a-2b+c), value=a-(b-a)^2/(a-2b+c).
/// Coordinates are bounded by the validated input; all intermediates fit
/// i128 (largest products are below 2^98). Euclidean division yields floor.
fn axis_extremum(a:i64,b:i64,c:i64)->Result<Option<(i64,i64)>,Error>{
    let (a,b,c)=(i128::from(a),i128::from(b),i128::from(c));
    let d=a-2*b+c;
    if d==0{return Ok(None);}
    let t=a-b;
    if (d>0&&(t<=0||t>=d))||(d<0&&(t>=0||t<=d)){return Ok(None);}
    let mut n=a*d-(b-a)*(b-a);let mut den=d;
    if den<0{n=-n;den=-den;}
    let lo=n.div_euclid(den);let hi=lo+if n.rem_euclid(den)==0{0}else{1};
    Ok(Some((i64::try_from(lo).map_err(|_|Error::Limit("path extremum"))?,
        i64::try_from(hi).map_err(|_|Error::Limit("path extremum"))?)))
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GlyphPath{
    commands:Vec<GlyphPathCommand>,bounds:Option<GlyphPathBounds>,
    control_bounds:Option<GlyphPathBounds>,metrics_glyph:u16,overlap:bool,
}
impl GlyphPath{
    pub fn commands(&self)->&[GlyphPathCommand]{&self.commands}
    /// Geometric bounds of the closed contour paths, not hinted ink bounds.
    /// A move-only degenerate contour contributes its point.
    pub fn bounds(&self)->Option<GlyphPathBounds>{self.bounds}
    pub fn control_bounds(&self)->Option<GlyphPathBounds>{self.control_bounds}
    pub fn metrics_glyph(&self)->u16{self.metrics_glyph}
    pub fn overlap(&self)->bool{self.overlap}
}
fn emit(path:&mut Vec<GlyphPathCommand>,c:GlyphPathCommand,l:PathLimits)->Result<(),Error>{
    if path.len()>=l.max_commands{return Err(Error::Limit("glyph path commands"));}
    path.push(c);Ok(())
}
fn contour(points:&[FixedOutlinePoint],path:&mut Vec<GlyphPathCommand>,l:PathLimits)->Result<(),Error>{
    let first=points[0];let last=points[points.len()-1];
    let (origin,body)=if first.on_curve{(PathPoint::real(first),&points[1..])}
        else if last.on_curve{(PathPoint::real(last),&points[..points.len()-1])}
        else{(PathPoint::midpoint(last,first),points)};
    emit(path,GlyphPathCommand::MoveTo(origin),l)?;
    let mut pending:Option<FixedOutlinePoint>=None;
    for &p in body{
        if p.on_curve{
            let end=PathPoint::real(p);
            let cmd=match pending.take(){Some(q)=>GlyphPathCommand::QuadTo{control:PathPoint::real(q),end},
                None=>GlyphPathCommand::LineTo(end)};
            emit(path,cmd,l)?;
        }else if let Some(q)=pending.replace(p){
            emit(path,GlyphPathCommand::QuadTo{control:PathPoint::real(q),end:PathPoint::midpoint(q,p)},l)?;
        }
    }
    if let Some(q)=pending{emit(path,GlyphPathCommand::QuadTo{control:PathPoint::real(q),end:origin},l)?;}
    emit(path,GlyphPathCommand::Close,l)
}
/// Validate externally supplied assembled geometry before any indexing or
/// arithmetic. All real points must be covered exactly once by ordered ends.
fn validate(g:&AssembledGlyph,l:PathLimits)->Result<(),Error>{
    if g.points.len()>l.max_points||g.contour_ends.len()>l.max_contours{return Err(Error::Limit("glyph path geometry"));}
    let mut next=0;
    for &end in &g.contour_ends{
        if end<next||end>=g.points.len(){return Err(Error::Invalid("glyph path contour endpoints"));}
        next=end+1;
    }
    if next!=g.points.len(){return Err(Error::Invalid("glyph path incomplete contour coverage"));}
    if g.points.iter().any(|p|!(-POINT_LIMIT..=POINT_LIMIT).contains(&p.x)||!(-POINT_LIMIT..=POINT_LIMIT).contains(&p.y)){
        return Err(Error::Limit("glyph path coordinate"));
    }
    Ok(())
}
/// Construct a path from an already assembled simple or composite glyph.
/// No flattening, hinting, device scaling, fill-rule processing or rasterization.
pub fn build_glyph_path(g:&AssembledGlyph,l:PathLimits)->Result<GlyphPath,Error>{
    validate(g,l)?;
    let mut commands=Vec::new();let mut start=0;
    for &end in &g.contour_ends{
        contour(&g.points[start..=end],&mut commands,l)?;
        start=end+1;
    }
    let mut bounds:Option<GlyphPathBounds>=None;
    let mut current=PathPoint{x:0,y:0};let mut origin=current;
    for &cmd in &commands{
        match cmd{
            GlyphPathCommand::MoveTo(p)=>{
                if let Some(b)=&mut bounds{b.include(p);}else{bounds=Some(GlyphPathBounds::point(p));}
                current=p;origin=p;
            },
            GlyphPathCommand::LineTo(p)=>{bounds.as_mut().expect("contour move").include(p);current=p;},
            GlyphPathCommand::QuadTo{control,end}=>{
                bounds.as_mut().expect("contour move").quadratic(current,control,end)?;current=end;
            },
            GlyphPathCommand::Close=>{current=origin;}
        }
    }
    let control_bounds=g.points.first().map(|&p|{
        let mut b=GlyphPathBounds::point(PathPoint::real(p));
        for &p in &g.points{b.include(PathPoint::real(p));}b
    });
    Ok(GlyphPath{commands,bounds,control_bounds,metrics_glyph:g.metrics_glyph,overlap:g.overlap})
}
/// Same entry point for empty, simple and composite TrueType glyphs.
pub fn true_type_glyph_path(font:&TrueTypeOutlines<'_>,id:u16,assembly:CompositeLimits,path:PathLimits)->Result<GlyphPath,Error>{
    let g=assemble_true_type_glyph(font,id,assembly)?;
    build_glyph_path(&g,path)
}
