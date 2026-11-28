use rwcore::*;
fn run(s:&str)->Run{Run{text:s.into(),..Default::default()}}
fn p(s:&str)->Block{Block::Paragraph(Paragraph{runs:vec![run(s)],..Default::default()})}
fn doc(ss:&[&str])->Document{Document{blocks:ss.iter().map(|s|p(s)).collect(),..Default::default()}}
fn para(b:&Block)->&Paragraph{let Block::Paragraph(p)=b else{panic!()};p}
fn para_mut(b:&mut Block)->&mut Paragraph{let Block::Paragraph(p)=b else{panic!()};p}
fn tx(range:std::ops::Range<usize>,ops:Vec<StructureOp>)->StructureTransaction{StructureTransaction{cells:vec![],range,operations:ops}}
fn split(i:usize,run:usize,offset:usize,unit:TextUnit,anchors:SplitAnchors)->StructureOp{
    StructureOp::Split{paragraph:i,at:SplitPoint::InRun{point:RunPoint{run,offset,unit},anchors}}
}
fn join(i:usize,formatting:JoinFormatting)->StructureOp{StructureOp::JoinNext{paragraph:i,formatting}}
fn apply(d:&mut Document,t:&StructureTransaction)->Result<StructureReport,Error>{apply_structure_transaction(d,t,StructureLimits::default())}
fn pic()->InlinePicture{
    let id=PictureIdentity{id:1,name:"p".into(),description:None,title:None};
    InlinePicture{extent:PictureExtent{cx:635,cy:635},document_properties:id.clone(),picture_properties:id,embed:Some("img".into()),link:None}
}
fn inline(r:&mut Run,offset:usize){
    let i=r.pictures.len();r.pictures.push(InlinePictureAt{text_offset:offset,picture:pic()});
    r.drawing_order.push(RunDrawingAt{text_offset:offset,kind:RunDrawingKind::Inline(i),geometry:PictureGeometryState::Absent});
}
fn floating(r:&mut Run,offset:usize){
    let pos=AnchorPosition{relative_from:RelativeFrom::Page,value:PositionValue::Offset(0)};
    let f=FloatingPicture{picture:pic(),distances:AnchorDistances::default(),simple_position_enabled:false,simple_x:0,simple_y:0,horizontal:pos,vertical:pos,
        wrap:AnchorWrap::None,wrap_distances:AnchorDistances::default(),relative_height:0,behind_doc:false,locked:false,layout_in_cell:true,allow_overlap:true,hidden:None};
    r.drawing_order.push(RunDrawingAt{text_offset:offset,kind:RunDrawingKind::Floating(Box::new(f)),geometry:PictureGeometryState::Absent});
}
fn section(a:usize,b:usize,end:SectionEnd)->DocumentSection{DocumentSection{blocks:a..b,properties:None,end}}
#[test]fn unicode_split_preserves_formats(){
    let mut d=doc(&["中😀文"]);let p=para_mut(&mut d.blocks[0]);p.properties.keep_next=Some(true);p.runs[0].properties.bold=Some(true);
    apply(&mut d,&tx(0..1,vec![split(0,0,3,TextUnit::Utf16,SplitAnchors::Left)])).unwrap();
    assert_eq!(para(&d.blocks[0]).text(),"中😀");assert_eq!(para(&d.blocks[1]).text(),"文");
    for b in &d.blocks{assert_eq!(para(b).properties.keep_next,Some(true));assert_eq!(para(b).runs[0].properties.bold,Some(true));}
}
#[test]fn mixed_anchor_partition_and_inline_reindex(){
    for side in [SplitAnchors::Left,SplitAnchors::Right]{
        let mut d=doc(&["ABC"]);let r=&mut para_mut(&mut d.blocks[0]).runs[0];
        inline(r,0);floating(r,1);inline(r,1);inline(r,2);
        apply(&mut d,&tx(0..1,vec![split(0,0,1,TextUnit::Utf8,side)])).unwrap();
        let a=para(&d.blocks[0]).runs[0].drawings().unwrap();let b=para(&d.blocks[1]).runs[0].drawings().unwrap();
        assert_eq!(a.len()+b.len(),4);
        if side==SplitAnchors::Left{assert_eq!(a.len(),3);assert_eq!(b[0].text_offset,1);}
        else{assert_eq!(a.len(),1);assert_eq!(b.iter().map(|p|p.text_offset).collect::<Vec<_>>(),vec![0,0,1]);}
        let right=&para(&d.blocks[1]).runs[0];assert_eq!(right.pictures.len(),if side==SplitAnchors::Left{1}else{2});
    }
}
#[test]fn legacy_inline_partition(){
    let mut d=doc(&["ab"]);let r=&mut para_mut(&mut d.blocks[0]).runs[0];inline(r,1);r.drawing_order.clear();
    apply(&mut d,&tx(0..1,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Right)])).unwrap();
    assert!(para(&d.blocks[0]).runs[0].pictures.is_empty());assert_eq!(para(&d.blocks[1]).runs[0].pictures[0].text_offset,0);
}
#[test]fn between_runs_and_empty_paragraph_split(){
    let mut d=doc(&["a"]);para_mut(&mut d.blocks[0]).runs.push(run("b"));
    apply(&mut d,&tx(0..1,vec![StructureOp::Split{paragraph:0,at:SplitPoint::BetweenRuns(1)}])).unwrap();
    assert_eq!(para(&d.blocks[0]).text(),"a");assert_eq!(para(&d.blocks[1]).text(),"b");
    apply(&mut d,&tx(0..1,vec![StructureOp::Split{paragraph:0,at:SplitPoint::BetweenRuns(0)}])).unwrap();
    assert!(para(&d.blocks[0]).runs.is_empty());
}
#[test]fn join_preserves_runs_and_formatting_policy(){
    let mut d=doc(&["a","b"]);para_mut(&mut d.blocks[1]).properties.keep_next=Some(true);
    let before=d.clone();
    assert!(apply(&mut d,&tx(0..2,vec![join(0,JoinFormatting::RequireEqual)])).is_err());assert_eq!(d,before);
    apply(&mut d,&tx(0..2,vec![join(0,JoinFormatting::KeepRight)])).unwrap();
    assert_eq!(para(&d.blocks[0]).text(),"ab");assert_eq!(para(&d.blocks[0]).runs.len(),2);assert_eq!(para(&d.blocks[0]).properties.keep_next,Some(true));
}
#[test]fn section_ranges_shift_and_terminal_stays_right(){
    let mut d=doc(&["a","bc","d"]);d.sections=vec![section(0,2,SectionEnd::Paragraph),section(2,3,SectionEnd::Body)];
    apply(&mut d,&tx(1..2,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Left)])).unwrap();
    assert_eq!(d.sections[0].blocks,0..3);assert_eq!(d.sections[1].blocks,3..4);assert_eq!(para(&d.blocks[2]).text(),"c");
    apply(&mut d,&tx(1..3,vec![join(0,JoinFormatting::KeepLeft)])).unwrap();
    assert_eq!(d.sections[0].blocks,0..2);assert_eq!(d.sections[1].blocks,2..3);
}
#[test]fn refuses_cross_section_and_nonparagraph(){
    let mut d=doc(&["a","b"]);d.sections=vec![section(0,1,SectionEnd::Paragraph),section(1,2,SectionEnd::Body)];
    let before=d.clone();assert!(apply(&mut d,&tx(0..2,vec![join(0,JoinFormatting::KeepLeft)])).is_err());assert_eq!(d,before);
    d.sections.clear();d.blocks.insert(1,Block::Unsupported{name:"x".into()});
    let before=d.clone();assert!(apply(&mut d,&tx(0..3,vec![join(0,JoinFormatting::KeepLeft)])).is_err());assert_eq!(d,before);
}
#[test]fn nested_cell_transaction_preserves_body_sections(){
    let inner=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![p("ab"),p("cd")],..Default::default()}],..Default::default()}],..Default::default()};
    let outer=Table{rows:vec![TableRow{cells:vec![TableCell{blocks:vec![Block::Table(Box::new(inner))],..Default::default()}],..Default::default()}],..Default::default()};
    let mut d=Document{blocks:vec![Block::Table(Box::new(outer)),p("body")],sections:vec![section(0,2,SectionEnd::Body)],..Default::default()};
    let mut t=tx(0..2,vec![join(0,JoinFormatting::KeepLeft)]);t.cells=vec![CellStep{block:0,row:0,cell:0},CellStep{block:0,row:0,cell:0}];
    apply(&mut d,&t).unwrap();let path=ParagraphPath{cells:t.cells,paragraph:0};
    assert_eq!(path.get(&d).unwrap().text(),"abcd");assert_eq!(d.sections[0].blocks,0..2);
}
#[test]fn late_failure_rolls_back_all_changes(){
    let mut d=doc(&["ab","中"]);let before=d.clone();
    let t=tx(0..2,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Left),split(2,0,1,TextUnit::Utf8,SplitAnchors::Left)]);
    assert!(apply(&mut d,&t).is_err());assert_eq!(d,before);
}
#[test]fn limits_and_invalid_anchors_rejected(){
    let mut d=doc(&["ab"]);let before=d.clone();let t=tx(0..1,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Left)]);
    for l in [StructureLimits{max_operations:0,..Default::default()},StructureLimits{max_runs:1,..Default::default()},StructureLimits{max_sibling_blocks:1,..Default::default()},StructureLimits{max_text_bytes:1,..Default::default()}]{
        assert!(apply_structure_transaction(&mut d,&t,l).is_err());assert_eq!(d,before);
    }
    inline(&mut para_mut(&mut d.blocks[0]).runs[0],3);let before=d.clone();assert!(apply(&mut d,&t).is_err());assert_eq!(d,before);
}
#[test]fn malformed_section_coverage_rejected(){
    let mut d=doc(&["ab","cd"]);d.sections=vec![section(0,1,SectionEnd::Body)];let before=d.clone();
    assert!(apply(&mut d,&tx(0..1,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Left)])).is_err());assert_eq!(d,before);
}
#[test]fn sequential_relative_indices_and_outside_content(){
    let mut d=doc(&["left","abc","def","right"]);
    let t=tx(1..3,vec![split(0,0,1,TextUnit::Utf8,SplitAnchors::Left),join(1,JoinFormatting::KeepLeft)]);
    assert_eq!(apply(&mut d,&t).unwrap(),StructureReport{operations:2,before_paragraphs:2,after_paragraphs:2});
    assert_eq!(d.blocks.iter().map(|b|para(b).text()).collect::<Vec<_>>(),vec!["left","a","bcdef","right"]);
}
