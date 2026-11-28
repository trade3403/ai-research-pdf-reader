use rwcore::{parse_table, Limits, Table, CellAddress, CellWidthMeasurement, CellInsets,
    TableDirection, TableLayout, Error, allocate_table_columns, layout_table_rows, CellMeasurement};
fn table(body:&str)->Table{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
fn m(row:usize,cell:usize,minimum:u32,preferred:u32)->CellWidthMeasurement{
    CellWidthMeasurement{address:CellAddress{row,cell},minimum,preferred}
}
fn alloc(t:&Table,w:u32,ms:&[CellWidthMeasurement])->Result<rwcore::TableColumnLayout,Error>{
    allocate_table_columns(t,w,ms,TableDirection::LeftToRight,CellInsets::default(),1)
}
#[test]
fn equal_allocation_preserves_integer_remainder(){
    let t=table("<w:tr><w:tc/><w:tc/><w:tc/></w:tr>");
    let a=alloc(&t,10,&[m(0,0,0,0),m(0,1,0,0),m(0,2,0,0)]).unwrap();
    assert_eq!(a.column_widths,vec![4,3,3]);
    assert_eq!(a.width,10);
}
#[test]
fn percentage_table_and_cell_preferences(){
    let t=table("<w:tblPr><w:tblW w:type='pct' w:w='2500'/><w:tblLayout w:type='fixed'/></w:tblPr><w:tr><w:tc><w:tcPr><w:tcW w:type='pct' w:w='3000'/></w:tcPr></w:tc><w:tc><w:tcPr><w:tcW w:type='dxa' w:w='200'/></w:tcPr></w:tc></w:tr>");
    let a=alloc(&t,1000,&[m(0,0,0,0),m(0,1,0,0)]).unwrap();
    assert_eq!(a.layout,TableLayout::Fixed);
    assert_eq!(a.column_widths,vec![300,200]);
}
#[test]
fn fixed_mode_reports_overflow_autofit_honors_minimum(){
    let t=table("<w:tblPr><w:tblLayout w:type='fixed'/></w:tblPr><w:tblGrid><w:gridCol w:w='50'/><w:gridCol w:w='50'/></w:tblGrid><w:tr><w:tc/><w:tc/></w:tr>");
    let ms=[m(0,0,80,90),m(0,1,10,10)];
    let a=alloc(&t,100,&ms).unwrap();
    assert_eq!(a.column_widths,vec![50,50]);
    assert_eq!(a.cells[0].minimum_overflow,30);
    let mut t=t;
    t.appearance.layout=Some(TableLayout::Autofit);
    let a=alloc(&t,100,&ms).unwrap();
    assert!(a.column_widths[0]>=80);
    assert!(a.cells.iter().all(|c|c.minimum_overflow==0));
}
#[test]
fn overlapping_span_minima_share_width(){
    let t=table("<w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/></w:tcPr></w:tc><w:tc/></w:tr><w:tr><w:tc/><w:tc><w:tcPr><w:gridSpan w:val='2'/></w:tcPr></w:tc></w:tr>");
    let a=alloc(&t,102,&[m(0,0,100,100),m(0,1,1,1),m(1,0,1,1),m(1,1,100,100)]).unwrap();
    assert_eq!(a.column_widths.iter().sum::<u32>(),102);
    assert!(a.cells.iter().all(|c|c.minimum_overflow==0));
    assert!(alloc(&t,100,&[m(0,0,100,100),m(0,1,1,1),m(1,0,1,1),m(1,1,100,100)]).is_err());
}
#[test]
fn margins_and_row_geometry_integration(){
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='10'/><w:right w:w='20'/></w:tblCellMar></w:tblPr><w:tr><w:tc/></w:tr>");
    let a=alloc(&t,100,&[m(0,0,50,70)]).unwrap();
    assert_eq!(a.cells[0].content_width,70);
    let p=layout_table_rows(&t,&a.column_widths,&[CellMeasurement{address:CellAddress{row:0,cell:0},height:40}],(0,0),TableDirection::LeftToRight,CellInsets::default(),1).unwrap();
    assert_eq!(p.geometry.cells()[0].content.width,70);
}
#[test]
fn surplus_weights_and_soft_preference_shortfalls(){
    let t=table("<w:tblGrid><w:gridCol w:w='20'/><w:gridCol w:w='60'/></w:tblGrid><w:tr><w:tc/><w:tc/></w:tr>");
    let a=alloc(&t,100,&[m(0,0,0,0),m(0,1,0,0)]).unwrap();
    assert_eq!(a.column_widths,vec![25,75]);
    let a=alloc(&t,40,&[m(0,0,0,20),m(0,1,0,60)]).unwrap();
    assert_eq!(a.column_widths.iter().sum::<u32>(),40);
    assert!(a.cells.iter().all(|c|c.preferred_shortfall>0));
}
#[test]
fn validates_budget_and_measurement_contract(){
    let t=table("<w:tblPr><w:tblW w:w='101'/></w:tblPr><w:tr><w:tc/></w:tr>");
    assert_eq!(alloc(&t,100,&[m(0,0,1,1)]),Err(Error::Invalid("table width outside budget")));
    let t=table("<w:tr><w:tc/></w:tr>");
    assert!(alloc(&t,100,&[]).is_err());
    assert!(alloc(&t,100,&[m(0,1,1,1)]).is_err());
    assert!(alloc(&t,100,&[m(0,0,2,1)]).is_err());
    assert!(alloc(&t,0,&[m(0,0,0,0)]).is_err());
}
#[test]
fn extreme_widths_are_checked_and_fixed_padding_is_hard(){
    let t=table("<w:tr><w:tc/><w:tc/></w:tr>");
    assert!(alloc(&t,u32::MAX,&[m(0,0,u32::MAX,u32::MAX),m(0,1,1,1)]).is_err());
    let t=table("<w:tblPr><w:tblLayout w:type='fixed'/></w:tblPr><w:tr><w:tc><w:tcPr><w:tcMar><w:left w:w='60'/><w:right w:w='60'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    assert_eq!(alloc(&t,100,&[m(0,0,0,0)]),Err(Error::Invalid("minimum table width exceeds budget")));
}
