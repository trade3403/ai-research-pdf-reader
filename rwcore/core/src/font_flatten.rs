//! C4.3d: bounded dyadic quadratic flattening in the C4.3c path grid.
use crate::{Error,GlyphPath,GlyphPathCommand,PathPoint,GlyphPathBounds};

const COORD_LIMIT:i64=1<<47;
const HARD_DEPTH:u8=24;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FlattenLimits{
    pub max_depth:u8,pub max_vertices:usize,pub max_contours:usize,
}
impl Default for FlattenLimits{
    fn default()->Self{Self{max_depth:20,max_vertices:262144,max_contours:4096}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FlattenOptions{
    /// Maximum componentwise curve-to-chord deviation in PATH_ONE units,
    /// including rounding of generated vertices. Must be positive.
    pub tolerance:i64,pub limits:FlattenLimits,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ContourOrientation{Clockwise,CounterClockwise,ZeroSignedArea}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FlatContour{vertices:Vec<PathPoint>,area2:i128}
impl FlatContour{
    pub fn vertices(&self)->&[PathPoint]{&self.vertices}
    /// Closing edge is implicit, even if the last vertex equals the first.
    /// Zero-length edges are retained; consumers may skip them.
    pub fn edges(&self)->impl Iterator<Item=(PathPoint,PathPoint)>+'_{
        self.vertices.iter().copied().zip(self.vertices.iter().copied().cycle().skip(1)).take(self.vertices.len())
    }
    /// Twice signed polygon area, in squared path units (y-up coordinates).
    /// Self-intersections may cancel; not an absolute filled area.
    pub fn signed_area2(&self)->i128{self.area2}
    pub fn orientation(&self)->ContourOrientation{
        match self.area2.cmp(&0){
            std::cmp::Ordering::Less=>ContourOrientation::Clockwise,
            std::cmp::Ordering::Greater=>ContourOrientation::CounterClockwise,
            std::cmp::Ordering::Equal=>ContourOrientation::ZeroSignedArea,
        }
    }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FlattenedGlyph{
    contours:Vec<FlatContour>,vertex_count:usize,bounds:Option<GlyphPathBounds>,
    metrics_glyph:u16,overlap:bool,tolerance:i64,
}
impl FlattenedGlyph{
    pub fn contours(&self)->&[FlatContour]{&self.contours}
    pub fn vertex_count(&self)->usize{self.vertex_count}
    /// Bounds of the resulting polygon vertices, not original curve bounds.
    pub fn bounds(&self)->Option<GlyphPathBounds>{self.bounds}
    pub fn metrics_glyph(&self)->u16{self.metrics_glyph}
    pub fn overlap(&self)->bool{self.overlap}
    pub fn tolerance(&self)->i64{self.tolerance}
}
fn check_point(p:PathPoint)->Result<(),Error>{
    if !(-COORD_LIMIT..=COORD_LIMIT).contains(&p.x)||!(-COORD_LIMIT..=COORD_LIMIT).contains(&p.y){
        return Err(Error::Limit("flatten coordinate"));
    }
    Ok(())
}
fn charge(n:&mut usize,extra:usize,l:FlattenLimits)->Result<(),Error>{
    let next=n.checked_add(extra).ok_or(Error::Limit("flatten vertex count"))?;
    if next>l.max_vertices{return Err(Error::Limit("flatten vertices"));}
    *n=next;Ok(())
}
fn second(a:i64,b:i64,c:i64)->i128{
    (i128::from(a)-2*i128::from(b)+i128::from(c)).abs()
}
/// For a quadratic over a subinterval of length 1/N, deviation from its
/// parameter-matched chord is <= |A-2B+C|/(4*N^2) per axis.
/// Intermediate vertices rounded to the path grid add at most 1/2 unit.
/// N=1 has exact endpoints, so no rounding allowance is necessary.
/// Select the smallest power of two satisfying the conservative bound.
fn subdivisions(a:PathPoint,b:PathPoint,c:PathPoint,o:FlattenOptions)->Result<usize,Error>{
    let d=second(a.x,b.x,c.x).max(second(a.y,b.y,c.y));
    let tol=i128::from(o.tolerance);
    if d<=4*tol{return Ok(1);}
    let mut n=1usize;
    for _ in 0..o.limits.max_depth.min(HARD_DEPTH){
        n*=2;
        let n128=n as i128;
        if d<=(4*tol-2)*n128*n128{return Ok(n);}
    }
    Err(Error::Limit("flatten depth for tolerance"))
}
fn rounded(n:i128,d:i128)->Result<i64,Error>{
    let q=n/d;let r=n%d;
    i64::try_from(q+if r.abs()*2>=d{n.signum()}else{0}).map_err(|_|Error::Limit("flatten coordinate"))
}
fn axis(a:i64,b:i64,c:i64,k:i128,n:i128)->Result<i64,Error>{
    let j=n-k;
    // |coordinate| <= 2^47; N <= 2^24, products fit i128.
    let v=i128::from(a)*j*j+2*i128::from(b)*j*k+i128::from(c)*k*k;
    rounded(v,n*n)
}
fn quadratic(a:PathPoint,b:PathPoint,c:PathPoint,o:FlattenOptions,vertices:&mut Vec<PathPoint>,count:&mut usize)->Result<(),Error>{
    check_point(a)?;check_point(b)?;check_point(c)?;
    let n=subdivisions(a,b,c,o)?;
    charge(count,n,o.limits)?;
    for k in 1..n{
        vertices.push(PathPoint{x:axis(a.x,b.x,c.x,k as i128,n as i128)?,
            y:axis(a.y,b.y,c.y,k as i128,n as i128)?});
    }
    vertices.push(c);
    Ok(())
}
fn area2(v:&[PathPoint])->Result<i128,Error>{
    let mut sum=0i128;
    for (a,b) in v.iter().zip(v.iter().cycle().skip(1)).take(v.len()){
        let cross=i128::from(a.x)*i128::from(b.y)-i128::from(a.y)*i128::from(b.x);
        sum=sum.checked_add(cross).ok_or(Error::Limit("flatten signed area"))?;
    }
    Ok(sum)
}
fn include(b:&mut Option<GlyphPathBounds>,p:PathPoint){
    if let Some(b)=b{
        b.x_min=b.x_min.min(p.x);b.y_min=b.y_min.min(p.y);
        b.x_max=b.x_max.max(p.x);b.y_max=b.y_max.max(p.y);
    }else{*b=Some(GlyphPathBounds{x_min:p.x,y_min:p.y,x_max:p.x,y_max:p.y});}
}
/// Preserve ordered closed contours. Every generated chord satisfies the
/// specified L-infinity error bound, with exact integer evaluation at dyadic
/// parameters and nearest ties-away rounding. No clipping, winding fill,
/// hinting, stroke expansion, pixel mapping or coverage calculation.
pub fn flatten_glyph_path(path:&GlyphPath,o:FlattenOptions)->Result<FlattenedGlyph,Error>{
    if o.tolerance<=0{return Err(Error::Invalid("flatten tolerance"));}
    let mut out=FlattenedGlyph{contours:Vec::new(),vertex_count:0,bounds:None,
        metrics_glyph:path.metrics_glyph(),overlap:path.overlap(),tolerance:o.tolerance};
    let mut active:Option<Vec<PathPoint>>=None;
    for &cmd in path.commands(){
        match cmd{
            GlyphPathCommand::MoveTo(p)=>{
                if active.is_some(){return Err(Error::Invalid("flatten unclosed contour"));}
                if out.contours.len()>=o.limits.max_contours{return Err(Error::Limit("flatten contours"));}
                check_point(p)?;charge(&mut out.vertex_count,1,o.limits)?;
                active=Some(vec![p]);
            },
            GlyphPathCommand::LineTo(p)=>{
                check_point(p)?;let v=active.as_mut().ok_or(Error::Invalid("flatten missing move"))?;
                charge(&mut out.vertex_count,1,o.limits)?;v.push(p);
            },
            GlyphPathCommand::QuadTo{control,end}=>{
                let v=active.as_mut().ok_or(Error::Invalid("flatten missing move"))?;
                let a=*v.last().ok_or(Error::Invalid("flatten empty contour"))?;
                quadratic(a,control,end,o,v,&mut out.vertex_count)?;
            },
            GlyphPathCommand::Close=>{
                let v=active.take().ok_or(Error::Invalid("flatten missing contour"))?;
                let area2=area2(&v)?;
                for &p in &v{include(&mut out.bounds,p);}
                out.contours.push(FlatContour{vertices:v,area2});
            }
        }
    }
    if active.is_some(){return Err(Error::Invalid("flatten unclosed contour"));}
    Ok(out)
}
