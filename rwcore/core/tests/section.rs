use rwcore::{parse_section, Limits, Error};
use rwcore::section::{Orientation, SectionBreak};
const NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn section(body: &str) -> String { format!("<w:sectPr xmlns:w='{NS}'>{body}</w:sectPr>") }

#[test]
fn paper_dimensions_and_margins() {
    let s = parse_section(section("<w:pgSz w:w='11906' w:h='16838' w:orient='portrait'/><w:pgMar w:top='-100' w:bottom='1440' w:left='1440' w:right='1440' w:header='720' w:footer='720' w:gutter='0'/>").as_bytes(), Limits::default()).unwrap();
    assert_eq!((s.width_twips,s.height_twips),(Some(11906),Some(16838)));
    assert_eq!(s.orientation, Some(Orientation::Portrait));
    assert_eq!(s.margins.top,Some(-100));
    assert_eq!(s.margins.right,Some(1440));
    assert_eq!(s.margins.gutter,Some(0));
    assert!(s.diagnostics.is_empty());
}
#[test]
fn landscape_equal_columns() {
    let s = parse_section(section("<w:type w:val='continuous'/><w:pgSz w:w='15840' w:h='12240' w:orient='landscape'/><w:cols w:num='2' w:space='360' w:sep='1'/>").as_bytes(), Limits::default()).unwrap();
    assert_eq!(s.break_type,Some(SectionBreak::Continuous));
    assert_eq!(s.orientation,Some(Orientation::Landscape));
    let c = s.columns.unwrap();
    assert_eq!((c.count,c.spacing_twips,c.separator),(Some(2),Some(360),Some(true)));
    assert_eq!(c.equal_width,None);
}
#[test]
fn unequal_columns_preserve_widths() {
    let s = parse_section(section("<w:cols w:equalWidth='0' w:num='2'><w:col w:w='3200' w:space='480'/><w:col w:w='5000'/></w:cols>").as_bytes(),Limits::default()).unwrap();
    let c = s.columns.unwrap();
    assert_eq!(c.equal_width,Some(false));
    assert_eq!(c.explicit[0].width_twips,Some(3200));
    assert_eq!(c.explicit[1].width_twips,Some(5000));
    assert!(s.diagnostics.is_empty());
}
#[test]
fn all_break_types_and_missing_values() {
    for (v,b) in [("nextPage",SectionBreak::NextPage),("nextColumn",SectionBreak::NextColumn),("evenPage",SectionBreak::EvenPage),("oddPage",SectionBreak::OddPage),("continuous",SectionBreak::Continuous)] {
        let s = parse_section(section(&format!("<w:type w:val='{v}'/>")).as_bytes(),Limits::default()).unwrap();
        assert_eq!(s.break_type,Some(b));
        assert_eq!(s.width_twips,None);
    }
    assert_eq!(parse_section(section("").as_bytes(),Limits::default()).unwrap(),Default::default());
}
#[test]
fn invalid_duplicate_and_bounded_columns() {
    for body in ["<w:pgSz w:w='0'/>","<w:pgMar w:left='-1'/>","<w:pgSz/><w:pgSz/>","<w:cols w:num='65'/>","<w:cols w:sep='maybe'/>","<w:type w:val='other'/>"] {
        assert!(parse_section(section(body).as_bytes(),Limits::default()).is_err(),"{body}");
    }
    let many = format!("<w:cols>{}</w:cols>","<w:col w:w='1'/>".repeat(65));
    assert_eq!(parse_section(section(&many).as_bytes(),Limits::default()),Err(Error::Limit("section column count")));
}
#[test]
fn namespace_limits_and_unsupported_metadata() {
    let s = section("<w:pgSz w:w='100' w:code='9'/><w:headerReference/><w:pgNumType/>");
    let strict = s.replace(NS,"http://purl.oclc.org/ooxml/wordprocessingml/main");
    let p = parse_section(strict.as_bytes(),Limits::default()).unwrap();
    assert_eq!(p.width_twips,Some(100));
    assert_eq!(p.diagnostics.len(),3);
    assert_eq!(parse_section(s.as_bytes(),Limits { max_part_bytes: 1, ..Limits::default() }),Err(Error::Limit("part bytes")));
    assert!(parse_section(s.as_bytes(),Limits { max_xml_nodes: 1, ..Limits::default() }).is_err());
    assert!(parse_section(b"<sectPr/>",Limits::default()).is_err());
}
