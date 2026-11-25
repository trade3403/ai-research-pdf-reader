use rwcore::{read_docx, Block, Error, Limits};
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

const CT: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const REL: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
const WORD: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t xml:space="preserve">中文 &amp; SCI </w:t><w:tab/><w:t>论文</w:t><w:br/><w:t>second</w:t></w:r></w:p><w:tbl/><w:sectPr/></w:body></w:document>"#;

fn package(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for &(name, text) in entries {
        zip.start_file(name, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn fixture(xml: &str, rel: &str) -> Vec<u8> {
    package(&[("[Content_Types].xml", CT), ("_rels/.rels", rel), ("word/document.xml", xml)])
}

#[test]
fn paragraph_text_and_diagnostics() {
    let document = read_docx(&fixture(WORD, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &document.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.text(), "中文 & SCI \t论文\nsecond");
    assert_eq!(document.blocks.len(), 2);
    assert!(matches!(&document.blocks[1], Block::Unsupported { name } if name == "tbl"));
    assert_eq!(document.diagnostics.len(), 1);
    assert_eq!(document.sections.len(), 1);
    assert_eq!(document.sections[0].blocks, 0..2);
    assert_eq!(document.sections[0].end, rwcore::SectionEnd::Body);
    assert_eq!(document.sections[0].properties, Some(Default::default()));
}

#[test]
fn rejects_invalid_zip() {
    assert!(matches!(read_docx(b"invalid", Limits::default()), Err(Error::Zip(_))));
}
#[test]
fn rejects_missing_main_part() {
    let bytes = package(&[("[Content_Types].xml", CT), ("_rels/.rels", REL)]);
    assert_eq!(read_docx(&bytes, Limits::default()), Err(Error::MissingPart("word/document.xml")));
}
#[test]
fn rejects_malformed_xml() {
    assert!(matches!(read_docx(&fixture("<broken", REL), Limits::default()), Err(Error::Xml(_))));
}
#[test]
fn rejects_external_relationship() {
    let rel = REL.replace("Target=", "TargetMode=\"External\" Target=");
    assert!(matches!(read_docx(&fixture(WORD, &rel), Limits::default()), Err(Error::Unsupported(_))));
}
#[test]
fn rejects_dtd() {
    let xml = format!("<!DOCTYPE w:document [<!ENTITY foo 'x'>]>{WORD}");
    assert!(matches!(read_docx(&fixture(&xml, REL), Limits::default()), Err(Error::Xml(_))));
}
#[test]
fn archive_size_limit() {
    let limits = Limits { max_archive_bytes: 1, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("archive bytes")));
}
#[test]
fn part_size_limit() {
    let limits = Limits { max_part_bytes: 10, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("part bytes")));
}
#[test]
fn entry_count_limit() {
    let limits = Limits { max_entries: 2, ..Limits::default() };
    assert_eq!(read_docx(&fixture(WORD, REL), limits), Err(Error::Limit("archive entries")));
}
#[test]
fn node_limit() {
    let limits = Limits { max_xml_nodes: 1, ..Limits::default() };
    assert!(matches!(read_docx(&fixture(WORD, REL), limits), Err(Error::Xml(_))));
}

#[test]
fn reads_direct_formatting_in_docx_runs() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:rPr><w:b/><w:i w:val="0"/><w:sz w:val="24"/><w:color w:val="12ABFF"/><w:rFonts w:ascii="Arial" w:eastAsia="宋体"/></w:rPr><w:t>中文</w:t></w:r><w:r><w:t>plain</w:t></w:r></w:p></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &doc.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.text(), "中文plain");
    let f = &p.runs[0].properties;
    assert_eq!(f.bold, Some(true));
    assert_eq!(f.italic, Some(false));
    assert_eq!(f.size_half_points, Some(24));
    assert_eq!(f.color_rgb, Some([18,171,255]));
    assert_eq!(f.east_asia_font.as_deref(), Some("宋体"));
    assert_eq!(p.runs[1].properties, Default::default());
    assert!(doc.diagnostics.is_empty());
}
#[test]
fn duplicate_run_container_and_invalid_property_are_reported() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:rPr><w:b w:val="bad"/></w:rPr><w:rPr><w:b/></w:rPr><w:t>x</w:t></w:r></w:p></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &doc.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.runs[0].properties.bold, None);
    assert_eq!(doc.diagnostics.len(), 2);
}


#[test]
fn reads_direct_paragraph_properties() {
    use rwcore::opc::{ParagraphAlignment, LineRule};
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/><w:jc w:val="center"/><w:spacing w:before="0" w:after="120" w:line="360" w:lineRule="auto"/><w:ind w:left="-120" w:right="240" w:firstLine="480"/><w:keepNext/><w:keepLines w:val="0"/><w:pageBreakBefore/></w:pPr><w:r><w:t>标题</w:t></w:r></w:p><w:p/></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &doc.blocks[0] else { panic!("paragraph expected") };
    let f = &p.properties;
    assert_eq!(p.text(), "标题");
    assert_eq!(f.style_id.as_deref(), Some("Heading1"));
    assert_eq!(f.alignment, Some(ParagraphAlignment::Center));
    assert_eq!((f.before_twips, f.after_twips, f.line), (Some(0), Some(120), Some(360)));
    assert_eq!(f.line_rule, Some(LineRule::Auto));
    assert_eq!((f.left_twips, f.right_twips, f.first_line_twips), (Some(-120), Some(240), Some(480)));
    assert_eq!((f.keep_next, f.keep_lines, f.page_break_before), (Some(true), Some(false), Some(true)));
    let Block::Paragraph(empty) = &doc.blocks[1] else { panic!("paragraph expected") };
    assert_eq!(empty.properties, Default::default());
    assert!(doc.diagnostics.is_empty());
}
#[test]
fn paragraph_properties_report_invalid_and_duplicate_values() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:jc w:val="bad"/><w:jc w:val="left"/><w:spacing w:before="-1" w:lineRule="bad"/><w:ind w:firstLine="120" w:hanging="240"/><w:keepNext w:val="bad"/><w:numPr/></w:pPr><w:pPr/></w:p></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &doc.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.properties.alignment, None);
    assert_eq!(p.properties.before_twips, None);
    assert_eq!(p.properties.keep_next, None);
    assert_eq!(doc.diagnostics.len(), 8);
}


#[test]
fn reads_numbered_paragraphs_and_explicit_cancellation() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:numPr><w:ilvl w:val="8"/><w:numId w:val="42"/></w:numPr></w:pPr><w:r><w:t>Methods</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:numId w:val="0"/></w:numPr></w:pPr></w:p><w:p/></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(a) = &doc.blocks[0] else { panic!("paragraph expected") };
    let Block::Paragraph(b) = &doc.blocks[1] else { panic!("paragraph expected") };
    let Block::Paragraph(c) = &doc.blocks[2] else { panic!("paragraph expected") };
    assert_eq!((a.properties.num_id, a.properties.num_level), (Some(42), Some(8)));
    assert_eq!((b.properties.num_id, b.properties.num_level), (Some(0), None));
    assert_eq!((c.properties.num_id, c.properties.num_level), (None, None));
    assert!(doc.diagnostics.is_empty());
}

#[test]
fn malformed_numbering_is_diagnosed_without_inventing_defaults() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:numPr><w:ilvl w:val="9"/><w:numId w:val="-1"/><w:numId w:val="5"/><w:ins/></w:numPr></w:pPr></w:p></w:body></w:document>"#;
    let doc = read_docx(&fixture(xml, REL), Limits::default()).unwrap();
    let Block::Paragraph(p) = &doc.blocks[0] else { panic!("paragraph expected") };
    assert_eq!((p.properties.num_id, p.properties.num_level), (None, None));
    assert_eq!(doc.diagnostics.len(), 4);
}

#[test]
fn numbering_references_survive_style_cascade_and_cancellation() {
    use rwcore::{resolve_formatting, Paragraph, Run};
    use rwcore::opc::parse_styles;
    let styles = br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="List" w:default="1"><w:pPr><w:numPr><w:numId w:val="12"/><w:ilvl w:val="2"/></w:numPr></w:pPr></w:style></w:styles>"#;
    let catalog = parse_styles(styles, Limits::default()).unwrap();
    let mut p = Paragraph::default();
    let r = Run::default();
    let props = resolve_formatting(&catalog, &p, &r).unwrap().paragraph;
    assert_eq!((props.num_id, props.num_level), (Some(12), Some(2)));
    p.properties.num_id = Some(0);
    let props = resolve_formatting(&catalog, &p, &r).unwrap().paragraph;
    assert_eq!(props.num_id, Some(0));
    p.properties.num_id = Some(24);
    p.properties.num_level = Some(0);
    let props = resolve_formatting(&catalog, &p, &r).unwrap().paragraph;
    assert_eq!((props.num_id, props.num_level), (Some(24), Some(0)));
}


fn section_doc(body: &str) -> Result<rwcore::Document, Error> {
    let xml = format!("<w:document xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'><w:body>{body}</w:body></w:document>");
    read_docx(&fixture(&xml, REL), Limits::default())
}

#[test]
fn sections_terminate_own_content_ranges() {
    use rwcore::{SectionEnd, section::{Orientation, SectionBreak}};
    let d = section_doc("<w:p><w:r><w:t>Introduction</w:t></w:r></w:p><w:p><w:pPr><w:sectPr><w:pgSz w:w='11906' w:h='16838'/><w:cols w:num='2'/></w:sectPr></w:pPr><w:r><w:t>Methods</w:t></w:r></w:p><w:tbl/><w:p><w:r><w:t>Results</w:t></w:r></w:p><w:sectPr><w:type w:val='continuous'/><w:pgSz w:w='15840' w:h='12240' w:orient='landscape'/></w:sectPr>").unwrap();
    assert_eq!(d.blocks.len(), 4);
    assert_eq!(d.sections.len(), 2);
    assert_eq!(d.sections[0].blocks, 0..2);
    assert_eq!(d.sections[0].end, SectionEnd::Paragraph);
    assert_eq!(d.sections[0].properties.as_ref().unwrap().columns.as_ref().unwrap().count, Some(2));
    assert_eq!(d.sections[1].blocks, 2..4);
    assert_eq!(d.sections[1].end, SectionEnd::Body);
    let props = d.sections[1].properties.as_ref().unwrap();
    assert_eq!(props.orientation, Some(Orientation::Landscape));
    assert_eq!(props.break_type, Some(SectionBreak::Continuous));
    let Block::Paragraph(p) = &d.blocks[1] else { panic!("paragraph expected") };
    assert_eq!(p.text(),"Methods");
    assert!(p.properties.diagnostics.is_empty());
    assert_eq!(d.diagnostics.len(),1);
}

#[test]
fn implicit_final_and_empty_sections_preserve_boundaries() {
    use rwcore::SectionEnd;
    let empty = section_doc("").unwrap();
    assert_eq!(empty.sections[0].blocks, 0..0);
    assert_eq!(empty.sections[0].end, SectionEnd::Implicit);
    assert!(empty.sections[0].properties.is_none());
    let d = section_doc("<w:p><w:pPr><w:sectPr/></w:pPr></w:p>").unwrap();
    assert_eq!(d.sections.len(),2);
    assert_eq!(d.sections[0].blocks,0..1);
    assert_eq!(d.sections[1].blocks,1..1);
    assert_eq!(d.sections[1].end,SectionEnd::Implicit);
    let d = section_doc("<w:sectPr/>").unwrap();
    assert_eq!(d.sections[0].blocks,0..0);
    assert_eq!(d.sections[0].end,SectionEnd::Body);
}

#[test]
fn rejects_ambiguous_and_invalid_section_boundaries() {
    assert_eq!(section_doc("<w:sectPr/><w:p/>"),Err(Error::Invalid("content after final section properties")));
    assert_eq!(section_doc("<w:sectPr/><w:sectPr/>"),Err(Error::Invalid("content after final section properties")));
    assert_eq!(section_doc("<w:p><w:pPr><w:sectPr/><w:sectPr/></w:pPr></w:p>"),Err(Error::Invalid("duplicate paragraph section properties")));
    assert_eq!(section_doc("<w:p><w:pPr><w:sectPr><w:pgSz w:w='0'/></w:sectPr></w:pPr></w:p>"),Err(Error::Invalid("zero page or column dimension")));
}

#[test]
fn section_warnings_propagate_without_changing_text() {
    let d = section_doc("<w:p><w:pPr><w:sectPr><w:headerReference/></w:sectPr></w:pPr><w:r><w:t>科研</w:t></w:r></w:p><w:sectPr><w:pgNumType/></w:sectPr>").unwrap();
    assert_eq!(d.diagnostics.len(),2);
    assert_eq!(d.sections[0].properties.as_ref().unwrap().diagnostics.len(),1);
    assert_eq!(d.sections[1].properties.as_ref().unwrap().diagnostics.len(),1);
    let Block::Paragraph(p) = &d.blocks[0] else { panic!("paragraph expected") };
    assert_eq!(p.text(),"科研");
    assert!(p.properties.diagnostics.is_empty());
}
