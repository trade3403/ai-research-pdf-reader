use rwcore::{parse_table, Limits, Table, TableBorderPlan, TableDirection, Error};
fn table(body:&str)->Table {
    parse_table(format!("<w:tbl xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:tbl>").as_bytes(),Limits::default()).unwrap()
}
const FRAME:&str="<w:tblPr><w:tblBorders><w:top w:val='single' w:sz='12'/><w:bottom w:val='single' w:sz='12'/><w:insideH w:val='nil'/><w:insideV w:val='nil'/></w:tblBorders></w:tblPr>";
const HEADER:&str="<w:tr><w:tc><w:tcPr><w:tcBorders><w:bottom w:val='single' w:sz='4'/></w:tcBorders></w:tcPr></w:tc><w:tc><w:tcPr><w:tcBorders><w:bottom w:val='single' w:sz='4'/></w:tcBorders></w:tcPr></w:tc></w:tr>";
const BODY:&str="<w:tr><w:tc/><w:tc/></w:tr>";
#[test]
fn recognizes_three_complete_lines_with_multiple_body_rows(){
    let t=table(&format!("{FRAME}{HEADER}{BODY}{BODY}"));
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    let lines=p.three_line(1).unwrap().unwrap();
    assert_eq!(lines.header_boundary,1);
    assert_eq!(lines.top.size_eighth_points,Some(12));
    assert_eq!(lines.middle.size_eighth_points,Some(4));
    assert_eq!(lines.bottom.size_eighth_points,Some(12));
    assert!(p.horizontal(2,0).unwrap().is_none());
    assert!(p.vertical(1,1).unwrap().is_none());
}
#[test]
fn explicit_cell_none_overrides_table_line(){
    let t=table("<w:tblPr><w:tblBorders><w:top w:val='single'/></w:tblBorders></w:tblPr><w:tr><w:tc><w:tcPr><w:tcBorders><w:top w:val='none'/></w:tcBorders></w:tcPr></w:tc><w:tc/></w:tr>");
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    assert!(p.horizontal(0,0).unwrap().is_none());
    assert!(p.horizontal(0,1).unwrap().is_some());
}
#[test]
fn suppresses_borders_inside_rectangular_merge(){
    let t=table("<w:tblPr><w:tblBorders><w:insideH w:val='single'/><w:insideV w:val='single'/></w:tblBorders></w:tblPr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge w:val='restart'/><w:tcBorders><w:bottom w:val='single'/></w:tcBorders></w:tcPr></w:tc><w:tc/></w:tr><w:tr><w:tc><w:tcPr><w:gridSpan w:val='2'/><w:vMerge/></w:tcPr></w:tc><w:tc/></w:tr>");
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    assert!(p.horizontal(1,0).unwrap().is_none());
    assert!(p.horizontal(1,1).unwrap().is_none());
    assert!(p.vertical(0,1).unwrap().is_none());
    assert!(p.vertical(0,2).unwrap().is_some());
    assert!(p.horizontal(1,2).unwrap().is_some());
}
#[test]
fn conflicts_and_direction_are_explicit(){
    let t=table("<w:tr><w:tc><w:tcPr><w:tcBorders><w:end w:val='single'/></w:tcBorders></w:tcPr></w:tc><w:tc><w:tcPr><w:tcBorders><w:left w:val='double'/></w:tcBorders></w:tcPr></w:tc></w:tr>");
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    assert_eq!(p.vertical(0,1),Err(Error::Unsupported("conflicting adjacent cell borders")));
    let p=TableBorderPlan::new(&t,TableDirection::RightToLeft).unwrap();
    assert_eq!(p.vertical(0,0).unwrap().unwrap().style,"single");
    assert_eq!(p.vertical(0,1).unwrap().unwrap().style,"double");
}
#[test]
fn rejects_non_three_line_shapes(){
    let incomplete=HEADER.replacen("<w:bottom w:val='single' w:sz='4'/>","",1);
    for body in [
        format!("{FRAME}{incomplete}{BODY}"),
        format!("{}{HEADER}{BODY}",FRAME.replace("insideV w:val='nil'","insideV w:val='single'")),
        format!("{FRAME}{HEADER}<w:tr><w:tc/></w:tr>"),
        format!("{FRAME}{HEADER}{}",BODY.replace("<w:tc/>","<w:tc><w:tcPr><w:tcBorders><w:bottom w:val='nil'/></w:tcBorders></w:tcPr></w:tc>"))
    ] {
        let t=table(&body);
        let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
        assert!(p.three_line(1).unwrap().is_none());
    }
}
#[test]
fn bounds_and_unsupported_topology(){
    let t=table(&format!("{FRAME}{HEADER}{BODY}"));
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    assert!(p.horizontal(3,0).is_err());
    assert!(p.vertical(2,0).is_err());
    assert!(p.three_line(0).is_err());
    let t=table("<w:tr><w:tc><w:tcPr><w:tcBorders><w:tl2br w:val='single'/></w:tcBorders></w:tcPr></w:tc></w:tr>");
    assert!(matches!(TableBorderPlan::new(&t,TableDirection::LeftToRight),Err(Error::Unsupported("nonperimeter border topology"))));
    let t=Table{grid:vec![None;256],rows:vec![Default::default();4000],..Default::default()};
    let p=TableBorderPlan::new(&t,TableDirection::LeftToRight).unwrap();
    assert_eq!(p.three_line(1),Err(Error::Limit("three-line grid scan")));
}
