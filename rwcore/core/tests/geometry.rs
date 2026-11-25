use rwcore::{parse_section, compute_page_geometry, GutterSide, Rect, Limits, Error, SectionProperties};
const NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn section(body: &str) -> SectionProperties {
    parse_section(format!("<w:sectPr xmlns:w='{NS}'>{body}</w:sectPr>").as_bytes(), Limits::default()).unwrap()
}
const PAGE: &str = "<w:pgSz w:w='11906' w:h='16838'/><w:pgMar w:top='1440' w:bottom='1440' w:left='1440' w:right='1440'/>";

#[test]
fn a4_content_and_single_column() {
    let g = compute_page_geometry(&section(PAGE), GutterSide::Left).unwrap();
    assert_eq!(g.page, Rect { x:0,y:0,width:11906,height:16838 });
    assert_eq!(g.content, Rect { x:1440,y:1440,width:9026,height:13958 });
    assert_eq!(g.columns, vec![g.content]);
    assert_eq!(g.trailing_width,0);
}
#[test]
fn landscape_dimensions_not_swapped() {
    let s = section("<w:pgSz w:w='15840' w:h='12240' w:orient='landscape'/><w:pgMar w:top='720' w:bottom='720' w:left='720' w:right='720'/>");
    let g = compute_page_geometry(&s,GutterSide::Left).unwrap();
    assert_eq!((g.page.width,g.page.height),(15840,12240));
    assert_eq!((g.content.width,g.content.height),(14400,10800));
}
#[test]
fn explicit_gutter_placement() {
    let s = section("<w:pgSz w:w='1000' w:h='2000'/><w:pgMar w:top='100' w:bottom='100' w:left='100' w:right='100' w:gutter='50'/>");
    for (side,x,y,w,h) in [(GutterSide::Left,150,100,750,1800),(GutterSide::Right,100,100,750,1800),(GutterSide::Top,100,150,800,1750)] {
        assert_eq!(compute_page_geometry(&s,side).unwrap().content,Rect{x,y,width:w,height:h});
    }
}
#[test]
fn equal_columns_distribute_remainder_exactly() {
    let s = section(&format!("{PAGE}<w:cols w:num='3' w:space='360' w:sep='1'/>"));
    let g = compute_page_geometry(&s,GutterSide::Left).unwrap();
    assert_eq!(g.columns.iter().map(|c| c.width).collect::<Vec<_>>(),vec![2769,2769,2768]);
    assert_eq!(g.columns.iter().map(|c| c.x).collect::<Vec<_>>(),vec![1440,4569,7698]);
    assert_eq!(g.columns.last().unwrap().x + g.columns.last().unwrap().width,g.content.x + g.content.width);
    assert!(g.column_separator);
}
#[test]
fn unequal_columns_keep_gap_and_trailing_space() {
    let s = section(&format!("{PAGE}<w:cols w:num='2' w:equalWidth='0'><w:col w:w='3000' w:space='500'/><w:col w:w='5000'/></w:cols>"));
    let g = compute_page_geometry(&s,GutterSide::Left).unwrap();
    assert_eq!((g.columns[0].x,g.columns[0].width),(1440,3000));
    assert_eq!((g.columns[1].x,g.columns[1].width),(4940,5000));
    assert_eq!(g.trailing_width,526);
}
#[test]
fn invalid_missing_and_unsupported_geometry() {
    assert_eq!(compute_page_geometry(&SectionProperties::default(),GutterSide::Left),Err(Error::Invalid("missing explicit page geometry property")));
    let mut s=section(PAGE);
    s.margins.top=Some(-100);
    assert_eq!(compute_page_geometry(&s,GutterSide::Left),Err(Error::Unsupported("negative vertical margin geometry")));
    s.margins.top=Some(1440); s.margins.left=Some(11906);
    assert_eq!(compute_page_geometry(&s,GutterSide::Left),Err(Error::Invalid("nonpositive content dimension")));
    s.margins.left=Some(u32::MAX); s.margins.gutter=Some(1);
    assert_eq!(compute_page_geometry(&s,GutterSide::Left),Err(Error::Limit("page geometry arithmetic")));
}
#[test]
fn rejects_column_overflow_and_count_mismatch() {
    for cols in ["<w:cols w:num='2' w:space='10000'/>",
        "<w:cols w:num='2' w:equalWidth='0'><w:col w:w='100'/></w:cols>",
        "<w:cols w:equalWidth='0'><w:col w:w='10000'/></w:cols>",
        "<w:cols w:num='3' w:space='4294967295'/>"] {
        assert!(compute_page_geometry(&section(&format!("{PAGE}{cols}")),GutterSide::Left).is_err());
    }
}
