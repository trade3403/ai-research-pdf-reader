use rwcore::{parse_table, Limits, Table, Paragraph, CellAddress, Error, Rect, CellInsets,
    TableDirection, TableMeasurer, NestedLayoutOptions, NestedBlockLayout, NestedTableLayout,
    layout_nested_table};
fn table(body:&str)->Table{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
struct Measure;
impl TableMeasurer for Measure{
    fn cell_widths(&mut self,_:&Table,_:CellAddress)->Result<(u32,u32),Error>{Ok((0,0))}
    fn paragraph_height(&mut self,p:&Paragraph,w:u32)->Result<u32,Error>{
        if p.runs.is_empty(){Ok(0)}else if w>=50{Ok(20)}else{Ok(40)}
    }
}
const O:NestedLayoutOptions=NestedLayoutOptions{direction:TableDirection::LeftToRight,
    default_insets:CellInsets{left:0,right:0,top:0,bottom:0},min_column_width:1,min_auto_height:1};
fn child<'a,'b>(p:&'b NestedTableLayout<'a>,i:usize)->&'b NestedTableLayout<'a>{
    let NestedBlockLayout::Table(t)=&p.cells[0].blocks[i] else {panic!("child expected")};
    t
}
#[test]
fn nested_width_height_feedback_and_absolute_positions(){
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='10'/><w:right w:w='10'/><w:top w:w='5'/><w:bottom w:w='5'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p><w:tbl><w:tblPr><w:tblW w:type='pct' w:w='2500'/></w:tblPr><w:tr><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>C</w:t></w:r></w:p></w:tc></w:tr>");
    let p=layout_nested_table(&t,100,(100,200),O,&mut Measure).unwrap();
    assert_eq!(p.bounds,Rect{x:100,y:200,width:100,height:90});
    let c=child(&p,1);
    assert_eq!(c.bounds,Rect{x:110,y:225,width:40,height:40});
    assert_eq!(c.rows.geometry.bounds(),Rect{x:0,y:0,width:40,height:40});
    let NestedBlockLayout::Paragraph{bounds,..}=&p.cells[0].blocks[2] else {panic!("paragraph")};
    assert_eq!(bounds.y,265);
}
#[test]
fn nested_vertical_alignment_and_three_levels(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='100' w:hRule='exact'/></w:trPr><w:tc><w:tcPr><w:vAlign w:val='bottom'/></w:tcPr><w:tbl><w:tr><w:tc><w:tbl><w:tr><w:tc><w:p><w:r><w:t>X</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc></w:tr></w:tbl></w:tc></w:tr>");
    let p=layout_nested_table(&t,100,(7,9),O,&mut Measure).unwrap();
    assert_eq!(child(&p,0).bounds.y,89);
    assert_eq!(child(child(&p,0),0).bounds,Rect{x:7,y:89,width:100,height:20});
}
#[test]
fn exact_row_reports_unclipped_child_overflow(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='5' w:hRule='exact'/></w:trPr><w:tc><w:tbl><w:tr><w:tc><w:p><w:r><w:t>X</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc></w:tr>");
    let p=layout_nested_table(&t,100,(0,0),O,&mut Measure).unwrap();
    assert_eq!(p.bounds.height,5);
    assert_eq!(p.rows.placements[0].overflow_twips,15);
    assert_eq!(child(&p,0).bounds.height,20);
}
#[test]
fn child_budget_is_actual_parent_content_width(){
    let t=table("<w:tr><w:tc><w:tbl><w:tblPr><w:tblW w:w='101'/></w:tblPr><w:tr><w:tc/></w:tr></w:tbl></w:tc></w:tr>");
    assert!(matches!(layout_nested_table(&t,100,(0,0),O,&mut Measure),Err(Error::Invalid("table width outside budget"))));
}
#[test]
fn merged_anchor_children_and_continuation_policy(){
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr><w:tbl><w:tr><w:tc><w:p><w:r><w:t>X</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr>");
    let p=layout_nested_table(&t,100,(0,0),O,&mut Measure).unwrap();
    assert_eq!(p.cells.len(),1);
    assert_eq!(p.bounds.height,20);
    assert_eq!(p.rows.geometry.cell_at(1,0).unwrap().outer.height,20);
    let mut bad=t;
    bad.rows[1].cells[0].blocks.push(rwcore::Block::Table(Box::new(Table::default())));
    assert!(matches!(layout_nested_table(&bad,100,(0,0),O,&mut Measure),Err(Error::Unsupported("nonempty merge continuation layout"))));
}
#[test]
fn public_models_are_bounded_and_unknown_blocks_rejected(){
    let mut t=table("<w:tr><w:tc/></w:tr>");
    for _ in 1..17{
        let mut parent=table("<w:tr><w:tc/></w:tr>");
        parent.rows[0].cells[0].blocks.push(rwcore::Block::Table(Box::new(t)));
        t=parent;
    }
    assert!(matches!(layout_nested_table(&t,100,(0,0),O,&mut Measure),Err(Error::Limit("nested layout depth"))));
    let t=table("<w:tr><w:tc><w:sdt/></w:tc></w:tr>");
    assert!(matches!(layout_nested_table(&t,100,(0,0),O,&mut Measure),Err(Error::Unsupported("nested layout block"))));
}
#[test]
fn absolute_coordinate_overflow_is_checked(){
    let t=table("<w:tr><w:tc><w:p><w:r><w:t>X</w:t></w:r></w:p></w:tc></w:tr>");
    assert!(matches!(layout_nested_table(&t,100,(u32::MAX,0),O,&mut Measure),Err(Error::Limit("nested table coordinates"))));
}
