//! Independent CPU rasterizer. 2x2 sample coverage, opaque white background.
use crate::{graphics::{BlendMode,Color,Command,DisplayList,FillRule,LineCap,LineJoin,Path,Segment,Stroke},text::{Matrix,Point},Document,Error,PageInfo,Result};
use std::sync::Arc;
mod png;
mod rendertext;
mod images;
mod glyph_area;
mod glyph_geometry;
mod page_viewport;
pub mod math_geometry;
pub mod math_tree;
pub use page_viewport::PageViewport;
pub use glyph_geometry::{GlyphGeometry,PageGlyphGeometry};
use crate::fonts::standard::StandardFont;
use std::collections::BTreeMap;
const MAX_PIXELS:usize=4_000_000;
const MAX_SIDE:usize=8192;
const MAX_EDGES:usize=200_000;
const MAX_WORK:usize=200_000_000;
const MAX_CLIP_BYTES:usize=64*1024*1024;
#[derive(Debug,Clone,Copy)]
pub struct Options{pub scale:f64,pub allow_missing_text:bool}
/// Optional higher precision coverage for filled glyphs. Existing vector, image,
/// clipping and group surfaces remain 2x2; no font bytecode or subpixel RGB rendering.
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub enum TextQuality{
    #[default] Standard,
    Fine,
    /// Experimental Fine coverage plus bounded horizontal baseline alignment.
    /// Does not execute TrueType instructions or interpret Type1/CFF stem hints.
    FineGrid,
    /// Experimental continuous horizontal span coverage, 8 vertical rows per pixel.
    Area,
}
impl Default for Options{fn default()->Self{Self{scale:1.,allow_missing_text:false}}}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Bitmap{pub width:usize,pub height:usize,pub rgba:Vec<u8>,pub missing_text:bool}
impl Bitmap{
    pub fn png(&self)->Result<Vec<u8>>{png::encode(self.width,self.height,&self.rgba)}
}
/// Explicit caller-owned Standard 14 substitute programs. No filesystem lookup,
/// system fonts, silent substitution, or bundled third-party outline assets.
#[derive(Default)]
pub struct StandardFontSet{fonts:BTreeMap<Vec<u8>,Vec<u8>>,bytes:usize}
impl StandardFontSet{
    pub fn new()->Self{Self::default()}
    pub fn insert(&mut self,face:StandardFont,mut program:Vec<u8>)->Result<()>{
        if program.is_empty()||program.len()>crate::fonts::truetype::MAX_FONT_BYTES{return Err(Error::Limit("replacement font bytes"));}
        if program.starts_with(b"OTTO"){program=crate::fonts::cff::opentype_cff(&program)?.to_vec();}
        let old=self.fonts.get(face.name()).map_or(0,Vec::len);
        let total=self.bytes-old+program.len();
        if total>32*1024*1024{return Err(Error::Limit("replacement font set bytes"));}
        // Only formats with native name-based outlines are accepted.
        if !(program.starts_with(&[1,0])||program.starts_with(&[0x80,1])||program.starts_with(b"%!")){
            return Err(Error::Unsupported("replacement font format"));
        }
        self.fonts.insert(face.name().to_vec(),program);self.bytes=total;Ok(())
    }
    fn get(&self,face:StandardFont)->Option<&[u8]>{self.fonts.get(face.name()).map(Vec::as_slice)}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SubstitutedBitmap{
    pub bitmap:Bitmap,
    /// Unique Standard 14 faces for which caller-supplied outlines were loaded.
    pub substituted_fonts:Vec<StandardFont>,
    /// Bounded vector/image ICC profile decisions. Native paths do not
    /// imply complete intent handling; opacity masks are not display colors.
    pub icc_profiles:Vec<crate::icc::ProfileDiagnostic>,
}
fn viewport(p:&PageInfo,scale:f64)->Result<(Matrix,usize,usize,f64,f64)>{
    let [x0,y0,x1,y1]=p.crop_box;
    if !scale.is_finite()||scale<=0.||p.crop_box.iter().any(|v|!v.is_finite())||x1<=x0||y1<=y0{return Err(Error::Invalid("render viewport"));}
    let (m,w,h)=match p.rotation{
        0=>(Matrix([scale,0.,0.,-scale,-x0*scale,y1*scale]),(x1-x0)*scale,(y1-y0)*scale),
        90=>(Matrix([0.,scale,scale,0.,-y0*scale,-x0*scale]),(y1-y0)*scale,(x1-x0)*scale),
        180=>(Matrix([-scale,0.,0.,scale,x1*scale,-y0*scale]),(x1-x0)*scale,(y1-y0)*scale),
        270=>(Matrix([0.,-scale,-scale,0.,y1*scale,x1*scale]),(y1-y0)*scale,(x1-x0)*scale),
        _=>return Err(Error::Invalid("render rotation")),
    };
    if !w.is_finite()||!h.is_finite()||w<=0.||h<=0.||w.ceil()>MAX_SIDE as f64||h.ceil()>MAX_SIDE as f64{return Err(Error::Limit("render dimensions"));}
    let (iw,ih)=(w.ceil() as usize,h.ceil() as usize);
    if iw*ih>MAX_PIXELS{return Err(Error::Limit("render pixels"));}
    Ok((m,iw,ih,w,h))
}
fn rgb(c:Color)->[u8;3]{
    let values=match c{Color::Gray(g)=>[g;3],Color::Rgb(v)=>v,Color::Cmyk([c,m,y,k])=>[1.-(c+k).min(1.),1.-(m+k).min(1.),1.-(y+k).min(1.)]};
    values.map(|x|(x.clamp(0.,1.)*255.).round() as u8)
}
fn add(a:Point,b:Point)->Point{[a[0]+b[0],a[1]+b[1]]}
fn sub(a:Point,b:Point)->Point{[a[0]-b[0],a[1]-b[1]]}
fn mul(a:Point,s:f64)->Point{[a[0]*s,a[1]*s]}
fn cross(a:Point,b:Point)->f64{a[0]*b[1]-a[1]*b[0]}
fn len(a:Point)->f64{a[0].hypot(a[1])}
fn mid(a:Point,b:Point)->Point{mul(add(a,b),0.5)}
fn normal(a:Point)->Point{[-a[1],a[0]]}
fn inverse(m:Matrix)->Result<Matrix>{
    let [a,b,c,d,e,f]=m.0;let det=a*d-b*c;
    if !det.is_finite()||det==0.{return Err(Error::Unsupported("singular render transform"));}
    let inv=Matrix([d/det,-b/det,-c/det,a/det,(c*f-d*e)/det,(b*e-a*f)/det]);
    if inv.0.iter().any(|v|!v.is_finite()){return Err(Error::Limit("inverse transform"));}
    Ok(inv)
}
struct Budget{work:usize,stage:&'static str}
impl Default for Budget{fn default()->Self{Self{work:0,stage:"raster work (setup)"}}}
impl Budget{fn charge(&mut self,n:usize)->Result<()>{
    self.work=self.work.checked_add(n).ok_or(Error::Limit(self.stage))?;
    if self.work>MAX_WORK{return Err(Error::Limit(self.stage));}Ok(())
}}
#[derive(Debug)]
struct Subpath{points:Vec<Point>,closed:bool}
fn push(p:&mut Vec<Point>,v:Point,b:&mut Budget)->Result<()>{
    b.charge(1)?;if p.len()>=MAX_EDGES{return Err(Error::Limit("flattened path"));}p.push(v);Ok(())
}
fn cubic(p:&mut Vec<Point>,a:Point,b:Point,c:Point,d:Point,depth:usize,budget:&mut Budget)->Result<()>{
    budget.charge(1)?;
    // Control points near the chord's one-third points also detect collinear backtracking.
    if len(sub(b,add(a,mul(sub(d,a),1./3.)))).max(len(sub(c,add(a,mul(sub(d,a),2./3.)))))<=0.20{
        return push(p,d,budget);
    }
    if depth>=20{return Err(Error::Limit("curve subdivision"));}
    let ab=mid(a,b);let bc=mid(b,c);let cd=mid(c,d);let abc=mid(ab,bc);let bcd=mid(bc,cd);let m=mid(abc,bcd);
    cubic(p,a,ab,abc,m,depth+1,budget)?;cubic(p,m,bcd,cd,d,depth+1,budget)
}
fn flatten(path:&Path,m:Matrix,budget:&mut Budget)->Result<Vec<Subpath>>{
    let mut out=Vec::new();let mut p=Vec::new();let mut start=None;let mut at=None;let mut count=0usize;
    fn flush(out:&mut Vec<Subpath>,p:&mut Vec<Point>,closed:bool,count:&mut usize)->Result<()>{
        if !p.is_empty(){
            *count=count.checked_add(p.len()).ok_or(Error::Limit("flattened path"))?;
            if *count>MAX_EDGES{return Err(Error::Limit("flattened path"));}
            out.push(Subpath{points:std::mem::take(p),closed});
        }Ok(())
    }
    for seg in path{
        match *seg{
            Segment::Move(q)=>{flush(&mut out,&mut p,false,&mut count)?;let q=m.point(q)?;push(&mut p,q,budget)?;start=Some(q);at=Some(q);}
            Segment::Line(q)=>{let prev=at.ok_or(Error::Invalid("line start"))?;if p.is_empty(){push(&mut p,prev,budget)?;}let q=m.point(q)?;push(&mut p,q,budget)?;at=Some(q);}
            Segment::Cubic(a,b,c)=>{let prev=at.ok_or(Error::Invalid("curve start"))?;if p.is_empty(){push(&mut p,prev,budget)?;}let c=m.point(c)?;cubic(&mut p,prev,m.point(a)?,m.point(b)?,c,0,budget)?;at=Some(c);}
            Segment::Close=>{let q=start.ok_or(Error::Invalid("close start"))?;flush(&mut out,&mut p,true,&mut count)?;at=Some(q);}
        }
    }
    flush(&mut out,&mut p,false,&mut count)?;Ok(out)
}
type Edge=(Point,Point);
fn edges(paths:&[Subpath])->Result<Vec<Edge>>{
    let mut out=Vec::new();
    for p in paths{
        if p.points.len()<2{continue;}
        for i in 0..p.points.len(){
            let (a,b)=(p.points[i],p.points[(i+1)%p.points.len()]);
            if a[1]!=b[1]{if out.len()>=MAX_EDGES{return Err(Error::Limit("raster edges"));}out.push((a,b));}
        }
    }
    Ok(out)
}
fn span_bound(x:f64,max:usize)->usize{(x*2.-0.5).ceil().clamp(0.,max as f64) as usize}
/// Coverage is stored only in the pixel-aligned rectangle touched by a path.
/// Subpixel positions remain in absolute viewport coordinates.
struct Coverage{x:usize,y:usize,w:usize,h:usize,data:Vec<u8>}
impl Coverage{
    fn empty()->Self{Self{x:0,y:0,w:0,h:0,data:Vec::new()}}
    fn bounds(&self,other:&Self)->(usize,usize,usize,usize){
        let x=self.x.max(other.x);let y=self.y.max(other.y);
        (x,y,(self.x+self.w).min(other.x+other.w).max(x),(self.y+self.h).min(other.y+other.h).max(y))
    }
    fn at(&self,x:usize,y:usize)->u8{
        if x<self.x||y<self.y||x>=self.x+self.w||y>=self.y+self.h{0}
        else{self.data[(y-self.y)*self.w+x-self.x]}
    }
    fn intersect_clip(&self,clip:&Self,budget:&mut Budget)->Result<Self>{
        let (x,y,x1,y1)=self.bounds(clip);
        if x==x1||y==y1{return Ok(Self::empty());}
        let (w,h)=(x1-x,y1-y);
        budget.charge(w*h)?;
        let mut data=vec![0;w*h];
        for row in 0..h{
            let a=(y+row-self.y)*self.w+x-self.x;
            let b=(y+row-clip.y)*clip.w+x-clip.x;
            for col in 0..w{data[row*w+col]=self.data[a+col]&clip.data[b+col];}
        }
        Ok(Self{x,y,w,h,data})
    }
}
fn mask(paths:&[Subpath],rule:FillRule,w:usize,h:usize,clip:&Coverage,budget:&mut Budget)->Result<Coverage>{
    let e=edges(paths)?;
    if e.is_empty(){return Ok(Coverage::empty());}
    budget.charge(e.len())?;
    let lo=|i:usize|e.iter().map(|(a,b)|a[i].min(b[i])).fold(f64::INFINITY,f64::min);
    let hi=|i:usize|e.iter().map(|(a,b)|a[i].max(b[i])).fold(f64::NEG_INFINITY,f64::max);
    // Restrict the output rectangle before allocating or scanning. Edges
    // outside the horizontal clip still contribute to the winding number.
    let (sx0,sx1)=(span_bound(lo(0),w*2).max(clip.x*2),span_bound(hi(0),w*2).min((clip.x+clip.w)*2));
    let (sy0,sy1)=(span_bound(lo(1),h*2).max(clip.y*2),span_bound(hi(1),h*2).min((clip.y+clip.h)*2));
    if sx0>=sx1||sy0>=sy1{return Ok(Coverage::empty());}
    let (x,y)=(sx0/2,sy0/2);
    let (mw,mh)=(sx1.div_ceil(2)-x,sy1.div_ceil(2)-y);
    budget.charge(mw*mh)?;
    let mut out=Coverage{x,y,w:mw,h:mh,data:vec![0;mw*mh]};
    // Sort edges by their first covered subpixel row. Keep only edges
    // intersecting the current row, computing x from original endpoints
    // rather than accumulating incremental floating-point drift.
    let mut events=Vec::with_capacity(e.len());
    for (a,b) in e{
        let first=span_bound(a[1].min(b[1]),h*2).max(sy0);
        let last=span_bound(a[1].max(b[1]),h*2).min(sy1);
        if first<last{events.push((first,last,a,b));}
    }
    budget.charge(events.len().saturating_mul(1+usize::BITS as usize-events.len().max(1).leading_zeros() as usize))?;
    events.sort_unstable_by_key(|e|e.0);
    let mut next=0;let mut active:Vec<usize>=Vec::new();
    let mut hits=Vec::<(f64,i32)>::new();
    for sy in sy0..sy1{
        budget.charge(active.len()+1)?;
        active.retain(|&i|events[i].1>sy);
        while next<events.len()&&events[next].0<=sy{
            if events[next].1>sy{active.push(next);}
            next+=1;
        }
        budget.charge(active.len())?;
        let y=(sy as f64+0.5)*0.5;hits.clear();
        for &i in &active{
            let (_,_,a,b)=events[i];
            hits.push((a[0]+(y-a[1])/(b[1]-a[1])*(b[0]-a[0]),if b[1]>a[1]{1}else{-1}));
        }
        budget.charge(hits.len().saturating_mul(1+usize::BITS as usize-hits.len().max(1).leading_zeros() as usize))?;
        hits.sort_unstable_by(|a,b|a.0.total_cmp(&b.0));
        let mut winding=0i32;let mut prev=0.;
        for &(x,delta) in &hits{
            let inside=match rule{FillRule::NonZero=>winding!=0,FillRule::EvenOdd=>winding%2!=0};
            if inside{
                let a=span_bound(prev,w*2).clamp(sx0,sx1);let b=span_bound(x,w*2).clamp(sx0,sx1);
                budget.charge(b.saturating_sub(a))?;
                for sx in a..b{out.data[(sy/2-out.y)*mw+sx/2-out.x]|=1<<((sy%2)*2+sx%2);}
            }
            winding+=delta;prev=x;
        }
    }
    Ok(out)
}
fn polygon(out:&mut Vec<Subpath>,mut pts:Vec<Point>,count:&mut usize,budget:&mut Budget)->Result<()>{
    if pts.len()<3{return Ok(());}
    let area=pts.iter().zip(pts.iter().cycle().skip(1)).take(pts.len()).map(|(&a,&b)|cross(a,b)).sum::<f64>();
    if !area.is_finite(){return Err(Error::Limit("stroke polygon"));}
    if area==0.{return Ok(());}
    if area<0.{pts.reverse();}
    *count=count.checked_add(pts.len()).ok_or(Error::Limit("stroke edges"))?;
    if *count>MAX_EDGES{return Err(Error::Limit("stroke edges"));}
    budget.charge(pts.len())?;
    out.push(Subpath{points:pts,closed:true});Ok(())
}
fn disc(out:&mut Vec<Subpath>,p:Point,r:f64,count:&mut usize,b:&mut Budget)->Result<()>{
    let n=(std::f64::consts::PI*(r/0.2).sqrt()).ceil().max(12.) as usize;
    if n>4096{return Err(Error::Limit("round stroke subdivision"));}
    let pts=(0..n).map(|i|{let t=std::f64::consts::TAU*i as f64/n as f64;add(p,[r*t.cos(),r*t.sin()])}).collect();
    polygon(out,pts,count,b)
}
// Split flattened device-space paths into continuous painted runs. Dash
// lengths and phase are defined in stroke user space and uniformly scaled.
// Zero-length elements (including dotted round-cap patterns) are deliberately
// rejected in this first implementation rather than silently erased.
fn dash_paths(paths:&[Subpath],s:&Stroke,scale:f64,budget:&mut Budget)->Result<Vec<Subpath>>{
    let mut pattern:Vec<f64>=s.dash.iter().map(|x|x*scale).collect();
    if pattern.iter().any(|v|!v.is_finite()||*v<=0.){
        return Err(Error::Unsupported("zero or nonfinite dash length"));
    }
    if pattern.len()%2==1{let copy=pattern.clone();pattern.extend(copy);}
    let period:f64=pattern.iter().sum();
    let phase=s.dash_phase*scale;
    if !period.is_finite()||period<=0.||!phase.is_finite()||phase<0.{
        return Err(Error::Invalid("dash period or phase"));
    }
    let mut offset=phase%period;let mut initial=0;
    while offset>=pattern[initial]{
        offset-=pattern[initial];initial=(initial+1)%pattern.len();
        if initial==0{offset=0.;break;}
    }
    let initial_remaining=pattern[initial]-offset;
    let mut out=Vec::new();let mut work=0usize;let mut points=0usize;
    fn flush(out:&mut Vec<Subpath>,run:&mut Vec<Point>){
        if run.len()>=2{out.push(Subpath{points:std::mem::take(run),closed:false});}else{run.clear();}
    }
    for path in paths{
        let mut p=path.points.clone();p.dedup();
        if path.closed&&p.len()>1&&p.first()==p.last(){p.pop();}
        if p.len()<2{
            if initial%2==0&&path.points.len()>1{
                points=points.checked_add(path.points.len()).ok_or(Error::Limit("dash points"))?;
                if points>MAX_EDGES{return Err(Error::Limit("dash points"));}
                out.push(Subpath{points:path.points.clone(),closed:false});
            }
            continue;
        }
        let start=out.len();let mut run=Vec::new();let mut idx=initial;let mut remaining=initial_remaining;
        let n=p.len();let segments=if path.closed{n}else{n-1};
        for i in 0..segments{
            let a=p[i];let b=p[(i+1)%n];let delta=sub(b,a);let distance=len(delta);
            if distance==0.{continue;}
            if !distance.is_finite(){return Err(Error::Invalid("dash segment length"));}
            let mut at=0.;
            while at<distance{
                budget.charge(1)?;work+=1;
                if work>MAX_EDGES{return Err(Error::Limit("dash subdivisions"));}
                let left=distance-at;let step=remaining.min(left);
                let end=if step>=left{distance}else{at+step};
                if end<=at{return Err(Error::Limit("dash precision"));}
                let from=if at==0.{a}else{add(a,mul(delta,at/distance))};
                let to=if end==distance{b}else{add(a,mul(delta,end/distance))};
                if idx%2==0{
                    if run.is_empty(){run.push(from);points+=1;}
                    if run.last()!=Some(&to){run.push(to);points+=1;}
                    if points>MAX_EDGES{return Err(Error::Limit("dash points"));}
                }
                at=end;
                if step>=remaining{
                    if idx%2==0{flush(&mut out,&mut run);}
                    idx=(idx+1)%pattern.len();remaining=pattern[idx];
                }else{remaining-=step;}
            }
        }
        flush(&mut out,&mut run);
        // Closed contours whose painted runs meet at the seam need a join,
        // not two caps. Reconnect the last and first runs exactly once.
        if path.closed&&out.len()>start{
            let last=out.len()-1;
            if out[start].points.first()==Some(&p[0])&&out[last].points.last()==Some(&p[0]){
                if start==last{out[start].closed=true;}
                else{
                    let mut tail=out.pop().ok_or(Error::Invalid("dash seam"))?.points;
                    tail.extend(out[start].points.iter().skip(1).copied());
                    out[start].points=tail;
                }
            }
        }
    }
    Ok(out)
}
fn stroke_paths(paths:&[Subpath],s:&Stroke,view:Matrix,budget:&mut Budget)->Result<Vec<Subpath>>{
    let m=view.compose(s.ctm)?;let [a,b,c,d,_,_]=m.0;
    let (sx,sy)=(a.hypot(b),c.hypot(d));
    // First implementation supports uniform scaling/rotation/reflection. No false
    // approximation of anisotropic pen geometry.
    if sx==0.||sy==0.||(sx-sy).abs()>1e-8*sx.max(sy)||(a*c+b*d).abs()>1e-8*sx*sy{
        return Err(Error::Unsupported("nonuniform stroke transform"));
    }
    let r=if s.width==0.{0.5}else{s.width*sx*0.5};
    if !r.is_finite()||r<=0.||r>1e6{return Err(Error::Limit("stroke radius"));}
    let dashed;
    let paths=if s.dash.is_empty(){paths}else{dashed=dash_paths(paths,s,sx,budget)?;&dashed};
    let mut out=Vec::new();let mut count=0usize;
    for path in paths{
        let mut p=path.points.clone();p.dedup();
        if path.closed&&p.len()>1&&p.first()==p.last(){p.pop();}
        if p.len()<2{
            if path.points.len()>1&&s.cap==LineCap::Round&&!p.is_empty(){disc(&mut out,p[0],r,&mut count,budget)?;}
            continue;
        }
        let n=p.len();let segments=if path.closed{n}else{n-1};
        let mut dirs=Vec::with_capacity(segments);
        for i in 0..segments{
            let (a,b)=(p[i],p[(i+1)%n]);let u=mul(sub(b,a),1./len(sub(b,a)));dirs.push(u);
            let k=mul(normal(u),r);
            let a=if !path.closed&&i==0&&s.cap==LineCap::Square{sub(a,mul(u,r))}else{a};
            let b=if !path.closed&&i+1==segments&&s.cap==LineCap::Square{add(b,mul(u,r))}else{b};
            polygon(&mut out,vec![add(a,k),add(b,k),sub(b,k),sub(a,k)],&mut count,budget)?;
        }
        if !path.closed&&s.cap==LineCap::Round{disc(&mut out,p[0],r,&mut count,budget)?;disc(&mut out,p[n-1],r,&mut count,budget)?;}
        let first=if path.closed{0}else{1};let end=if path.closed{n}else{n-1};
        for i in first..end{
            let (u,v)=(dirs[(i+segments-1)%segments],dirs[i%segments]);let turn=cross(u,v);
            if s.join==LineJoin::Round{disc(&mut out,p[i],r,&mut count,budget)?;continue;}
            if turn.abs()<1e-10{continue;}
            let side=if turn>0.{-1.}else{1.};
            let k=add(p[i],mul(normal(u),r*side));let l=add(p[i],mul(normal(v),r*side));
            let mut poly=vec![p[i],k];
            if s.join==LineJoin::Miter{
                let t=cross(sub(l,k),v)/turn;let tip=add(k,mul(u,t));
                if len(sub(tip,p[i]))<=s.miter_limit*r{poly.push(tip);}
            }
            poly.push(l);polygon(&mut out,poly,&mut count,budget)?;
        }
    }
    Ok(out)
}
// Each group uses 28 bytes per output pixel, plus 16 bytes for non-isolated coverage.
// The cap counts all live offscreen buffers, excluding the existing root surface.
const MAX_GROUP_BYTES:usize=128*1024*1024;
const MAX_GROUP_DEPTH:usize=8;
// Cumulative generated mask bytes per render, even if earlier masks were dropped.
const MAX_SOFT_MASK_BYTES:usize=64*1024*1024;
// Buffer coordinates are in page pixels; each pixel contains four samples.
#[derive(Clone,Copy,Debug)]
struct Region{x:usize,y:usize,w:usize,h:usize}
impl Region{
    fn pixels(self)->usize{self.w*self.h}
    fn index(self,x:usize,y:usize,sample:usize)->Option<usize>{
        if x<self.x||y<self.y||x>=self.x+self.w||y>=self.y+self.h{return None;}
        Some(((y-self.y)*self.w+x-self.x)*4+sample)
    }
    fn page_sample(self,i:usize,page_w:usize)->usize{
        let p=i/4;((self.y+p/self.w)*page_w+self.x+p%self.w)*4+i%4
    }
}
struct GroupLayer{
    region:Region,allocated:usize,
    samples:Vec<u8>,alpha:Option<Vec<f32>>,opacity:f64,blend:BlendMode,stack_depth:usize,
    coverage:Option<Vec<f32>>,knockout:bool,soft_mask:Option<Arc<Vec<u8>>>,
}
struct Surface{w:usize,h:usize,text_quality:TextQuality,region:Region,samples:Vec<u8>,clip:Arc<Coverage>,stack:Vec<(Arc<Coverage>,f64,f64,BlendMode,Option<Arc<Vec<u8>>>)>,budget:Budget,fill_alpha:f64,stroke_alpha:f64,stencil:[u8;3],blend_mode:BlendMode,alpha:Option<Vec<f32>>,groups:Vec<GroupLayer>,soft_mask:Option<Arc<Vec<u8>>>,mask_bytes:usize,mask_clip:Option<Arc<Coverage>>,page_clip:Arc<Coverage>,image_cache:BTreeMap<((u32,u16),crate::icc::RenderingIntent),Arc<images::Image>>,image_cache_bytes:usize,icc_cache:images::IccCache}
impl Surface{
    fn local_index(&self,page_sample:usize)->Option<usize>{
        let p=page_sample/4;self.region.index(p%self.w,p/self.w,page_sample%4)
    }
    // coverage indices are local to the active child layer.
    fn mark_coverage(&mut self,index:usize,alpha:f64){
        if let Some(coverage)=self.groups.last_mut().and_then(|g|g.coverage.as_mut()){
            let a=f64::from(coverage[index]);
            coverage[index]=(alpha+a*(1.-alpha)) as f32;
        }
    }
    fn mask_factor(&self,i:usize)->f64{self.soft_mask.as_ref().map_or(1.,|m|f64::from(m[i])/255.)}
    fn blend(&mut self,i:usize,c:[u8;3],alpha:f64){
        let page_sample=i/3;
        let alpha=(alpha*self.mask_factor(page_sample)).clamp(0.,1.);
        if alpha==0.{return;}
        let Some(local)=self.local_index(page_sample) else{return;};
        let i=local*3;
        self.mark_coverage(local,alpha);
        let ab=self.alpha.as_ref().map_or(1.,|a|f64::from(a[local]));
        let ao=alpha+ab*(1.-alpha);
        if ab==0.||(alpha==1.&&self.blend_mode==BlendMode::Normal){
            self.samples[i..i+3].copy_from_slice(&c);
        }else{
            let b:[f64;3]=std::array::from_fn(|k|f64::from(self.samples[i+k])/255.);
            let src=c.map(|v|f64::from(v)/255.);
            let f=self.blend_mode.rgb(b,src);
            for k in 0..3{
                let premul=(1.-alpha)*ab*b[k]+alpha*((1.-ab)*src[k]+ab*f[k]);
                self.samples[i+k]=(premul/ao*255.).round().clamp(0.,255.)as u8;
            }
        }
        if let Some(a)=&mut self.alpha{a[local]=ao as f32;}
    }
    fn begin_group(&mut self,opacity:f64,blend:BlendMode,isolated:bool,knockout:bool)->Result<()>{
        // The mandatory Form BBox clip has already been applied. Isolated
        // soft masks instead start with the root viewport clip.
        let region=Region{x:self.clip.x,y:self.clip.y,w:self.clip.w,h:self.clip.h};
        let pixels=region.pixels();
        let allocated=pixels*if isolated{28}else{44};
        let used=self.groups.iter().map(|g|g.allocated).sum::<usize>();
        if self.groups.len()>=MAX_GROUP_DEPTH||used+allocated>MAX_GROUP_BYTES{return Err(Error::Limit("transparency group buffers"));}
        self.budget.charge(pixels*if isolated{4}else{8})?;
        let mut colors=vec![0;pixels*12];let mut alpha=vec![0.;pixels*4];
        if !isolated&&pixels>0{
            for row in 0..region.h{
                let a=self.region.index(region.x,region.y+row,0).ok_or(Error::Invalid("group outside parent region"))?;
                if self.region.index(region.x+region.w-1,region.y+row,3).is_none(){return Err(Error::Invalid("group outside parent region"));}
                let b=row*region.w*4;let n=region.w*4;
                colors[b*3..(b+n)*3].copy_from_slice(&self.samples[a*3..(a+n)*3]);
                if let Some(parent)=&self.alpha{alpha[b..b+n].copy_from_slice(&parent[a..a+n]);}
                else{alpha[b..b+n].fill(1.);}
            }
        }
        let parent=GroupLayer{region:std::mem::replace(&mut self.region,region),allocated,
            samples:std::mem::replace(&mut self.samples,colors),
            alpha:self.alpha.replace(alpha),opacity,blend,stack_depth:self.stack.len(),
            coverage:if isolated{None}else{Some(vec![0.;pixels*4])},knockout,soft_mask:self.soft_mask.take()};
        self.groups.push(parent);
        self.fill_alpha=1.;self.stroke_alpha=1.;self.blend_mode=BlendMode::Normal;
        Ok(())
    }
    fn end_group(&mut self)->Result<()>{
        let parent=self.groups.pop().ok_or(Error::Invalid("group stack underflow"))?;
        if self.stack.len()!=parent.stack_depth{return Err(Error::Invalid("group graphics stack"));}
        let region=std::mem::replace(&mut self.region,parent.region);
        let colors=std::mem::replace(&mut self.samples,parent.samples);
        let alpha=std::mem::replace(&mut self.alpha,parent.alpha).ok_or(Error::Invalid("missing group alpha"))?;
        self.blend_mode=parent.blend;self.soft_mask=parent.soft_mask;
        self.budget.charge(region.pixels()*if parent.coverage.is_some(){8}else{4})?;
        // Composite the original group region, not the child's final clip.
        if let Some(coverage)=parent.coverage{
            for (i,&ag) in coverage.iter().enumerate(){
                let ag=f64::from(ag);
                if ag<=0.{continue;}
                let page=region.page_sample(i,self.w);
                let scale=parent.opacity*self.mask_factor(page);
                if scale<=0.{continue;}
                let j=self.local_index(page).ok_or(Error::Invalid("group outside parent region"))?;
                let ab=self.alpha.as_ref().map_or(1.,|a|f64::from(a[j]));
                let ar=f64::from(alpha[i]);let src=i*3;let dst=j*3;
                if parent.blend==BlendMode::Normal{
                    let ao=ab*(1.-scale)+ar*scale;
                    for k in 0..3{
                        let p=f64::from(self.samples[dst+k])*ab*(1.-scale)+f64::from(colors[src+k])*ar*scale;
                        self.samples[dst+k]=if ao>0.{(p/ao).round().clamp(0.,255.)as u8}else{0};
                    }
                    if let Some(a)=&mut self.alpha{a[j]=ao as f32;}
                    self.mark_coverage(j,ag*scale);
                }else{
                    let c=std::array::from_fn(|k|((f64::from(colors[src+k])*ar-f64::from(self.samples[dst+k])*ab*(1.-ag))/ag).round().clamp(0.,255.)as u8);
                    self.blend(page*3,c,ag*parent.opacity);
                }
            }
        }else{
            for (i,&a) in alpha.iter().enumerate(){
                if a>0.{self.blend(region.page_sample(i,self.w)*3,[colors[i*3],colors[i*3+1],colors[i*3+2]],f64::from(a)*parent.opacity);}
            }
        }
        Ok(())
    }
    fn begin_soft_mask(&mut self)->Result<()>{
        if self.mask_clip.is_some(){return Err(Error::Unsupported("nested soft mask"));}
        let n=self.w*self.h*4;
        if n>MAX_SOFT_MASK_BYTES-self.mask_bytes{return Err(Error::Limit("soft mask bytes"));}
        self.mask_bytes+=n;
        // Soft masks have page-wide values, including BC outside the Form.
        // Do not restrict them to the parent group's current local region.
        self.mask_clip=Some(std::mem::replace(&mut self.clip,Arc::clone(&self.page_clip)));
        self.begin_group(1.,BlendMode::Normal,true,false)?;
        Ok(())
    }
    fn end_soft_mask(&mut self,luminosity:bool,backdrop:[f64;3])->Result<()>{
        let parent=self.groups.pop().ok_or(Error::Invalid("soft mask group stack"))?;
        if self.stack.len()!=parent.stack_depth{return Err(Error::Invalid("soft mask graphics stack"));}
        self.region=parent.region;
        let colors=std::mem::replace(&mut self.samples,parent.samples);
        let alpha=std::mem::replace(&mut self.alpha,parent.alpha).ok_or(Error::Invalid("soft mask alpha"))?;
        self.clip=self.mask_clip.take().ok_or(Error::Invalid("soft mask clip"))?;
        self.budget.charge(self.w*self.h*4)?;
        let mut values=Vec::with_capacity(alpha.len());
        for (i,&a) in alpha.iter().enumerate(){
            let a=f64::from(a);
            let v=if luminosity{
                let c=std::array::from_fn::<_,3,_>(|k|a*f64::from(colors[i*3+k])/255.+(1.-a)*backdrop[k]);
                0.3*c[0]+0.59*c[1]+0.11*c[2]
            }else{a};
            values.push((v.clamp(0.,1.)*255.).round()as u8);
        }
        self.soft_mask=Some(Arc::new(values));self.blend_mode=parent.blend;
        Ok(())
    }
    fn knockout_sample(&mut self,i:usize){
        let page=i/3;
        let Some(local)=self.local_index(page) else{return;};
        let Some(g)=self.groups.last_mut().filter(|g|g.knockout) else{return;};
        let a=self.alpha.as_mut().expect("group alpha");
        let i=local*3;
        if let Some(coverage)=g.coverage.as_mut(){
            coverage[local]=0.;
            let pixel=page/4;
            let j=g.region.index(pixel%self.w,pixel/self.w,page%4).expect("nonisolated region inside parent");
            self.samples[i..i+3].copy_from_slice(&g.samples[j*3..j*3+3]);
            a[local]=g.alpha.as_ref().map_or(1.,|a|a[j]);
        }else{
            self.samples[i..i+3].fill(0);a[local]=0.;
        }
    }
    fn paint(&mut self,mask:&Coverage,c:[u8;3],alpha:f64)->Result<()>{
        let (x,y,x1,y1)=mask.bounds(&self.clip);
        self.budget.charge((x1-x)*(y1-y)*4)?;
        for row in y..y1{
            for col in x..x1{
                let i=row*self.w+col;let m=mask.at(col,row)&self.clip.at(col,row);
                for s in 0..4{if m&(1<<s)!=0{let at=i*12+s*3;self.knockout_sample(at);self.blend(at,c,alpha);}}
            }
        }Ok(())
    }
    // Each existing 2x2 surface sample receives the average of four
    // fine samples within its quadrant. The coarse clip is deliberately
    // retained; this is coverage AA, not full-scene 4x4 supersampling.
    fn paint_fine_glyph(&mut self,outline:&Path,m:Matrix,c:[u8;3],alpha:f64)->Result<()>{
        let m=Matrix([2.,0.,0.,2.,0.,0.]).compose(m)?;
        let p=flatten(outline,m,&mut self.budget)?;
        // mask() uses only the bounding rectangle of this argument.
        let bounds=Coverage{x:self.clip.x*2,y:self.clip.y*2,w:self.clip.w*2,h:self.clip.h*2,data:Vec::new()};
        let fine=mask(&p,FillRule::NonZero,self.w*2,self.h*2,&bounds,&mut self.budget)?;
        if fine.w==0||fine.h==0{return Ok(());}
        let x0=fine.x/2;let y0=fine.y/2;
        let x1=(fine.x+fine.w).div_ceil(2);let y1=(fine.y+fine.h).div_ceil(2);
        self.budget.charge((x1-x0)*(y1-y0)*4)?;
        for y in y0..y1{for x in x0..x1{
            let clip=self.clip.at(x,y);
            for q in 0..4{
                if clip&(1<<q)==0{continue;}
                let n=fine.at(x*2+q%2,y*2+q/2).count_ones();
                if n!=0{self.blend((y*self.w+x)*12+q*3,c,alpha*f64::from(n)/4.);}
            }
        }}
        Ok(())
    }
    fn paint_area_glyph(&mut self,outline:&Path,m:Matrix,c:[u8;3],alpha:f64)->Result<()>{
        let m=Matrix([2.,0.,0.,2.,0.,0.]).compose(m)?;
        let paths=flatten(outline,m,&mut self.budget)?;
        let area=glyph_area::coverage(&paths,&self.clip,&mut self.budget)?;
        let(x0,y0,x1,y1)=area.bounds();
        self.budget.charge((x1-x0)*(y1-y0)*4)?;
        for y in y0..y1{for x in x0..x1{
            let clip=self.clip.at(x,y);
            for q in 0..4{
                if clip&(1<<q)==0{continue;}
                let a=area.at(x*2+q%2,y*2+q/2);
                if a>0.{self.blend((y*self.w+x)*12+q*3,c,alpha*a);}
            }
        }}
        Ok(())
    }
    fn image(&mut self,doc:&Document<'_>,id:(u32,u16),m:Matrix,intent:crate::icc::RenderingIntent)->Result<()>{
        self.budget.stage="raster work (image decode)";
        let img=if let Some(img)=self.image_cache.get(&(id,intent)){Arc::clone(img)}else{
            let img=Arc::new(images::load(doc,id,&mut self.budget,&mut self.icc_cache,intent)?);
            let bytes=img.rgb.capacity()+img.alpha.as_ref().map_or(0,Vec::capacity);
            // No eviction/reload churn, no caching unbounded numbers of tiny images.
            if self.image_cache.len()<128&&bytes<=32*1024*1024-self.image_cache_bytes{
                self.image_cache_bytes+=bytes;self.image_cache.insert((id,intent),Arc::clone(&img));
            }
            img
        };
        self.budget.stage="raster work (image sampling)";
        let inv=inverse(m)?;
        let corners=[m.point([0.,0.])?,m.point([1.,0.])?,m.point([0.,1.])?,m.point([1.,1.])?];
        let lo=|axis:usize|corners.iter().map(|p|p[axis]).fold(f64::INFINITY,f64::min);
        let hi=|axis:usize|corners.iter().map(|p|p[axis]).fold(f64::NEG_INFINITY,f64::max);
        let (x0,x1)=(span_bound(lo(0),self.w*2).max(self.clip.x*2),span_bound(hi(0),self.w*2).min((self.clip.x+self.clip.w)*2));
        let (y0,y1)=(span_bound(lo(1),self.h*2).max(self.clip.y*2),span_bound(hi(1),self.h*2).min((self.clip.y+self.clip.h)*2));
        if x0>=x1||y0>=y1{return Ok(());}
        self.budget.charge((x1-x0)*(y1-y0)*if img.interpolate{4}else{1})?;
        for y in y0..y1{for x in x0..x1{
            let i=y/2*self.w+x/2;let s=(y%2)*2+x%2;
            if self.clip.at(x/2,y/2)&(1<<s)==0{continue;}
            let [u,v]=inv.point([(x as f64+0.5)*0.5,(y as f64+0.5)*0.5])?;
            if !(0.0..1.0).contains(&u)||!(0.0..1.0).contains(&v){continue;}
            let (color,alpha)=img.sample(u,1.-v,self.stencil);
            let alpha=alpha*self.fill_alpha;
            self.blend(i*12+s*3,color,alpha);
        }}Ok(())
    }
}
fn render(doc:&Document<'_>,page:&PageInfo,list:&DisplayList,opt:Options,substitutes:Option<&StandardFontSet>,text_quality:TextQuality,degree:crate::icc::AdaptationDegree)->Result<SubstitutedBitmap>{
    let (view,w,h,exact_w,exact_h)=viewport(page,opt.scale)?;
    let mut clip=vec![0u8;w*h];
    for y in 0..span_bound(exact_h,h*2){for x in 0..span_bound(exact_w,w*2){clip[y/2*w+x/2]|=1<<((y%2)*2+x%2);}}
    let clip=Arc::new(Coverage{x:0,y:0,w,h,data:clip});
    let mut s=Surface{w,h,text_quality,region:Region{x:0,y:0,w,h},samples:vec![255;w*h*12],clip:Arc::clone(&clip),stack:Vec::new(),budget:Budget::default(),fill_alpha:1.,stroke_alpha:1.,stencil:[0;3],blend_mode:BlendMode::Normal,alpha:None,groups:Vec::new(),soft_mask:None,mask_bytes:0,mask_clip:None,page_clip:clip,image_cache:BTreeMap::new(),image_cache_bytes:0,icc_cache:images::IccCache::default()};
    s.icc_cache.degree=degree;
    let mut missing_text=false;
    let mut text=rendertext::TextRenderer::new();
    for c in &list.commands{
        s.budget.stage=match c{
            Command::Clip{..}=>"raster work (clip)",
            Command::Paint{..}=>"raster work (path)",
            Command::Image{..}=>"raster work (image)",
            Command::Text{..}=>"raster work (text)",
            Command::BeginGroup{..}|Command::BeginNonIsolatedGroup{..}|Command::BeginKnockoutGroup{..}|Command::EndGroup=>"raster work (group)",
            Command::BeginSoftMask|Command::EndSoftMask{..}=>"raster work (soft mask)",
            _=>"raster work (state)",
        };
        s.budget.charge(1)?;
        match c{
            Command::Save=>{
                if s.stack.len()>=64||(s.stack.len()+2)*w*h>MAX_CLIP_BYTES{return Err(Error::Limit("raster clip stack"));}
                s.stack.push((Arc::clone(&s.clip),s.fill_alpha,s.stroke_alpha,s.blend_mode,s.soft_mask.clone()));text.save();
            }
            Command::Restore=>{let (clip,fa,sa,bm,sm)=s.stack.pop().ok_or(Error::Invalid("raster restore"))?;s.clip=clip;s.fill_alpha=fa;s.stroke_alpha=sa;s.blend_mode=bm;s.soft_mask=sm;text.restore()?;}
            Command::Opacity{fill,stroke}=>{s.fill_alpha=*fill;s.stroke_alpha=*stroke;}
            Command::Blend(b)=>s.blend_mode=*b,
            Command::BeginGroup{opacity,blend}=>s.begin_group(*opacity,*blend,true,false)?,
            Command::BeginNonIsolatedGroup{opacity,blend}=>s.begin_group(*opacity,*blend,false,false)?,
            Command::BeginKnockoutGroup{opacity,blend,isolated}=>s.begin_group(*opacity,*blend,*isolated,true)?,
            Command::EndGroup=>s.end_group()?,
            Command::BeginSoftMask=>s.begin_soft_mask()?,
            Command::EndSoftMask{luminosity,backdrop}=>s.end_soft_mask(*luminosity,*backdrop)?,
            Command::ClearSoftMask=>s.soft_mask=None,
            Command::StencilColor(c)=>s.stencil=rgb(*c),
            Command::Clip{path,rule}=>{
                let p=flatten(path,view,&mut s.budget)?;
                let m=mask(&p,*rule,w,h,&s.clip,&mut s.budget)?;
                s.clip=Arc::new(m.intersect_clip(&s.clip,&mut s.budget)?);
            }
            Command::Paint{path,fill,stroke}=>{
                let p=flatten(path,view,&mut s.budget)?;
                if let Some((c,rule))=fill{let m=mask(&p,*rule,w,h,&s.clip,&mut s.budget)?;s.paint(&m,rgb(*c),s.fill_alpha)?;}
                if let Some(st)=stroke{let p=stroke_paths(&p,st,view,&mut s.budget)?;let m=mask(&p,FillRule::NonZero,w,h,&s.clip,&mut s.budget)?;s.paint(&m,rgb(st.color),s.stroke_alpha)?;}
            }
            Command::Image{object_id,ctm,intent}=>s.image(doc,*object_id,view.compose(*ctm)?,*intent)?,
            Command::Text{operation,ctm,fill,stroke,resource_owner,..}=>{
                // Explicit preview mode skips all text, as in B10.1. Strict mode never silently skips.
                if opt.allow_missing_text{missing_text=true;continue;}
                text.execute(doc,page,Some(&mut s),operation,view,*ctm,*fill,stroke,substitutes,*resource_owner)?;
            }
        }
    }
    if !s.stack.is_empty()||!s.groups.is_empty()||s.alpha.is_some(){return Err(Error::Invalid("unbalanced raster stack"));}
    let mut rgba=Vec::with_capacity(w*h*4);
    for samples in s.samples.chunks_exact(12){
        for c in 0..3{rgba.push(((u16::from(samples[c])+u16::from(samples[c+3])+u16::from(samples[c+6])+u16::from(samples[c+9])+2)/4) as u8);}
        rgba.push(255);
    }
    let mut icc_profiles=list.icc_profiles.clone();
    icc_profiles.extend(s.icc_cache.diagnostics);
    Ok(SubstitutedBitmap{bitmap:Bitmap{width:w,height:h,rgba,missing_text},substituted_fonts:text.substituted_fonts(),icc_profiles})
}
impl Document<'_>{
    pub fn render_page(&self,page:&PageInfo,options:Options,content_limit:usize)->Result<Bitmap>{
        // Reject invalid viewport before decoding page content.
        viewport(page,options.scale)?;
        let list=self.page_display_list(page,content_limit)?;
        Ok(render(self,page,&list,options,None,TextQuality::Standard,crate::icc::AdaptationDegree::default())?.bitmap)
    }
    /// Explicit substitute outlines; caller must provide appropriately licensed,
    /// compatible font programs. Pixel equivalence to original fonts is not promised.
    pub fn render_page_with_standard_fonts(&self,page:&PageInfo,options:Options,content_limit:usize,fonts:&StandardFontSet)->Result<SubstitutedBitmap>{
        viewport(page,options.scale)?;
        let list=self.page_display_list(page,content_limit)?;
        render(self,page,&list,options,Some(fonts),TextQuality::Standard,crate::icc::AdaptationDegree::default())
    }
    /// Opt-in glyph coverage quality, with optional caller-owned substitute fonts.
    /// Fine mode averages 4x4 coverage into the existing 2x2 compositing grid.
    pub fn render_page_with_text_quality(&self,page:&PageInfo,options:Options,content_limit:usize,fonts:Option<&StandardFontSet>,quality:TextQuality)->Result<SubstitutedBitmap>{
        self.render_page_with_adaptation(page,options,content_limit,fonts,quality,crate::icc::AdaptationDegree::default())
    }
    /// Opt-in source CHAD policy; fixed for one page, for absolute matrix/TRC ICC only.
    pub fn render_page_with_adaptation(&self,page:&PageInfo,options:Options,content_limit:usize,fonts:Option<&StandardFontSet>,quality:TextQuality,degree:crate::icc::AdaptationDegree)->Result<SubstitutedBitmap>{
        viewport(page,options.scale)?;
        let list=self.page_display_list_with_adaptation(page,content_limit,degree)?;
        render(self,page,&list,options,fonts,quality,degree)
    }
    /// Strict unhinted outline geometry, without allocating or painting a bitmap.
    /// Uses the supported display-list interpreter, including Form resources.
    /// Does not test clipping, occlusion, stroke extents or semantic math roles.
    pub fn page_glyph_geometry(&self,page:&PageInfo,content_limit:usize,fonts:Option<&StandardFontSet>)->Result<PageGlyphGeometry>{
        let list=self.page_display_list(page,content_limit)?;
        rendertext::TextRenderer::collect_geometry(self,page,&list,fonts)
    }
    /// Local geometric formula candidates and their original glyph records.
    /// The region is in unrotated PDF user space; no raster allocation.
    pub fn page_math_geometry(&self,page:&PageInfo,content_limit:usize,fonts:Option<&StandardFontSet>,region:[f64;4])->Result<(PageGlyphGeometry,math_geometry::MathRegion)>{
        let list=self.page_display_list(page,content_limit)?;
        let geometry=rendertext::TextRenderer::collect_geometry(self,page,&list,fonts)?;
        let math=math_geometry::analyze_region(&geometry,&list,region)?;
        Ok((geometry,math))
    }
}
