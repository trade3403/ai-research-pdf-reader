//! Independent CPU rasterizer. 2x2 sample coverage, opaque white background.
use crate::{graphics::{BlendMode,Color,Command,DisplayList,FillRule,LineCap,LineJoin,Path,Segment,Stroke},text::{Matrix,Point},Document,Error,PageInfo,Result};
use std::sync::Arc;
mod png;
mod rendertext;
mod images;
use crate::fonts::standard::StandardFont;
use std::collections::BTreeMap;
const MAX_PIXELS:usize=4_000_000;
const MAX_SIDE:usize=8192;
const MAX_EDGES:usize=200_000;
const MAX_WORK:usize=200_000_000;
const MAX_CLIP_BYTES:usize=64*1024*1024;
#[derive(Debug,Clone,Copy)]
pub struct Options{pub scale:f64,pub allow_missing_text:bool}
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
    pub fn insert(&mut self,face:StandardFont,program:Vec<u8>)->Result<()>{
        if program.is_empty()||program.len()>crate::fonts::truetype::MAX_FONT_BYTES{return Err(Error::Limit("replacement font bytes"));}
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
#[derive(Default)]
struct Budget{work:usize}
impl Budget{fn charge(&mut self,n:usize)->Result<()>{
    self.work=self.work.checked_add(n).ok_or(Error::Limit("raster work"))?;
    if self.work>MAX_WORK{return Err(Error::Limit("raster work"));}Ok(())
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
    fn intersect_clip(&self,clip:&[u8],w:usize,h:usize,budget:&mut Budget)->Result<Vec<u8>>{
        // A clip is persistent and page-sized; only its intersecting rows need
        // sample comparisons. Charge the full allocation, unlike transient masks.
        budget.charge(w*h+self.w*self.h)?;
        let mut out=vec![0;w*h];
        for row in 0..self.h{
            let base=(self.y+row)*w+self.x;
            for col in 0..self.w{out[base+col]=self.data[row*self.w+col]&clip[base+col];}
        }
        Ok(out)
    }
}
fn mask(paths:&[Subpath],rule:FillRule,w:usize,h:usize,budget:&mut Budget)->Result<Coverage>{
    let e=edges(paths)?;
    if e.is_empty(){return Ok(Coverage::empty());}
    budget.charge(e.len())?;
    let lo=|i:usize|e.iter().map(|(a,b)|a[i].min(b[i])).fold(f64::INFINITY,f64::min);
    let hi=|i:usize|e.iter().map(|(a,b)|a[i].max(b[i])).fold(f64::NEG_INFINITY,f64::max);
    let (sx0,sx1)=(span_bound(lo(0),w*2),span_bound(hi(0),w*2));
    let (sy0,sy1)=(span_bound(lo(1),h*2),span_bound(hi(1),h*2));
    if sx0>=sx1||sy0>=sy1{return Ok(Coverage::empty());}
    let (x,y)=(sx0/2,sy0/2);
    let (mw,mh)=(sx1.div_ceil(2)-x,sy1.div_ceil(2)-y);
    budget.charge(mw*mh)?;
    let mut out=Coverage{x,y,w:mw,h:mh,data:vec![0;mw*mh]};
    let mut hits=Vec::<(f64,i32)>::new();
    for sy in sy0..sy1{
        budget.charge(e.len())?;
        let y=(sy as f64+0.5)*0.5;hits.clear();
        for &(a,b) in &e{
            if y>=a[1].min(b[1])&&y<a[1].max(b[1]){
                hits.push((a[0]+(y-a[1])/(b[1]-a[1])*(b[0]-a[0]),if b[1]>a[1]{1}else{-1}));
            }
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
fn stroke_paths(paths:&[Subpath],s:&Stroke,view:Matrix,budget:&mut Budget)->Result<Vec<Subpath>>{
    if !s.dash.is_empty(){return Err(Error::Unsupported("dashed stroke rasterization"));}
    let m=view.compose(s.ctm)?;let [a,b,c,d,_,_]=m.0;
    let (sx,sy)=(a.hypot(b),c.hypot(d));
    // First implementation supports uniform scaling/rotation/reflection. No false
    // approximation of anisotropic pen geometry.
    if sx==0.||sy==0.||(sx-sy).abs()>1e-8*sx.max(sy)||(a*c+b*d).abs()>1e-8*sx*sy{
        return Err(Error::Unsupported("nonuniform stroke transform"));
    }
    let r=if s.width==0.{0.5}else{s.width*sx*0.5};
    if !r.is_finite()||r<=0.||r>1e6{return Err(Error::Limit("stroke radius"));}
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
struct GroupLayer{
    samples:Vec<u8>,alpha:Option<Vec<f32>>,opacity:f64,blend:BlendMode,stack_depth:usize,
    coverage:Option<Vec<f32>>,knockout:bool,soft_mask:Option<Arc<Vec<u8>>>,
}
struct Surface{w:usize,h:usize,samples:Vec<u8>,clip:Arc<Vec<u8>>,stack:Vec<(Arc<Vec<u8>>,f64,f64,BlendMode,Option<Arc<Vec<u8>>>)>,budget:Budget,fill_alpha:f64,stroke_alpha:f64,stencil:[u8;3],blend_mode:BlendMode,alpha:Option<Vec<f32>>,groups:Vec<GroupLayer>,soft_mask:Option<Arc<Vec<u8>>>,mask_bytes:usize,mask_clip:Option<Arc<Vec<u8>>>,page_clip:Arc<Vec<u8>>}
impl Surface{
    fn mark_coverage(&mut self,index:usize,alpha:f64){
        if let Some(coverage)=self.groups.last_mut().and_then(|g|g.coverage.as_mut()){
            let a=f64::from(coverage[index]);
            coverage[index]=(alpha+a*(1.-alpha)) as f32;
        }
    }
    fn mask_factor(&self,i:usize)->f64{self.soft_mask.as_ref().map_or(1.,|m|f64::from(m[i])/255.)}
    fn blend(&mut self,i:usize,c:[u8;3],alpha:f64){
        let alpha=(alpha*self.mask_factor(i/3)).clamp(0.,1.);
        if alpha==0.{return;}
        self.mark_coverage(i/3,alpha);
        let ab=self.alpha.as_ref().map_or(1.,|a|f64::from(a[i/3]));
        let ao=alpha+ab*(1.-alpha);
        if ab==0.||(alpha==1.&&self.blend_mode==BlendMode::Normal){
            self.samples[i..i+3].copy_from_slice(&c);
        }else{
            let b:[f64;3]=std::array::from_fn(|k|f64::from(self.samples[i+k])/255.);
            let src=c.map(|v|f64::from(v)/255.);
            let f=self.blend_mode.rgb(b,src);
            // Straight color; blend only where the source overlaps backdrop alpha.
            for k in 0..3{
                let premul=(1.-alpha)*ab*b[k]+alpha*((1.-ab)*src[k]+ab*f[k]);
                self.samples[i+k]=(premul/ao*255.).round().clamp(0.,255.)as u8;
            }
        }
        if let Some(a)=&mut self.alpha{a[i/3]=ao as f32;}
    }
    fn begin_group(&mut self,opacity:f64,blend:BlendMode,isolated:bool,knockout:bool)->Result<()>{
        let pixels=self.w*self.h;
        let used=self.groups.iter().map(|g|if g.coverage.is_some(){44}else{28}).sum::<usize>();
        if self.groups.len()>=MAX_GROUP_DEPTH||(used+if isolated{28}else{44})*pixels>MAX_GROUP_BYTES{return Err(Error::Limit("transparency group buffers"));}
        self.budget.charge(pixels*if isolated{4}else{8})?;
        let colors=if isolated{vec![0;pixels*12]}else{self.samples.clone()};
        let alpha=if isolated{vec![0.;pixels*4]}else{self.alpha.clone().unwrap_or_else(||vec![1.;pixels*4])};
        let parent=GroupLayer{samples:std::mem::replace(&mut self.samples,colors),
            alpha:self.alpha.replace(alpha),opacity,blend,stack_depth:self.stack.len(),
            coverage:if isolated{None}else{Some(vec![0.;pixels*4])},knockout,soft_mask:self.soft_mask.take()};
        self.groups.push(parent);
        self.fill_alpha=1.;self.stroke_alpha=1.;self.blend_mode=BlendMode::Normal;
        Ok(())
    }
    fn end_group(&mut self)->Result<()>{
        let parent=self.groups.pop().ok_or(Error::Invalid("group stack underflow"))?;
        if self.stack.len()!=parent.stack_depth{return Err(Error::Invalid("group graphics stack"));}
        let colors=std::mem::replace(&mut self.samples,parent.samples);
        let alpha=std::mem::replace(&mut self.alpha,parent.alpha).ok_or(Error::Invalid("missing group alpha"))?;
        self.blend_mode=parent.blend;self.soft_mask=parent.soft_mask;
        self.budget.charge(self.w*self.h*if parent.coverage.is_some(){8}else{4})?;
        // Group coverage includes clip and BBox; no repeated clip on exit.
        if let Some(coverage)=parent.coverage{
            for (i,&ag) in coverage.iter().enumerate(){
                let ag=f64::from(ag);let scale=parent.opacity*self.mask_factor(i);
                if ag<=0.||scale<=0.{continue;}
                let ab=self.alpha.as_ref().map_or(1.,|a|f64::from(a[i]));
                let ar=f64::from(alpha[i]);let at=i*3;
                if parent.blend==BlendMode::Normal{
                    // Equivalent to backdrop removal + normal composite, without
                    // dividing by small coverage or rounding recovered RGB.
                    let ao=ab*(1.-scale)+ar*scale;
                    for k in 0..3{
                        let p=f64::from(self.samples[at+k])*ab*(1.-scale)+f64::from(colors[at+k])*ar*scale;
                        self.samples[at+k]=if ao>0.{(p/ao).round().clamp(0.,255.)as u8}else{0};
                    }
                    if let Some(a)=&mut self.alpha{a[i]=ao as f32;}
                    self.mark_coverage(i,ag*scale);
                }else{
                    // Recover the group contribution in premultiplied space.
                    // RGB8 intermediate precision can affect small coverage.
                    let c=std::array::from_fn(|k|((f64::from(colors[at+k])*ar-f64::from(self.samples[at+k])*ab*(1.-ag))/ag).round().clamp(0.,255.)as u8);
                    self.blend(at,c,ag*parent.opacity);
                }
            }
        }else{
            for (i,&a) in alpha.iter().enumerate(){
                if a>0.{self.blend(i*3,[colors[i*3],colors[i*3+1],colors[i*3+2]],f64::from(a)*parent.opacity);}
            }
        }
        Ok(())
    }
    fn begin_soft_mask(&mut self)->Result<()>{
        if self.mask_clip.is_some(){return Err(Error::Unsupported("nested soft mask"));}
        let n=self.w*self.h*4;
        if n>MAX_SOFT_MASK_BYTES-self.mask_bytes{return Err(Error::Limit("soft mask bytes"));}
        self.mask_bytes+=n;
        self.begin_group(1.,BlendMode::Normal,true,false)?;
        self.mask_clip=Some(std::mem::replace(&mut self.clip,Arc::clone(&self.page_clip)));
        Ok(())
    }
    fn end_soft_mask(&mut self,luminosity:bool,backdrop:[f64;3])->Result<()>{
        let parent=self.groups.pop().ok_or(Error::Invalid("soft mask group stack"))?;
        if self.stack.len()!=parent.stack_depth{return Err(Error::Invalid("soft mask graphics stack"));}
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
        // ExtGState emits the caller's constant opacity and blend immediately
        // after mask creation. Old soft mask is intentionally replaced.
        Ok(())
    }
    fn knockout_sample(&mut self,i:usize){
        let Some(g)=self.groups.last_mut().filter(|g|g.knockout) else{return;};
        let a=self.alpha.as_mut().expect("group alpha");
        if let Some(coverage)=g.coverage.as_mut(){
            coverage[i/3]=0.;
            self.samples[i..i+3].copy_from_slice(&g.samples[i..i+3]);
            a[i/3]=g.alpha.as_ref().map_or(1.,|a|a[i/3]);
        }else{
            self.samples[i..i+3].fill(0);a[i/3]=0.;
        }
    }
    fn paint(&mut self,mask:&Coverage,c:[u8;3],alpha:f64)->Result<()>{
        self.budget.charge(mask.w*mask.h*4)?;
        for row in 0..mask.h{
            let base=(mask.y+row)*self.w+mask.x;
            for col in 0..mask.w{
                let i=base+col;let m=mask.data[row*mask.w+col]&self.clip[i];
                for s in 0..4{if m&(1<<s)!=0{let at=i*12+s*3;self.knockout_sample(at);self.blend(at,c,alpha);}}
            }
        }Ok(())
    }
    fn image(&mut self,doc:&Document<'_>,id:(u32,u16),m:Matrix)->Result<()>{
        let img=images::load(doc,id,&mut self.budget)?;
        let inv=inverse(m)?;
        let corners=[m.point([0.,0.])?,m.point([1.,0.])?,m.point([0.,1.])?,m.point([1.,1.])?];
        let lo=|axis:usize|corners.iter().map(|p|p[axis]).fold(f64::INFINITY,f64::min);
        let hi=|axis:usize|corners.iter().map(|p|p[axis]).fold(f64::NEG_INFINITY,f64::max);
        let (x0,x1)=(span_bound(lo(0),self.w*2),span_bound(hi(0),self.w*2));
        let (y0,y1)=(span_bound(lo(1),self.h*2),span_bound(hi(1),self.h*2));
        self.budget.charge((x1-x0)*(y1-y0)*if img.interpolate{4}else{1})?;
        for y in y0..y1{for x in x0..x1{
            let i=y/2*self.w+x/2;let s=(y%2)*2+x%2;
            if self.clip[i]&(1<<s)==0{continue;}
            let [u,v]=inv.point([(x as f64+0.5)*0.5,(y as f64+0.5)*0.5])?;
            if !(0.0..1.0).contains(&u)||!(0.0..1.0).contains(&v){continue;}
            let (color,alpha)=img.sample(u,1.-v,self.stencil);
            let alpha=alpha*self.fill_alpha;
            self.blend(i*12+s*3,color,alpha);
        }}Ok(())
    }
}
fn render(doc:&Document<'_>,page:&PageInfo,list:&DisplayList,opt:Options,substitutes:Option<&StandardFontSet>)->Result<SubstitutedBitmap>{
    let (view,w,h,exact_w,exact_h)=viewport(page,opt.scale)?;
    let mut clip=vec![0u8;w*h];
    for y in 0..span_bound(exact_h,h*2){for x in 0..span_bound(exact_w,w*2){clip[y/2*w+x/2]|=1<<((y%2)*2+x%2);}}
    let mut s=Surface{w,h,samples:vec![255;w*h*12],clip:Arc::new(clip.clone()),stack:Vec::new(),budget:Budget::default(),fill_alpha:1.,stroke_alpha:1.,stencil:[0;3],blend_mode:BlendMode::Normal,alpha:None,groups:Vec::new(),soft_mask:None,mask_bytes:0,mask_clip:None,page_clip:Arc::new(clip)};
    let mut missing_text=false;
    let mut text=rendertext::TextRenderer::new();
    for c in &list.commands{
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
                let m=mask(&p,*rule,w,h,&mut s.budget)?;
                s.clip=Arc::new(m.intersect_clip(&s.clip,w,h,&mut s.budget)?);
            }
            Command::Paint{path,fill,stroke}=>{
                let p=flatten(path,view,&mut s.budget)?;
                if let Some((c,rule))=fill{let m=mask(&p,*rule,w,h,&mut s.budget)?;s.paint(&m,rgb(*c),s.fill_alpha)?;}
                if let Some(st)=stroke{let p=stroke_paths(&p,st,view,&mut s.budget)?;let m=mask(&p,FillRule::NonZero,w,h,&mut s.budget)?;s.paint(&m,rgb(st.color),s.stroke_alpha)?;}
            }
            Command::Image{object_id,ctm}=>s.image(doc,*object_id,view.compose(*ctm)?)?,
            Command::Text{operation,ctm,fill,stroke,scope}=>{
                // Explicit preview mode skips all text, as in B10.1. Strict mode never silently skips.
                if opt.allow_missing_text{missing_text=true;continue;}
                if scope.is_some(){return Err(Error::Unsupported("Form text rasterization"));}
                text.execute(doc,page,&mut s,operation,view,*ctm,*fill,stroke,substitutes)?;
            }
        }
    }
    if !s.stack.is_empty()||!s.groups.is_empty()||s.alpha.is_some(){return Err(Error::Invalid("unbalanced raster stack"));}
    let mut rgba=Vec::with_capacity(w*h*4);
    for samples in s.samples.chunks_exact(12){
        for c in 0..3{rgba.push(((u16::from(samples[c])+u16::from(samples[c+3])+u16::from(samples[c+6])+u16::from(samples[c+9])+2)/4) as u8);}
        rgba.push(255);
    }
    Ok(SubstitutedBitmap{bitmap:Bitmap{width:w,height:h,rgba,missing_text},substituted_fonts:text.substituted_fonts()})
}
impl Document<'_>{
    pub fn render_page(&self,page:&PageInfo,options:Options,content_limit:usize)->Result<Bitmap>{
        // Reject invalid viewport before decoding page content.
        viewport(page,options.scale)?;
        let list=self.page_display_list(page,content_limit)?;
        Ok(render(self,page,&list,options,None)?.bitmap)
    }
    /// Explicit substitute outlines; caller must provide appropriately licensed,
    /// compatible font programs. Pixel equivalence to original fonts is not promised.
    pub fn render_page_with_standard_fonts(&self,page:&PageInfo,options:Options,content_limit:usize,fonts:&StandardFontSet)->Result<SubstitutedBitmap>{
        viewport(page,options.scale)?;
        let list=self.page_display_list(page,content_limit)?;
        render(self,page,&list,options,Some(fonts))
    }
}
