//! Independent CPU rasterizer. 2x2 sample coverage, opaque white background.
use crate::{graphics::{BlendMode,Color,Command,DisplayList,FillRule,LineCap,LineJoin,Path,Segment,Stroke},text::{Matrix,Point},Document,Error,PageInfo,Result};
use std::sync::Arc;
mod png;
mod rendertext;
mod images;
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
fn mask(paths:&[Subpath],rule:FillRule,w:usize,h:usize,budget:&mut Budget)->Result<Vec<u8>>{
    budget.charge(w*h)?;
    let mut out=vec![0u8;w*h];let e=edges(paths)?;
    if e.is_empty(){return Ok(out);}
    let lo=e.iter().map(|(a,b)|a[1].min(b[1])).fold(f64::INFINITY,f64::min);
    let hi=e.iter().map(|(a,b)|a[1].max(b[1])).fold(f64::NEG_INFINITY,f64::max);
    let mut hits=Vec::<(f64,i32)>::new();
    for sy in span_bound(lo,h*2)..span_bound(hi,h*2){
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
                let a=span_bound(prev,w*2);let b=span_bound(x,w*2);
                budget.charge(b.saturating_sub(a))?;
                for sx in a..b{out[sy/2*w+sx/2]|=1<<((sy%2)*2+sx%2);}
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
// Each group uses 12 RGB bytes and 16 alpha bytes per output pixel.
// The cap counts all live offscreen buffers, excluding the existing root surface.
const MAX_GROUP_BYTES:usize=128*1024*1024;
const MAX_GROUP_DEPTH:usize=8;
struct GroupLayer{
    samples:Vec<u8>,alpha:Option<Vec<f32>>,opacity:f64,blend:BlendMode,stack_depth:usize,
}
struct Surface{w:usize,h:usize,samples:Vec<u8>,clip:Arc<Vec<u8>>,stack:Vec<(Arc<Vec<u8>>,f64,f64,BlendMode)>,budget:Budget,fill_alpha:f64,stroke_alpha:f64,stencil:[u8;3],blend_mode:BlendMode,alpha:Option<Vec<f32>>,groups:Vec<GroupLayer>}
impl Surface{
    fn blend(&mut self,i:usize,c:[u8;3],alpha:f64){
        let alpha=alpha.clamp(0.,1.);
        if alpha==0.{return;}
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
    fn begin_group(&mut self,opacity:f64,blend:BlendMode)->Result<()>{
        let pixels=self.w*self.h;
        if self.groups.len()>=MAX_GROUP_DEPTH||(self.groups.len()+1)*pixels*28>MAX_GROUP_BYTES{return Err(Error::Limit("transparency group buffers"));}
        self.budget.charge(pixels*4)?;
        let parent=GroupLayer{samples:std::mem::replace(&mut self.samples,vec![0;pixels*12]),
            alpha:self.alpha.replace(vec![0.;pixels*4]),opacity,blend,stack_depth:self.stack.len()};
        self.groups.push(parent);
        self.fill_alpha=1.;self.stroke_alpha=1.;self.blend_mode=BlendMode::Normal;
        Ok(())
    }
    fn end_group(&mut self)->Result<()>{
        let parent=self.groups.pop().ok_or(Error::Invalid("group stack underflow"))?;
        if self.stack.len()!=parent.stack_depth{return Err(Error::Invalid("group graphics stack"));}
        let colors=std::mem::replace(&mut self.samples,parent.samples);
        let alpha=std::mem::replace(&mut self.alpha,parent.alpha).ok_or(Error::Invalid("missing group alpha"))?;
        self.blend_mode=parent.blend;
        self.budget.charge(self.w*self.h*4)?;
        // Group coverage already contains the inherited clip and Form BBox;
        // do not apply the final child's clip again at group exit.
        for (i,&a) in alpha.iter().enumerate(){
            if a>0.{self.blend(i*3,[colors[i*3],colors[i*3+1],colors[i*3+2]],f64::from(a)*parent.opacity);}
        }
        // The surrounding Restore command restores the caller's complete state.
        Ok(())
    }
    fn paint(&mut self,mask:&[u8],c:[u8;3],alpha:f64)->Result<()>{
        self.budget.charge(self.w*self.h*4)?;
        for (i,&m) in mask.iter().enumerate(){
            let m=m&self.clip[i];for s in 0..4{if m&(1<<s)!=0{self.blend(i*12+s*3,c,alpha);}}
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
fn render(doc:&Document<'_>,page:&PageInfo,list:&DisplayList,opt:Options)->Result<Bitmap>{
    let (view,w,h,exact_w,exact_h)=viewport(page,opt.scale)?;
    let mut clip=vec![0u8;w*h];
    for y in 0..span_bound(exact_h,h*2){for x in 0..span_bound(exact_w,w*2){clip[y/2*w+x/2]|=1<<((y%2)*2+x%2);}}
    let mut s=Surface{w,h,samples:vec![255;w*h*12],clip:Arc::new(clip),stack:Vec::new(),budget:Budget::default(),fill_alpha:1.,stroke_alpha:1.,stencil:[0;3],blend_mode:BlendMode::Normal,alpha:None,groups:Vec::new()};
    let mut missing_text=false;
    let mut text=rendertext::TextRenderer::new();
    for c in &list.commands{
        s.budget.charge(1)?;
        match c{
            Command::Save=>{
                if s.stack.len()>=64||(s.stack.len()+2)*w*h>MAX_CLIP_BYTES{return Err(Error::Limit("raster clip stack"));}
                s.stack.push((Arc::clone(&s.clip),s.fill_alpha,s.stroke_alpha,s.blend_mode));text.save();
            }
            Command::Restore=>{let (clip,fa,sa,bm)=s.stack.pop().ok_or(Error::Invalid("raster restore"))?;s.clip=clip;s.fill_alpha=fa;s.stroke_alpha=sa;s.blend_mode=bm;text.restore()?;}
            Command::Opacity{fill,stroke}=>{s.fill_alpha=*fill;s.stroke_alpha=*stroke;}
            Command::Blend(b)=>s.blend_mode=*b,
            Command::BeginGroup{opacity,blend}=>s.begin_group(*opacity,*blend)?,
            Command::EndGroup=>s.end_group()?,
            Command::StencilColor(c)=>s.stencil=rgb(*c),
            Command::Clip{path,rule}=>{
                let p=flatten(path,view,&mut s.budget)?;
                let mut m=mask(&p,*rule,w,h,&mut s.budget)?;
                for (v,c) in m.iter_mut().zip(s.clip.iter()){*v&=*c;}
                s.clip=Arc::new(m);
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
                text.execute(doc,page,&mut s,operation,view,*ctm,*fill,stroke)?;
            }
        }
    }
    if !s.stack.is_empty()||!s.groups.is_empty()||s.alpha.is_some(){return Err(Error::Invalid("unbalanced raster stack"));}
    let mut rgba=Vec::with_capacity(w*h*4);
    for samples in s.samples.chunks_exact(12){
        for c in 0..3{rgba.push(((u16::from(samples[c])+u16::from(samples[c+3])+u16::from(samples[c+6])+u16::from(samples[c+9])+2)/4) as u8);}
        rgba.push(255);
    }
    Ok(Bitmap{width:w,height:h,rgba,missing_text})
}
impl Document<'_>{
    pub fn render_page(&self,page:&PageInfo,options:Options,content_limit:usize)->Result<Bitmap>{
        // Reject invalid viewport before decoding page content.
        viewport(page,options.scale)?;
        let list=self.page_display_list(page,content_limit)?;
        render(self,page,&list,options)
    }
}
