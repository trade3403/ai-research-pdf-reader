use rwcore::{parse_table, resolve_table_grid, Limits, Error, Block, Table, TableRow, TableCell, CellAddress};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn table(body: &str) -> Table {
    parse_table(format!("<w:tbl xmlns:w='{W}'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
#[test]
fn horizontal_and_omitted_slots() {
    let t=table("<w:tblGrid><w:gridCol/><w:gridCol/><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:trPr><w:gridBefore w:val='1'/></w:trPr><w:tc><w:tcPr><w:gridSpan w:val='2'/></w:tcPr><w:p/></w:tc></w:tr>");
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.columns,4);
    assert!(g.cell_at(0,0).is_none());
    assert!(g.cell_at(0,3).is_none());
    assert_eq!(g.cell_at(0,1),g.cell_at(0,2));
    assert_eq!(g.anchor_at(0,2).unwrap().column_span,2);
    assert!(g.cell_at(1,0).is_none());
}
#[test]
fn rectangular_merge_preserves_continuation_content() {
    let t=table("<w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/></w:tcPr><w:p><w:r><w:t>top</w:t></w:r></w:p></w:tc><w:tc/></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr><w:p><w:r><w:t>保留</w:t></w:r></w:p></w:tc><w:tc/></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr>");
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.columns,3);
    let a=g.anchor_at(2,1).unwrap();
    assert_eq!(a.address,CellAddress{row:0,cell:0});
    assert_eq!((a.row_span,a.column_span),(3,2));
    assert_eq!(g.anchors.len(),4);
    let c=g.cell_at(1,1).unwrap();
    assert_eq!(c.address,CellAddress{row:1,cell:0});
    let Block::Paragraph(p)=&c.source.blocks[0] else {panic!("paragraph expected")};
    assert_eq!(p.text(),"保留");
    assert!(std::ptr::eq(c.source,&t.rows[1].cells[0]));
}
#[test]
fn restart_replaces_previous_anchor_and_parallel_merges_are_independent() {
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr>");
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.anchor_at(0,0).unwrap().row_span,1);
    assert_eq!(g.anchor_at(2,0).unwrap().address.row,1);
    assert_eq!(g.anchor_at(2,0).unwrap().row_span,2);
    assert_eq!(g.anchor_at(1,1).unwrap().address.row,0);
    assert_eq!(g.anchor_at(1,1).unwrap().row_span,2);
    assert_eq!(g.anchor_at(2,1).unwrap().row_span,1);
}
#[test]
fn orphan_changed_span_and_interrupted_merges_rejected() {
    let orphan="<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc></w:tr>";
    let restart="<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/></w:tcPr></w:tc></w:tr>";
    let wide="<w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc></w:tr>";
    for body in [orphan.to_owned(),format!("{restart}{wide}"),format!("{restart}<w:tr><w:tc/></w:tr>{orphan}"),format!("{restart}<w:tr><w:trPr><w:gridBefore w:val='1'/></w:trPr><w:tc><w:tcPr><w:vMerge/></w:tcPr></w:tc></w:tr>")] {
        let t=table(&body);
        assert_eq!(resolve_table_grid(&t),Err(Error::Invalid("unmatched vertical merge continuation")));
    }
}
#[test]
fn declared_grid_overflow_rejected_and_missing_grid_inferred() {
    let t=table("<w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc/><w:tc/></w:tr>");
    assert_eq!(resolve_table_grid(&t),Err(Error::Invalid("row exceeds declared table grid")));
    let t=table("<w:tr><w:trPr><w:gridBefore w:val='2'/><w:gridAfter w:val='3'/></w:trPr><w:tc/></w:tr>");
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.columns,6);
    assert!(g.cell_at(0,5).is_none());
}
#[test]
fn public_model_bounds_rechecked_and_sparse_empty_rows() {
    let mut t=Table{rows:vec![TableRow{cells:vec![TableCell{grid_span:Some(0),..Default::default()}],..Default::default()}],..Default::default()};
    assert_eq!(resolve_table_grid(&t),Err(Error::Invalid("zero table grid span")));
    t.rows[0].cells[0].grid_span=Some(257);
    assert_eq!(resolve_table_grid(&t),Err(Error::Limit("table columns")));
    t.rows=vec![TableRow::default();16385];
    assert_eq!(resolve_table_grid(&t),Err(Error::Limit("table rows")));
    t.rows.truncate(16384);
    t.grid=vec![None;256];
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.rows.len(),16384);
    assert!(g.anchors.is_empty());
    assert!(g.cell_at(16383,255).is_none());
}
