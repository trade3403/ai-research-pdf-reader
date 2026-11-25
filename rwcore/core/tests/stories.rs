use rwcore::{read_docx, read_docx_with_stories, parse_story, parse_page_settings, Block, Limits, Error};
use rwcore::section::HeaderFooterKind as K;
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const H: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
const F: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
fn fixture(body: &str, rels: &str, header: &str, header_mime: &str) -> Vec<u8> {
    let main = format!("<w:document xmlns:w='{W}' xmlns:r='{R}'><w:body>{body}</w:body></w:document>");
    let ct = format!("<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Override PartName='/custom/main.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/><Override PartName='/custom/h.xml' ContentType='{header_mime}'/><Override PartName='/custom/f.xml' ContentType='{F}'/><Override PartName='/custom/settings.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml'/></Types>");
    let root = format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='main' Type='{R}/officeDocument' Target='custom/main.xml'/></Relationships>");
    let rel = format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'>{rels}</Relationships>");
    let footer = format!("<w:ftr xmlns:w='{W}'><w:p><w:r><w:t>Footer</w:t></w:r></w:p></w:ftr>");
    let settings = format!("<w:settings xmlns:w='{W}'><w:evenAndOddHeaders/></w:settings>");
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    for (name,data) in [("[Content_Types].xml",ct.as_str()),("_rels/.rels",root.as_str()),("custom/main.xml",main.as_str()),("custom/_rels/main.xml.rels",rel.as_str()),("custom/h.xml",header),("custom/f.xml",footer.as_str()),("custom/settings.xml",settings.as_str())] {
        z.start_file(name,SimpleFileOptions::default()).unwrap();
        z.write_all(data.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}
fn header() -> String { format!("<w:hdr xmlns:w='{W}'><w:p><w:pPr><w:jc w:val='center'/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>科研</w:t><w:tab/><w:t>SCI</w:t><w:br/><w:t>Journal</w:t></w:r></w:p><w:tbl/></w:hdr>") }
fn rels() -> String { format!("<Relationship Id='h1' Type='{R}/header' Target='h.xml'/><Relationship Id='h2' Type='{R}/header' Target='h.xml'/><Relationship Id='f' Type='{R}/footer' Target='f.xml'/><Relationship Id='s' Type='{R}/settings' Target='settings.xml'/>") }
const BODY: &str = "<w:p><w:pPr><w:sectPr><w:headerReference w:type='default' r:id='h1'/></w:sectPr></w:pPr></w:p><w:p/><w:sectPr><w:headerReference w:type='even' r:id='h2'/><w:footerReference w:type='default' r:id='f'/></w:sectPr>";

#[test]
fn loads_referenced_stories_once_and_keeps_body_unchanged() {
    let bytes=fixture(BODY,&rels(),&header(),H);
    let base=read_docx(&bytes,Limits::default()).unwrap();
    assert!(base.header_footer.is_none());
    let d=read_docx_with_stories(&bytes,Limits::default()).unwrap();
    assert_eq!(d.blocks,base.blocks);
    assert_eq!(d.sections,base.sections);
    let c=d.header_footer.unwrap();
    assert_eq!(c.parts.len(),2);
    assert_eq!(c.headers["h1"],c.headers["h2"]);
    let h=&c.parts["custom/h.xml"];
    let Block::Paragraph(p)=&h.blocks[0] else {panic!("paragraph expected")};
    assert_eq!(p.text(),"科研\tSCI\nJournal");
    assert_eq!(p.runs[0].properties.bold,Some(true));
    assert!(p.properties.alignment.is_some());
    assert!(matches!(&h.blocks[1],Block::Unsupported{name} if name=="tbl"));
    assert_eq!(h.diagnostics.len(),1);
    assert_eq!(c.settings.unwrap().even_and_odd_headers,Some(true));
}
#[test]
fn missing_external_wrong_mime_and_wrong_root_are_rejected() {
    let body="<w:sectPr><w:headerReference w:type='default' r:id='h1'/></w:sectPr>";
    let bytes=fixture(body,"",&header(),H);
    assert_eq!(read_docx_with_stories(&bytes,Limits::default()),Err(Error::Invalid("missing header/footer relationship")));
    let external=format!("<Relationship Id='h1' Type='{R}/header' Target='https://example.org/h.xml' TargetMode='External'/>");
    assert_eq!(read_docx_with_stories(&fixture(body,&external,&header(),H),Limits::default()),Err(Error::Unsupported("external header/footer reference")));
    assert_eq!(read_docx_with_stories(&fixture(body,&rels(),&header(),"application/xml"),Limits::default()),Err(Error::Invalid("header/footer content type")));
    assert_eq!(read_docx_with_stories(&fixture(body,&rels(),&header().replace("w:hdr","w:ftr"),H),Limits::default()),Err(Error::Invalid("story/settings root")));
}
#[test]
fn unreferenced_story_not_parsed() {
    let d=read_docx_with_stories(&fixture("<w:p/>",&rels(),"<broken",H),Limits::default()).unwrap();
    assert!(d.header_footer.unwrap().parts.is_empty());
}
#[test]
fn story_strict_namespace_and_limits() {
    let s=header().replace(W,"http://purl.oclc.org/ooxml/wordprocessingml/main");
    assert!(parse_story(s.as_bytes(),K::Header,Limits::default()).is_ok());
    assert_eq!(parse_story(s.as_bytes(),K::Footer,Limits::default()),Err(Error::Invalid("story/settings root")));
    assert_eq!(parse_story(s.as_bytes(),K::Header,Limits{max_part_bytes:1,..Limits::default()}),Err(Error::Limit("part bytes")));
    assert!(parse_story(s.as_bytes(),K::Header,Limits{max_xml_nodes:1,..Limits::default()}).is_err());
}
#[test]
fn settings_preserve_absent_false_and_reject_duplicates() {
    for (body,expected) in [("",None),("<w:evenAndOddHeaders/>",Some(true)),("<w:evenAndOddHeaders w:val='0'/>",Some(false))] {
        let s=format!("<w:settings xmlns:w='{W}'>{body}</w:settings>");
        assert_eq!(parse_page_settings(s.as_bytes(),Limits::default()).unwrap().even_and_odd_headers,expected);
    }
    let s=format!("<w:settings xmlns:w='{W}'><w:evenAndOddHeaders/><w:evenAndOddHeaders/></w:settings>");
    assert_eq!(parse_page_settings(s.as_bytes(),Limits::default()),Err(Error::Invalid("duplicate evenAndOddHeaders")));
}
