use rwcore::*;
fn path(run:usize)->RunPath{RunPath{cells:vec![],paragraph:0,run}}
fn doc(runs:Vec<Run>)->Document{Document{blocks:vec![Block::Paragraph(Paragraph{runs,..Default::default()})],..Default::default()}}
fn run(text:&str)->Run{Run{text:text.into(),..Default::default()}}
fn edit(run:usize,start:usize,end:usize,expected:&str,replacement:&str)->TextEdit{
    TextEdit{path:path(run),start,end,unit:TextUnit::Utf8,expected:expected.into(),replacement:replacement.into(),
        insertion:InsertAffinity::Before,interior:InteriorAnchorPolicy::Reject}
}
fn apply(d:&mut Document,edits:&[TextEdit])->Result<EditReport,Error>{apply_text_transaction(d,edits,EditLimits::default())}
fn picture()->InlinePicture{
    let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};
    InlinePicture{extent:PictureExtent{cx:635,cy:635},document_properties:id.clone(),picture_properties:id,embed:Some("img".into()),link:None}
}
fn mixed()->Run{
    let mut r=run("ABC");
    r.pictures.push(InlinePictureAt{text_offset:1,picture:picture()});
    r.drawing_order.push(RunDrawingAt{text_offset:1,kind:RunDrawingKind::Inline(0),geometry:PictureGeometryState::Absent});
    let pos=AnchorPosition{relative_from:RelativeFrom::Page,value:PositionValue::Offset(0)};
    let f=FloatingPicture{picture:picture(),distances:AnchorDistances::default(),simple_position_enabled:false,
        simple_x:0,simple_y:0,horizontal:pos,vertical:pos,wrap:AnchorWrap::None,wrap_distances:AnchorDistances::default(),
        relative_height:0,behind_doc:false,locked:false,layout_in_cell:true,allow_overlap:true,hidden:None};
    r.drawing_order.push(RunDrawingAt{text_offset:1,kind:RunDrawingKind::Floating(Box::new(f)),geometry:PictureGeometryState::Absent});
    r.pictures.push(InlinePictureAt{text_offset:2,picture:picture()});
    r.drawing_order.push(RunDrawingAt{text_offset:2,kind:RunDrawingKind::Inline(1),geometry:PictureGeometryState::Absent});
    r
}
#[test]fn unicode_index_conversion(){
    let t="中😀e\u{301}";
    assert_eq!(text_byte_offset(t,2,TextUnit::UnicodeScalar).unwrap(),7);
    assert_eq!(text_byte_offset(t,3,TextUnit::Utf16).unwrap(),7);
    assert_eq!(text_offset_from_byte(t,7,TextUnit::Utf16).unwrap(),3);
    assert!(text_byte_offset(t,2,TextUnit::Utf16).is_err());
    assert!(text_byte_offset(t,1,TextUnit::Utf8).is_err());
    assert_eq!(text_byte_offset(t,4,TextUnit::UnicodeScalar).unwrap(),t.len());
}
#[test]fn sequential_edits_and_format_preservation(){
    let mut r=run("中文😀");r.properties.bold=Some(true);
    let mut d=doc(vec![r]);let mut a=edit(0,1,2,"文","字");a.unit=TextUnit::UnicodeScalar;
    let mut b=edit(0,2,4,"😀","!");b.unit=TextUnit::Utf16;
    assert_eq!(apply(&mut d,&[a,b]).unwrap(),EditReport{operations:2,touched_runs:1});
    let r=path(0).get(&d).unwrap();assert_eq!(r.text,"中字!");assert_eq!(r.properties.bold,Some(true));
}
#[test]fn rollback_on_late_failure(){
    let mut d=doc(vec![run("a"),run("b")]);let before=d.clone();
    assert!(apply(&mut d,&[edit(0,0,1,"a","ok"),edit(1,0,1,"wrong","x")]).is_err());
    assert_eq!(d,before);
}
#[test]fn insertion_affinity_updates_both_indices(){
    for (aff,expected) in [(InsertAffinity::Before,4),(InsertAffinity::After,1)]{
        let mut d=doc(vec![mixed()]);let mut e=edit(0,1,1,"","中");e.insertion=aff;
        apply(&mut d,&[e]).unwrap();let r=path(0).get(&d).unwrap();
        assert_eq!(r.pictures[0].text_offset,expected);
        assert_eq!(r.drawings().unwrap().iter().map(|a|a.text_offset).collect::<Vec<_>>(),vec![expected,expected,5]);
        assert_eq!(r.text,"A中BC");
    }
}
#[test]fn deletion_crossing_anchor_rejects_atomically(){
    let mut d=doc(vec![mixed()]);let before=d.clone();
    assert!(apply(&mut d,&[edit(0,0,2,"AB","")]).is_err());assert_eq!(d,before);
}
#[test]fn preserve_crossed_anchors_and_order(){
    let mut d=doc(vec![mixed()]);let mut e=edit(0,0,2,"AB","中");e.interior=InteriorAnchorPolicy::PreserveAtStart;
    apply(&mut d,&[e]).unwrap();let r=path(0).get(&d).unwrap();
    let a=r.drawings().unwrap();assert_eq!(r.text,"中C");
    assert_eq!(a.iter().map(|p|p.text_offset).collect::<Vec<_>>(),vec![0,0,3]);
    assert!(matches!(a[0].kind,PictureKindRef::Inline(_)));assert!(matches!(a[1].kind,PictureKindRef::Floating(_)));
    assert_eq!(r.pictures[1].text_offset,3);
}
#[test]fn boundary_anchors_survive_replacement(){
    let mut d=doc(vec![mixed()]);apply(&mut d,&[edit(0,1,2,"B","xyz")]).unwrap();
    let r=path(0).get(&d).unwrap();assert_eq!(r.text,"AxyzC");
    assert_eq!(r.drawings().unwrap().iter().map(|p|p.text_offset).collect::<Vec<_>>(),vec![1,1,4]);
}
#[test]fn legacy_inline_only_updates(){
    let mut r=mixed();r.drawing_order.clear();let mut d=doc(vec![r]);
    apply(&mut d,&[edit(0,0,0,"","!")]).unwrap();
    assert_eq!(path(0).get(&d).unwrap().pictures[0].text_offset,2);
}
#[test]fn nested_table_edit_preserves_structure(){
    let mut d=doc(vec![run("body")]);
    let inner=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![Block::Paragraph(Paragraph{runs:vec![run("cell")],..Default::default()})],..Default::default()}],..Default::default()}],..Default::default()};
    let outer=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![Block::Table(Box::new(inner))],..Default::default()}],..Default::default()}],..Default::default()};
    d.blocks.push(Block::Table(Box::new(outer)));
    let mut e=edit(0,0,4,"cell","changed");e.path.cells=vec![CellStep{block:1,row:0,cell:0},CellStep{block:0,row:0,cell:0}];
    apply(&mut d,&[e.clone()]).unwrap();
    assert_eq!(e.path.get(&d).unwrap().text,"changed");assert_eq!(path(0).get(&d).unwrap().text,"body");
}
#[test]fn rejects_bad_paths_boundaries_and_xml(){
    let mut d=doc(vec![run("中")]);let before=d.clone();
    for e in [edit(1,0,0,"","a"),edit(0,1,3,"",""),edit(0,3,0,"",""),edit(0,0,3,"中","\0")]{
        assert!(apply(&mut d,&[e]).is_err());assert_eq!(d,before);
    }
}
#[test]fn stale_picture_index_rejected(){
    let mut r=mixed();r.drawing_order[0].text_offset=0;let mut d=doc(vec![r]);let before=d.clone();
    assert!(apply(&mut d,&[edit(0,0,1,"A","x")]).is_err());assert_eq!(d,before);
}
#[test]fn bounded_transaction_and_empty_noop(){
    let mut d=doc(vec![run("abc"),run("def")]);let before=d.clone();
    assert_eq!(apply(&mut d,&[]).unwrap(),EditReport{operations:0,touched_runs:0});
    let ops=[edit(0,0,0,"","!"),edit(1,0,0,"","!")];
    for l in [EditLimits{max_operations:1,..Default::default()},EditLimits{max_run_bytes:3,..Default::default()},
        EditLimits{max_staged_bytes:7,..Default::default()},EditLimits{max_input_bytes:1,..Default::default()}]{
        assert!(apply_text_transaction(&mut d,&ops,l).is_err());assert_eq!(d,before);
    }
}
