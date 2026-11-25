use rwcore::{parse_table, resolve_table_grid, Limits, TableWidth, TableEdge, TableColor};
fn table(body:&str)->Result<rwcore::Table,rwcore::Error>{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default())
}
#[test]
fn width_units_preserved(){
    let t=table("<w:tblPr><w:tblW w:w='5000' w:type='pct'/></w:tblPr><w:tr><w:tc><w:tcPr><w:tcW w:w='2000'/></w:tcPr></w:tc><w:tc><w:tcPr><w:tcW w:type='auto'/></w:tcPr></w:tc><w:tc><w:tcPr><w:tcW w:type='nil'/></w:tcPr></w:tc></w:tr>").unwrap();
    assert_eq!(t.appearance.width,Some(TableWidth::Percent50(5000)));
    assert_eq!(t.rows[0].cells[0].appearance.width,Some(TableWidth::Twips(2000)));
    assert_eq!(t.rows[0].cells[1].appearance.width,Some(TableWidth::Auto));
    assert_eq!(t.rows[0].cells[2].appearance.width,Some(TableWidth::Nil));
}
#[test]
fn border_units_and_none(){
    let t=table("<w:tblPr><w:tblBorders><w:top w:val='single' w:sz='12' w:space='2' w:color='123ABC'/><w:insideV w:val='nil'/></w:tblBorders></w:tblPr><w:tr><w:tc><w:tcPr><w:tcBorders><w:bottom w:val='none' w:color='auto'/></w:tcBorders></w:tcPr></w:tc></w:tr>").unwrap();
    let b=t.appearance.borders.as_ref().unwrap();
    assert_eq!(b[&TableEdge::Top].size_eighth_points,Some(12));
    assert_eq!(b[&TableEdge::Top].space_points,Some(2));
    assert_eq!(b[&TableEdge::Top].color,Some(TableColor::Rgb(0x123abc)));
    assert_eq!(b[&TableEdge::InsideV].style,"nil");
    assert_eq!(t.rows[0].cells[0].appearance.borders.as_ref().unwrap()[&TableEdge::Bottom].style,"none");
}
#[test]
fn margin_zero_absent_and_merge_independent(){
    let t=table("<w:tblPr><w:tblCellMar><w:top w:w='120'/><w:start w:w='200'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/><w:tcMar><w:top w:w='0'/></w:tcMar></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc></w:tr>").unwrap();
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.anchor_at(1,1).unwrap().row_span,2);
    assert_eq!(t.appearance.margins.as_ref().unwrap()[&TableEdge::Top],TableWidth::Twips(120));
    assert_eq!(g.cell_at(0,1).unwrap().source.appearance.margins.as_ref().unwrap()[&TableEdge::Top],TableWidth::Twips(0));
    assert!(g.cell_at(1,1).unwrap().source.appearance.margins.is_none());
}
#[test]
fn duplicate_and_malformed_rejected(){
    for b in ["<w:tblPr><w:tblW w:w='-1'/></w:tblPr>","<w:tblPr><w:tblW w:w='10'/><w:tblW w:w='20'/></w:tblPr>","<w:tblPr><w:tblBorders><w:top w:val='single'/><w:top w:val='nil'/></w:tblBorders></w:tblPr>","<w:tblPr><w:tblBorders><w:top w:val='single' w:color='ZZ0000'/></w:tblBorders></w:tblPr>","<w:tblPr><w:tblCellMar><w:left w:w='1'/><w:left w:w='2'/></w:tblCellMar></w:tblPr>"]{assert!(table(b).is_err(),"{b}");}
}
#[test]
fn theme_diagnostics_and_empty_map(){
    let t=table("<w:tblPr><w:tblBorders><w:top w:val='single' w:themeColor='accent1'/><w:unknown/></w:tblBorders><w:tblCellMar/></w:tblPr>").unwrap();
    assert_eq!(t.diagnostics.len(),2);
    assert!(t.appearance.margins.as_ref().unwrap().is_empty());
    assert!(table("").unwrap().appearance.margins.is_none());
}
