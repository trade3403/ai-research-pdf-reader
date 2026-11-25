//! C4.3e: deterministic bounded supersampled grayscale rasterization.
use crate::{Error,FlattenedGlyph,PathPoint,PATH_ONE};
use std::cmp::Ordering;

const PIXEL:i64=65536;
const DEVICE_LIMIT:i64=1<<40;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum GlyphFillRule{NonZero,EvenOdd}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RasterTransform{
    pub units_per_em:u16,
    /// Font pixels per em in 26.6 units, positive.
    pub ppem_26_6:u32,
    /// Baseline origin in y-down device 26.6 coordinates.
    pub origin_x_26_6:i64,pub origin_y_26_6:i64,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RasterViewport{pub left:i32,pub top:i32,pub width:u32,pub height:u32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RasterLimits{
    pub max_width:u32,pub max_height:u32,pub max_pixels:usize,
    pub max_edges:usize,pub max_samples:u64,pub max_edge_tests:u64,
}
impl Default for RasterLimits{
    fn default()->Self{Self{max_width:4096,max_height:4096,max_pixels:4*1024*1024,
        max_edges:262144,max_samples:64*1024*1024,max_edge_tests:64*1024*1024}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct RasterOptions{
    pub transform:RasterTransform,pub viewport:RasterViewport,
    pub fill:GlyphFillRule,
    /// Regular sample centers per pixel axis: 1, 2, 4, 8 or 16.
    pub samples_per_axis:u8,pub limits:RasterLimits,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GlyphBitmap{
    pub viewport:RasterViewport,
    /// Row-major, top-to-bottom straight coverage, no gamma adjustment.
    pub alpha:Vec<u8>,
}
impl GlyphBitmap{
    pub fn pixel(&self,x:u32,y:u32)->Option<u8>{
        if x>=self.viewport.width||y>=self.viewport.height{return None;}
        let i=usize::try_from(y).ok()?.checked_mul(usize::try_from(self.viewport.width).ok()?)?
            .checked_add(usize::try_from(x).ok()?)?;
        self.alpha.get(i).copied()
    }
}
#[derive(Debug,Clone,Copy)]
struct DevicePoint{x:i64,y:i64}
#[derive(Debug,Clone,Copy)]
struct Edge{a:DevicePoint,b:DevicePoint,w:i64}
#[derive(Debug,Clone,Copy)]
struct Crossing{num:i128,den:i128,w:i64}
fn device(v:i128)->Result<i64,Error>{
    if v< -i128::from(DEVICE_LIMIT)||v>i128::from(DEVICE_LIMIT){return Err(Error::Limit("raster device coordinate"));}
    Ok(v as i64)
}
fn round(n:i128,d:i128)->i128{
    let q=n/d;let r=n%d;
    q+if r.abs()*2>=d{n.signum()}else{0}
}
fn transform(p:PathPoint,t:RasterTransform)->Result<DevicePoint,Error>{
    let den=i128::from(t.units_per_em)*i128::from(PATH_ONE);
    let scale=i128::from(t.ppem_26_6)*1024;
    let x=i128::from(t.origin_x_26_6)*1024+round(i128::from(p.x)*scale,den);
    let y=i128::from(t.origin_y_26_6)*1024-round(i128::from(p.y)*scale,den);
    Ok(DevicePoint{x:device(x)?,y:device(y)?})
}
fn product(a:u64,b:u64)->Result<u64,Error>{a.checked_mul(b).ok_or(Error::Limit("raster work arithmetic"))}
fn validate(o:RasterOptions)->Result<usize,Error>{
    if !(16..=16384).contains(&o.transform.units_per_em)||o.transform.ppem_26_6==0{
        return Err(Error::Invalid("raster scale"));
    }
    if !matches!(o.samples_per_axis,1|2|4|8|16){return Err(Error::Invalid("raster samples"));}
    device(i128::from(o.transform.origin_x_26_6)*1024)?;
    device(i128::from(o.transform.origin_y_26_6)*1024)?;
    let v=o.viewport;
    if v.width>o.limits.max_width||v.height>o.limits.max_height{return Err(Error::Limit("raster dimensions"));}
    for (start,len) in [(v.left,v.width),(v.top,v.height)]{
        device(i128::from(start)*i128::from(PIXEL))?;
        device((i128::from(start)+i128::from(len))*i128::from(PIXEL))?;
    }
    let pixels=product(u64::from(v.width),u64::from(v.height))?;
    let size=usize::try_from(pixels).map_err(|_|Error::Limit("raster pixel count"))?;
    if size>o.limits.max_pixels{return Err(Error::Limit("raster pixels"));}
    let n=u64::from(o.samples_per_axis);
    if product(product(pixels,n)?,n)?>o.limits.max_samples{return Err(Error::Limit("raster samples work"));}
    Ok(size)
}
fn collect_edges(g:&FlattenedGlyph,o:RasterOptions)->Result<Vec<Edge>,Error>{
    // Charge input edges, including horizontal and zero-length ones, before
    // coordinate transformation or allocation.
    if g.vertex_count()>o.limits.max_edges{return Err(Error::Limit("raster input edges"));}
    let mut edges=Vec::new();
    for contour in g.contours(){
        for (a,b) in contour.edges(){
            let a=transform(a,o.transform)?;let b=transform(b,o.transform)?;
            if a.y<b.y{edges.push(Edge{a,b,w:1});}
            else if a.y>b.y{edges.push(Edge{a:b,b:a,w:-1});}
        }
    }
    Ok(edges)
}
fn crossings_at(edges:&[Edge],y:i64,out:&mut Vec<Crossing>){
    out.clear();
    // Half-open vertical interval avoids double-counting shared vertices.
    for e in edges{
        if y<e.a.y||y>=e.b.y{continue;}
        let den=i128::from(e.b.y-e.a.y);
        let num=i128::from(e.a.x)*den+i128::from(y-e.a.y)*i128::from(e.b.x-e.a.x);
        out.push(Crossing{num,den,w:e.w});
    }
    // Device coordinates <= 2^40, denominators <= 2^41; products fit i128.
    out.sort_unstable_by(|a,b|(a.num*b.den).cmp(&(b.num*a.den)));
}
fn covered(w:i64,rule:GlyphFillRule)->bool{
    match rule{GlyphFillRule::NonZero=>w!=0,GlyphFillRule::EvenOdd=>w.rem_euclid(2)!=0}
}
/// Rasterize previously flattened outlines into a caller-selected viewport.
/// Intersection tests are exact on the rounded 16.16 device grid. Samples
/// exactly on a crossing are classified after all equal crossings (left
/// inclusive, right exclusive). Coverage is an N*N sample estimate, not
/// analytic area or hinted glyph coverage.
pub fn rasterize_flat_glyph(g:&FlattenedGlyph,o:RasterOptions)->Result<GlyphBitmap,Error>{
    let size=validate(o)?;
    let edges=collect_edges(g,o)?;
    let v=o.viewport;
    let n=u64::from(o.samples_per_axis);
    let e=u64::try_from(edges.len()).map_err(|_|Error::Limit("raster edges"))?;
    if size!=0&&product(product(e,u64::from(v.height))?,n)?>o.limits.max_edge_tests{
        return Err(Error::Limit("raster scan edge work"));
    }
    let mut bitmap=GlyphBitmap{viewport:v,alpha:vec![0;size]};
    if size==0||edges.is_empty(){return Ok(bitmap);}
    let width=usize::try_from(v.width).map_err(|_|Error::Limit("raster width"))?;
    let step=PIXEL/i64::from(o.samples_per_axis);
    let half=step/2;
    let mut row=vec![0u16;width];
    let mut crosses=Vec::new();
    let total=(n*n) as u16;
    for iy in 0..v.height{
        row.fill(0);
        for sy in 0..o.samples_per_axis{
            let y=(i64::from(v.top)+i64::from(iy))*PIXEL+i64::from(sy)*step+half;
            crossings_at(&edges,y,&mut crosses);
            let mut index=0usize;let mut winding=0i64;
            for (ix,count) in row.iter_mut().enumerate(){
                let origin=(i64::from(v.left)+ix as i64)*PIXEL;
                for sx in 0..o.samples_per_axis{
                    let x=origin+i64::from(sx)*step+half;
                    while let Some(c)=crosses.get(index){
                        if c.num.cmp(&(i128::from(x)*c.den))==Ordering::Greater{break;}
                        winding=winding.checked_add(c.w).ok_or(Error::Limit("raster winding"))?;
                        index+=1;
                    }
                    if covered(winding,o.fill){*count+=1;}
                }
            }
        }
        let offset=iy as usize*width;
        for (ix,&count) in row.iter().enumerate(){
            bitmap.alpha[offset+ix]=((u32::from(count)*255+u32::from(total)/2)/u32::from(total)) as u8;
        }
    }
    Ok(bitmap)
}
