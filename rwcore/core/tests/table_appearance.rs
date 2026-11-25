use rwcore::{parse_table, resolve_table_grid, Limits, TableColor, TableLayout, CellVerticalAlign, RowHeightRule, RowHeight};
fn table(body:&str)->Result<rwcore::Table,rwcore::Error>{
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default())
}
#[test]
fn shading_and_raw_pattern(){
    let t=table("<w:tblPr><w:shd w:val='clear' w:fill='ABCDEF' w:color='auto'/></w:tblPr><w:tr><w:tc><w:tcPr><w:shd w:val='pct20' w:fill='ffffff' w:color='123456'/></w:tcPr></w:tc></w:tr>").unwrap();
    let s=t.appearance.shading.as_ref().unwrap();
    assert_eq!(s.pattern.as_deref(),Some("clear"));
    assert_eq!(s.fill,Some(TableColor::Rgb(0xabcdef)));
    assert_eq!(s.color,Some(TableColor::Auto));
    let s=t.rows[0].cells[0].appearance.shading.as_ref().unwrap();
    assert_eq!(s.pattern.as_deref(),Some("pct20"));
    assert_eq!(s.color,Some(TableColor::Rgb(0x123456)));
}
#[test]
fn row_height_preserves_explicit_and_absent(){
    let t=table("<w:tr><w:trPr><w:trHeight w:val='500' w:hRule='exact'/></w:trPr></w:tr><w:tr><w:trPr><w:trHeight w:val='300' w:hRule='atLeast'/></w:trPr></w:tr><w:tr><w:trPr><w:trHeight w:hRule='auto'/></w:trPr></w:tr><w:tr><w:trPr><w:trHeight w:val='0'/></w:trPr></w:tr><w:tr/>").unwrap();
    assert_eq!(t.rows[0].height,Some(RowHeight{twips:Some(500),rule:Some(RowHeightRule::Exact)}));
    assert_eq!(t.rows[1].height.unwrap().rule,Some(RowHeightRule::AtLeast));
    assert_eq!(t.rows[2].height,Some(RowHeight{twips:None,rule:Some(RowHeightRule::Auto)}));
    assert_eq!(t.rows[3].height,Some(RowHeight{twips:Some(0),rule:None}));
    assert!(t.rows[4].height.is_none());
}
#[test]
fn layout_and_alignment(){
    let t=table("<w:tblPr><w:tblLayout w:type='fixed'/></w:tblPr><w:tr><w:tc><w:tcPr><w:vAlign w:val='top'/></w:tcPr></w:tc><w:tc><w:tcPr><w:vAlign w:val='center'/></w:tcPr></w:tc><w:tc><w:tcPr><w:vAlign w:val='bottom'/></w:tcPr></w:tc><w:tc><w:tcPr><w:vAlign w:val='both'/></w:tcPr></w:tc></w:tr>").unwrap();
    assert_eq!(t.appearance.layout,Some(TableLayout::Fixed));
    for (c,v) in t.rows[0].cells.iter().zip([CellVerticalAlign::Top,CellVerticalAlign::Center,CellVerticalAlign::Bottom,CellVerticalAlign::Both]){
        assert_eq!(c.appearance.vertical_align,Some(v));
    }
    assert_eq!(table("<w:tblPr><w:tblLayout w:type='autofit'/></w:tblPr>").unwrap().appearance.layout,Some(TableLayout::Autofit));
    assert!(table("").unwrap().appearance.layout.is_none());
}
#[test]
fn unsupported_theme_and_wrong_scope_diagnosed(){
    let t=table("<w:tblPr><w:shd w:fill='FFFFFF' w:themeFill='accent1'/><w:vAlign w:val='center'/></w:tblPr><w:tr><w:tc><w:tcPr><w:tblLayout w:type='fixed'/></w:tcPr></w:tc></w:tr>").unwrap();
    assert_eq!(t.diagnostics.len(),3);
    assert_eq!(t.appearance.shading.as_ref().unwrap().fill,Some(TableColor::Rgb(0xffffff)));
    assert!(t.appearance.vertical_align.is_none());
    assert!(t.rows[0].cells[0].appearance.layout.is_none());
}
#[test]
fn malformed_and_duplicate_metadata(){
    for b in ["<w:tblPr><w:shd w:fill='XX0000'/></w:tblPr>","<w:tblPr><w:shd/><w:shd/></w:tblPr>","<w:tblPr><w:tblLayout w:type='bad'/></w:tblPr>","<w:tr><w:trPr><w:trHeight w:val='-1'/></w:trPr></w:tr>","<w:tr><w:trPr><w:trHeight w:hRule='exact'/></w:trPr></w:tr>","<w:tr><w:trPr><w:trHeight/><w:trHeight/></w:trPr></w:tr>","<w:tr><w:tc><w:tcPr><w:vAlign w:val='bad'/></w:tcPr></w:tc></w:tr>"]{assert!(table(b).is_err(),"{b}");}
}
#[test]
fn appearance_and_merge_remain_independent(){
    let t=table("<w:tr><w:tc><w:tcPr><w:vMerge w:val='restart'/><w:vAlign w:val='center'/></w:tcPr></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:vMerge/><w:shd w:fill='00FF00'/></w:tcPr></w:tc></w:tr>").unwrap();
    let g=resolve_table_grid(&t).unwrap();
    assert_eq!(g.anchor_at(1,0).unwrap().row_span,2);
    assert_eq!(g.cell_at(0,0).unwrap().source.appearance.vertical_align,Some(CellVerticalAlign::Center));
    assert_eq!(g.cell_at(1,0).unwrap().source.appearance.shading.as_ref().unwrap().fill,Some(TableColor::Rgb(0xff00)));
}
