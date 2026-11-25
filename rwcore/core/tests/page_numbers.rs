use rwcore::{parse_section, DocumentSection, SectionEnd, FieldKind, FieldEncoding, StoryField, Limits, Error, PageNumberPlan, evaluate_page_field};
fn section(body: &str) -> DocumentSection {
    let xml=format!("<w:sectPr xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'>{body}</w:sectPr>");
    DocumentSection { blocks:0..0, properties:Some(parse_section(xml.as_bytes(),Limits::default()).unwrap()), end:SectionEnd::Body }
}
fn field(kind: FieldKind, instruction: &str) -> StoryField {
    StoryField { block_index:0, encoding:FieldEncoding::Simple, kind, instruction:instruction.into(), cached_text:Some("old".into()) }
}

#[test]
fn section_restart_and_continuation_keep_physical_counts() {
    let sections=[section("<w:pgNumType w:fmt='lowerRoman'/>"),section("<w:pgNumType w:start='1'/>"),section("")];
    let p=PageNumberPlan::new(&sections,&[3,10,2]).unwrap();
    assert_eq!(p.total_pages(),15);
    let a=p.context(0,2).unwrap();
    assert_eq!((a.physical_page,a.displayed_page,a.section_pages),(3,3,3));
    assert_eq!(evaluate_page_field(&field(FieldKind::Page,"PAGE"),a).unwrap(),"iii");
    let b=p.context(1,0).unwrap();
    assert_eq!((b.physical_page,b.displayed_page),(4,1));
    let c=p.context(2,1).unwrap();
    assert_eq!((c.physical_page,c.displayed_page),(15,12));
}
#[test]
fn dynamic_count_fields_and_cache_unchanged() {
    let p=PageNumberPlan::new(&[section("<w:pgNumType w:start='7' w:fmt='upperRoman'/>"),section("")],&[4,8]).unwrap();
    let c=p.context(0,1).unwrap();
    for (kind,instr,expected) in [(FieldKind::Page,"PAGE","VIII"),(FieldKind::NumPages,"NUMPAGES","12"),(FieldKind::SectionPages,"SECTIONPAGES","4")] {
        let f=field(kind,instr);
        assert_eq!(evaluate_page_field(&f,c).unwrap(),expected);
        assert_eq!(f.cached_text.as_deref(),Some("old"));
    }
}
#[test]
fn numeric_switch_overrides_and_letter_format() {
    let p=PageNumberPlan::new(&[section("<w:pgNumType w:start='27'/>")],&[1]).unwrap();
    let c=p.context(0,0).unwrap();
    for (instr,expected) in [(r"PAGE \* ROMAN","XXVII"),(r"PAGE \* roman","xxvii"),(r"PAGE \* ALPHABETIC","AA"),(r"PAGE \* alphabetic","aa"),(r"page \* Arabic \* MERGEFORMAT","27")] {
        assert_eq!(evaluate_page_field(&field(FieldKind::Page,instr),c).unwrap(),expected);
    }
}
#[test]
fn unsupported_switches_commands_and_chapter_numbering() {
    let p=PageNumberPlan::new(&[section("")],&[1]).unwrap();
    let c=p.context(0,0).unwrap();
    for instr in [r"PAGE \# 000",r"PAGE \*",r"PAGE \* ROMAN \* Arabic","DATE","NUMPAGES"] {
        assert!(evaluate_page_field(&field(FieldKind::Page,instr),c).is_err());
    }
    let p=PageNumberPlan::new(&[section("<w:pgNumType w:chapStyle='1'/>")],&[1]).unwrap();
    assert_eq!(evaluate_page_field(&field(FieldKind::Page,"PAGE"),p.context(0,0).unwrap()),Err(Error::Unsupported("chapter page numbering")));
}
#[test]
fn count_and_index_boundaries() {
    let sections=[section("")];
    assert!(PageNumberPlan::new(&sections,&[]).is_err());
    assert!(PageNumberPlan::new(&sections,&[0]).is_err());
    assert!(PageNumberPlan::new(&[],&[]).is_err());
    let p=PageNumberPlan::new(&sections,&[1]).unwrap();
    assert!(p.context(1,0).is_err());
    assert!(p.context(0,1).is_err());
}
#[test]
fn checked_overflow_and_large_plan_without_page_allocation() {
    let p=PageNumberPlan::new(&[section("")],&[u32::MAX]).unwrap();
    assert_eq!(p.context(0,u32::MAX-1).unwrap().physical_page,u32::MAX);
    assert_eq!(PageNumberPlan::new(&[section(""),section("")],&[u32::MAX,1]),Err(Error::Limit("page number overflow")));
    assert_eq!(PageNumberPlan::new(&[section(""),section("<w:pgNumType w:start='1'/>")],&[u32::MAX,1]),Err(Error::Limit("document page count overflow")));
    assert_eq!(PageNumberPlan::new(&[section("<w:pgNumType w:start='2'/>")],&[u32::MAX]),Err(Error::Limit("page number overflow")));
}
#[test]
fn explicit_zero_and_negative_starts() {
    let p=PageNumberPlan::new(&[section("<w:pgNumType w:start='0'/>")],&[2]).unwrap();
    let c=p.context(0,0).unwrap();
    assert_eq!(evaluate_page_field(&field(FieldKind::Page,"PAGE"),c).unwrap(),"0");
    assert!(evaluate_page_field(&field(FieldKind::Page,r"PAGE \* ROMAN"),c).is_err());
    assert_eq!(PageNumberPlan::new(&[section("<w:pgNumType w:start='-1'/>")],&[1]),Err(Error::Unsupported("negative page number start")));
}
