use rwcore::{parse_table, resolve_table_grid, resolve_effective_cell, Block, Error, Limits, Table, TableDirection, TableEdge, TableWidth};
const W:&str="http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn table(body:&str)->Result<Table,Error>{
    parse_table(format!("<w:tbl xmlns:w='{W}'>{body}</w:tbl>").as_bytes(),Limits::default())
}
fn nested(t:&Table,index:usize)->&Table{
    let Block::Table(n)=&t.rows[0].cells[0].blocks[index] else {panic!("nested table expected")};
    n
}
#[test]
fn mixed_cell_blocks_preserve_order_and_text(){
    let t=table("<w:tr><w:tc><w:p><w:r><w:t>Before</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>科研</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>After</w:t></w:r></w:p></w:tc></w:tr>").unwrap();
    let blocks=&t.rows[0].cells[0].blocks;
    assert_eq!(blocks.len(),3);
    let Block::Paragraph(p)=&blocks[0] else {panic!("first paragraph")};
    assert_eq!(p.text(),"Before");
    let Block::Paragraph(p)=&nested(&t,1).rows[0].cells[0].blocks[0] else {panic!("inner paragraph")};
    assert_eq!(p.text(),"科研");
    let Block::Paragraph(p)=&blocks[2] else {panic!("last paragraph")};
    assert_eq!(p.text(),"After");
    assert!(t.diagnostics.is_empty());
}
#[test]
fn independent_nested_grids_and_margins(){
    let t=table("<w:tblPr><w:tblCellMar><w:top w:w='100'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tbl><w:tblPr><w:tblCellMar><w:top w:w='200'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc></w:tr></w:tbl></w:tc></w:tr>").unwrap();
    let n=nested(&t,0);
    let outer=resolve_table_grid(&t).unwrap();
    let inner=resolve_table_grid(n).unwrap();
    assert_eq!(outer.columns,1);
    assert_eq!(inner.columns,2);
    assert_eq!(inner.anchor_at(1,1).unwrap().row_span,2);
    for (table,g,v) in [(&t,&outer,100),(n,&inner,200)] {
        let e=resolve_effective_cell(&table.appearance,&g.cell_at(0,0).unwrap().source.appearance,TableDirection::LeftToRight).unwrap();
        assert_eq!(e.margins[&TableEdge::Top].value,TableWidth::Twips(v));
    }
}
#[test]
fn nesting_depth_boundary(){
    fn body(levels:usize)->String{
        let mut s="<w:tr><w:tc><w:p/></w:tc></w:tr>".to_owned();
        for _ in 1..levels {s=format!("<w:tr><w:tc><w:tbl>{s}</w:tbl></w:tc></w:tr>");}
        s
    }
    let t=table(&body(16)).unwrap();
    let mut current=&t;
    for _ in 1..16 {current=nested(current,0);}
    assert_eq!(current.rows.len(),1);
    assert_eq!(table(&body(17)),Err(Error::Limit("table nesting depth")));
}
#[test]
fn total_table_and_row_budgets(){
    assert_eq!(table(&format!("<w:tr><w:tc>{}</w:tc></w:tr>","<w:tbl/>".repeat(4096))),Err(Error::Limit("total tables")));
    let rows="<w:tr/>".repeat(16384);
    assert_eq!(table(&format!("<w:tr><w:tc><w:tbl>{rows}</w:tbl><w:tbl>{rows}</w:tbl></w:tc></w:tr>")),Err(Error::Limit("total table rows")));
}
#[test]
fn nested_diagnostics_and_malformed_metadata(){
    let t=table("<w:tr><w:tc><w:tbl><w:tblPr><w:tblLook/></w:tblPr></w:tbl><w:p/></w:tc></w:tr>").unwrap();
    assert_eq!(nested(&t,0).diagnostics.len(),1);
    assert_eq!(t.diagnostics,nested(&t,0).diagnostics);
    assert_eq!(table("<w:tr><w:tc><w:tbl><w:tr><w:tc><w:tcPr><w:gridSpan w:val='0'/></w:tcPr></w:tc></w:tr></w:tbl></w:tc></w:tr>"),Err(Error::Invalid("zero table grid span")));
}
#[test]
fn strict_namespace_and_opaque_foreign_nested_table(){
    let xml="<w:tbl xmlns:w='http://purl.oclc.org/ooxml/wordprocessingml/main' xmlns:x='urn:foreign'><w:tr><w:tc><x:tbl/><w:tbl/></w:tc></w:tr></w:tbl>";
    let t=parse_table(xml.as_bytes(),Limits::default()).unwrap();
    assert!(matches!(&t.rows[0].cells[0].blocks[0],Block::Unsupported{name} if name=="tbl"));
    assert!(nested(&t,1).rows.is_empty());
    assert_eq!(t.diagnostics.len(),1);
}
