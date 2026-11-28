use rwcore::{parse_table, Limits, Table, CellAddress, CellMeasurement, CellInsets, TableDirection,
    CellVerticalAlign, Rect, Error, layout_table_rows, place_vertical};
fn table(body:&str)->Table{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
fn m(row:usize,cell:usize,height:u32)->CellMeasurement{CellMeasurement{address:CellAddress{row,cell},height}}
fn layout<'a>(t:&'a Table,w:&[u32],ms:&[CellMeasurement])->Result<rwcore::TableRowLayout<'a>,Error>{
    layout_table_rows(t,w,ms,(0,0),TableDirection::LeftToRight,CellInsets::default(),10)
}
#[test]
fn auto_height_uses_tallest_cell_and_insets(){
    let t=table("<w:tblPr><w:tblCellMar><w:top w:w='5'/><w:bottom w:w='7'/></w:tblCellMar></w:tblPr><w:tr><w:tc/><w:tc/></w:tr>");
    let p=layout(&t,&[100,100],&[m(0,0,30),m(0,1,50)]).unwrap();
    assert_eq!(p.row_heights,vec![62]);
    assert_eq!(p.placements[0].rect.y,5);
    assert_eq!(p.placements[1].overflow_twips,0);
}
#[test]
fn at_least_and_exact_heights_and_overflow(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='80' w:hRule='atLeast'/></w:trPr><w:tc/></w:tr><w:tr><w:trPr><w:trHeight w:val='40' w:hRule='exact'/></w:trPr><w:tc/></w:tr>");
    let p=layout(&t,&[100],&[m(0,0,20),m(1,0,60)]).unwrap();
    assert_eq!(p.row_heights,vec![80,40]);
    assert_eq!(p.placements[1].rect.height,40);
    assert_eq!(p.placements[1].overflow_twips,20);
}
#[test]
fn merged_constraint_grows_last_flexible_row(){
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc><w:tc/></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr><w:tr><w:trPr><w:trHeight w:val='20' w:hRule='exact'/></w:trPr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr>");
    let p=layout(&t,&[100,100],&[m(0,0,100),m(0,1,30),m(1,1,20),m(2,1,10)]).unwrap();
    assert_eq!(p.row_heights,vec![30,50,20]);
    assert_eq!(p.geometry.cell_at(2,0).unwrap().outer.height,100);
    assert_eq!(p.placements.len(),4);
}
#[test]
fn fixed_merge_overflow_not_resized(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='20' w:hRule='exact'/></w:trPr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc></w:tr><w:tr><w:trPr><w:trHeight w:val='30' w:hRule='exact'/></w:trPr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc></w:tr>");
    let p=layout(&t,&[100],&[m(0,0,90)]).unwrap();
    assert_eq!(p.row_heights,vec![20,30]);
    assert_eq!(p.placements[0].overflow_twips,40);
}
#[test]
fn vertical_alignment_with_odd_space_and_overflow(){
    let r=Rect{x:5,y:10,width:100,height:51};
    for (a,y) in [(CellVerticalAlign::Top,10),(CellVerticalAlign::Center,25),(CellVerticalAlign::Bottom,41)]{
        assert_eq!(place_vertical(r,20,a).unwrap(),(Rect{x:5,y,width:100,height:20},0));
        assert_eq!(place_vertical(r,60,a).unwrap(),(r,9));
    }
    assert_eq!(place_vertical(r,20,CellVerticalAlign::Both),Err(Error::Unsupported("distributed cell alignment")));
}
#[test]
fn alignment_uses_cell_content_rect(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='100' w:hRule='exact'/></w:trPr><w:tc><w:tcPr><w:vAlign w:val='bottom'/><w:tcMar><w:top w:w='10'/><w:bottom w:w='15'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    let p=layout(&t,&[100],&[m(0,0,30)]).unwrap();
    assert_eq!(p.placements[0].rect.y,55);
    assert_eq!(p.placements[0].rect.height,30);
}
#[test]
fn measurement_validation_and_positive_minimum(){
    let t=table("<w:tr><w:tc/></w:tr>");
    assert!(matches!(layout(&t,&[100],&[]),Err(Error::Invalid("cell measurement count"))));
    assert!(matches!(layout(&t,&[100],&[m(1,0,20)]),Err(Error::Invalid("cell measurement address"))));
    assert!(layout_table_rows(&t,&[100],&[m(0,0,0)],(0,0),TableDirection::LeftToRight,CellInsets::default(),0).is_err());
    assert_eq!(layout(&t,&[100],&[m(0,0,0)]).unwrap().row_heights,vec![10]);
}
#[test]
fn arithmetic_and_invalid_exact_height_fail(){
    let t=table("<w:tr><w:tc><w:tcPr><w:tcMar><w:top w:w='1'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    assert!(matches!(layout(&t,&[100],&[m(0,0,u32::MAX)]),Err(Error::Limit("table row arithmetic"))));
    let t=table("<w:tr><w:trPr><w:trHeight w:val='0' w:hRule='exact'/></w:trPr><w:tc/></w:tr>");
    assert!(layout(&t,&[100],&[m(0,0,1)]).is_err());
    assert!(place_vertical(Rect{x:0,y:u32::MAX,width:1,height:1},0,CellVerticalAlign::Top).is_err());
}
