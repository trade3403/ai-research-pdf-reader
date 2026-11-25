use rwcore::{parse_table, Block, Limits, Error, VerticalMerge};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn table(body: &str) -> Result<rwcore::Table,Error> {
    parse_table(format!("<w:tbl xmlns:w='{W}'>{body}</w:tbl>").as_bytes(),Limits::default())
}

#[test]
fn basic_grid_rows_cells_and_unicode() {
    let t=table("<w:tblGrid><w:gridCol w:w='2000'/><w:gridCol w:w='3000'/></w:tblGrid><w:tr><w:tc><w:p><w:pPr><w:jc w:val='center'/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>实验</w:t><w:tab/><w:t>SCI</w:t><w:br/><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p/></w:tc></w:tr>").unwrap();
    assert_eq!(t.grid,vec![Some(2000),Some(3000)]);
    assert_eq!(t.rows.len(),1);
    assert_eq!(t.rows[0].cells.len(),2);
    let Block::Paragraph(p)=&t.rows[0].cells[0].blocks[0] else {panic!("paragraph expected")};
    assert_eq!(p.text(),"实验\tSCI\n1");
    assert_eq!(p.runs[0].properties.bold,Some(true));
    assert!(p.properties.alignment.is_some());
    assert!(t.diagnostics.is_empty());
}
#[test]
fn raw_merge_and_grid_offsets_preserved() {
    let t=table("<w:tblGrid><w:gridCol/><w:gridCol/><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:trPr><w:gridBefore w:val='1'/><w:gridAfter w:val='1'/></w:trPr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/></w:tcPr><w:p/></w:tc></w:tr><w:tr><w:trPr><w:gridBefore w:val='1'/><w:gridAfter w:val='1'/></w:trPr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr>").unwrap();
    assert_eq!(t.rows[0].grid_before,Some(1));
    assert_eq!(t.rows[0].grid_after,Some(1));
    assert_eq!(t.rows[0].cells[0].grid_span,Some(2));
    assert_eq!(t.rows[0].cells[0].vertical_merge,Some(VerticalMerge::Restart));
    assert_eq!(t.rows[1].cells[0].vertical_merge,Some(VerticalMerge::Continue));
    assert!(t.diagnostics.is_empty());
}
#[test]
fn opaque_nested_table_and_ragged_row_diagnostics() {
    let t=table("<w:tblPr><w:tblLook/></w:tblPr><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:tc><w:tbl/><w:p/></w:tc></w:tr>").unwrap();
    assert!(matches!(&t.rows[0].cells[0].blocks[0],Block::Unsupported{name} if name=="tbl"));
    assert_eq!(t.diagnostics.len(),3);
    assert_eq!(t.grid,vec![None,None]);
}
#[test]
fn strict_namespace_and_foreign_elements() {
    let s=format!("<w:tbl xmlns:w='{W}' xmlns:x='urn:foreign'><x:tr/><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>").replace(W,"http://purl.oclc.org/ooxml/wordprocessingml/main");
    let t=parse_table(s.as_bytes(),Limits::default()).unwrap();
    assert_eq!(t.rows.len(),1);
    assert_eq!(t.diagnostics.len(),1);
    assert!(parse_table(b"<tbl/>",Limits::default()).is_err());
}
#[test]
fn malformed_and_duplicate_metadata_rejected() {
    for body in ["<w:tblGrid/><w:tblGrid/>","<w:tblGrid><w:gridCol w:w='-1'/></w:tblGrid>",
        "<w:tr><w:tc><w:tcPr><w:gridSpan w:val='0'/></w:tcPr></w:tc></w:tr>",
        "<w:tr><w:tc><w:tcPr><w:vMerge w:val='bad'/></w:tcPr></w:tc></w:tr>",
        "<w:tr><w:trPr/><w:trPr/></w:tr>"] {
        assert!(table(body).is_err(),"{body}");
    }
}
#[test]
fn table_limits_and_grid_occupancy() {
    let cols="<w:gridCol/>".repeat(257);
    assert_eq!(table(&format!("<w:tblGrid>{cols}</w:tblGrid>")),Err(Error::Limit("table columns")));
    assert_eq!(table("<w:tr><w:tc><w:tcPr><w:gridSpan w:val='256'/></w:tcPr></w:tc><w:tc/></w:tr>"),Err(Error::Limit("table columns")));
    let rows="<w:tr/>".repeat(16385);
    assert_eq!(table(&rows),Err(Error::Limit("table rows")));
    let s=format!("<w:tbl xmlns:w='{W}'><w:tr/></w:tbl>");
    assert_eq!(parse_table(s.as_bytes(),Limits{max_part_bytes:1,..Limits::default()}),Err(Error::Limit("part bytes")));
    assert!(parse_table(s.as_bytes(),Limits{max_xml_nodes:1,..Limits::default()}).is_err());
}
