use rwcore::*;
fn edge(parent:usize,x:i32,y:i32)->Option<GposAttachmentEdge>{Some(GposAttachmentEdge{parent,design_delta:(x,y),pixel_delta:(0,0)})}
#[test]fn chain_propagates_late_parent_placement(){
    let e=[None,edge(0,20,30),edge(1,5,7)];
    let o=[(0,0),(100,0),(100,0)];let mut p=[GposRunAdjustment::default();3];
    p[0].design.x_placement=10;p[1].design.y_placement=2;p[2].design.x_advance=7;
    let v=resolve_gpos_attachments(&e,&o,&[(0,0);3],&p,Default::default()).unwrap();
    assert_eq!((v[1].design.x_placement,v[1].design.y_placement),(-70,32));
    assert_eq!((v[2].design.x_placement,v[2].design.y_placement),(-65,39));assert_eq!(v[2].design.x_advance,7);
}
#[test]fn forward_parent_and_forest(){
    let e=[edge(2,3,4),None,None,edge(2,5,6)];
    let v=resolve_gpos_attachments(&e,&[(0,0);4],&[(0,0);4],&[Default::default();4],Default::default()).unwrap();
    assert_eq!(v[0].design.x_placement,3);assert_eq!(v[3].design.y_placement,6);assert_eq!(v[1],Default::default());
}
#[test]fn cycles_and_invalid_parents(){
    for e in [[edge(1,0,0),edge(0,0,0)],[edge(0,0,0),None],[edge(2,0,0),None]]{
        assert!(resolve_gpos_attachments(&e,&[(0,0);2],&[(0,0);2],&[Default::default();2],Default::default()).is_err());
    }
}
#[test]fn exact_limits_even_when_parent_already_resolved(){
    let e=[None,edge(0,0,0),edge(1,0,0)];let o=[(0,0);3];let p=[Default::default();3];
    let l=GposAttachmentLimits{max_glyphs:3,max_edges:2,max_depth:3};
    assert!(resolve_gpos_attachments(&e,&o,&o,&p,l).is_ok());
    for l in [GposAttachmentLimits{max_glyphs:2,..l},GposAttachmentLimits{max_edges:1,..l},GposAttachmentLimits{max_depth:2,..l}]{
        assert!(resolve_gpos_attachments(&e,&o,&o,&p,l).is_err());
    }
}
#[test]fn separate_pixel_space_and_advances(){
    let e=[None,Some(GposAttachmentEdge{parent:0,design_delta:(30,40),pixel_delta:(-2,3)})];
    let mut p=[GposRunAdjustment::default();2];p[0].pixels.x_placement=5;p[1].pixels.y_advance=9;
    let v=resolve_gpos_attachments(&e,&[(0,0),(100,0)],&[(0,0),(7,0)],&p,Default::default()).unwrap();
    assert_eq!(v[1].design.x_placement,-70);assert_eq!(v[1].pixels.x_placement,-4);assert_eq!(v[1].pixels.y_placement,3);assert_eq!(v[1].pixels.y_advance,9);
}
#[test]fn overflow_and_cancellation(){
    let e=[None,edge(0,1,0)];let mut p=[GposRunAdjustment::default();2];p[0].design.x_placement=i32::MAX;
    assert!(resolve_gpos_attachments(&e,&[(0,0);2],&[(0,0);2],&p,Default::default()).is_err());
    let v=resolve_gpos_attachments(&e,&[(i32::MAX,0),(i32::MAX,0)],&[(0,0);2],&[Default::default();2],Default::default()).unwrap();
    assert_eq!(v[1].design.x_placement,1);assert_eq!(p[0].design.x_placement,i32::MAX);
}
#[test]fn anchors_to_edge_and_unresolved_point(){
    let a=GposAnchor{x:30,y:40,detail:GposAnchorDetail::Coordinate};
    let b=GposAnchor{x:10,y:15,detail:GposAnchorDetail::ContourPoint(0)};
    let e=GposAttachmentEdge::from_anchors(0,&a,&b,GposRunMode::DesignOnly).unwrap();assert_eq!(e.design_delta,(20,25));
    assert!(GposAttachmentEdge::from_anchors(0,&a,&b,GposRunMode::DevicePixels{x_ppem:12,y_ppem:12}).is_err());
}
#[test]fn deep_chain_iterative(){
    let n=20000;let mut e=vec![None;n];for i in 0..n-1{e[i]=edge(i+1,1,0);}
    let o=vec![(0,0);n];let p=vec![Default::default();n];
    let v=resolve_gpos_attachments(&e,&o,&o,&p,Default::default()).unwrap();assert_eq!(v[0].design.x_placement,19999);
}
#[test]fn length_mismatch_and_empty(){
    assert!(resolve_gpos_attachments(&[],&[(0,0)],&[],&[],Default::default()).is_err());
    assert!(resolve_gpos_attachments(&[],&[],&[],&[],Default::default()).unwrap().is_empty());
}
