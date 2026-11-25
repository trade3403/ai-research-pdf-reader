use rwcore::{parse_story, Error, Limits, FieldKind, FieldEncoding};
use rwcore::section::HeaderFooterKind;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn story(body: &str) -> Result<rwcore::Story, Error> {
    parse_story(format!("<w:ftr xmlns:w='{W}'>{body}</w:ftr>").as_bytes(),HeaderFooterKind::Footer,Limits::default())
}

#[test]
fn simple_page_fields_keep_instruction_and_cache() {
    let s=story(r#"<w:p><w:fldSimple w:instr=" PAGE \* ROMAN "><w:r><w:t>III</w:t></w:r></w:fldSimple><w:fldSimple w:instr="NUMPAGES"><w:r><w:t>12</w:t></w:r></w:fldSimple><w:fldSimple w:instr="sectionpages"><w:r><w:t>5</w:t></w:r></w:fldSimple></w:p>"#).unwrap();
    assert_eq!(s.fields.iter().map(|f| f.kind).collect::<Vec<_>>(),vec![FieldKind::Page,FieldKind::NumPages,FieldKind::SectionPages]);
    assert_eq!(s.fields[0].encoding,FieldEncoding::Simple);
    assert_eq!(s.fields[0].instruction,r" PAGE \* ROMAN ");
    assert_eq!(s.fields[0].cached_text.as_deref(),Some("III"));
    assert_eq!(s.fields[2].cached_text.as_deref(),Some("5"));
}

#[test]
fn complex_split_instruction_and_cached_runs() {
    let s=story("<w:p><w:r><w:t>Page </w:t><w:fldChar w:fldCharType='begin'/></w:r><w:r><w:instrText> NU</w:instrText></w:r><w:r><w:instrText>MPAGES </w:instrText><w:fldChar w:fldCharType='separate'/><w:t>1</w:t></w:r><w:r><w:t>2</w:t><w:fldChar w:fldCharType='end'/><w:t> end</w:t></w:r></w:p>").unwrap();
    let f=&s.fields[0];
    assert_eq!(f.kind,FieldKind::NumPages);
    assert_eq!(f.encoding,FieldEncoding::Complex);
    assert_eq!(f.instruction," NUMPAGES ");
    assert_eq!(f.cached_text.as_deref(),Some("12"));
}

#[test]
fn source_order_and_absent_cache() {
    let s=story("<w:tbl/><w:p><w:r><w:fldChar w:fldCharType='begin'/><w:instrText>PAGE</w:instrText><w:fldChar w:fldCharType='end'/></w:r><w:fldSimple w:instr='DATE'/></w:p><w:p><w:fldSimple w:instr='SECTIONPAGES'/></w:p>").unwrap();
    assert_eq!(s.fields.iter().map(|f| f.block_index).collect::<Vec<_>>(),vec![1,1,2]);
    assert_eq!(s.fields[0].cached_text,None);
    assert_eq!(s.fields[1].kind,FieldKind::Other);
    assert_eq!(s.fields[1].cached_text.as_deref(),Some(""));
}

#[test]
fn malformed_and_nested_fields_rejected() {
    for body in ["<w:r><w:fldChar w:fldCharType='end'/></w:r>",
        "<w:r><w:instrText>PAGE</w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType='begin'/></w:r>",
        "<w:r><w:fldChar w:fldCharType='begin'/><w:fldChar w:fldCharType='begin'/></w:r>",
        "<w:fldSimple w:instr='PAGE'><w:fldSimple w:instr='NUMPAGES'/></w:fldSimple>",
        "<w:fldSimple/>",
        "<w:r><w:fldChar w:fldCharType='begin'/><w:fldChar w:fldCharType='separate'/><w:fldChar w:fldCharType='separate'/></w:r>"] {
        assert!(story(&format!("<w:p>{body}</w:p>")).is_err(),"{body}");
    }
}

#[test]
fn field_limits_bound_instruction_cache_and_count() {
    let instr="a".repeat(16385);
    assert_eq!(story(&format!("<w:p><w:fldSimple w:instr='{instr}'/></w:p>")),Err(Error::Limit("story field text")));
    let cache="a".repeat(65537);
    assert_eq!(story(&format!("<w:p><w:fldSimple w:instr='PAGE'><w:r><w:t>{cache}</w:t></w:r></w:fldSimple></w:p>")),Err(Error::Limit("story field text")));
    let many="<w:fldSimple w:instr='PAGE'/>".repeat(4097);
    assert_eq!(story(&format!("<w:p>{many}</w:p>")),Err(Error::Limit("story field count")));
}
