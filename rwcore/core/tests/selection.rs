use rwcore::*;
fn run(s:&str)->Run{Run{text:s.into(),..Default::default()}}
fn para(texts:&[&str])->Paragraph{Paragraph{runs:texts.iter().map(|s|run(s)).collect(),..Default::default()}}
fn doc(p:Paragraph)->Document{Document{blocks:vec![Block::Paragraph(p)],..Default::default()}}
fn path()->ParagraphPath{ParagraphPath{cells:vec![],paragraph:0}}
fn pt(run:usize,offset:usize)->RunPoint{RunPoint{run,offset,unit:TextUnit::Utf8}}
fn sel(anchor:RunPoint,focus:RunPoint)->Selection{Selection{anchor,focus}}
fn edit(a:RunPoint,b:RunPoint,expected:&str,replacement:&str)->SelectionEdit{
    SelectionEdit{paragraph:path(),selection:sel(a,b),expected:expected.into(),replacement:replacement.into(),
        insertion:InsertAffinity::Before,interior:InteriorAnchorPolicy::Reject}
}
fn apply(d:&mut Document,e:&SelectionEdit)->Result<EditReport,Error>{replace_selection(d,e,SelectionLimits::default())}
fn pic()->InlinePicture{
    let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};
    InlinePicture{extent:PictureExtent{cx:635,cy:635},document_properties:id.clone(),picture_properties:id,embed:Some("img".into()),link:None}
}
fn add_picture(r:&mut Run,offset:usize){
    r.pictures.push(InlinePictureAt{text_offset:offset,picture:pic()});
    let i=r.pictures.len()-1;
    r.drawing_order.push(RunDrawingAt{text_offset:offset,kind:RunDrawingKind::Inline(i),geometry:PictureGeometryState::Absent});
}
#[test]fn paragraph_index_units_and_roundtrip(){
    let p=para(&["中","😀","e\u{301}"]);let i=ParagraphIndex::new(&p,SelectionLimits::default()).unwrap();
    assert_eq!(i.len(TextUnit::Utf8),10);assert_eq!(i.len(TextUnit::UnicodeScalar),4);assert_eq!(i.len(TextUnit::Utf16),5);
    let q=RunPoint{run:1,offset:2,unit:TextUnit::Utf16};
    assert_eq!(i.global_offset(q,TextUnit::Utf8).unwrap(),7);
    assert_eq!(i.locate(3,TextUnit::Utf16,RunBoundary::First).unwrap(),BytePoint{run:1,byte:4});
    assert!(i.locate(2,TextUnit::Utf16,RunBoundary::First).is_err());
    assert!(i.locate(1,TextUnit::Utf8,RunBoundary::Last).is_err());
}
#[test]fn boundary_affinity_including_empty_runs(){
    let p=para(&["a","","","b"]);let i=ParagraphIndex::new(&p,SelectionLimits::default()).unwrap();
    assert_eq!(i.locate(1,TextUnit::Utf8,RunBoundary::First).unwrap(),BytePoint{run:0,byte:1});
    assert_eq!(i.locate(1,TextUnit::Utf8,RunBoundary::Last).unwrap(),BytePoint{run:3,byte:0});
    let p=para(&["",""]);let i=ParagraphIndex::new(&p,SelectionLimits::default()).unwrap();
    assert_eq!(i.locate(0,TextUnit::Utf16,RunBoundary::Last).unwrap().run,1);
}
#[test]fn reverse_selection_and_mixed_units(){
    let p=para(&["ab中","😀","cd"]);let i=ParagraphIndex::new(&p,SelectionLimits::default()).unwrap();
    let s=sel(pt(2,1),RunPoint{run:0,offset:2,unit:TextUnit::UnicodeScalar});
    assert!(i.resolve(s).unwrap().reversed);
    assert_eq!(i.selected_text(s).unwrap(),"中😀c");
}
#[test]fn cross_run_replacement_preserves_styles_and_structure(){
    let mut p=para(&["abc","DEF","ghi"]);p.runs[0].properties.bold=Some(true);p.runs[1].properties.italic=Some(true);
    let mut d=doc(p);
    let e=edit(pt(2,1),pt(0,2),"cDEFg","中");
    assert_eq!(apply(&mut d,&e).unwrap(),EditReport{operations:3,touched_runs:3});
    let p=path().get(&d).unwrap();assert_eq!(p.text(),"ab中hi");assert_eq!(p.runs.len(),3);
    assert_eq!(p.runs[0].text,"ab中");assert_eq!(p.runs[1].text,"");assert_eq!(p.runs[2].text,"hi");
    assert_eq!(p.runs[0].properties.bold,Some(true));assert_eq!(p.runs[1].properties.italic,Some(true));
}
#[test]fn selection_rejects_internal_shared_boundary_images(){
    let mut p=para(&["ab","","cd"]);add_picture(&mut p.runs[0],2);add_picture(&mut p.runs[1],0);
    let mut d=doc(p);let before=d.clone();
    assert!(apply(&mut d,&edit(pt(0,1),pt(2,1),"bc","X")).is_err());assert_eq!(d,before);
}
#[test]fn preserve_images_in_original_runs(){
    let mut p=para(&["ab","","cd"]);add_picture(&mut p.runs[0],2);add_picture(&mut p.runs[1],0);add_picture(&mut p.runs[2],1);
    let mut d=doc(p);let mut e=edit(pt(0,1),pt(2,2),"bcd","中");e.interior=InteriorAnchorPolicy::PreserveAtStart;
    apply(&mut d,&e).unwrap();let p=path().get(&d).unwrap();
    assert_eq!(p.text(),"a中");assert_eq!(p.runs[0].drawings().unwrap()[0].text_offset,4);
    assert_eq!(p.runs[1].drawings().unwrap()[0].text_offset,0);assert_eq!(p.runs[2].drawings().unwrap()[0].text_offset,0);
}
#[test]fn endpoint_anchors_survive(){
    let mut p=para(&["ab","cd"]);add_picture(&mut p.runs[0],1);add_picture(&mut p.runs[1],1);
    let mut d=doc(p);apply(&mut d,&edit(pt(0,1),pt(1,1),"bc","X")).unwrap();
    let p=path().get(&d).unwrap();assert_eq!(p.text(),"aXd");
    assert_eq!(p.runs[0].pictures[0].text_offset,1);assert_eq!(p.runs[1].pictures[0].text_offset,0);
}
#[test]fn collapsed_insert_and_affinity(){
    let mut p=para(&["ab","cd"]);add_picture(&mut p.runs[1],0);
    let mut d=doc(p);let mut e=edit(pt(1,0),pt(1,0),"","😀");e.insertion=InsertAffinity::After;
    apply(&mut d,&e).unwrap();let p=path().get(&d).unwrap();
    assert_eq!(p.text(),"ab😀cd");assert_eq!(p.runs[1].pictures[0].text_offset,0);
}
#[test]fn stale_text_and_invalid_position_rollback(){
    let mut d=doc(para(&["ab","中"]));let before=d.clone();
    for e in [edit(pt(0,1),pt(1,3),"wrong","x"),edit(pt(0,1),pt(1,1),"","x"),edit(pt(0,0),pt(2,0),"","x")]{
        assert!(apply(&mut d,&e).is_err());assert_eq!(d,before);
    }
}
#[test]fn budget_and_late_transaction_failure(){
    let mut d=doc(para(&["ab","cd"]));let before=d.clone();let e=edit(pt(0,1),pt(1,1),"bc","XYZ");
    for l in [SelectionLimits{max_runs:1,..Default::default()},SelectionLimits{max_paragraph_bytes:4,..Default::default()},
        SelectionLimits{edit:EditLimits{max_operations:1,..Default::default()},..Default::default()},
        SelectionLimits{edit:EditLimits{max_run_bytes:3,..Default::default()},..Default::default()}]{
        assert!(replace_selection(&mut d,&e,l).is_err());assert_eq!(d,before);
    }
}
#[test]fn equal_text_noop_and_empty_paragraph(){
    let mut d=doc(para(&["ab","cd"]));let before=d.clone();
    assert_eq!(apply(&mut d,&edit(pt(0,1),pt(1,1),"bc","bc")).unwrap(),EditReport{operations:0,touched_runs:0});
    assert_eq!(d,before);let p=Paragraph::default();let i=ParagraphIndex::new(&p,SelectionLimits::default()).unwrap();
    assert_eq!(i.len(TextUnit::Utf8),0);assert!(i.locate(0,TextUnit::Utf8,RunBoundary::First).is_err());
}
#[test]fn nested_cell_selection(){
    let p=para(&["ab","cd"]);let t=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![Block::Paragraph(p)],..Default::default()}],..Default::default()}],..Default::default()};
    let mut d=Document{blocks:vec![Block::Table(Box::new(t))],..Default::default()};
    let mut e=edit(pt(0,1),pt(1,1),"bc","X");e.paragraph.cells=vec![CellStep{block:0,row:0,cell:0}];
    apply(&mut d,&e).unwrap();assert_eq!(e.paragraph.get(&d).unwrap().text(),"aXd");
}
