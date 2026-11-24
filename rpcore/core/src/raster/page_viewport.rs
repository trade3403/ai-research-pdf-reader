//! Coordinate mapping shared with the actual raster viewport.
use super::{viewport,inverse};
use crate::{PageInfo,Error,Result,text::{Matrix,Point,Quad}};

/// Coordinates are local to the rendered page, origin at its top-left.
/// Scale is pixels per PDF user-space unit (normally one point).
/// Caller handles scroll offset, CSS scaling and device pixel ratio.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct PageViewport{
    forward:Matrix,
    backward:Matrix,
    width:usize,
    height:usize,
    exact:[f64;2],
}
impl PageViewport{
    /// Same CropBox, rotation, ceil rounding and pixel limits as rendering.
    pub fn new(page:&PageInfo,scale:f64)->Result<Self>{
        let (forward,width,height,w,h)=viewport(page,scale)?;
        if forward.0.iter().any(|v|!v.is_finite()){return Err(Error::Limit("viewport transform"));}
        let backward=inverse(forward)?;
        Ok(Self{forward,backward,width,height,exact:[w,h]})
    }
    pub fn bitmap_size(&self)->(usize,usize){(self.width,self.height)}
    /// Exact content extent, excluding fractional-pixel padding.
    pub fn content_size(&self)->[f64;2]{self.exact}
    pub fn pdf_to_view(&self,p:Point)->Result<Point>{self.forward.point(p)}
    pub fn view_to_pdf(&self,p:Point)->Result<Point>{self.backward.point(p)}
    pub fn pdf_quad_to_view(&self,q:Quad)->Result<Quad>{
        Ok([self.pdf_to_view(q[0])?,self.pdf_to_view(q[1])?,self.pdf_to_view(q[2])?,self.pdf_to_view(q[3])?])
    }
    /// Convert ordered [xmin,ymin,xmax,ymax] bounds, preserving all corners.
    pub fn pdf_rect_to_view(&self,r:[f64;4])->Result<[f64;4]>{map_rect(self.forward,r)}
    pub fn view_rect_to_pdf(&self,r:[f64;4])->Result<[f64;4]>{map_rect(self.backward,r)}
    /// Crop a view-space rectangle to the actual page content extent.
    /// A zero-area intersection is not a highlight.
    pub fn clip_rect(&self,r:[f64;4])->Result<Option<[f64;4]>>{
        validate_rect(r)?;
        let b=[r[0].max(0.),r[1].max(0.),r[2].min(self.exact[0]),r[3].min(self.exact[1])];
        Ok((b[0]<b[2]&&b[1]<b[3]).then_some(b))
    }
}
pub(super) fn validate_rect(r:[f64;4])->Result<()>{
    if r.iter().any(|v|!v.is_finite()||v.abs()>1e12)||r[0]>r[2]||r[1]>r[3]{return Err(Error::Invalid("viewport rectangle"));}
    Ok(())
}
fn map_rect(m:Matrix,r:[f64;4])->Result<[f64;4]>{
    validate_rect(r)?;
    let mut out=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];
    for p in [[r[0],r[1]],[r[2],r[1]],[r[2],r[3]],[r[0],r[3]]]{
        let [x,y]=m.point(p)?;
        out[0]=out[0].min(x);out[1]=out[1].min(y);out[2]=out[2].max(x);out[3]=out[3].max(y);
    }
    Ok(out)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn page(rotation:i32)->PageInfo{
        PageInfo{object_id:(1,0),media_box:[-20.,-10.,180.,100.],crop_box:[-10.,5.,90.,55.],rotation,resources:None}
    }
    #[test]fn crop_rotation_and_round_trip(){
        for (rot,origin,opposite,size) in [
            (0,[0.,75.],[150.,0.],(150,75)),
            (90,[0.,0.],[75.,150.],(75,150)),
            (180,[150.,0.],[0.,75.],(150,75)),
            (270,[75.,150.],[0.,0.],(75,150)),
        ]{
            let v=PageViewport::new(&page(rot),1.5).unwrap();
            assert_eq!(v.bitmap_size(),size);
            assert_eq!(v.pdf_to_view([-10.,5.]).unwrap(),origin);
            assert_eq!(v.pdf_to_view([90.,55.]).unwrap(),opposite);
            let p=[12.25,18.75];let q=v.view_to_pdf(v.pdf_to_view(p).unwrap()).unwrap();
            for i in 0..2{assert!((q[i]-p[i]).abs()<1e-9);}
            let r=[1.,8.,12.,20.];let q=v.view_rect_to_pdf(v.pdf_rect_to_view(r).unwrap()).unwrap();
            for i in 0..4{assert!((q[i]-r[i]).abs()<1e-9);}
        }
    }
    #[test]fn padding_and_invalid_inputs(){
        let v=PageViewport::new(&page(0),0.333).unwrap();
        assert_eq!(v.bitmap_size(),(34,17));
        assert_eq!(v.clip_rect([33.5,0.,34.,1.]).unwrap(),None);
        let [w,h]=v.content_size();
        assert_eq!(v.clip_rect([-1.,-1.,100.,100.]).unwrap(),Some([0.,0.,w,h]));
        assert!(v.pdf_rect_to_view([1.,0.,0.,1.]).is_err());
        assert!(v.view_to_pdf([f64::NAN,0.]).is_err());
        for scale in [0.,-1.,f64::INFINITY,1000.]{assert!(PageViewport::new(&page(0),scale).is_err());}
        assert!(PageViewport::new(&page(45),1.).is_err());
    }
}
