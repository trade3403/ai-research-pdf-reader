use rwcore::{parse_table, Limits, Table, Rect, TableDirection, CellInsets, Error,
    compute_table_geometry, compute_declared_grid_geometry};
fn table(body:&str)->Table{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
const LTR:TableDirection=TableDirection::LeftToRight;
const Z:CellInsets=CellInsets{left:0,right:0,top:0,bottom:0};
#[test]
fn unequal_grid_coordinates_and_origin(){
    let t=table("<w:tblGrid><w:gridCol w:w='100'/><w:gridCol w:w='200'/></w:tblGrid><w:tr><w:tc/><w:tc/></w:tr><w:tr><w:tc/><w:tc/></w:tr>");
    let g=compute_declared_grid_geometry(&t,&[50,80],(10,20),LTR,Z).unwrap();
    assert_eq!(g.bounds(),Rect{x:10,y:20,width:300,height:130});
    assert_eq!(g.column_offsets(),&[0,100,300]);
    assert_eq!(g.row_offsets(),&[0,50,130]);
    assert_eq!(g.cell_at(1,1).unwrap().outer,Rect{x:110,y:70,width:200,height:80});
    assert_eq!(g.cells().len(),4);
}
#[test]
fn rectangular_merges_have_single_geometry(){
    let t=table("<w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/></w:tcPr></w:tc><w:tc/></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr>");
    let g=compute_table_geometry(&t,&[100,200,300],&[40,60],(0,0),LTR,Z).unwrap();
    let a=g.cell_at(0,0).unwrap();
    assert_eq!(a.outer,Rect{x:0,y:0,width:300,height:100});
    assert_eq!((a.column_span,a.row_span),(2,2));
    assert!(std::ptr::eq(a,g.cell_at(1,1).unwrap()));
    assert_eq!(g.cells().len(),3);
}
#[test]
fn inherited_margins_explicit_zero_and_default_insets(){
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='20'/><w:top w:w='10'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:tcMar><w:left w:w='0'/><w:bottom w:type='nil'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    let g=compute_table_geometry(&t,&[100],&[60],(5,7),LTR,CellInsets{left:2,right:6,top:2,bottom:2}).unwrap();
    let c=g.cell_at(0,0).unwrap();
    assert_eq!(c.insets,CellInsets{left:0,right:6,top:10,bottom:0});
    assert_eq!(c.content,Rect{x:5,y:17,width:94,height:50});
}
#[test]
fn logical_margin_direction_does_not_reorder_grid(){
    let t=table("<w:tr><w:tc><w:tcPr><w:tcMar><w:start w:w='30'/></w:tcMar></w:tcPr></w:tc><w:tc/></w:tr>");
    let a=compute_table_geometry(&t,&[100,200],&[50],(0,0),LTR,Z).unwrap();
    let b=compute_table_geometry(&t,&[100,200],&[50],(0,0),TableDirection::RightToLeft,Z).unwrap();
    assert_eq!(a.cell_at(0,0).unwrap().content.x,30);
    assert_eq!(b.cell_at(0,0).unwrap().content.x,0);
    assert_eq!(b.cell_at(0,0).unwrap().content.width,70);
    assert_eq!(a.column_offsets(),b.column_offsets());
}
#[test]
fn omitted_slots_remain_empty(){
    let t=table("<w:tr><w:trPr><w:gridBefore w:val='1'/><w:gridAfter w:val='1'/></w:trPr><w:tc/></w:tr>");
    let g=compute_table_geometry(&t,&[10,20,30],&[50],(0,0),LTR,Z).unwrap();
    assert!(g.cell_at(0,0).is_none());
    assert!(g.cell_at(0,2).is_none());
    assert!(g.cell_at(1,1).is_none());
    assert_eq!(g.cell_at(0,1).unwrap().outer.x,10);
}
#[test]
fn dimension_counts_missing_and_zero_are_rejected(){
    let t=table("<w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc/></w:tr>");
    assert!(matches!(compute_declared_grid_geometry(&t,&[10],(0,0),LTR,Z),Err(Error::Invalid("missing declared column width"))));
    for (w,h) in [(vec![],vec![10]),(vec![10],vec![]),(vec![0],vec![10]),(vec![10],vec![0])]{
        assert!(compute_table_geometry(&t,&w,&h,(0,0),LTR,Z).is_err());
    }
    assert!(compute_table_geometry(&Table::default(),&[],&[],(0,0),LTR,Z).is_err());
}
#[test]
fn overflow_and_overlarge_margins_fail_but_zero_content_allowed(){
    let t=table("<w:tr><w:tc/><w:tc/></w:tr>");
    assert!(matches!(compute_table_geometry(&t,&[u32::MAX,1],&[10],(0,0),LTR,Z),Err(Error::Limit("table geometry arithmetic"))));
    assert!(compute_table_geometry(&t,&[10,10],&[10],(u32::MAX,0),LTR,Z).is_err());
    let i=CellInsets{left:6,right:5,top:0,bottom:0};
    assert!(matches!(compute_table_geometry(&t,&[10,10],&[10],(0,0),LTR,i),Err(Error::Invalid("cell margins exceed geometry"))));
    let g=compute_table_geometry(&t,&[10,10],&[10],(0,0),LTR,CellInsets{left:5,right:5,top:5,bottom:5}).unwrap();
    assert_eq!(g.cell_at(0,0).unwrap().content.width,0);
    assert_eq!(g.cell_at(0,0).unwrap().content.height,0);
}
#[test]
fn nonfixed_margins_and_invalid_merge_fail_explicitly(){
    let t=table("<w:tr><w:tc><w:tcPr><w:tcMar><w:top w:type='auto'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    assert!(matches!(compute_table_geometry(&t,&[10],&[10],(0,0),LTR,Z),Err(Error::Unsupported("nonfixed cell margin"))));
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc></w:tr>");
    assert!(compute_table_geometry(&t,&[10],&[10],(0,0),LTR,Z).is_err());
}
