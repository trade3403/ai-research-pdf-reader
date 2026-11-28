//! C2.4e: DrawingML crop metadata and reversible source-to-shape geometry.
//! Coordinates are EMU, with x right and y down. No pixel resampling or painting.
use crate::{Error,Limits};
use crate::opc::{named,xml};
use roxmltree::Node;

const PIC:&str="http://schemas.openxmlformats.org/drawingml/2006/picture";
const PICS:&str="http://purl.oclc.org/ooxml/drawingml/picture";
const A:&str="http://schemas.openxmlformats.org/drawingml/2006/main";
const AS:&str="http://purl.oclc.org/ooxml/drawingml/main";
const MAX_EMU:i64=27_273_042_316_900;
const TURN:i32=21_600_000;
const QUARTER:i32=5_400_000;

/// Crop in thousandths of a percent; 100000 represents the full source.
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]
pub struct PictureCrop{pub left:u32,pub top:u32,pub right:u32,pub bottom:u32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PictureTransform{
    pub x:i64,pub y:i64,pub width:u64,pub height:u64,
    /// DrawingML angle, clockwise in y-down space, 60000 units per degree.
    pub rotation:i32,pub flip_h:bool,pub flip_v:bool,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PictureGeometry{pub crop:PictureCrop,pub transform:PictureTransform}
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct GeometryPoint{pub x:f64,pub y:f64}
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct GeometryBounds{pub min:GeometryPoint,pub max:GeometryPoint}

/// Validated affine mapping from normalized source coordinates to EMU.
/// The crop edges map to the transformed shape edges. Mapping a point outside
/// the crop is permitted (extrapolation); it does not imply visibility.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct PictureMapping{
    crop:PictureCrop,transform:PictureTransform,
    cosine:f64,sine:f64,
}
fn optional<'a,'i>(n:Node<'a,'i>,ns:&str,key:&str)->Result<Option<Node<'a,'i>>,Error>{
    let mut it=n.children().filter(|c|named(*c,ns,key));
    let first=it.next();
    if it.next().is_some(){return Err(Error::Invalid("duplicate picture geometry element"));}
    Ok(first)
}
fn required<'a,'i>(n:Node<'a,'i>,ns:&str,key:&str)->Result<Node<'a,'i>,Error>{
    optional(n,ns,key)?.ok_or(Error::Invalid("missing picture geometry element"))
}
fn integer<T:std::str::FromStr>(n:Node<'_, '_>,key:&str)->Result<T,Error>{
    n.attribute(key).ok_or(Error::Invalid("missing picture geometry attribute"))?
        .parse().map_err(|_|Error::Invalid("invalid picture geometry integer"))
}
fn flag(n:Node<'_, '_>,key:&str)->Result<bool,Error>{
    match n.attribute(key){
        None|Some("false"|"0")=>Ok(false),
        Some("true"|"1")=>Ok(true),
        _=>Err(Error::Invalid("invalid picture transform boolean")),
    }
}
impl PictureCrop{
    fn validate(self)->Result<(),Error>{
        if u64::from(self.left)+u64::from(self.right)>=100000||
            u64::from(self.top)+u64::from(self.bottom)>=100000{
            return Err(Error::Invalid("empty or out-of-range picture crop"));
        }
        Ok(())
    }
    fn normalized(self)->(f64,f64,f64,f64){
        (f64::from(self.left)/100000.0,f64::from(self.top)/100000.0,
         f64::from(100000-self.left-self.right)/100000.0,
         f64::from(100000-self.top-self.bottom)/100000.0)
    }
    /// Conservative integer pixel window: floor the leading edges, ceil the
    /// trailing edges. Returned as (left, top, right, bottom), half-open.
    pub fn pixel_window(self,width:u32,height:u32)->Result<(u32,u32,u32,u32),Error>{
        self.validate()?;
        if width==0||height==0{return Err(Error::Invalid("zero source image dimension"));}
        let floor=|d:u32,p:u32|(u64::from(d)*u64::from(p)/100000) as u32;
        let ceil=|d:u32,p:u32|((u64::from(d)*u64::from(p)+99999)/100000) as u32;
        Ok((floor(width,self.left),floor(height,self.top),
            ceil(width,100000-self.right),ceil(height,100000-self.bottom)))
    }
}
impl PictureGeometry{
    pub fn mapping(self)->Result<PictureMapping,Error>{
        self.crop.validate()?;
        let t=self.transform;
        if !(-MAX_EMU..=MAX_EMU).contains(&t.x)||!(-MAX_EMU..=MAX_EMU).contains(&t.y)
            ||!(1..=MAX_EMU as u64).contains(&t.width)||!(1..=MAX_EMU as u64).contains(&t.height){
            return Err(Error::Invalid("picture transform coordinate bounds"));
        }
        let angle=t.rotation.rem_euclid(TURN);
        // Exact cardinal directions avoid spurious one-EMU bounding errors.
        let (cosine,sine)=match angle{
            0=>(1.0,0.0),QUARTER=>(0.0,1.0),
            10_800_000=>(-1.0,0.0),16_200_000=>(0.0,-1.0),
            _=>{let r=f64::from(angle)*std::f64::consts::TAU/f64::from(TURN);
                (r.cos(),r.sin())}
        };
        Ok(PictureMapping{crop:self.crop,transform:t,cosine,sine})
    }
}
fn finite(p:GeometryPoint)->Result<GeometryPoint,Error>{
    if !p.x.is_finite()||!p.y.is_finite(){return Err(Error::Invalid("nonfinite picture point"));}
    Ok(p)
}
impl PictureMapping{
    fn center(self)->(f64,f64){
        let t=self.transform;
        (t.x as f64+t.width as f64/2.0,t.y as f64+t.height as f64/2.0)
    }
    fn local_to_page(self,u:f64,v:f64)->Result<GeometryPoint,Error>{
        let t=self.transform;let (cx,cy)=self.center();
        let x=(u-0.5)*t.width as f64*if t.flip_h{-1.0}else{1.0};
        let y=(v-0.5)*t.height as f64*if t.flip_v{-1.0}else{1.0};
        finite(GeometryPoint{x:cx+self.cosine*x-self.sine*y,y:cy+self.sine*x+self.cosine*y})
    }
    /// Flip in local shape coordinates, then rotate about the shape center.
    pub fn source_to_page(self,p:GeometryPoint)->Result<GeometryPoint,Error>{
        finite(p)?;
        let (l,t,w,h)=self.crop.normalized();
        self.local_to_page((p.x-l)/w,(p.y-t)/h)
    }
    /// Inverse mapping. The returned normalized source point is not clamped.
    pub fn page_to_source(self,p:GeometryPoint)->Result<GeometryPoint,Error>{
        finite(p)?;
        let t=self.transform;let (cx,cy)=self.center();
        let dx=p.x-cx;let dy=p.y-cy;
        let x=(self.cosine*dx+self.sine*dy)/t.width as f64*if t.flip_h{-1.0}else{1.0}+0.5;
        let y=(-self.sine*dx+self.cosine*dy)/t.height as f64*if t.flip_v{-1.0}else{1.0}+0.5;
        let (l,top,w,h)=self.crop.normalized();
        finite(GeometryPoint{x:l+x*w,y:top+y*h})
    }
    /// Four transformed corners in source order: TL, TR, BR, BL.
    pub fn corners(self)->Result<[GeometryPoint;4],Error>{
        Ok([self.local_to_page(0.0,0.0)?,self.local_to_page(1.0,0.0)?,
            self.local_to_page(1.0,1.0)?,self.local_to_page(0.0,1.0)?])
    }
    pub fn bounds(self)->Result<GeometryBounds,Error>{
        let points=self.corners()?;
        let mut min=points[0];let mut max=points[0];
        for p in points{
            min.x=min.x.min(p.x);min.y=min.y.min(p.y);
            max.x=max.x.max(p.x);max.y=max.y.max(p.y);
        }
        Ok(GeometryBounds{min,max})
    }
}
fn percent(n:Node<'_, '_>,key:&str)->Result<u32,Error>{
    n.attribute(key).map(|s|s.parse::<u32>().map_err(|_|Error::Unsupported("picture crop integer nonnegative subset")))
        .transpose().map(|v|v.unwrap_or(0))
}
/// Parse standalone pic:pic geometry. Transitional/strict namespaces supported;
/// only nonnegative integer srcRect cropping and explicit a:xfrm off/ext.
/// The caller selects the picture node and reconciles shape and wp:extent.
/// Tiled fills, stretch fillRect, group transforms and negative crop are not supported.
pub fn parse_picture_geometry(bytes:&[u8],limits:Limits)->Result<PictureGeometry,Error>{
    if bytes.len()>limits.max_part_bytes{return Err(Error::Limit("part bytes"));}
    let doc=xml(bytes,limits)?;
    parse_picture_geometry_node(doc.root_element())
}
pub(crate) fn parse_picture_geometry_node(root:Node<'_, '_>)->Result<PictureGeometry,Error>{
    let (pic,a)=if named(root,PIC,"pic"){(PIC,A)}else if named(root,PICS,"pic"){(PICS,AS)}
        else{return Err(Error::Unsupported("picture geometry root"));};
    let fill=required(root,pic,"blipFill")?;
    if optional(fill,a,"tile")?.is_some(){return Err(Error::Unsupported("tiled picture fill"));}
    if let Some(stretch)=optional(fill,a,"stretch")?{
        if let Some(r)=optional(stretch,a,"fillRect")?{
            for key in ["l","t","r","b"]{
                if percent(r,key)?!=0{return Err(Error::Unsupported("nonzero picture fillRect"));}
            }
        }
    }
    let crop=match optional(fill,a,"srcRect")?{
        None=>PictureCrop::default(),
        Some(n)=>PictureCrop{left:percent(n,"l")?,top:percent(n,"t")?,
            right:percent(n,"r")?,bottom:percent(n,"b")?},
    };
    let sp=required(root,pic,"spPr")?;
    let xfrm=required(sp,a,"xfrm")?;
    let off=required(xfrm,a,"off")?;let ext=required(xfrm,a,"ext")?;
    let rotation=xfrm.attribute("rot").map(|v|v.parse::<i32>()
        .map_err(|_|Error::Invalid("picture rotation"))).transpose()?.unwrap_or(0);
    let geometry=PictureGeometry{crop,transform:PictureTransform{
        x:integer(off,"x")?,y:integer(off,"y")?,width:integer(ext,"cx")?,height:integer(ext,"cy")?,
        rotation,flip_h:flag(xfrm,"flipH")?,flip_v:flag(xfrm,"flipV")?,
    }};
    geometry.mapping()?;
    Ok(geometry)
}
