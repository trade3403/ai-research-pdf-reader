//! Unhinted glyph outline geometry in unrotated PDF user space.
//! Geometry is not a visibility/occlusion test, nor a math recognizer.
use crate::{graphics::{Path,Segment},text::{Matrix,Point},Error,Result};

#[derive(Debug,Clone,PartialEq)]
pub struct GlyphGeometry{
    pub code:Vec<u8>,
    pub unicode:Option<String>,
    pub cid:Option<u32>,
    pub base_font:Option<Vec<u8>>,
    pub font_resource:Vec<u8>,
    pub font_resource_owner:Option<(u32,u16)>,
    pub form_scope:Option<(u32,u16)>,
    /// Index in the expanded display list, not the page content stream.
    pub command_index:usize,
    pub origin:Point,
    pub advance_end:Point,
    /// Text-to-user transform including font size, horizontal scaling and rise,
    /// before the font program's own matrix.
    pub em_matrix:Matrix,
    pub rendering_mode:u8,
    /// Unhinted outline AABB [xmin,ymin,xmax,ymax]. None for empty or invisible
    /// glyphs. Stroke width, clipping, masks and occlusion are not included.
    pub outline_bounds:Option<[f64;4]>,
}
#[derive(Debug,Clone,PartialEq)]
pub struct PageGlyphGeometry{
    pub glyphs:Vec<GlyphGeometry>,
    pub substituted_fonts:Vec<crate::fonts::standard::StandardFont>,
}
pub(super) struct Collector{
    pub(super) glyphs:Vec<GlyphGeometry>,
    pub(super) command_index:usize,
    pub(super) scope:Option<(u32,u16)>,
    segments:usize,
    bytes:usize,
}
impl Collector{
    pub(super) fn new()->Self{Self{glyphs:Vec::new(),command_index:0,scope:None,segments:0,bytes:0}}
    pub(super) fn bounds(&mut self,path:&Path,m:Matrix)->Result<Option<[f64;4]>>{
        self.segments=self.segments.checked_add(path.len()).ok_or(Error::Limit("glyph geometry segments"))?;
        if self.segments>2_000_000{return Err(Error::Limit("glyph geometry segments"));}
        outline_bounds(path,m)
    }
    pub(super) fn push(&mut self,g:GlyphGeometry)->Result<()>{
        let n=g.code.len()+g.unicode.as_ref().map_or(0,String::len)+g.base_font.as_ref().map_or(0,Vec::len)+g.font_resource.len();
        self.bytes=self.bytes.checked_add(n).ok_or(Error::Limit("glyph geometry bytes"))?;
        if self.bytes>16*1024*1024||self.glyphs.len()>=100_000{return Err(Error::Limit("glyph geometry output"));}
        self.glyphs.push(g);Ok(())
    }
}
fn include(b:&mut Option<[f64;4]>,p:Point){
    if let Some(v)=b{v[0]=v[0].min(p[0]);v[1]=v[1].min(p[1]);v[2]=v[2].max(p[0]);v[3]=v[3].max(p[1]);}
    else{*b=Some([p[0],p[1],p[0],p[1]]);}
}
fn lerp(a:Point,b:Point,t:f64)->Point{[a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t]}
fn evaluate(p:[Point;4],t:f64)->Point{
    let a=lerp(p[0],p[1],t);let b=lerp(p[1],p[2],t);let c=lerp(p[2],p[3],t);
    lerp(lerp(a,b,t),lerp(b,c,t),t)
}
fn cubic_bounds(b:&mut Option<[f64;4]>,p:[Point;4]){
    include(b,p[0]);include(b,p[3]);
    for k in 0..2{
        // Derivative in Bernstein form, divided by 3. Use the stable
        // quadratic formula; normalization avoids unnecessary cancellation.
        let d0=p[1][k]-p[0][k];let d1=p[2][k]-p[1][k];let d2=p[3][k]-p[2][k];
        let a=d0-2.*d1+d2;let q=2.*(d1-d0);let c=d0;
        let scale=a.abs().max(q.abs()).max(c.abs());
        if scale==0.{continue;}
        let(a,q,c)=(a/scale,q/scale,c/scale);
        let mut root=|t:f64|{if t>0.&&t<1.{include(b,evaluate(p,t));}};
        if a==0.{if q!=0.{root(-c/q);}continue;}
        let disc=q*q-4.*a*c;
        if disc<0.{continue;}
        let z=-0.5*(q+disc.sqrt().copysign(q));
        if z==0.{root(-q/(2.*a));}else{root(z/a);root(c/z);}
    }
}
fn outline_bounds(path:&Path,m:Matrix)->Result<Option<[f64;4]>>{
    let mut result=None;let mut current=None;let mut start=None;
    for seg in path{
        match *seg{
            Segment::Move(p)=>{let p=m.point(p)?;current=Some(p);start=Some(p);}
            Segment::Line(p)=>{
                let a=current.ok_or(Error::Invalid("glyph geometry line without move"))?;
                let b=m.point(p)?;include(&mut result,a);include(&mut result,b);current=Some(b);
            }
            Segment::Cubic(p,q,r)=>{
                let a=current.ok_or(Error::Invalid("glyph geometry curve without move"))?;
                let d=m.point(r)?;cubic_bounds(&mut result,[a,m.point(p)?,m.point(q)?,d]);current=Some(d);
            }
            Segment::Close=>{
                // Closing segment cannot extend the bounds of a drawn subpath.
                current=start;
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn cubic_extrema_not_control_box(){
        let p=vec![Segment::Move([0.,0.]),Segment::Cubic([0.,4.],[4.,4.],[4.,0.]),Segment::Close];
        assert_eq!(outline_bounds(&p,Matrix::IDENTITY).unwrap(),Some([0.,0.,4.,3.]));
        assert_eq!(outline_bounds(&p,Matrix([0.,1.,-1.,0.,5.,7.])).unwrap(),Some([2.,7.,5.,11.]));
        assert_eq!(outline_bounds(&vec![Segment::Move([1.,2.])],Matrix::IDENTITY).unwrap(),None);
    }
}
