use rwcore::*;
fn p(s:&str)->Block{Block::Paragraph(Paragraph{runs:vec![Run{text:s.into(),..Default::default()}],..Default::default()})}
fn doc(ss:&[&str])->Document{Document{blocks:ss.iter().map(|s|p(s)).collect(),..Default::default()}}
fn editor(ss:&[&str])->DocumentEditor{DocumentEditor::new(doc(ss),HistoryLimits::default())}
fn tx(range:std::ops::Range<usize>,operations:Vec<StructureOp>)->StructureTransaction{StructureTransaction{cells:vec![],range,operations}}
fn split(i:usize,offset:usize)->StructureOp{StructureOp::Split{paragraph:i,at:SplitPoint::InRun{point:RunPoint{run:0,offset,unit:TextUnit::Utf8},anchors:SplitAnchors::Right}}}
fn join(i:usize)->StructureOp{StructureOp::JoinNext{paragraph:i,formatting:JoinFormatting::KeepLeft}}
fn structure(e:&mut DocumentEditor,t:&StructureTransaction)->Result<StructureHistoryReport,Error>{e.structure(e.revision(),t,StructureLimits::default())}
fn path(paragraph:usize,run:usize)->RunPath{RunPath{cells:vec![],paragraph,run}}
fn edit(path:RunPath,a:usize,b:usize,expected:&str,replacement:&str)->TextEdit{TextEdit{path,start:a,end:b,unit:TextUnit::Utf8,expected:expected.into(),replacement:replacement.into(),insertion:InsertAffinity::Before,interior:InteriorAnchorPolicy::Reject}}
fn apply(e:&mut DocumentEditor,op:TextEdit){e.apply(e.revision(),&[op],EditLimits::default()).unwrap();}
fn para(d:&Document,i:usize)->&Paragraph{let Block::Paragraph(p)=&d.blocks[i] else{panic!()};p}
fn paragraph(d:&mut Document,i:usize)->&mut Paragraph{let Block::Paragraph(p)=&mut d.blocks[i] else{panic!()};p}
#[test]fn exact_split_join_undo_redo(){
    let mut e=editor(&["abc","def"]);let before=e.document().clone();
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();let split=e.document().clone();
    structure(&mut e,&tx(1..3,vec![join(0)])).unwrap();let joined=e.document().clone();
    assert_eq!(para(&joined,1).text(),"bcdef");
    e.undo(2).unwrap();assert_eq!(e.document(),&split);
    e.undo(3).unwrap();assert_eq!(e.document(),&before);
    e.redo(4).unwrap();assert_eq!(e.document(),&split);
    e.redo(5).unwrap();assert_eq!(e.document(),&joined);
}
#[test]fn interleaved_text_and_structure_history(){
    let mut e=editor(&["abc","def"]);let original=e.document().clone();
    apply(&mut e,edit(path(1,0),0,1,"d","D"));let text=e.document().clone();
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();let split=e.document().clone();
    apply(&mut e,edit(path(2,0),0,1,"D","Z"));let last=e.document().clone();
    e.undo(3).unwrap();assert_eq!(e.document(),&split);
    e.undo(4).unwrap();assert_eq!(e.document(),&text);
    e.undo(5).unwrap();assert_eq!(e.document(),&original);
    e.redo(6).unwrap();e.redo(7).unwrap();e.redo(8).unwrap();assert_eq!(e.document(),&last);
}
#[test]fn stale_path_revision_after_structure(){
    let mut e=editor(&["abc","def"]);let p=path(1,0);assert_eq!(e.run_at(0,&p).unwrap().text,"def");
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();
    assert!(e.run_at(0,&p).is_err());
    assert!(e.paragraph_at(0,&ParagraphPath{cells:vec![],paragraph:1}).is_err());
    assert!(e.apply(0,&[edit(p,0,0,"","x")],EditLimits::default()).is_err());
    assert_eq!(e.run_at(1,&path(2,0)).unwrap().text,"def");
}
#[test]fn section_ranges_restored_exactly(){
    let mut d=doc(&["ab","c"]);d.sections=vec![DocumentSection{blocks:0..1,properties:None,end:SectionEnd::Paragraph},DocumentSection{blocks:1..2,properties:None,end:SectionEnd::Body}];
    let mut e=DocumentEditor::new(d.clone(),HistoryLimits::default());
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();let after=e.document().clone();
    assert_eq!(after.sections[0].blocks,0..2);assert_eq!(after.sections[1].blocks,2..3);
    e.undo(1).unwrap();assert_eq!(e.document(),&d);e.redo(2).unwrap();assert_eq!(e.document(),&after);
}
fn picture()->InlinePicture{let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};InlinePicture{extent:PictureExtent{cx:635,cy:635},document_properties:id.clone(),picture_properties:id,embed:Some("r".into()),link:None}}
#[test]fn mixed_picture_payload_and_format_restored(){
    let mut d=doc(&["ABC"]);let p=paragraph(&mut d,0);p.properties.keep_next=Some(true);
    let r=&mut p.runs[0];r.properties.bold=Some(true);
    r.pictures.push(InlinePictureAt{text_offset:1,picture:picture()});
    r.drawing_order.push(RunDrawingAt{text_offset:1,kind:RunDrawingKind::Inline(0),geometry:PictureGeometryState::Absent});
    let pos=AnchorPosition{relative_from:RelativeFrom::Page,value:PositionValue::Offset(0)};
    let f=FloatingPicture{picture:picture(),distances:AnchorDistances::default(),simple_position_enabled:false,simple_x:0,simple_y:0,horizontal:pos,vertical:pos,wrap:AnchorWrap::None,wrap_distances:AnchorDistances::default(),relative_height:0,behind_doc:false,locked:false,layout_in_cell:true,allow_overlap:true,hidden:None};
    r.drawing_order.push(RunDrawingAt{text_offset:2,kind:RunDrawingKind::Floating(Box::new(f)),geometry:PictureGeometryState::Absent});
    let mut e=DocumentEditor::new(d.clone(),HistoryLimits::default());
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();let after=e.document().clone();
    assert_eq!(para(&after,1).runs[0].drawings().unwrap().len(),2);
    e.undo(1).unwrap();assert_eq!(e.document(),&d);e.redo(2).unwrap();assert_eq!(e.document(),&after);
}
#[test]fn nested_path_shift_and_recovery(){
    let t=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![p("ab"),p("cd")],..Default::default()}],..Default::default()}],..Default::default()};
    let mut d=doc(&["xy"]);d.blocks.push(Block::Table(Box::new(t)));
    let mut e=DocumentEditor::new(d.clone(),HistoryLimits::default());
    let mut cell=tx(0..2,vec![join(0)]);cell.cells=vec![CellStep{block:1,row:0,cell:0}];
    structure(&mut e,&cell).unwrap();let first=e.document().clone();
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();
    let rp=RunPath{cells:vec![CellStep{block:2,row:0,cell:0}],paragraph:0,run:1};
    apply(&mut e,edit(rp,0,1,"c","C"));let after=e.document().clone();
    e.undo(3).unwrap();e.undo(4).unwrap();assert_eq!(e.document(),&first);
    e.undo(5).unwrap();assert_eq!(e.document(),&d);
    e.redo(6).unwrap();e.redo(7).unwrap();e.redo(8).unwrap();assert_eq!(e.document(),&after);
}
#[test]fn failed_structure_preserves_redo_and_document(){
    let mut e=editor(&["ab","cd"]);
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();e.undo(1).unwrap();
    let before=e.document().clone();let u=e.usage();
    assert!(structure(&mut e,&tx(0..1,vec![split(0,1),join(1)])).is_err());
    assert_eq!(e.document(),&before);assert_eq!(e.usage(),u);assert_eq!(e.redo_len(),1);assert_eq!(e.revision(),2);
    e.redo(2).unwrap();
}
#[test]fn successful_structure_invalidates_redo(){
    let mut e=editor(&["ab","cd"]);apply(&mut e,edit(path(0,0),0,1,"a","A"));e.undo(1).unwrap();
    structure(&mut e,&tx(0..2,vec![join(0)])).unwrap();assert_eq!(e.redo_len(),0);assert_eq!(e.redo(3).unwrap(),None);
}
#[test]fn structural_noop_preserves_history(){
    let mut e=editor(&["ab"]);apply(&mut e,edit(path(0,0),0,1,"a","A"));e.undo(1).unwrap();
    let t=tx(0..1,vec![StructureOp::Split{paragraph:0,at:SplitPoint::BetweenRuns(1)},join(0)]);
    let before=e.document().clone();let r=structure(&mut e,&t).unwrap();
    assert_eq!(r.history.revision,2);assert_eq!(e.document(),&before);assert_eq!(e.redo_len(),1);
}
#[test]fn mixed_history_eviction_retains_valid_suffix(){
    let mut e=DocumentEditor::new(doc(&["ab","cd"]),HistoryLimits{max_entries:2,..Default::default()});
    apply(&mut e,edit(path(0,0),0,1,"a","A"));
    structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();
    apply(&mut e,edit(path(2,0),0,1,"c","C"));assert_eq!(e.undo_len(),2);
    e.undo(3).unwrap();e.undo(4).unwrap();assert_eq!(para(e.document(),0).text(),"Ab");assert_eq!(e.undo(5).unwrap(),None);
}
#[test]fn structural_snapshot_budget_before_commit(){
    for l in [HistoryLimits{max_text_bytes:3,..Default::default()},HistoryLimits{max_run_changes:5,..Default::default()},HistoryLimits{max_entries:0,..Default::default()}]{
        let mut e=DocumentEditor::new(doc(&["ab"]),l);let before=e.document().clone();
        assert!(structure(&mut e,&tx(0..1,vec![split(0,1)])).is_err());assert_eq!(e.document(),&before);assert_eq!(e.revision(),0);
    }
}
#[test]fn usage_accounts_both_snapshots_and_stable_across_restore(){
    let mut e=editor(&["ab"]);structure(&mut e,&tx(0..1,vec![split(0,1)])).unwrap();
    let u=e.usage();assert_eq!(u.text_bytes,4);assert_eq!(u.run_changes,6);
    e.undo(1).unwrap();assert_eq!(e.usage(),u);e.redo(2).unwrap();assert_eq!(e.usage(),u);
}
