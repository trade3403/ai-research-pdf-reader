use rwcore::{Block, Document, DocumentSection, SectionEnd, Paragraph, parse_section,
    parse_story, StoryCatalog, PageSettings, PageStructurePlan, GutterSide, Limits, Error};
use rwcore::section::{HeaderFooterKind, HeaderFooterVariant};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const SIZE: &str = "<w:pgSz w:w='11906' w:h='16838'/><w:pgMar w:top='1440' w:bottom='1440' w:left='1440' w:right='1440'/>";
fn section(blocks: std::ops::Range<usize>, body: &str) -> DocumentSection {
    let xml=format!("<w:sectPr xmlns:w='{W}' xmlns:r='{R}'>{SIZE}{body}</w:sectPr>");
    DocumentSection {blocks, properties:Some(parse_section(xml.as_bytes(),Limits::default()).unwrap()),end:SectionEnd::Body}
}
fn fixture() -> Document {
    let mut c=StoryCatalog::default();
    let header=format!("<w:hdr xmlns:w='{W}'><w:p><w:fldSimple w:instr='PAGE'><w:r><w:t>old</w:t></w:r></w:fldSimple><w:fldSimple w:instr='DATE'/></w:p></w:hdr>");
    let footer=format!("<w:ftr xmlns:w='{W}'><w:p><w:fldSimple w:instr='NUMPAGES'/><w:fldSimple w:instr='SECTIONPAGES'/></w:p></w:ftr>");
    c.parts.insert("h.xml".into(),parse_story(header.as_bytes(),HeaderFooterKind::Header,Limits::default()).unwrap());
    c.parts.insert("f.xml".into(),parse_story(footer.as_bytes(),HeaderFooterKind::Footer,Limits::default()).unwrap());
    c.headers.insert("h".into(),"h.xml".into());
    c.footers.insert("f".into(),"f.xml".into());
    c.settings=Some(PageSettings{even_and_odd_headers:Some(true),..Default::default()});
    Document {
        blocks: vec![Block::Paragraph(Paragraph::default()),Block::Paragraph(Paragraph::default())],
        sections: vec![
            section(0..1,"<w:headerReference w:type='default' r:id='h'/><w:footerReference w:type='default' r:id='f'/><w:pgNumType w:fmt='lowerRoman'/>"),
            section(1..2,"<w:headerReference w:type='first' r:id='h'/><w:headerReference w:type='even' r:id='h'/><w:footerReference w:type='even' r:id='f'/><w:titlePg/><w:pgNumType w:start='1'/><w:cols w:num='2'/>")],
        header_footer:Some(c),..Default::default()
    }
}

#[test]
fn integrates_geometry_numbering_selection_and_field_values() {
    let d=fixture();
    let plan=PageStructurePlan::new(&d,&[3,4],&[GutterSide::Left,GutterSide::Left]).unwrap();
    assert_eq!(plan.total_pages(),7);
    let p=plan.page(0,2).unwrap();
    assert_eq!(p.section_blocks,0..1);
    assert_eq!(p.geometry.columns.len(),1);
    assert_eq!(p.header_fields().next().unwrap().value,Ok("iii".into()));
    let p=plan.page(1,0).unwrap();
    assert_eq!(p.numbers.physical_page,4);
    assert_eq!(p.numbers.displayed_page,1);
    assert_eq!(p.stories.variant,HeaderFooterVariant::First);
    assert!(p.stories.footer.is_none());
    assert_eq!(p.geometry.columns.len(),2);
    let p=plan.page(1,2).unwrap();
    assert_eq!(p.stories.variant,HeaderFooterVariant::Even);
    assert_eq!(p.footer_fields().map(|f|f.value.unwrap()).collect::<Vec<_>>(),vec!["7","4"]);
}

#[test]
fn physical_parity_and_inherited_story_use_current_section_context() {
    let d=fixture();
    let plan=PageStructurePlan::new(&d,&[3,4],&[GutterSide::Left;2]).unwrap();
    let p=plan.page(1,1).unwrap();
    assert_eq!(p.numbers.physical_page,5);
    assert_eq!(p.numbers.displayed_page,2);
    assert_eq!(p.stories.variant,HeaderFooterVariant::Default);
    assert_eq!(p.stories.header.unwrap().source_section,0);
    let fields=p.header_fields().collect::<Vec<_>>();
    assert_eq!(fields[0].value,Ok("2".into()));
    assert_eq!(fields[0].source.cached_text.as_deref(),Some("old"));
    assert_eq!(fields[1].value,Err(Error::Unsupported("page field command")));
    assert_eq!(p.footer_fields().next().unwrap().value,Ok("7".into()));
}

#[test]
fn rejects_bad_coverage_and_missing_resources() {
    let mut d=fixture();
    assert!(PageStructurePlan::new(&d,&[1,1],&[GutterSide::Left]).is_err());
    d.sections[1].blocks=0..2;
    assert!(matches!(PageStructurePlan::new(&d,&[1,1],&[GutterSide::Left;2]),Err(Error::Invalid("invalid section block coverage"))));
    d.sections[1].blocks=1..2;
    d.header_footer=None;
    assert!(matches!(PageStructurePlan::new(&d,&[1,1],&[GutterSide::Left;2]),Err(Error::Invalid("header/footer stories not loaded"))));
}

#[test]
fn missing_geometry_and_invalid_page_requests_fail() {
    let mut d=fixture();
    d.sections[0].properties=None;
    assert!(matches!(PageStructurePlan::new(&d,&[1,1],&[GutterSide::Left;2]),Err(Error::Invalid("missing section geometry properties"))));
    let d=fixture();
    let p=PageStructurePlan::new(&d,&[1,1],&[GutterSide::Left;2]).unwrap();
    assert!(p.page(2,0).is_err());
    assert!(p.page(0,1).is_err());
}

#[test]
fn huge_page_counts_do_not_allocate_per_page() {
    let mut d=fixture();
    d.sections.truncate(1);
    d.sections[0].blocks=0..2;
    let plan=PageStructurePlan::new(&d,&[u32::MAX],&[GutterSide::Left]).unwrap();
    assert_eq!(plan.page(0,u32::MAX-1).unwrap().numbers.physical_page,u32::MAX);
}
