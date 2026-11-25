use rwcore::{read_docx,read_docx_with_stories,parse_table,parse_story,Block,Run,RunContent,
    Limits,Error,FormattedBlock,load_media,MediaLimits,MediaBinding};
use rwcore::section::HeaderFooterKind;
use std::io::{Cursor,Write};
use zip::{ZipWriter,write::SimpleFileOptions};
const W:&str="http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R:&str="http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const A:&str="http://schemas.openxmlformats.org/drawingml/2006/main";
const WP:&str="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const PIC:&str="http://schemas.openxmlformats.org/drawingml/2006/picture";
fn drawing(id:&str)->String{
    format!("<w:drawing><wp:inline xmlns:wp='{WP}' xmlns:a='{A}' xmlns:pic='{PIC}' xmlns:r='{R}'><wp:extent cx='63500' cy='127000'/><wp:docPr id='7' name='Figure'/><a:graphic><a:graphicData uri='{PIC}'><pic:pic><pic:nvPicPr><pic:cNvPr id='0' name='image.png'/></pic:nvPicPr><pic:blipFill><a:blip r:embed='{id}'/></pic:blipFill><pic:spPr/></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>")
}
fn package(body:&str,header:&str)->Vec<u8>{
    let ct="<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Override PartName='/custom/main.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/><Override PartName='/custom/h.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml'/><Default Extension='png' ContentType='image/png'/></Types>";
    let root=format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='main' Type='{R}/officeDocument' Target='custom/main.xml'/></Relationships>");
    let main=format!("<w:document xmlns:w='{W}' xmlns:r='{R}'><w:body>{body}</w:body></w:document>");
    let header=format!("<w:hdr xmlns:w='{W}'>{header}</w:hdr>");
    let rel=format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='img' Type='{R}/image' Target='a.png'/><Relationship Id='h' Type='{R}/header' Target='h.xml'/></Relationships>");
    let hrel=format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='img' Type='{R}/image' Target='b.png'/></Relationships>");
    let mut z=ZipWriter::new(Cursor::new(Vec::new()));
    for (n,b) in [("[Content_Types].xml",ct.as_bytes()),("_rels/.rels",root.as_bytes()),
        ("custom/main.xml",main.as_bytes()),("custom/h.xml",header.as_bytes()),
        ("custom/_rels/main.xml.rels",rel.as_bytes()),("custom/_rels/h.xml.rels",hrel.as_bytes()),
        ("custom/a.png",b"\x89PNG\r\n\x1a\nA".as_slice()),("custom/b.png",b"\x89PNG\r\n\x1a\nB".as_slice())]{
        z.start_file(n,SimpleFileOptions::default()).unwrap();z.write_all(b).unwrap();
    }
    z.finish().unwrap().into_inner()
}
fn para(block:&Block)->&rwcore::Paragraph{
    let Block::Paragraph(p)=block else{panic!("paragraph expected")};p
}
#[test]
fn main_mixed_content_utf8_and_formatting(){
    let body=format!("<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>科研</w:t>{}<w:tab/><w:t>结果</w:t>{}<w:br/><w:t>end</w:t></w:r></w:p>",drawing("img"),drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let p=para(&d.blocks[0]);let r=&p.runs[0];
    assert_eq!(p.text(),"科研\t结果\nend");
    assert_eq!(r.pictures[0].text_offset,6);
    assert_eq!(r.pictures[1].text_offset,13);
    let items=r.inline_content().unwrap();
    assert_eq!(items.len(),5);
    assert_eq!(items[0],RunContent::Text("科研"));
    assert!(matches!(items[1],RunContent::Picture(_)));
    assert_eq!(items[2],RunContent::Text("\t结果"));
    assert!(matches!(items[3],RunContent::Picture(_)));
    assert_eq!(items[4],RunContent::Text("\nend"));
    let FormattedBlock::Paragraph(v)=d.formatted_blocks().next().unwrap().unwrap() else{panic!("format view")};
    let f=v.runs().next().unwrap().unwrap();
    assert_eq!(f.properties.bold,Some(true));
    assert!(std::ptr::eq(f.source,r));
    assert!(d.diagnostics.is_empty());
}
#[test]
fn adjacent_and_picture_only_runs_preserve_order(){
    let body=format!("<w:p><w:r>{}{}</w:r><w:r><w:t>A</w:t>{}</w:r></w:p>",drawing("one"),drawing("two"),drawing("three"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let p=para(&d.blocks[0]);let items=p.runs[0].inline_content().unwrap();
    assert_eq!(p.text(),"A");assert_eq!(items.len(),2);
    for (item,id) in items.iter().zip(["one","two"]){
        let RunContent::Picture(p)=item else{panic!("picture")};
        assert_eq!(p.embed.as_deref(),Some(id));
    }
    assert_eq!(p.runs[1].pictures[0].text_offset,1);
}
#[test]
fn table_and_nested_table_runs(){
    let body=format!("<w:tr><w:tc><w:p><w:r>{}</w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>内</w:t>{}</w:r></w:p></w:tc></w:tr></w:tbl><w:p/></w:tc></w:tr>",drawing("outer"),drawing("inner"));
    let xml=format!("<w:tbl xmlns:w='{W}'>{body}</w:tbl>");
    let t=parse_table(xml.as_bytes(),Limits::default()).unwrap();
    let b=&t.rows[0].cells[0].blocks;
    assert_eq!(para(&b[0]).runs[0].pictures[0].picture.embed.as_deref(),Some("outer"));
    let Block::Table(inner)=&b[1] else{panic!("nested")};
    assert_eq!(para(&inner.rows[0].cells[0].blocks[0]).runs[0].pictures[0].text_offset,3);
    assert!(t.diagnostics.is_empty());
}
#[test]
fn main_and_loaded_header_bind_source_scoped_media(){
    let p=format!("<w:p><w:r>{}</w:r></w:p>",drawing("img"));
    let body=format!("{p}<w:sectPr><w:headerReference w:type='default' r:id='h'/></w:sectPr>");
    let bytes=package(&body,&p);
    let d=read_docx_with_stories(&bytes,Limits::default()).unwrap();
    let c=load_media(&bytes,Limits::default(),MediaLimits::default()).unwrap();
    let main=&para(&d.blocks[0]).runs[0].pictures[0].picture;
    let h=&d.header_footer.as_ref().unwrap().parts["custom/h.xml"];
    let header=&para(&h.blocks[0]).runs[0].pictures[0].picture;
    assert!(matches!(main.bind(&c,&c.main_part).unwrap().embedded,Some(MediaBinding::Embedded{part_name:"custom/a.png",..})));
    assert!(matches!(header.bind(&c,"custom/h.xml").unwrap().embedded,Some(MediaBinding::Embedded{part_name:"custom/b.png",..})));
}
#[test]
fn standalone_footer_picture_and_strict_namespaces(){
    let x=format!("<w:ftr xmlns:w='{W}'><w:p><w:r>{}</w:r></w:p></w:ftr>",drawing("img"));
    let x=x.replace(W,"http://purl.oclc.org/ooxml/wordprocessingml/main")
        .replace("http://schemas.openxmlformats.org/drawingml/2006/","http://purl.oclc.org/ooxml/drawingml/")
        .replace(R,"http://purl.oclc.org/ooxml/officeDocument/relationships");
    let s=parse_story(x.as_bytes(),HeaderFooterKind::Footer,Limits::default()).unwrap();
    assert_eq!(para(&s.blocks[0]).runs[0].pictures.len(),1);
    assert!(s.diagnostics.is_empty());
}
#[test]
fn malformed_and_floating_drawings_are_diagnosed_without_text_loss(){
    let bad=drawing("img").replace("cx='63500'","cx='0'");
    let anchor=drawing("img").replace("wp:inline","wp:anchor");
    let body=format!("<w:p><w:r><w:t>before</w:t>{bad}{anchor}{}<w:t>after</w:t></w:r></w:p>",drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let r=&para(&d.blocks[0]).runs[0];
    assert_eq!(r.text,"beforeafter");assert_eq!(r.pictures.len(),1);
    assert_eq!(d.diagnostics.len(),2);
    assert_eq!(r.pictures[0].text_offset,6);
}
#[test]
fn mutated_offsets_are_validated(){
    let body=format!("<w:p><w:r><w:t>中</w:t>{}<w:t>文</w:t>{}</w:r></w:p>",drawing("img"),drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let mut r=para(&d.blocks[0]).runs[0].clone();
    r.pictures[0].text_offset=1;
    assert_eq!(r.inline_content(),Err(Error::Invalid("inline picture text offset")));
    r.pictures[0].text_offset=7;
    assert!(r.inline_content().is_err());
    r.pictures[0].text_offset=6;r.pictures[1].text_offset=3;
    assert!(r.inline_content().is_err());
}
#[test]
fn legacy_text_only_model_and_foreign_drawing(){
    let r=Run{text:"plain".into(),..Default::default()};
    assert_eq!(r.inline_content().unwrap(),vec![RunContent::Text("plain")]);
    assert!(Run::default().inline_content().unwrap().is_empty());
    let body="<w:p><w:r><x:drawing xmlns:x='urn:foreign'/><w:t>ok</w:t></w:r></w:p>";
    let d=read_docx(&package(body,""),Limits::default()).unwrap();
    assert_eq!(para(&d.blocks[0]).text(),"ok");
    assert!(para(&d.blocks[0]).runs[0].pictures.is_empty());
    assert_eq!(d.diagnostics.len(),1);
}
