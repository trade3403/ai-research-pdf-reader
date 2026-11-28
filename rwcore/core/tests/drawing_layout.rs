use rwcore::*;
fn span(start:i64,end:i64)->EmuSpan{EmuSpan{start,end}}
fn rect(x:i64,y:i64,r:i64,b:i64)->EmuRect{EmuRect{x:span(x,r),y:span(y,b)}}
fn pos(relative_from:RelativeFrom,value:PositionValue)->AnchorPosition{AnchorPosition{relative_from,value}}
fn context()->AnchorLayoutContext{
    AnchorLayoutContext{page:rect(0,0,1000,1000),content:rect(100,200,900,800),
        column:Some(span(100,500)),character:Some(span(180,180)),
        line:Some(span(250,270)),paragraph:Some(span(240,400)),
        horizontal_inside:InsideEdge::Start,vertical_inside:InsideEdge::Start}
}
fn anchor()->FloatingPicture{
    let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};
    FloatingPicture{picture:InlinePicture{extent:PictureExtent{cx:200,cy:100},
        document_properties:id.clone(),picture_properties:id,embed:Some("rId1".into()),link:None},
        distances:AnchorDistances::default(),simple_position_enabled:false,simple_x:0,simple_y:0,
        horizontal:pos(RelativeFrom::Column,PositionValue::Offset(200)),
        vertical:pos(RelativeFrom::Paragraph,PositionValue::Offset(10)),
        wrap:AnchorWrap::Square(WrapSide::BothSides),wrap_distances:AnchorDistances::default(),
        relative_height:2,behind_doc:false,locked:false,layout_in_cell:true,allow_overlap:false,hidden:None}
}
fn place(p:&FloatingPicture)->FloatingPlacement{place_floating_picture(p,&context()).unwrap()}
fn wrap(p:&FloatingPicture,line:EmuRect)->LineWrap{
    wrap_floating_line(&place(p),line,ContourPolicy::Reject).unwrap()
}
#[test]fn exact_twip_conversion(){
    assert_eq!(EmuRect::from_twips(Rect{x:1,y:2,width:3,height:4}),rect(635,1270,2540,3810));
}
#[test]fn signed_offset_and_measured_references(){
    let mut p=anchor();
    assert_eq!(place(&p).picture,rect(300,250,500,350));
    p.horizontal=pos(RelativeFrom::Character,PositionValue::Offset(-200));
    p.vertical=pos(RelativeFrom::Line,PositionValue::Offset(-300));
    assert_eq!(place(&p).picture,rect(-20,-50,180,50));
}
#[test]fn center_end_and_oversize_alignment(){
    let mut p=anchor();
    p.horizontal=pos(RelativeFrom::Margin,PositionValue::Align(AnchorAlignment::Center));
    p.vertical=pos(RelativeFrom::Page,PositionValue::Align(AnchorAlignment::Bottom));
    assert_eq!(place(&p).picture,rect(400,900,600,1000));
    p.picture.extent.cx=1001;
    assert_eq!(place(&p).picture.x,span(-1,1000));
}
#[test]fn inside_outside_and_margin_strips(){
    let mut p=anchor();let mut c=context();
    p.horizontal=pos(RelativeFrom::InsideMargin,PositionValue::Align(AnchorAlignment::Inside));
    p.vertical=pos(RelativeFrom::OutsideMargin,PositionValue::Align(AnchorAlignment::Outside));
    assert_eq!(place_floating_picture(&p,&c).unwrap().picture,rect(0,900,200,1000));
    c.horizontal_inside=InsideEdge::End;c.vertical_inside=InsideEdge::End;
    assert_eq!(place_floating_picture(&p,&c).unwrap().picture,rect(800,0,1000,100));
}
#[test]fn simple_position_precedes_missing_frames(){
    let mut c=context();c.column=None;c.paragraph=None;
    c.page=rect(50,50,1000,1000);
    let mut p=anchor();p.simple_position_enabled=true;p.simple_x=-100;p.simple_y=20;
    assert_eq!(place_floating_picture(&p,&c).unwrap().picture,rect(-50,70,150,170));
    p.simple_position_enabled=false;assert!(place_floating_picture(&p,&c).is_err());
}
#[test]fn distance_precedence_and_flags(){
    let mut p=anchor();p.distances.left=Some(40);p.distances.top=Some(10);
    p.wrap_distances.left=Some(0);p.wrap_distances.right=Some(30);p.behind_doc=true;
    let q=place(&p);assert_eq!(q.exclusion,rect(300,240,530,350));
    assert!(q.behind_doc&&q.layout_in_cell&&!q.allow_overlap);assert_eq!(q.relative_height,2);
}
#[test]fn both_sides_and_half_open_boundaries(){
    let p=anchor();let line=rect(100,249,900,251);
    assert_eq!(wrap(&p,line).available,vec![span(100,300),span(500,900)]);
    assert_eq!(wrap(&p,rect(100,230,900,250)).available,vec![span(100,900)]);
    assert_eq!(wrap(&p,rect(500,250,900,350)).available,vec![span(500,900)]);
}
#[test]fn side_policies_and_largest_tie(){
    let mut p=anchor();let line=rect(100,260,700,270);
    for (side,expected) in [(WrapSide::Left,span(100,300)),(WrapSide::Right,span(500,700)),
        (WrapSide::Largest,span(100,300))]{
        p.wrap=AnchorWrap::Square(side);assert_eq!(wrap(&p,line).available,vec![expected]);
    }
    assert_eq!(wrap(&p,rect(100,260,900,270)).available,vec![span(500,900)]);
    p.wrap=AnchorWrap::Square(WrapSide::Left);
    assert!(wrap(&p,rect(350,260,900,270)).available.is_empty());
}
#[test]fn top_bottom_none_and_hidden(){
    let mut p=anchor();let line=rect(100,260,900,270);
    p.wrap=AnchorWrap::TopAndBottom;assert!(wrap(&p,line).available.is_empty());
    p.hidden=Some(true);assert_eq!(wrap(&p,line).available,vec![line.x]);
    p.hidden=None;p.wrap=AnchorWrap::None;assert_eq!(wrap(&p,line).available,vec![line.x]);
}
#[test]fn explicit_contour_policy(){
    let mut p=anchor();let line=rect(100,260,900,270);
    for mode in [AnchorWrap::Tight(WrapSide::BothSides),AnchorWrap::Through(WrapSide::BothSides)]{
        p.wrap=mode;let q=place(&p);
        assert!(wrap_floating_line(&q,line,ContourPolicy::Reject).is_err());
        let r=wrap_floating_line(&q,line,ContourPolicy::RectangularFallback).unwrap();
        assert!(r.approximate_contour);assert_eq!(r.available,vec![span(100,300),span(500,900)]);
    }
}
#[test]fn clipping_and_complete_obstruction(){
    let p=anchor();
    assert!(wrap(&p,rect(350,260,450,270)).available.is_empty());
    assert_eq!(wrap(&p,rect(350,260,600,270)).available,vec![span(500,600)]);
}
#[test]fn invalid_public_geometry_and_arithmetic(){
    let mut c=context();let mut p=anchor();
    c.content.x=span(-1,900);assert!(place_floating_picture(&p,&c).is_err());
    c=context();p.horizontal.relative_from=RelativeFrom::Line;assert!(place_floating_picture(&p,&c).is_err());
    p=anchor();p.picture.extent.cx=u64::MAX;assert!(place_floating_picture(&p,&c).is_err());
    p=anchor();p.simple_position_enabled=true;p.simple_x=i64::MAX;assert!(place_floating_picture(&p,&c).is_err());
    p=anchor();assert!(wrap_floating_line(&place(&p),rect(5,0,4,1),ContourPolicy::Reject).is_err());
}
