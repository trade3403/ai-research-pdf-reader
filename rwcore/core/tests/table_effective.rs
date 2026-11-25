use rwcore::{parse_table, resolve_table_grid, resolve_effective_cell, resolve_preferred_width,
    Appearance, CellVerticalAlign, Error, Limits, PreferredWidth, PropertySource, Sourced,
    TableColor, TableDirection, TableEdge, TableLayout, TableWidth};
fn table(body: &str) -> rwcore::Table {
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
#[test]
fn margin_inheritance_zero_override_and_sources() {
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='100'/><w:top w:w='120'/><w:bottom w:w='80'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:tcMar><w:top w:w='0'/><w:right w:w='40'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    let c=&t.rows[0].cells[0].appearance;
    let e=resolve_effective_cell(&t.appearance,c,TableDirection::LeftToRight).unwrap();
    assert_eq!(e.margins[&TableEdge::Top],Sourced{value:TableWidth::Twips(0),source:PropertySource::Cell});
    assert_eq!(e.margins[&TableEdge::Left].source,PropertySource::Table);
    assert_eq!(e.margins[&TableEdge::Right].value,TableWidth::Twips(40));
    assert_eq!(t.appearance.margins.as_ref().unwrap()[&TableEdge::Top],TableWidth::Twips(120));
}
#[test]
fn direction_maps_before_cross_layer_override() {
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='10'/><w:right w:w='20'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:tcMar><w:start w:w='30'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    let c=&t.rows[0].cells[0].appearance;
    for (direction,edge,other) in [(TableDirection::LeftToRight,TableEdge::Left,TableEdge::Right),(TableDirection::RightToLeft,TableEdge::Right,TableEdge::Left)] {
        let e=resolve_effective_cell(&t.appearance,c,direction).unwrap();
        assert_eq!(e.margins[&edge].value,TableWidth::Twips(30));
        assert_eq!(e.margins[&other].source,PropertySource::Table);
        assert!(!e.margins.contains_key(&TableEdge::Start));
    }
}
#[test]
fn ambiguous_aliases_fail_and_identical_aliases_work() {
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='10'/><w:start w:w='20'/></w:tblCellMar></w:tblPr>");
    let c=Appearance::default();
    assert_eq!(resolve_effective_cell(&t.appearance,&c,TableDirection::LeftToRight),Err(Error::Invalid("conflicting margin aliases")));
    assert!(resolve_effective_cell(&t.appearance,&c,TableDirection::RightToLeft).is_ok());
    let t=table("<w:tblPr><w:tblCellMar><w:left w:w='10'/><w:start w:w='10'/></w:tblCellMar></w:tblPr>");
    assert_eq!(resolve_effective_cell(&t.appearance,&c,TableDirection::LeftToRight).unwrap().margins.len(),1);
}
#[test]
fn shading_is_whole_override_and_width_not_inherited() {
    let t=table("<w:tblPr><w:shd w:val='clear' w:fill='ABCDEF'/><w:tblW w:w='5000'/><w:tblLayout w:type='fixed'/></w:tblPr><w:tr><w:tc/><w:tc><w:tcPr><w:shd w:val='nil'/><w:tcW w:w='2500' w:type='pct'/><w:vAlign w:val='bottom'/></w:tcPr></w:tc></w:tr>");
    let a=resolve_effective_cell(&t.appearance,&t.rows[0].cells[0].appearance,TableDirection::LeftToRight).unwrap();
    assert_eq!(a.shading.unwrap().value.fill,Some(TableColor::Rgb(0xabcdef)));
    assert!(a.width.is_none());
    assert_eq!(a.layout.unwrap().value,TableLayout::Fixed);
    let b=resolve_effective_cell(&t.appearance,&t.rows[0].cells[1].appearance,TableDirection::LeftToRight).unwrap();
    assert_eq!(b.shading.unwrap().source,PropertySource::Cell);
    assert_eq!(b.shading.unwrap().value.pattern.as_deref(),Some("nil"));
    assert!(b.shading.unwrap().value.fill.is_none());
    assert_eq!(b.width.unwrap().value,TableWidth::Percent50(2500));
    assert_eq!(b.vertical_align.unwrap().value,CellVerticalAlign::Bottom);
}
#[test]
fn width_conversion_rounding_and_overflow() {
    for (value,base,expected) in [
        (TableWidth::Percent50(2500),9001,PreferredWidth::Twips(4501)),
        (TableWidth::Percent50(7500),100,PreferredWidth::Twips(150)),
        (TableWidth::Percent50(0),999,PreferredWidth::Twips(0)),
        (TableWidth::Percent50(5000),u32::MAX,PreferredWidth::Twips(u32::MAX)),
        (TableWidth::Twips(100),0,PreferredWidth::Twips(100)),
        (TableWidth::Auto,100,PreferredWidth::Auto),
        (TableWidth::Nil,100,PreferredWidth::Nil)] {
        assert_eq!(resolve_preferred_width(value,base),Ok(expected));
    }
    assert_eq!(resolve_preferred_width(TableWidth::Percent50(u32::MAX),u32::MAX),Err(Error::Limit("preferred width overflow")));
}
#[test]
fn empty_margin_container_inherits_and_missing_remains_unresolved() {
    let t=table("<w:tblPr><w:tblCellMar><w:top w:w='100'/></w:tblCellMar></w:tblPr><w:tr><w:tc><w:tcPr><w:tcMar/></w:tcPr></w:tc></w:tr>");
    let e=resolve_effective_cell(&t.appearance,&t.rows[0].cells[0].appearance,TableDirection::LeftToRight).unwrap();
    assert_eq!(e.margins.len(),1);
    assert_eq!(e.margins[&TableEdge::Top].source,PropertySource::Table);
    assert!(e.shading.is_none());
    assert!(e.vertical_align.is_none());
}
#[test]
fn merged_sources_keep_individual_effective_appearance() {
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/><w:tcMar><w:top w:w='20'/></w:tcMar></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/><w:tcMar><w:top w:w='30'/></w:tcMar></w:tcPr></w:tc></w:tr>");
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.anchor_at(1,0).unwrap().row_span,2);
    for (row,v) in [(0,20),(1,30)] {
        let e=resolve_effective_cell(&t.appearance,&g.cell_at(row,0).unwrap().source.appearance,TableDirection::LeftToRight).unwrap();
        assert_eq!(e.margins[&TableEdge::Top].value,TableWidth::Twips(v));
    }
}
