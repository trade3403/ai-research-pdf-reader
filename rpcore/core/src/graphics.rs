//! Bounded vector display-list interpreter, not a rasterizer.
//! Paths use unrotated PDF user coordinates; strokes retain the painting CTM.
use crate::{content::{self,Operation},text::{Matrix,Point},Document,Error,Object,PageInfo,Result};
use std::{collections::{BTreeSet,BTreeMap},sync::Arc};
const MAX_OPS:usize=100_000;
const MAX_SEGMENTS:usize=200_000;
const MAX_COMMANDS:usize=100_000;
const MAX_STACK:usize=64;
const MAX_FORMS:usize=32;
const MAX_DASH:usize=256;
const MAX_COORD:f64=1e12;

#[derive(Debug,Clone,PartialEq)]
pub enum Segment{Move(Point),Line(Point),Cubic(Point,Point,Point),Close}
pub type Path=Vec<Segment>;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FillRule{NonZero,EvenOdd}
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Color{Gray(f64),Rgb([f64;3]),Cmyk([f64;4])}
#[derive(Debug,Clone)]
enum Space{Gray,Rgb,Cmyk,Calibrated(crate::color::Calibrated),Icc(Arc<crate::color::IccTransform>),IccAlternate(usize)}
impl Space{
    fn initial(&self)->Result<Color>{Ok(match self{
        Self::Gray=>Color::Gray(0.),Self::Rgb=>Color::Rgb([0.;3]),Self::Cmyk=>Color::Cmyk([0.,0.,0.,1.]),
        Self::Calibrated(c)=>Color::Rgb(c.rgb([0.;3])),
        Self::Icc(t)=>Color::Rgb(t.rgb([0.;4])?),
        Self::IccAlternate(n)=>match n{1=>Color::Gray(0.),3=>Color::Rgb([0.;3]),_=>Color::Cmyk([0.;4])},
    })}
    fn color(&self,a:&[Object])->Result<Color>{Ok(match self{
        Self::Gray=>Color::Gray(nums::<1>(a)?[0].clamp(0.,1.)),
        Self::Rgb=>Color::Rgb(nums::<3>(a)?.map(|x|x.clamp(0.,1.))),
        Self::Cmyk=>Color::Cmyk(nums::<4>(a)?.map(|x|x.clamp(0.,1.))),
        Self::Calibrated(c)=>Color::Rgb(c.rgb(if c.components()==1{[nums::<1>(a)?[0],0.,0.]}else{nums::<3>(a)?})),
        Self::Icc(t)=>{
            let v=match t.components(){1=>[nums::<1>(a)?[0],0.,0.,0.],3=>{let [r,g,b]=nums::<3>(a)?;[r,g,b,0.]},_=>nums::<4>(a)?};
            Color::Rgb(t.rgb(v)?)
        }
        Self::IccAlternate(n)=>match n{1=>Self::Gray.color(a)?,3=>Self::Rgb.color(a)?,_=>Self::Cmyk.color(a)?},
    })}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LineCap{Butt,Round,Square}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LineJoin{Miter,Round,Bevel}
#[derive(Debug,Clone,PartialEq)]
pub struct Stroke{
    pub color:Color,pub width:f64,pub cap:LineCap,pub join:LineJoin,
    pub miter_limit:f64,pub dash:Vec<f64>,pub dash_phase:f64,pub ctm:Matrix,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum BlendMode{Normal,Multiply,Screen,Overlay,Darken,Lighten,Difference,Exclusion,HardLight,ColorDodge,ColorBurn,SoftLight,Hue,Saturation,Color,Luminosity}
impl BlendMode{
    fn parse(n:&[u8])->Option<Self>{Some(match n{
        b"Normal"|b"Compatible"=>Self::Normal,b"Multiply"=>Self::Multiply,b"Screen"=>Self::Screen,b"Overlay"=>Self::Overlay,
        b"Darken"=>Self::Darken,b"Lighten"=>Self::Lighten,b"Difference"=>Self::Difference,b"Exclusion"=>Self::Exclusion,
        b"HardLight"=>Self::HardLight,b"ColorDodge"=>Self::ColorDodge,b"ColorBurn"=>Self::ColorBurn,b"SoftLight"=>Self::SoftLight,b"Hue"=>Self::Hue,b"Saturation"=>Self::Saturation,b"Color"=>Self::Color,b"Luminosity"=>Self::Luminosity,_=>return None,
    })}
    /// Scalar formula for separable modes; use rgb() for nonseparable modes.
    pub fn channel(self,b:f64,s:f64)->f64{match self{
        Self::Normal=>s,Self::Multiply=>b*s,Self::Screen=>b+s-b*s,
        Self::Overlay=>if b<=0.5{2.*b*s}else{1.-2.*(1.-b)*(1.-s)},
        Self::HardLight=>if s<=0.5{2.*b*s}else{1.-2.*(1.-b)*(1.-s)},
        Self::Darken=>b.min(s),Self::Lighten=>b.max(s),Self::Difference=>(b-s).abs(),Self::Exclusion=>b+s-2.*b*s,
        Self::ColorDodge=>if b==0.{0.}else if s==1.{1.}else{(b/(1.-s)).min(1.)},
        Self::ColorBurn=>if b==1.{1.}else if s==0.{0.}else{1.-((1.-b)/s).min(1.)},
        Self::SoftLight=>if s<=0.5{b-(1.-2.*s)*b*(1.-b)}else{let d=if b<=0.25{((16.*b-12.)*b+4.)*b}else{b.sqrt()};b+(2.*s-1.)*(d-b)},
        Self::Hue|Self::Saturation|Self::Color|Self::Luminosity=>self.rgb([b;3],[s;3])[0],
    }}
    /// PDF separable and nonseparable blend functions in normalized device RGB.
    pub fn rgb(self,b:[f64;3],s:[f64;3])->[f64;3]{
        fn lum(c:[f64;3])->f64{0.3*c[0]+0.59*c[1]+0.11*c[2]}
        fn sat(c:[f64;3])->f64{c.iter().copied().fold(f64::NEG_INFINITY,f64::max)-c.iter().copied().fold(f64::INFINITY,f64::min)}
        fn set_lum(c:[f64;3],l:f64)->[f64;3]{
            let d=l-lum(c);let mut v=c.map(|x|x+d);let l=lum(v);
            let n=v.iter().copied().fold(f64::INFINITY,f64::min);let x=v.iter().copied().fold(f64::NEG_INFINITY,f64::max);
            if n<0.{v=v.map(|a|l+(a-l)*l/(l-n));}
            if x>1.{let x=v.iter().copied().fold(f64::NEG_INFINITY,f64::max);if x>1.{v=v.map(|a|l+(a-l)*(1.-l)/(x-l));}}
            v.map(|a|a.clamp(0.,1.))
        }
        fn set_sat(c:[f64;3],s:f64)->[f64;3]{
            let mut order=[0,1,2];order.sort_by(|&a,&b|c[a].total_cmp(&c[b]));
            let [lo,mid,hi]=order;let mut v=[0.;3];
            if c[hi]>c[lo]{v[mid]=(c[mid]-c[lo])*s/(c[hi]-c[lo]);v[hi]=s;}v
        }
        match self{
            Self::Hue=>set_lum(set_sat(s,sat(b)),lum(b)),
            Self::Saturation=>set_lum(set_sat(b,sat(s)),lum(b)),
            Self::Color=>set_lum(s,lum(b)),
            Self::Luminosity=>set_lum(b,lum(s)),
            _=>std::array::from_fn(|i|self.channel(b[i],s[i])),
        }
    }
}
#[derive(Debug,Clone,PartialEq)]
pub enum Command{
    Save,Restore,
    /// Normal source-over constant opacity, retained in drawing order.
    Opacity{fill:f64,stroke:f64},
    Blend(BlendMode),
    /// Begin an isolated, non-knockout RGB transparency group.
    BeginGroup{opacity:f64,blend:BlendMode},
    /// Non-isolated RGB group with inherited backdrop.
    BeginNonIsolatedGroup{opacity:f64,blend:BlendMode},
    BeginKnockoutGroup{opacity:f64,blend:BlendMode,isolated:bool},
    EndGroup,
    BeginSoftMask,
    EndSoftMask{luminosity:bool,backdrop:[f64;3]},
    ClearSoftMask,
    /// Current nonstroking color, needed by stencil images.
    StencilColor(Color),
    Paint{path:Path,fill:Option<(Color,FillRule)>,stroke:Option<Stroke>},
    /// Intersect current clipping region; path is applied after any painting.
    Clip{path:Path,rule:FillRule},
    /// Image reference and placement only. Image samples are not decoded here.
    Image{object_id:(u32,u16),ctm:Matrix,intent:crate::icc::RenderingIntent},
    /// Text operators are retained in order, but glyph rendering is deferred.
    Text{operation:Operation,ctm:Matrix,fill:Color,stroke:Stroke,scope:Option<(u32,u16)>,resource_owner:Option<(u32,u16)>},
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum Warning{TextRenderingDeferred,ImageRenderingDeferred,MarkedContentNotInterpreted,IccAlternateUsed,DeviceQualityHintNotInterpreted,IccIntentApproximation}
#[derive(Debug,Clone,PartialEq)]
pub struct DisplayList{
    /// Initial clip is the page CropBox; Rotate is not applied.
    pub crop_box:[f64;4],
    pub commands:Vec<Command>,
    pub warnings:Vec<Warning>,
    /// Unique ICC source decisions, including the alternate reason.
    pub icc_profiles:Vec<crate::icc::ProfileDiagnostic>,
}
#[derive(Clone)]
struct State{
    intent:crate::icc::RenderingIntent,fill_icc:Option<Vec<Object>>,stroke_icc:Option<Vec<Object>>,fill_values:Vec<Object>,stroke_values:Vec<Object>,
    ctm:Matrix,fill:Color,fill_space:Space,stroke:Stroke,stroke_space:Space,fill_alpha:f64,stroke_alpha:f64,blend:BlendMode,knockout:bool,in_mask:bool,
}
impl Default for State{
    fn default()->Self{
        Self{intent:crate::icc::RenderingIntent::RelativeColorimetric,fill_icc:None,stroke_icc:None,fill_values:Vec::new(),stroke_values:Vec::new(),ctm:Matrix::IDENTITY,fill:Color::Gray(0.),fill_space:Space::Gray,
            stroke:Stroke{color:Color::Gray(0.),width:1.,cap:LineCap::Butt,join:LineJoin::Miter,miter_limit:10.,dash:Vec::new(),dash_phase:0.,ctm:Matrix::IDENTITY},stroke_space:Space::Gray,fill_alpha:1.,stroke_alpha:1.,blend:BlendMode::Normal,knockout:false,in_mask:false}
    }
}
impl State{
    fn stroke(&self)->Stroke{let mut s=self.stroke.clone();s.ctm=self.ctm;s}
}
#[derive(Default)]
struct Current{
    segments:Path,point:Option<Point>,start:Option<Point>,clip:Option<FillRule>,
}
fn arity(a:&[Object],n:usize)->Result<()>{
    if a.len()==n{Ok(())}else{Err(Error::Invalid("graphics operand count"))}
}
fn number(o:&Object)->Result<f64>{
    if let Object::Number(n)=o{if n.is_finite()&&n.abs()<=MAX_COORD{return Ok(*n);}}
    Err(Error::Invalid("graphics number"))
}
fn nums<const N:usize>(a:&[Object])->Result<[f64;N]>{
    arity(a,N)?;let mut v=[0.;N];for(i,o)in a.iter().enumerate(){v[i]=number(o)?;}Ok(v)
}
fn name(o:&Object)->Result<&[u8]>{
    if let Object::Name(n)=o{Ok(n)}else{Err(Error::Invalid("graphics name"))}
}
fn device(n:&[u8])->Result<Space>{match n{
    b"DeviceGray"=>Ok(Space::Gray),b"DeviceRGB"=>Ok(Space::Rgb),b"DeviceCMYK"=>Ok(Space::Cmyk),
    _=>Err(Error::Unsupported("graphics color space")),
}}
struct Engine<'d,'a>{
    doc:&'d Document<'a>,out:DisplayList,
    active:BTreeSet<(u32,u16)>,ops:usize,segments:usize,bytes_left:usize,
    icc_cache:BTreeMap<((u32,u16),crate::icc::RenderingIntent),Space>,icc_bytes:usize,degree:crate::icc::AdaptationDegree,
}
impl<'d,'a> Engine<'d,'a>{
    fn command(&mut self,c:Command)->Result<()>{
        if self.out.commands.len()>=MAX_COMMANDS{return Err(Error::Limit("display list commands"));}
        self.out.commands.push(c);Ok(())
    }
    fn warning(&mut self,w:Warning){if !self.out.warnings.contains(&w){self.out.warnings.push(w);}}
    fn charge_segments(&mut self,n:usize)->Result<()>{
        if n>MAX_SEGMENTS.saturating_sub(self.segments){return Err(Error::Limit("display list path segments"));}
        self.segments+=n;Ok(())
    }
    fn push(&mut self,p:&mut Current,s:Segment)->Result<()>{
        self.charge_segments(1)?;p.segments.push(s);Ok(())
    }
    fn moveto(&mut self,p:&mut Current,at:Point)->Result<()>{
        self.push(p,Segment::Move(at))?;p.start=Some(at);p.point=Some(at);Ok(())
    }
    fn lineto(&mut self,p:&mut Current,at:Point)->Result<()>{
        if p.point.is_none(){return Err(Error::Invalid("path has no current point"));}
        self.push(p,Segment::Line(at))?;p.point=Some(at);Ok(())
    }
    fn cubic(&mut self,p:&mut Current,a:Point,b:Point,c:Point)->Result<()>{
        if p.point.is_none(){return Err(Error::Invalid("curve has no current point"));}
        self.push(p,Segment::Cubic(a,b,c))?;p.point=Some(c);Ok(())
    }
    fn close(&mut self,p:&mut Current)->Result<()>{
        let start=p.start.ok_or(Error::Invalid("close without subpath"))?;
        if !matches!(p.segments.last(),Some(Segment::Close)){self.push(p,Segment::Close)?;}
        p.point=Some(start);Ok(())
    }
    fn rect(&mut self,p:&mut Current,m:Matrix,r:[f64;4])->Result<()>{
        let [x,y,w,h]=r;
        self.moveto(p,m.point([x,y])?)?;
        for point in [[x+w,y],[x+w,y+h],[x,y+h]]{self.lineto(p,m.point(point)?)?;}
        self.close(p)
    }
    fn finish(&mut self,p:&mut Current,s:&State,fill:Option<FillRule>,stroke:bool,close:bool)->Result<()>{
        if close&&!p.segments.is_empty(){self.close(p)?;}
        let path=std::mem::take(&mut p.segments);
        if (fill.is_some()||stroke)&&!path.is_empty(){
            if p.clip.is_some(){self.charge_segments(path.len())?;}
            self.command(Command::Paint{path:path.clone(),
                fill:fill.map(|rule|(s.fill,rule)),stroke:stroke.then(||s.stroke())})?;
        }
        if let Some(rule)=p.clip.take(){self.command(Command::Clip{path,rule})?;}
        *p=Current::default();Ok(())
    }
    fn xobject(&mut self,resources:Option<&Object>,n:&[u8],s:&State,depth:usize,resource_owner:Option<(u32,u16)>)->Result<()>{
        if depth>=MAX_FORMS{return Err(Error::Limit("form depth"));}
        let r=crate::dict(self.doc.dereference(resources.ok_or(Error::Invalid("XObject Resources"))?)?)?;
        let x=crate::dict(self.doc.dereference(r.get(b"XObject".as_slice()).ok_or(Error::Invalid("XObject dictionary"))?)?)?;
        let id=crate::reference(x.get(n))?;
        if !self.active.insert(id){return Err(Error::Invalid("Form XObject cycle"));}
        let result=self.xobject_inner(id,resources,s,depth,resource_owner);
        self.active.remove(&id);result
    }
    fn xobject_inner(&mut self,id:(u32,u16),resources:Option<&Object>,s:&State,depth:usize,resource_owner:Option<(u32,u16)>)->Result<()>{
        let stream=self.doc.resolve(id.0,id.1)?;
        let Object::Stream{dict,..}=&stream else{return Err(Error::Invalid("XObject stream"));};
        match name(dict.get(b"Subtype".as_slice()).ok_or(Error::Invalid("XObject subtype"))?)?{
            b"Image"=>{
                self.warning(Warning::ImageRenderingDeferred);
                if matches!(dict.get(b"ImageMask".as_slice()),Some(Object::Bool(true))){self.command(Command::StencilColor(s.fill))?;}
                self.command(Command::Image{object_id:id,ctm:s.ctm,intent:s.intent})
            }
            b"Form"=>{
                let group_isolated=if let Some(group)=dict.get(b"Group".as_slice()){
                    let group=crate::dict(self.doc.dereference(group)?)?;
                    if !matches!(group.get(b"S".as_slice()).map(|v|self.doc.dereference(v)).transpose()?,Some(Object::Name(n)) if n==b"Transparency"){return Err(Error::Unsupported("Form group subtype"));}
                    let mut isolated=false;let mut knockout=false;
                    for (k,v) in &group{
                        let v=self.doc.dereference(v)?;
                        match k.as_slice(){
                            b"Type" if matches!(&v,Object::Name(n) if n==b"Group")=>{},
                            b"S" if matches!(&v,Object::Name(n) if n==b"Transparency")=>{},
                            b"I"=>if let Object::Bool(v)=v{isolated=v;}else{return Err(Error::Invalid("group I"));},
                            b"K"=>if let Object::Bool(v)=v{knockout=v;}else{return Err(Error::Invalid("group K"));},
                            b"CS" if matches!(&v,Object::Name(n) if n==b"DeviceRGB")=>{},
                            b"CS"=>return Err(Error::Unsupported("non-RGB transparency group color space")),
                            _=>return Err(Error::Unsupported("transparency group parameter")),
                        }
                    }
                    Some((isolated,knockout))
                }else{None};
                if let Some(v)=dict.get(b"FormType".as_slice()){
                    if number(v)?!=1.{return Err(Error::Unsupported("FormType"));}
                }
                let mut child=s.clone();
                if let Some(v)=dict.get(b"Matrix".as_slice()){
                    let Object::Array(a)=self.doc.dereference(v)? else{return Err(Error::Invalid("Form Matrix"));};
                    child.ctm=child.ctm.compose(Matrix(nums(&a)?))?;
                }
                let bbox=self.doc.form_rectangle(dict.get(b"BBox".as_slice()).ok_or(Error::Invalid("Form BBox"))?)?;
                let mut p=Current::default();
                if bbox[2]>bbox[0]&&bbox[3]>bbox[1]{self.rect(&mut p,child.ctm,[bbox[0],bbox[1],bbox[2]-bbox[0],bbox[3]-bbox[1]])?;}
                let data=crate::filters::decode(&stream,self.bytes_left)?;
                self.bytes_left-=data.len();
                let ops=content::parse(&data)?;
                self.command(Command::Save)?;
                // Clip the Form BBox before allocating its local transparency layer.
                self.command(Command::Clip{path:p.segments,rule:FillRule::NonZero})?;
                if let Some((isolated,knockout))=group_isolated{
                    child.knockout=knockout;
                    self.command(if knockout{Command::BeginKnockoutGroup{opacity:s.fill_alpha,blend:s.blend,isolated}}else if isolated{Command::BeginGroup{opacity:s.fill_alpha,blend:s.blend}}else{Command::BeginNonIsolatedGroup{opacity:s.fill_alpha,blend:s.blend}})?;
                    child.fill_alpha=1.;child.stroke_alpha=1.;child.blend=BlendMode::Normal;
                }
                // Absent Resources may use the page's resource environment.
                let owner=if dict.contains_key(b"Resources".as_slice()){Some(id)}else{resource_owner};
                self.run(&ops,dict.get(b"Resources".as_slice()).or(resources),child,depth+1,Some(id),owner)?;
                if group_isolated.is_some(){self.command(Command::EndGroup)?;}
                self.command(Command::Restore)
            }
            _=>Err(Error::Unsupported("XObject subtype")),
        }
    }
    fn icc_space(&mut self,a:&[Object],intent:crate::icc::RenderingIntent)->Result<Space>{
                    let id=match a.get(1){Some(Object::Ref(id,gen)) if a.len()==2=>(*id,*gen),_=>return Err(Error::Invalid("ICCBased profile reference"))};
                    if let Some(s)=self.icc_cache.get(&(id,intent)){return Ok(s.clone());}
                    if self.icc_cache.len()>=crate::color::ICC_PAGE_PROFILES{return Err(Error::Limit("ICC page profiles"));}
                    let (n,t,used,path)=crate::color::load_icc(self.doc,&a,crate::color::ICC_PAGE_BYTES-self.icc_bytes,intent,self.degree)?;
                    self.icc_bytes+=used;
                    self.out.icc_profiles.push(crate::icc::ProfileDiagnostic{object_id:id,source:crate::icc::ProfileUse::Vector,components:n,path});
                    let space=if let Some(t)=t{
                        // PDF ri / absolute and saturation paths are not implemented.
                        if !matches!(path,crate::icc::ProfilePath::Mpe{..}){self.warning(Warning::IccIntentApproximation);}
                        Space::Icc(Arc::new(t))
                    }else{
                        self.warning(Warning::IccAlternateUsed);
                        Space::IccAlternate(n)
                    };
                    self.icc_cache.insert((id,intent),space.clone());
                    Ok(space)
    }
    fn set_intent(&mut self,s:&mut State,intent:crate::icc::RenderingIntent)->Result<()>{
        if s.intent==intent{return Ok(());}
        if let Some(a)=&s.fill_icc{let space=self.icc_space(a,intent)?;s.fill=if s.fill_values.is_empty(){space.initial()?}else{space.color(&s.fill_values)?};s.fill_space=space;}
        if let Some(a)=&s.stroke_icc{let space=self.icc_space(a,intent)?;s.stroke.color=if s.stroke_values.is_empty(){space.initial()?}else{space.color(&s.stroke_values)?};s.stroke_space=space;}
        s.intent=intent;Ok(())
    }
    fn color_space(&mut self,resources:Option<&Object>,n:&[u8],intent:crate::icc::RenderingIntent)->Result<(Space,Option<Vec<Object>>)> {
        if let Ok(s)=device(n){return Ok((s,None));}
        let r=crate::dict(self.doc.dereference(resources.ok_or(Error::Unsupported("graphics color space"))?)?)?;
        let spaces=crate::dict(self.doc.dereference(r.get(b"ColorSpace".as_slice()).ok_or(Error::Unsupported("graphics color space"))?)?)?;
        let o=self.doc.dereference(spaces.get(n).ok_or(Error::Unsupported("graphics color space"))?)?;
        match o{
            Object::Name(n)=>device(&n).map(|s|(s,None)),
            Object::Array(a)=>match a.first(){
                Some(Object::Name(n)) if n==b"ICCBased"=>{
                    let space=self.icc_space(&a,intent)?;
                    Ok((space,Some(a)))
                }
                Some(Object::Name(n)) if n==b"Pattern"=>Err(Error::Unsupported("graphics Pattern color space")),
                Some(Object::Name(n)) if matches!(n.as_slice(),b"CalRGB"|b"CalGray"|b"Lab")=>{
                    Ok((Space::Calibrated(crate::color::Calibrated::parse(self.doc,&a)?),None))
                }
                Some(Object::Name(n)) if n==b"Separation"||n==b"DeviceN"=>Err(Error::Unsupported("graphics spot color space")),
                Some(Object::Name(n)) if n==b"Indexed"=>Err(Error::Unsupported("graphics Indexed color space")),
                _=>Err(Error::Unsupported("graphics complex color space")),
            },
            _=>Err(Error::Unsupported("graphics complex color space")),
        }
    }
    fn soft_mask(&mut self,resources:Option<&Object>,v:Object,s:&State,depth:usize,resource_owner:Option<(u32,u16)>)->Result<()>{
        if s.in_mask||s.knockout{return Err(Error::Unsupported("nested or knockout graphics soft mask"));}
        if depth>=MAX_FORMS{return Err(Error::Limit("soft mask depth"));}
        let d=crate::dict(v)?;
        let mut luminosity=false;let mut backdrop=[0.;3];
        for (k,v) in &d{
            let v=self.doc.dereference(v)?;
            match k.as_slice(){
                b"Type" if matches!(&v,Object::Name(n) if n==b"Mask")=>{},
                b"S"=>match v{Object::Name(n) if n==b"Alpha"=>{},Object::Name(n) if n==b"Luminosity"=>luminosity=true,_=>return Err(Error::Unsupported("soft mask subtype"))},
                b"G"=>{},
                b"TR" if matches!(&v,Object::Name(n) if n==b"Identity")=>{},
                b"BC"=>{let Object::Array(a)=v else{return Err(Error::Invalid("soft mask BC"));};backdrop=nums(&a)?;if backdrop.iter().any(|v|!(0.0..=1.0).contains(v)){return Err(Error::Invalid("soft mask BC range"));}}
                _=>return Err(Error::Unsupported("soft mask parameter or transfer function")),
            }
        }
        if !d.contains_key(b"S".as_slice()){return Err(Error::Invalid("soft mask subtype"));}
        let id=crate::reference(d.get(b"G".as_slice()))?;
        if !self.active.insert(id){return Err(Error::Invalid("soft mask cycle"));}
        let result=(||{
            let stream=self.doc.resolve(id.0,id.1)?;
            let Object::Stream{dict,..}=&stream else{return Err(Error::Invalid("soft mask Form"));};
            if !matches!(dict.get(b"Subtype".as_slice()),Some(Object::Name(n)) if n==b"Form"){return Err(Error::Invalid("soft mask Form subtype"));}
            let group=crate::dict(self.doc.dereference(dict.get(b"Group".as_slice()).ok_or(Error::Invalid("soft mask group"))?)?)?;
            for (k,v) in &group{
                let v=self.doc.dereference(v)?;
                match k.as_slice(){
                    b"Type" if matches!(&v,Object::Name(n) if n==b"Group")=>{},
                    b"S" if matches!(&v,Object::Name(n) if n==b"Transparency")=>{},
                    b"I" if v==Object::Bool(true)=>{},
                    b"K" if v==Object::Bool(false)=>{},
                    b"CS" if matches!(&v,Object::Name(n) if n==b"DeviceRGB")=>{},
                    _=>return Err(Error::Unsupported("soft mask requires isolated RGB non-knockout group")),
                }
            }
            if !matches!(group.get(b"I".as_slice()).map(|v|self.doc.dereference(v)).transpose()?,Some(Object::Bool(true)))||
                !matches!(group.get(b"S".as_slice()).map(|v|self.doc.dereference(v)).transpose()?,Some(Object::Name(n)) if n==b"Transparency")||
                !matches!(group.get(b"CS".as_slice()).map(|v|self.doc.dereference(v)).transpose()?,Some(Object::Name(n)) if n==b"DeviceRGB"){
                return Err(Error::Unsupported("soft mask requires isolated RGB non-knockout group"));
            }
            if let Some(v)=dict.get(b"FormType".as_slice()){if number(v)?!=1.{return Err(Error::Unsupported("soft mask FormType"));}}
            let mut child=s.clone();child.in_mask=true;child.fill_alpha=1.;child.stroke_alpha=1.;child.blend=BlendMode::Normal;
            if let Some(v)=dict.get(b"Matrix".as_slice()){
                let Object::Array(a)=self.doc.dereference(v)? else{return Err(Error::Invalid("soft mask Matrix"));};
                child.ctm=child.ctm.compose(Matrix(nums(&a)?))?;
            }
            let bbox=self.doc.form_rectangle(dict.get(b"BBox".as_slice()).ok_or(Error::Invalid("soft mask BBox"))?)?;
            let mut p=Current::default();if bbox[2]>bbox[0]&&bbox[3]>bbox[1]{self.rect(&mut p,child.ctm,[bbox[0],bbox[1],bbox[2]-bbox[0],bbox[3]-bbox[1]])?;}
            let data=crate::filters::decode(&stream,self.bytes_left)?;self.bytes_left-=data.len();let ops=content::parse(&data)?;
            self.command(Command::BeginSoftMask)?;
            self.command(Command::Clip{path:p.segments,rule:FillRule::NonZero})?;
            let owner=if dict.contains_key(b"Resources".as_slice()){Some(id)}else{resource_owner};
                self.run(&ops,dict.get(b"Resources".as_slice()).or(resources),child,depth+1,Some(id),owner)?;
            self.command(Command::EndSoftMask{luminosity,backdrop})
        })();
        self.active.remove(&id);result
    }
    fn ext_gstate(&mut self,resources:Option<&Object>,n:&[u8],s:&mut State,depth:usize,resource_owner:Option<(u32,u16)>)->Result<()>{
        let r=crate::dict(self.doc.dereference(resources.ok_or(Error::Invalid("ExtGState Resources"))?)?)?;
        let states=crate::dict(self.doc.dereference(r.get(b"ExtGState".as_slice()).ok_or(Error::Invalid("ExtGState dictionary"))?)?)?;
        let d=crate::dict(self.doc.dereference(states.get(n).ok_or(Error::Invalid("ExtGState name"))?)?)?;
        for (k,v) in &d{
            let v=self.doc.dereference(v)?;
            match k.as_slice(){
                b"Type" if matches!(&v,Object::Name(n) if n==b"ExtGState")=>{},
                b"ca"|b"CA"=>{let a=number(&v)?;if !(0.0..=1.0).contains(&a){return Err(Error::Invalid("ExtGState alpha"));}if k==b"ca"{s.fill_alpha=a;}else{s.stroke_alpha=a;}}
                b"BM"=>{
                    s.blend=match &v{
                        Object::Name(n)=>BlendMode::parse(n),
                        Object::Array(a)=>a.iter().find_map(|v|if let Object::Name(n)=v{BlendMode::parse(n)}else{None}),
                        _=>None,
                    }.ok_or(Error::Unsupported("blend mode"))?;
                },
                b"SMask" if matches!(&v,Object::Name(n) if n==b"None")=>{self.command(Command::ClearSoftMask)?;}
                b"SMask" if matches!(&v,Object::Dict(_))=>{if !crate::dict(v.clone())?.contains_key(b"G".as_slice()){return Err(Error::Unsupported("ExtGState graphics soft mask"));}self.soft_mask(resources,v,s,depth,resource_owner)?;}
                b"AIS" if v==Object::Bool(false)=>{},
                b"TK" if v==Object::Bool(true)=>{},
                b"LW"=>{let x=number(&v)?;if x<0.{return Err(Error::Invalid("ExtGState line width"));}s.stroke.width=x;}
                b"LC"=>s.stroke.cap=match number(&v)?{0.=>LineCap::Butt,1.=>LineCap::Round,2.=>LineCap::Square,_=>return Err(Error::Invalid("ExtGState line cap"))},
                b"LJ"=>s.stroke.join=match number(&v)?{0.=>LineJoin::Miter,1.=>LineJoin::Round,2.=>LineJoin::Bevel,_=>return Err(Error::Invalid("ExtGState line join"))},
                b"ML"=>{let x=number(&v)?;if x<1.{return Err(Error::Invalid("ExtGState miter limit"));}s.stroke.miter_limit=x;}
                b"D"=>{
                    let Object::Array(a)=v else{return Err(Error::Invalid("ExtGState dash"));};
                    arity(&a,2)?;let Object::Array(values)=&a[0] else{return Err(Error::Invalid("ExtGState dash array"));};
                    if values.len()>MAX_DASH{return Err(Error::Limit("dash array"));}
                    let dash=values.iter().map(number).collect::<Result<Vec<_>>>()?;
                    if dash.iter().any(|&x|x<0.)||(!dash.is_empty()&&dash.iter().all(|&x|x==0.)){return Err(Error::Invalid("dash lengths"));}
                    let phase=number(&a[1])?;if phase<0.{return Err(Error::Invalid("dash phase"));}
                    s.stroke.dash=dash;s.stroke.dash_phase=phase;
                }
                // These quality hints do not change the abstract path geometry.
                b"FL" if (0.0..=100.).contains(&number(&v)?)=>{},
                b"SM" if (0.0..=1.).contains(&number(&v)?)=>{},
                b"RI"=>self.set_intent(s,crate::icc::RenderingIntent::from_pdf_name(name(&v)?)?)?,
                b"SA" if matches!(v,Object::Bool(_))=>{
                    if v==Object::Bool(true){self.warning(Warning::DeviceQualityHintNotInterpreted);}
                }
                b"OP"|b"op" if v==Object::Bool(false)=>{},
                b"HT"|b"BG"|b"BG2"|b"UCR"|b"UCR2" if matches!(&v,Object::Name(n) if n==b"Default")=>{},
                b"OP"|b"op"=>return Err(Error::Unsupported("ExtGState overprint")),
                b"SMask"=>return Err(Error::Unsupported("ExtGState graphics soft mask")),
                b"AIS"=>return Err(Error::Unsupported("ExtGState alpha is shape")),
                b"TK"=>return Err(Error::Unsupported("ExtGState text knockout")),
                b"Font"=>return Err(Error::Unsupported("ExtGState font")),
                b"HT"=>return Err(Error::Unsupported("ExtGState halftone")),
                b"BG"|b"BG2"|b"UCR"|b"UCR2"=>return Err(Error::Unsupported("ExtGState black generation or undercolor removal")),
                b"SA"=>return Err(Error::Invalid("ExtGState stroke adjustment")),
                b"OPM" if matches!(number(&v)?,0.|1.)=>{},
                b"TR"|b"TR2" if matches!(&v,Object::Name(n) if n==b"Identity"||n==b"Default")=>{},
                b"TR"|b"TR2"=>return Err(Error::Unsupported("ExtGState transfer function")),
                _=>return Err(Error::Unsupported("ExtGState unknown or invalid parameter")),
            }
        }
        self.command(Command::Opacity{fill:s.fill_alpha,stroke:s.stroke_alpha})?;
        self.command(Command::Blend(s.blend))
    }
    fn run(&mut self,ops:&[Operation],resources:Option<&Object>,mut s:State,depth:usize,scope:Option<(u32,u16)>,resource_owner:Option<(u32,u16)>)->Result<()>{
        let mut stack=Vec::new();let mut p=Current::default();let mut in_text=false;
        for op in ops{
            self.ops+=1;if self.ops>MAX_OPS{return Err(Error::Limit("cumulative graphics operations"));}
            let a=&op.operands;let w=op.operator.as_slice();
            // Text clipping changes later graphics. Do not pretend it was applied.
            if in_text&&!matches!(w,b"ET"|b"Tc"|b"Tw"|b"Tz"|b"TL"|b"Tf"|b"Tr"|b"Ts"|b"Td"|b"TD"|b"Tm"|b"T*"|b"Tj"|b"TJ"|b"'"|b"\""|b"BDC"|b"BMC"|b"EMC"|b"MP"|b"DP"|b"g"|b"G"|b"rg"|b"RG"|b"k"|b"K"|b"CS"|b"cs"|b"SC"|b"SCN"|b"sc"|b"scn"|b"gs"){
                return Err(Error::Invalid("graphics operator in text object"));
            }
            match w{
                b"q"=>{arity(a,0)?;if stack.len()>=MAX_STACK{return Err(Error::Limit("graphics stack"));}stack.push(s.clone());self.command(Command::Save)?;}
                b"Q"=>{arity(a,0)?;s=stack.pop().ok_or(Error::Invalid("graphics stack underflow"))?;self.command(Command::Restore)?;}
                b"cm"=>{s.ctm=s.ctm.compose(Matrix(nums(a)?))?;}
                b"m"=>{self.moveto(&mut p,s.ctm.point(nums(a)?)?)?;}
                b"l"=>{self.lineto(&mut p,s.ctm.point(nums(a)?)?)?;}
                b"c"=>{let [x1,y1,x2,y2,x3,y3]=nums(a)?;self.cubic(&mut p,s.ctm.point([x1,y1])?,s.ctm.point([x2,y2])?,s.ctm.point([x3,y3])?)?;}
                b"v"=>{let [x2,y2,x3,y3]=nums(a)?;let at=p.point.ok_or(Error::Invalid("curve current point"))?;self.cubic(&mut p,at,s.ctm.point([x2,y2])?,s.ctm.point([x3,y3])?)?;}
                b"y"=>{let [x1,y1,x3,y3]=nums(a)?;let at=s.ctm.point([x3,y3])?;self.cubic(&mut p,s.ctm.point([x1,y1])?,at,at)?;}
                b"h"=>{arity(a,0)?;self.close(&mut p)?;}
                b"re"=>self.rect(&mut p,s.ctm,nums(a)?)?,
                b"W"|b"W*"=>{arity(a,0)?;if p.clip.is_some(){return Err(Error::Invalid("duplicate pending clip"));}p.clip=Some(if w==b"W"{FillRule::NonZero}else{FillRule::EvenOdd});}
                b"S"|b"s"|b"f"|b"F"|b"f*"|b"B"|b"B*"|b"b"|b"b*"|b"n"=>{
                    arity(a,0)?;
                    let fill=match w{b"f"|b"F"|b"B"|b"b"=>Some(FillRule::NonZero),b"f*"|b"B*"|b"b*"=>Some(FillRule::EvenOdd),_=>None};
                    if s.knockout&&matches!(w,b"B"|b"B*"|b"b"|b"b*"){return Err(Error::Unsupported("combined fill stroke in knockout"));}
                    self.finish(&mut p,&s,fill,matches!(w,b"S"|b"s"|b"B"|b"B*"|b"b"|b"b*"),matches!(w,b"s"|b"b"|b"b*"))?;
                }
                b"w"=>{let v=nums::<1>(a)?[0];if v<0.{return Err(Error::Invalid("line width"));}s.stroke.width=v;}
                b"J"=>{s.stroke.cap=match nums::<1>(a)?[0]{0.=>LineCap::Butt,1.=>LineCap::Round,2.=>LineCap::Square,_=>return Err(Error::Invalid("line cap"))};}
                b"j"=>{s.stroke.join=match nums::<1>(a)?[0]{0.=>LineJoin::Miter,1.=>LineJoin::Round,2.=>LineJoin::Bevel,_=>return Err(Error::Invalid("line join"))};}
                b"M"=>{let v=nums::<1>(a)?[0];if v<1.{return Err(Error::Invalid("miter limit"));}s.stroke.miter_limit=v;}
                b"d"=>{
                    arity(a,2)?;let Object::Array(values)=&a[0] else{return Err(Error::Invalid("dash array"));};
                    if values.len()>MAX_DASH{return Err(Error::Limit("dash array"));}
                    let dash=values.iter().map(number).collect::<Result<Vec<_>>>()?;
                    if dash.iter().any(|&x|x<0.)||(!dash.is_empty()&&dash.iter().all(|&x|x==0.)){return Err(Error::Invalid("dash lengths"));}
                    let phase=number(&a[1])?;if phase<0.{return Err(Error::Invalid("dash phase"));}
                    s.stroke.dash=dash;s.stroke.dash_phase=phase;
                }
                b"g"|b"G"|b"rg"|b"RG"|b"k"|b"K"=>{
                    let space=match w{b"g"|b"G"=>Space::Gray,b"rg"|b"RG"=>Space::Rgb,_=>Space::Cmyk};
                    let color=space.color(a)?;
                    if w[0].is_ascii_uppercase(){s.stroke_space=space;s.stroke.color=color;s.stroke_icc=None;s.stroke_values.clear();}else{s.fill_space=space;s.fill=color;s.fill_icc=None;s.fill_values.clear();}
                }
                b"CS"|b"cs"=>{arity(a,1)?;let (space,icc)=self.color_space(resources,name(&a[0])?,s.intent)?;let initial=space.initial()?;if w==b"CS"{s.stroke_space=space;s.stroke.color=initial;s.stroke_icc=icc;s.stroke_values.clear();}else{s.fill_space=space;s.fill=initial;s.fill_icc=icc;s.fill_values.clear();}}
                b"SC"|b"SCN"=>{s.stroke.color=s.stroke_space.color(a)?;if s.stroke_icc.is_some(){s.stroke_values=a.to_vec();}},
                b"sc"|b"scn"=>{s.fill=s.fill_space.color(a)?;if s.fill_icc.is_some(){s.fill_values=a.to_vec();}},
                b"Do"=>{if s.knockout||s.in_mask{return Err(Error::Unsupported("XObject inside path-only knockout or soft mask"));}arity(a,1)?;self.xobject(resources,name(&a[0])?,&s,depth,resource_owner)?;}
                b"BT"|b"ET"|b"Tc"|b"Tw"|b"Tz"|b"TL"|b"Tf"|b"Tr"|b"Ts"|b"Td"|b"TD"|b"Tm"|b"T*"|b"Tj"|b"TJ"|b"'"|b"\""=> {
                    if s.knockout||s.in_mask{return Err(Error::Unsupported("text inside path-only knockout or soft mask"));}
                    match w{
                        b"BT"=>{arity(a,0)?;if in_text{return Err(Error::Invalid("nested BT"));}in_text=true;}
                        b"ET"=>{arity(a,0)?;if !in_text{return Err(Error::Invalid("ET without BT"));}in_text=false;}
                        b"Tr"=>{let v=nums::<1>(a)?[0];if !(0.0..=7.0).contains(&v)||v.fract()!=0.{return Err(Error::Invalid("text render mode"));}if v>=4.{return Err(Error::Unsupported("text clipping"));}}
                        _=>{}
                    }
                    self.warning(Warning::TextRenderingDeferred);
                    self.command(Command::Text{operation:op.clone(),ctm:s.ctm,fill:s.fill,stroke:s.stroke(),scope,resource_owner})?;
                }
                b"BMC"|b"BDC"|b"EMC"|b"MP"|b"DP"=>self.warning(Warning::MarkedContentNotInterpreted),
                b"ri"=>{arity(a,1)?;self.set_intent(&mut s,crate::icc::RenderingIntent::from_pdf_name(name(&a[0])?)?)?;}
                b"i"=>{let v=nums::<1>(a)?[0];if !(0.0..=100.).contains(&v){return Err(Error::Invalid("flatness"));}}
                b"gs"=>{arity(a,1)?;self.ext_gstate(resources,name(&a[0])?,&mut s,depth,resource_owner)?;}
                b"sh"=>return Err(Error::Unsupported("shading")),
                _=>return Err(Error::Unsupported("graphics operator")),
            }
        }
        if !stack.is_empty()||in_text||p.clip.is_some()||!p.segments.is_empty(){return Err(Error::Invalid("unbalanced graphics stream"));}
        Ok(())
    }
}
impl Document<'_>{
    /// Build a partial vector display list with explicit deferred-content warnings.
    /// content_limit counts decoded page and expanded Form bytes cumulatively.
    pub fn page_display_list(&self,page:&PageInfo,content_limit:usize)->Result<DisplayList>{
        self.page_display_list_with_adaptation(page,content_limit,crate::icc::AdaptationDegree::default())
    }
    pub fn page_display_list_with_adaptation(&self,page:&PageInfo,content_limit:usize,degree:crate::icc::AdaptationDegree)->Result<DisplayList>{
        let limit=content_limit.min(content::MAX_CONTENT_BYTES);
        let data=self.page_content(page,limit)?;
        let ops=content::parse(&data)?;
        let mut e=Engine{doc:self,out:DisplayList{crop_box:page.crop_box,commands:Vec::new(),warnings:Vec::new(),icc_profiles:Vec::new()},
            active:BTreeSet::new(),ops:0,segments:0,bytes_left:limit-data.len(),icc_cache:BTreeMap::new(),icc_bytes:0,degree};
        e.run(&ops,page.resources.as_ref(),State::default(),0,None,None)?;
        Ok(e.out)
    }
}
