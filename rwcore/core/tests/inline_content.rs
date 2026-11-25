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
    package_with_png(body,header,b"\x89PNG\r\n\x1a\nA")
}
fn package_with_png(body:&str,header:&str,png:&[u8])->Vec<u8>{
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
        ("custom/a.png",png),("custom/b.png",b"\x89PNG\r\n\x1a\nB".as_slice())]{
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

fn geometric_drawing(id:&str)->String{
    drawing(id).replace("<pic:spPr/>","<pic:spPr><a:xfrm rot='5400000' flipH='1'><a:off x='0' y='0'/><a:ext cx='63500' cy='127000'/></a:xfrm></pic:spPr>")
        .replace("</pic:blipFill>","<a:srcRect l='10000'/></pic:blipFill>")
}
fn floating_drawing(id:&str)->String{
    geometric_drawing(id).replace("wp:inline","wp:anchor")
        .replace("<wp:extent","<wp:simplePos x='10' y='20'/><wp:positionH relativeFrom='page'><wp:posOffset>0</wp:posOffset></wp:positionH><wp:positionV relativeFrom='page'><wp:posOffset>0</wp:posOffset></wp:positionV><wp:wrapNone/><wp:extent")
        .replace("<wp:anchor ","<wp:anchor simplePos='1' relativeHeight='1' behindDoc='0' locked='0' layoutInCell='1' allowOverlap='1' ")
}
#[test]
fn mixed_floating_inline_order_and_old_view(){
    use rwcore::{PictureKindRef,PictureGeometryState};
    let body=format!("<w:p><w:r><w:t>中</w:t>{}{}<w:t>文</w:t>{}</w:r></w:p>",geometric_drawing("img"),floating_drawing("img"),drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let r=&para(&d.blocks[0]).runs[0];
    assert_eq!(r.text,"中文");assert_eq!(r.pictures.len(),2);
    let a=r.drawings().unwrap();
    assert_eq!(a.iter().map(|p|p.text_offset).collect::<Vec<_>>(),vec![3,3,6]);
    assert!(matches!(a[0].kind,PictureKindRef::Inline(_)));
    assert!(matches!(a[1].kind,PictureKindRef::Floating(_)));
    assert!(matches!(a[0].geometry,PictureGeometryState::Parsed(_)));
    assert!(matches!(a[2].geometry,PictureGeometryState::Absent));
    assert_eq!(r.inline_content().unwrap().len(),4);
    assert!(d.diagnostics.is_empty());
}
#[test]
fn combined_resource_scope_nested_table_and_header(){
    use rwcore::{read_docx_with_media,PictureKindRef};
    let p=format!("<w:p><w:r>{}</w:r></w:p>",floating_drawing("img"));
    let body=format!("<w:tbl><w:tr><w:tc><w:tbl><w:tr><w:tc>{p}</w:tc></w:tr></w:tbl><w:p/></w:tc></w:tr></w:tbl><w:sectPr><w:headerReference w:type='default' r:id='h'/></w:sectPr>");
    let d=read_docx_with_media(&package(&body,&p),Limits::default(),MediaLimits::default()).unwrap();
    let a=d.pictures(2).unwrap();
    assert_eq!(a.len(),2);assert_eq!(a[0].source_part,"custom/main.xml");assert_eq!(a[1].source_part,"custom/h.xml");
    assert!(a.iter().all(|p|matches!(p.drawing.kind,PictureKindRef::Floating(_))));
    assert!(matches!(a[0].bind(&d.media).unwrap().embedded,Some(MediaBinding::Embedded{part_name:"custom/a.png",..})));
    assert!(matches!(a[1].bind(&d.media).unwrap().embedded,Some(MediaBinding::Embedded{part_name:"custom/b.png",..})));
    assert!(d.pictures(1).is_err());
}
#[test]
fn geometry_failure_preserves_picture_and_text(){
    use rwcore::PictureGeometryState;
    let bad=geometric_drawing("img").replace("flipH='1'","flipH='bad'");
    let body=format!("<w:p><w:r><w:t>A</w:t>{bad}<w:t>B</w:t></w:r></w:p>");
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let r=&para(&d.blocks[0]).runs[0];assert_eq!(r.text,"AB");
    let a=r.drawings().unwrap();assert_eq!(a.len(),1);
    assert!(matches!(a[0].geometry,PictureGeometryState::Unavailable(_)));
    assert!(a[0].mapping_at(0,0).is_err());
}
#[test]
fn combined_placement_and_geometry_reconciliation(){
    use rwcore::{AnchorLayoutContext,EmuRect,EmuSpan,InsideEdge,GeometryPoint};
    let body=format!("<w:p><w:r>{}</w:r></w:p>",floating_drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let a=para(&d.blocks[0]).runs[0].drawings().unwrap();
    let rect=EmuRect{x:EmuSpan{start:0,end:1000000},y:EmuSpan{start:0,end:1000000}};
    let ctx=AnchorLayoutContext{page:rect,content:rect,column:None,character:None,line:None,paragraph:None,horizontal_inside:InsideEdge::Start,vertical_inside:InsideEdge::Start};
    let p=a[0].floating_placement(&ctx).unwrap().unwrap();
    assert_eq!(p.picture.x.start,10);assert_eq!(p.picture.y.start,20);
    let m=a[0].mapping_at(p.picture.x.start,p.picture.y.start).unwrap().unwrap();
    let center=m.source_to_page(GeometryPoint{x:0.55,y:0.5}).unwrap();
    assert!((center.x-31760.0).abs()<1e-8);assert!((center.y-63520.0).abs()<1e-8);
    let bad=geometric_drawing("img").replace("<a:off x='0'","<a:off x='1'");
    let body=format!("<w:p><w:r>{bad}</w:r></w:p>");
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    assert!(para(&d.blocks[0]).runs[0].drawings().unwrap()[0].mapping_at(0,0).is_err());
}
#[test]
fn rejects_stale_combined_order(){
    let body=format!("<w:p><w:r><w:t>中</w:t>{}{}</w:r></w:p>",drawing("img"),floating_drawing("img"));
    let d=read_docx(&package(&body,""),Limits::default()).unwrap();
    let mut r=para(&d.blocks[0]).runs[0].clone();
    r.drawing_order[0].text_offset=1;assert!(r.drawings().is_err());
    r.drawing_order[0].text_offset=3;r.drawing_order[0].kind=rwcore::RunDrawingKind::Inline(1);assert!(r.drawings().is_err());
    r.drawing_order.clear();assert_eq!(r.drawings().unwrap().len(),1);
}
#[test]
fn missing_resource_and_invalid_header_are_explicit(){
    use rwcore::{read_docx_with_media,ImageLimits};
    let body=format!("<w:p><w:r>{}{}</w:r></w:p>",geometric_drawing("missing"),geometric_drawing("img"));
    let d=read_docx_with_media(&package(&body,""),Limits::default(),MediaLimits::default()).unwrap();
    let a=d.pictures(2).unwrap();assert!(a[0].bind(&d.media).is_err());
    assert!(a[1].embedded_dimensions(&d.media,ImageLimits::default()).is_err());
    assert!(a[1].pixel_window(&d.media,ImageLimits::default()).is_err());
}
#[test]
fn source_pixels_and_crop_window_from_embedded_header(){
    use rwcore::{read_docx_with_media,ImageLimits,ImageDimensions};
    let body=format!("<w:p><w:r>{}</w:r></w:p>",geometric_drawing("img"));
    let bytes=package_with_png(&body,"",b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x64\x00\x00\x00\x32\x08\x02\x00\x00\x00\x00\x00\x00\x00");
    let d=read_docx_with_media(&bytes,Limits::default(),MediaLimits::default()).unwrap();
    let a=d.pictures(1).unwrap()[0];
    assert_eq!(a.embedded_dimensions(&d.media,ImageLimits::default()).unwrap(),Some(ImageDimensions{width:100,height:50}));
    assert_eq!(a.pixel_window(&d.media,ImageLimits::default()).unwrap(),Some((10,0,100,50)));
}
#[test]
fn strict_floating_footer_uses_same_tree_parser(){
    use rwcore::{PictureKindRef,PictureGeometryState};
    let x=format!("<w:ftr xmlns:w='{W}'><w:p><w:r>{}</w:r></w:p></w:ftr>",floating_drawing("img"));
    let x=x.replace(W,"http://purl.oclc.org/ooxml/wordprocessingml/main")
        .replace("http://schemas.openxmlformats.org/drawingml/2006/","http://purl.oclc.org/ooxml/drawingml/")
        .replace(R,"http://purl.oclc.org/ooxml/officeDocument/relationships");
    let s=parse_story(x.as_bytes(),HeaderFooterKind::Footer,Limits::default()).unwrap();
    let a=para(&s.blocks[0]).runs[0].drawings().unwrap();
    assert!(matches!(a[0].kind,PictureKindRef::Floating(_)));assert!(matches!(a[0].geometry,PictureGeometryState::Parsed(_)));
    assert!(s.diagnostics.is_empty());
}
