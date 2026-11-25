//! C2.4d-1: bounded floating picture anchor metadata, not layout.
use crate::{Error,InlinePicture,Limits};
use crate::drawing_inline::parse_anchor_picture_node;
use crate::opc::{named,xml};
use roxmltree::Node;

const W:&str="http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WS:&str="http://purl.oclc.org/ooxml/wordprocessingml/main";
const WP:&str="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const WPS:&str="http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing";
const MAX_COORD:i64=27_273_042_316_900;

#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub struct AnchorDistances{
    pub top:Option<u32>,pub bottom:Option<u32>,pub left:Option<u32>,pub right:Option<u32>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RelativeFrom{
    Character,Column,InsideMargin,LeftMargin,Margin,OutsideMargin,Page,RightMargin,
    BottomMargin,Line,Paragraph,TopMargin,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum AnchorAlignment{Left,Right,Center,Inside,Outside,Top,Bottom}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PositionValue{Offset(i32),Align(AnchorAlignment)}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct AnchorPosition{pub relative_from:RelativeFrom,pub value:PositionValue}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum WrapSide{BothSides,Left,Right,Largest}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum AnchorWrap{
    None,TopAndBottom,Square(WrapSide),Tight(WrapSide),Through(WrapSide),
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct FloatingPicture{
    /// Shared picture payload only: this does not represent an inline run item.
    /// Relationship binding uses the actual owning part, via picture.bind().
    pub picture:InlinePicture,
    pub distances:AnchorDistances,
    pub simple_position_enabled:bool,
    pub simple_x:i64,pub simple_y:i64,
    pub horizontal:AnchorPosition,pub vertical:AnchorPosition,
    pub wrap:AnchorWrap,
    /// Optional wrap-specific distances, distinct from anchor-level distances.
    pub wrap_distances:AnchorDistances,
    pub relative_height:u32,pub behind_doc:bool,pub locked:bool,
    pub layout_in_cell:bool,pub allow_overlap:bool,pub hidden:Option<bool>,
}
fn one<'a,'i>(node:Node<'a,'i>,ns:&str,key:&str)->Result<Node<'a,'i>,Error>{
    let mut it=node.children().filter(|n|named(*n,ns,key));
    let first=it.next().ok_or(Error::Invalid("missing anchor element"))?;
    if it.next().is_some(){return Err(Error::Invalid("duplicate anchor element"));}
    Ok(first)
}
fn attr<'a,'i>(node:Node<'a,'i>,key:&str)->Result<&'a str,Error>{
    node.attribute(key).ok_or(Error::Invalid("missing anchor attribute"))
}
fn boolean(value:&str)->Result<bool,Error>{
    match value{"true"|"1"=>Ok(true),"false"|"0"=>Ok(false),_=>Err(Error::Invalid("invalid anchor boolean"))}
}
fn uint(value:&str)->Result<u32,Error>{
    value.parse().map_err(|_|Error::Invalid("invalid anchor unsigned value"))
}
fn coord(value:&str)->Result<i64,Error>{
    let v=value.parse::<i64>().map_err(|_|Error::Invalid("invalid anchor coordinate"))?;
    if !(-MAX_COORD..=MAX_COORD).contains(&v){return Err(Error::Invalid("anchor coordinate range"));}
    Ok(v)
}
fn distances(node:Node<'_, '_>)->Result<AnchorDistances,Error>{
    let d=|key|node.attribute(key).map(uint).transpose();
    Ok(AnchorDistances{top:d("distT")?,bottom:d("distB")?,left:d("distL")?,right:d("distR")?})
}
fn position(node:Node<'_, '_>,ns:&str,horizontal:bool)->Result<AnchorPosition,Error>{
    use RelativeFrom::*;
    let relative_from=match (horizontal,attr(node,"relativeFrom")?){
        (true,"character")=>Character,(true,"column")=>Column,
        (true,"leftMargin")=>LeftMargin,(true,"rightMargin")=>RightMargin,
        (false,"bottomMargin")=>BottomMargin,(false,"line")=>Line,
        (false,"paragraph")=>Paragraph,(false,"topMargin")=>TopMargin,
        (_,"insideMargin")=>InsideMargin,(_,"outsideMargin")=>OutsideMargin,
        (_,"margin")=>Margin,(_,"page")=>Page,
        _=>return Err(Error::Invalid("anchor relative reference")),
    };
    let mut children=node.children().filter(|n|n.is_element());
    let child=children.next().ok_or(Error::Invalid("missing anchor position choice"))?;
    if children.next().is_some()||child.children().any(|n|n.is_element()){
        return Err(Error::Invalid("invalid anchor position choice"));
    }
    // XML text may be split by comments; collect all direct text, under XML budget.
    let value=child.children().filter_map(|n|if n.is_text(){n.text()}else{None}).collect::<String>();
    let value=value.trim();
    let value=if named(child,ns,"posOffset"){
        PositionValue::Offset(value.parse::<i32>().map_err(|_|Error::Invalid("anchor offset range"))?)
    }else if named(child,ns,"align"){
        use AnchorAlignment::*;
        PositionValue::Align(match (horizontal,value){
            (true,"left")=>Left,(true,"right")=>Right,
            (false,"top")=>Top,(false,"bottom")=>Bottom,
            (_,"center")=>Center,(_,"inside")=>Inside,(_,"outside")=>Outside,
            _=>return Err(Error::Invalid("anchor alignment")),
        })
    }else{return Err(Error::Invalid("unknown anchor position choice"));};
    Ok(AnchorPosition{relative_from,value})
}
fn wrap(node:Node<'_, '_>,ns:&str)->Result<(AnchorWrap,AnchorDistances),Error>{
    let names=["wrapNone","wrapTopAndBottom","wrapSquare","wrapTight","wrapThrough"];
    let mut it=node.children().filter(|n|names.iter().any(|s|named(*n,ns,s)));
    let n=it.next().ok_or(Error::Invalid("missing anchor wrap"))?;
    if it.next().is_some(){return Err(Error::Invalid("duplicate anchor wrap"));}
    let side=||match attr(n,"wrapText")?{
        "bothSides"=>Ok(WrapSide::BothSides),"left"=>Ok(WrapSide::Left),
        "right"=>Ok(WrapSide::Right),"largest"=>Ok(WrapSide::Largest),
        _=>Err(Error::Invalid("anchor wrap side")),
    };
    let w=match n.tag_name().name(){
        "wrapNone"=>AnchorWrap::None,"wrapTopAndBottom"=>AnchorWrap::TopAndBottom,
        "wrapSquare"=>AnchorWrap::Square(side()?),"wrapTight"=>AnchorWrap::Tight(side()?),
        "wrapThrough"=>AnchorWrap::Through(side()?),_=>return Err(Error::Invalid("anchor wrap")),
    };
    Ok((w,distances(n)?))
}
/// Standalone wp:anchor or w:drawing with exactly one wp:anchor.
/// Strict and transitional namespaces cannot be mixed in selected paths.
/// Required position/flag/wrap choices are validated; polygon, effectExtent,
/// unknown optional properties, VML, positioning and painting are not interpreted.
pub fn parse_floating_picture(bytes:&[u8],limits:Limits)->Result<FloatingPicture,Error>{
    if bytes.len()>limits.max_part_bytes{return Err(Error::Limit("part bytes"));}
    let tree=xml(bytes,limits)?;
    let root=tree.root_element();
    let anchor=if named(root,W,"drawing")||named(root,WS,"drawing"){
        let ns=if named(root,W,"drawing"){WP}else{WPS};
        if root.children().filter(|n|n.is_element()).count()!=1{
            return Err(Error::Unsupported("multiple drawing children"));
        }
        one(root,ns,"anchor")?
    }else{root};
    parse_floating_picture_node(anchor)
}
pub(crate) fn parse_floating_picture_node(anchor:Node<'_, '_>)->Result<FloatingPicture,Error>{
    let ns=if named(anchor,WP,"anchor"){WP}else if named(anchor,WPS,"anchor"){WPS}
        else{return Err(Error::Unsupported("floating picture root"));};
    let simple=one(anchor,ns,"simplePos")?;
    let (wrap,wrap_distances)=wrap(anchor,ns)?;
    Ok(FloatingPicture{
        picture:parse_anchor_picture_node(anchor)?,
        distances:distances(anchor)?,
        simple_position_enabled:anchor.attribute("simplePos").map(boolean).transpose()?.unwrap_or(false),
        simple_x:coord(attr(simple,"x")?)?,simple_y:coord(attr(simple,"y")?)?,
        horizontal:position(one(anchor,ns,"positionH")?,ns,true)?,
        vertical:position(one(anchor,ns,"positionV")?,ns,false)?,
        wrap,wrap_distances,
        relative_height:uint(attr(anchor,"relativeHeight")?)?,
        behind_doc:boolean(attr(anchor,"behindDoc")?)?,
        locked:boolean(attr(anchor,"locked")?)?,
        layout_in_cell:boolean(attr(anchor,"layoutInCell")?)?,
        allow_overlap:boolean(attr(anchor,"allowOverlap")?)?,
        hidden:anchor.attribute("hidden").map(boolean).transpose()?,
    })
}
