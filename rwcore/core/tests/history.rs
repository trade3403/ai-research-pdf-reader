use rwcore::*;
fn doc(texts:&[&str])->Document{
    Document{blocks:vec![Block::Paragraph(Paragraph{runs:texts.iter().map(|s|Run{text:(*s).into(),..Default::default()}).collect(),..Default::default()})],..Default::default()}
}
fn path(run:usize)->RunPath{RunPath{cells:vec![],paragraph:0,run}}
fn text(e:&DocumentEditor,i:usize)->&str{&path(i).get(e.document()).unwrap().text}
fn edit(i:usize,a:usize,b:usize,expected:&str,replacement:&str)->TextEdit{
    TextEdit{path:path(i),start:a,end:b,unit:TextUnit::Utf8,expected:expected.into(),replacement:replacement.into(),insertion:InsertAffinity::Before,interior:InteriorAnchorPolicy::Reject}
}
fn apply(e:&mut DocumentEditor,ops:&[TextEdit])->Result<HistoryEditReport,Error>{
    e.apply(e.revision(),ops,EditLimits::default())
}
fn editor()->DocumentEditor{DocumentEditor::new(doc(&["abc","def"]),HistoryLimits::default())}
fn pic()->InlinePicture{
    let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};
    InlinePicture{extent:PictureExtent{cx:635,cy:635},document_properties:id.clone(),picture_properties:id,embed:Some("img".into()),link:None}
}
fn mixed_doc()->Document{
    let mut d=doc(&["ABC"]);let Block::Paragraph(p)=&mut d.blocks[0] else{panic!()};
    let r=&mut p.runs[0];r.properties.bold=Some(true);
    r.pictures.push(InlinePictureAt{text_offset:1,picture:pic()});
    r.drawing_order.push(RunDrawingAt{text_offset:1,kind:RunDrawingKind::Inline(0),geometry:PictureGeometryState::Absent});
    let pos=AnchorPosition{relative_from:RelativeFrom::Page,value:PositionValue::Offset(0)};
    let f=FloatingPicture{picture:pic(),distances:AnchorDistances::default(),simple_position_enabled:false,simple_x:0,simple_y:0,
        horizontal:pos,vertical:pos,wrap:AnchorWrap::None,wrap_distances:AnchorDistances::default(),relative_height:0,
        behind_doc:false,locked:false,layout_in_cell:true,allow_overlap:true,hidden:None};
    r.drawing_order.push(RunDrawingAt{text_offset:2,kind:RunDrawingKind::Floating(Box::new(f)),geometry:PictureGeometryState::Absent});
    d
}
#[test]fn multi_run_transaction_undo_redo(){
    let mut e=editor();let original=e.document().clone();
    let r=apply(&mut e,&[edit(0,0,1,"a","中"),edit(1,0,3,"def","😀")]).unwrap();
    assert_eq!(r.history.changed_runs,2);assert_eq!(e.revision(),1);assert_eq!(e.undo_len(),1);
    let after=e.document().clone();
    assert_eq!(e.undo(1).unwrap().unwrap().revision,2);assert_eq!(e.document(),&original);
    assert_eq!(e.redo(2).unwrap().unwrap().revision,3);assert_eq!(e.document(),&after);
}
#[test]fn exact_picture_anchor_and_format_restoration(){
    let d=mixed_doc();let mut e=DocumentEditor::new(d.clone(),HistoryLimits::default());
    let mut op=edit(0,0,3,"ABC","中");op.interior=InteriorAnchorPolicy::PreserveAtStart;
    apply(&mut e,&[op]).unwrap();
    let after=e.document().clone();assert_eq!(path(0).get(&after).unwrap().drawings().unwrap().iter().map(|p|p.text_offset).collect::<Vec<_>>(),vec![0,0]);
    e.undo(1).unwrap();assert_eq!(e.document(),&d);
    e.redo(2).unwrap();assert_eq!(e.document(),&after);
}
#[test]fn redo_branch_discarded_only_after_success(){
    let mut e=editor();apply(&mut e,&[edit(0,0,1,"a","A")]).unwrap();e.undo(1).unwrap();
    assert!(apply(&mut e,&[edit(0,0,1,"z","Z")]).is_err());assert_eq!(e.redo_len(),1);
    apply(&mut e,&[edit(0,0,1,"a","B")]).unwrap();assert_eq!(e.redo_len(),0);assert_eq!(text(&e,0),"Bbc");
    assert_eq!(e.redo(3).unwrap(),None);
}
#[test]fn stale_revisions_rejected_without_change(){
    let mut e=editor();apply(&mut e,&[edit(0,0,1,"a","A")]).unwrap();let before=e.document().clone();
    assert!(e.apply(0,&[edit(0,0,1,"A","B")],EditLimits::default()).is_err());
    assert!(e.undo(0).is_err());assert!(e.redo(0).is_err());assert_eq!(e.document(),&before);assert_eq!(e.revision(),1);
}
#[test]fn noops_and_net_noop_preserve_redo(){
    let mut e=editor();apply(&mut e,&[edit(0,0,1,"a","A")]).unwrap();e.undo(1).unwrap();
    let r=apply(&mut e,&[edit(0,0,1,"a","A"),edit(0,0,1,"A","a")]).unwrap();
    assert_eq!(r.history.changed_runs,0);assert_eq!(e.revision(),2);assert_eq!(e.redo_len(),1);
    apply(&mut e,&[]).unwrap();assert_eq!(e.revision(),2);
}
#[test]fn failed_late_operation_preserves_everything(){
    let mut e=editor();let before=e.document().clone();let u=e.usage();
    assert!(apply(&mut e,&[edit(0,0,1,"a","A"),edit(1,0,1,"x","X")]).is_err());
    assert_eq!(e.document(),&before);assert_eq!(e.usage(),u);assert_eq!(e.undo_len(),0);assert_eq!(e.revision(),0);
}
#[test]fn oldest_entry_eviction_and_usage(){
    let mut e=DocumentEditor::new(doc(&["abc"]),HistoryLimits{max_entries:2,..Default::default()});
    for (a,b) in [("a","A"),("A","B"),("B","C")]{apply(&mut e,&[edit(0,0,1,a,b)]).unwrap();}
    assert_eq!(e.undo_len(),2);assert_eq!(e.usage().text_bytes,12);
    e.undo(3).unwrap();e.undo(4).unwrap();assert_eq!(text(&e,0),"Abc");assert_eq!(e.undo(5).unwrap(),None);
    assert_eq!(e.usage().text_bytes,12);assert_eq!(e.redo_len(),2);
}
#[test]fn text_budget_eviction_and_oversized_rejection(){
    let mut e=DocumentEditor::new(doc(&["abc"]),HistoryLimits{max_text_bytes:7,..Default::default()});
    apply(&mut e,&[edit(0,0,1,"a","A")]).unwrap();
    let r=apply(&mut e,&[edit(0,0,1,"A","B")]).unwrap();assert_eq!(r.history.evicted_entries,1);
    let before=e.document().clone();assert!(apply(&mut e,&[edit(0,0,1,"B","123456")]).is_err());
    assert_eq!(e.document(),&before);assert_eq!(e.undo_len(),1);assert_eq!(e.revision(),2);
}
#[test]fn zero_entries_and_anchor_budget(){
    for l in [HistoryLimits{max_entries:0,..Default::default()},HistoryLimits{max_anchor_offsets:5,..Default::default()}]{
        let d=mixed_doc();let mut e=DocumentEditor::new(d.clone(),l);
        assert!(apply(&mut e,&[edit(0,0,0,"","!")]).is_err());assert_eq!(e.document(),&d);assert_eq!(e.revision(),0);
    }
}
#[test]fn selection_history_is_one_transaction(){
    let mut e=editor();let before=e.document().clone();
    let s=SelectionEdit{paragraph:ParagraphPath{cells:vec![],paragraph:0},
        selection:Selection{anchor:RunPoint{run:0,offset:1,unit:TextUnit::Utf8},focus:RunPoint{run:1,offset:1,unit:TextUnit::Utf8}},
        expected:"bcd".into(),replacement:"中".into(),insertion:InsertAffinity::Before,interior:InteriorAnchorPolicy::Reject};
    let r=e.replace(0,&s,SelectionLimits::default()).unwrap();assert_eq!(r.history.changed_runs,2);
    assert_eq!(text(&e,0),"a中");assert_eq!(text(&e,1),"ef");assert_eq!(e.undo_len(),1);
    e.undo(1).unwrap();assert_eq!(e.document(),&before);
}
#[test]fn redo_order_and_usage_after_new_branch(){
    let mut e=editor();apply(&mut e,&[edit(0,0,1,"a","A")]).unwrap();apply(&mut e,&[edit(0,0,1,"A","B")]).unwrap();
    e.undo(2).unwrap();e.undo(3).unwrap();e.redo(4).unwrap();assert_eq!(text(&e,0),"Abc");
    apply(&mut e,&[edit(0,0,1,"A","C")]).unwrap();assert_eq!(e.usage().run_changes,2);assert_eq!(e.redo_len(),0);
}
#[test]fn run_change_budget_and_empty_history(){
    let mut e=DocumentEditor::new(doc(&["abc","def"]),HistoryLimits{max_run_changes:1,..Default::default()});
    assert_eq!(e.undo(0).unwrap(),None);assert_eq!(e.redo(0).unwrap(),None);
    let before=e.document().clone();
    assert!(apply(&mut e,&[edit(0,0,1,"a","A"),edit(1,0,1,"d","D")]).is_err());
    assert_eq!(e.document(),&before);assert_eq!(e.usage(),HistoryUsage::default());assert_eq!(e.into_document(),before);
}
