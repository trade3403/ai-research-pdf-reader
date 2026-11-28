//! C2.4d-2: deterministic signed EMU placement and rectangular line exclusion.
//! All frames use a common physical-page coordinate system. No font layout,
//! collision resolution, clipping, contour polygons or drawing is performed.
use crate::{AnchorAlignment,AnchorPosition,AnchorWrap,Error,FloatingPicture,PositionValue,Rect,RelativeFrom,WrapSide};

// Bound public inputs before arithmetic; intermediate sums fit in i64.
const LIMIT:i64=1_i64<<50;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct EmuSpan{pub start:i64,pub end:i64}
impl EmuSpan{
    fn valid(self)->Result<Self,Error>{
        if self.start < -LIMIT || self.end > LIMIT || self.end < self.start{
            return Err(Error::Invalid("floating span bounds"));
        }
        Ok(self)
    }
    fn positive(self)->Result<Self,Error>{
        self.valid()?;
        if self.start==self.end{return Err(Error::Invalid("empty floating span"));}
        Ok(self)
    }
    pub fn length(self)->Result<i64,Error>{Ok(self.valid()?.end-self.start)}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct EmuRect{pub x:EmuSpan,pub y:EmuSpan}
impl EmuRect{
    /// Exact conversion of existing page/column geometry (635 EMU per twip).
    pub fn from_twips(r:Rect)->Self{
        let x=i64::from(r.x)*635;let y=i64::from(r.y)*635;
        Self{x:EmuSpan{start:x,end:x+i64::from(r.width)*635},
            y:EmuSpan{start:y,end:y+i64::from(r.height)*635}}
    }
    fn valid(self)->Result<Self,Error>{self.x.positive()?;self.y.positive()?;Ok(self)}
}
/// Explicit binding edge; vertical inside/outside is also supplied, never
/// inferred from page numbering or section direction.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InsideEdge{Start,End}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct AnchorLayoutContext{
    pub page:EmuRect,pub content:EmuRect,
    pub column:Option<EmuSpan>,pub character:Option<EmuSpan>,
    pub line:Option<EmuSpan>,pub paragraph:Option<EmuSpan>,
    pub horizontal_inside:InsideEdge,pub vertical_inside:InsideEdge,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FloatingPlacement{
    pub picture:EmuRect,
    pub exclusion:EmuRect,
    pub wrap:AnchorWrap,
    pub hidden:bool,pub behind_doc:bool,pub relative_height:u32,
    pub allow_overlap:bool,pub locked:bool,pub layout_in_cell:bool,
}
fn contained(outer:EmuSpan,inner:EmuSpan)->bool{
    outer.start<=inner.start&&inner.end<=outer.end
}
fn opposite(edge:InsideEdge)->InsideEdge{
    match edge{InsideEdge::Start=>InsideEdge::End,InsideEdge::End=>InsideEdge::Start}
}
fn margin_strip(page:EmuSpan,content:EmuSpan,edge:InsideEdge)->EmuSpan{
    match edge{
        InsideEdge::Start=>EmuSpan{start:page.start,end:content.start},
        InsideEdge::End=>EmuSpan{start:content.end,end:page.end},
    }
}
fn reference(c:&AnchorLayoutContext,r:RelativeFrom,h:bool)->Result<EmuSpan,Error>{
    use RelativeFrom::*;
    let (p,m,inside)=if h{(c.page.x,c.content.x,c.horizontal_inside)}
        else{(c.page.y,c.content.y,c.vertical_inside)};
    let required=|s:Option<EmuSpan>|s.ok_or(Error::Invalid("missing floating reference frame"));
    let span=match (h,r){
        (_,Page)=>p,(_,Margin)=>m,
        (_,InsideMargin)=>margin_strip(p,m,inside),
        (_,OutsideMargin)=>margin_strip(p,m,opposite(inside)),
        (true,LeftMargin)|(false,TopMargin)=>margin_strip(p,m,InsideEdge::Start),
        (true,RightMargin)|(false,BottomMargin)=>margin_strip(p,m,InsideEdge::End),
        (true,Column)=>required(c.column)?,(true,Character)=>required(c.character)?,
        (false,Line)=>required(c.line)?,(false,Paragraph)=>required(c.paragraph)?,
        _=>return Err(Error::Invalid("floating reference axis")),
    };
    // Character/caret and zero-sized margin reference frames may be points.
    span.valid()
}
fn position(c:&AnchorLayoutContext,p:AnchorPosition,size:i64,h:bool)->Result<i64,Error>{
    let frame=reference(c,p.relative_from,h)?;
    let inside=if h{c.horizontal_inside}else{c.vertical_inside};
    use AnchorAlignment::*;
    let edge=match p.value{
        PositionValue::Offset(d)=>return Ok(frame.start+i64::from(d)),
        PositionValue::Align(Center)=>return Ok(frame.start+(frame.end-frame.start-size).div_euclid(2)),
        PositionValue::Align(Left) if h=>InsideEdge::Start,
        PositionValue::Align(Right) if h=>InsideEdge::End,
        PositionValue::Align(Top) if !h=>InsideEdge::Start,
        PositionValue::Align(Bottom) if !h=>InsideEdge::End,
        PositionValue::Align(Inside)=>inside,
        PositionValue::Align(Outside)=>opposite(inside),
        _=>return Err(Error::Invalid("floating alignment axis")),
    };
    Ok(match edge{InsideEdge::Start=>frame.start,InsideEdge::End=>frame.end-size})
}
fn extent(v:u64)->Result<i64,Error>{
    let v=i64::try_from(v).map_err(|_|Error::Limit("floating extent"))?;
    if !(1..=LIMIT).contains(&v){return Err(Error::Invalid("floating extent bounds"));}
    Ok(v)
}
fn distance(base:Option<u32>,specific:Option<u32>)->i64{
    i64::from(specific.or(base).unwrap_or(0))
}
/// Resolve one anchor. Simple position is relative to the page origin and
/// overrides H/V positioning. No automatic clamping or cell-relative rewrite:
/// callers supply measured reference frames. Wrap-specific distances override
/// anchor distances per edge, including explicit zero. Metadata flags are
/// retained for later paint order and collision policy.
pub fn place_floating_picture(p:&FloatingPicture,c:&AnchorLayoutContext)->Result<FloatingPlacement,Error>{
    c.page.valid()?;c.content.valid()?;
    if !contained(c.page.x,c.content.x)||!contained(c.page.y,c.content.y){
        return Err(Error::Invalid("content outside floating page"));
    }
    let w=extent(p.picture.extent.cx)?;let h=extent(p.picture.extent.cy)?;
    let (x,y)=if p.simple_position_enabled{
        if !(-LIMIT..=LIMIT).contains(&p.simple_x)||!(-LIMIT..=LIMIT).contains(&p.simple_y){
            return Err(Error::Invalid("simple floating position bounds"));
        }
        (c.page.x.start+p.simple_x,c.page.y.start+p.simple_y)
    }else{
        (position(c,p.horizontal,w,true)?,position(c,p.vertical,h,false)?)
    };
    let picture=EmuRect{x:EmuSpan{start:x,end:x+w},y:EmuSpan{start:y,end:y+h}}.valid()?;
    let a=p.distances;let b=p.wrap_distances;
    let exclusion=EmuRect{
        x:EmuSpan{start:x-distance(a.left,b.left),end:x+w+distance(a.right,b.right)},
        y:EmuSpan{start:y-distance(a.top,b.top),end:y+h+distance(a.bottom,b.bottom)},
    }.valid()?;
    Ok(FloatingPlacement{picture,exclusion,wrap:p.wrap,hidden:p.hidden.unwrap_or(false),
        behind_doc:p.behind_doc,relative_height:p.relative_height,allow_overlap:p.allow_overlap,
        locked:p.locked,layout_in_cell:p.layout_in_cell})
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ContourPolicy{Reject,RectangularFallback}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct LineWrap{
    /// Sorted, nonempty half-open horizontal intervals in the supplied line.
    pub available:Vec<EmuSpan>,
    /// True when tight/through was conservatively approximated by a rectangle.
    pub approximate_contour:bool,
}
fn intersects(a:EmuSpan,b:EmuSpan)->bool{a.start<b.end&&b.start<a.end}
/// Single-anchor line avoidance. An entire positive-height line band is tested,
/// not just its baseline. No wrap/hidden anchors do not obstruct text.
/// Top/bottom blocks the line on intersection. Side choices are physical
/// left/right; largest ties choose left. No minimum glyph-width assumption.
/// Tight/through require explicit fallback opt-in; no polygon fidelity claim.
/// behindDoc does not override an explicit wrap mode in this subset.
pub fn wrap_floating_line(p:&FloatingPlacement,line:EmuRect,policy:ContourPolicy)->Result<LineWrap,Error>{
    line.valid()?;p.exclusion.valid()?;
    let full=||LineWrap{available:vec![line.x],approximate_contour:false};
    if p.hidden||p.wrap==AnchorWrap::None||!intersects(p.exclusion.y,line.y)||!intersects(p.exclusion.x,line.x){
        return Ok(full());
    }
    let (side,approximate_contour)=match p.wrap{
        AnchorWrap::None=>return Ok(full()),
        AnchorWrap::TopAndBottom=>return Ok(LineWrap{available:Vec::new(),approximate_contour:false}),
        AnchorWrap::Square(s)=>(s,false),
        AnchorWrap::Tight(s)|AnchorWrap::Through(s)=>{
            if policy==ContourPolicy::Reject{return Err(Error::Unsupported("floating wrap contour"));}
            (s,true)
        }
    };
    let left=EmuSpan{start:line.x.start,end:p.exclusion.x.start.max(line.x.start)};
    let right=EmuSpan{start:p.exclusion.x.end.min(line.x.end),end:line.x.end};
    let mut available=Vec::with_capacity(2);
    let choose_left=match side{WrapSide::BothSides|WrapSide::Left=>true,WrapSide::Right=>false,
        WrapSide::Largest=>left.length()?>=right.length()?};
    let choose_right=match side{WrapSide::BothSides|WrapSide::Right=>true,WrapSide::Left=>false,
        WrapSide::Largest=>!choose_left};
    if choose_left&&left.start<left.end{available.push(left);}
    if choose_right&&right.start<right.end{available.push(right);}
    Ok(LineWrap{available,approximate_contour})
}
