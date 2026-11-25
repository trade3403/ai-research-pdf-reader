use rwcore::{parse_inline_picture,emu_to_twips,Limits,Error,InlinePicture,PictureExtent,
    load_media,MediaLimits,MediaBinding};
use std::io::{Cursor,Write};
use zip::{ZipWriter,write::SimpleFileOptions};
const WP:&str="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A:&str="http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC:&str="http://schemas.openxmlformats.org/drawingml/2006/picture";
const R:&str="http://schemas.openxmlformats.org/officeDocument/2006/relationships";
fn picture(refs:&str)->String{
    format!("<wp:inline xmlns:wp='{WP}' xmlns:a='{A}' xmlns:pic='{PIC}' xmlns:r='{R}'><wp:extent cx='63500' cy='127000'/><wp:docPr id='7' name='Figure 1' descr='科研 &amp; 图' title='实验'/><a:graphic><a:graphicData uri='{PIC}'><pic:pic><pic:nvPicPr><pic:cNvPr id='0' name='image.png'/></pic:nvPicPr><pic:blipFill><a:blip {refs}/></pic:blipFill><pic:spPr/></pic:pic></a:graphicData></a:graphic></wp:inline>")
}
fn parse(s:&str)->Result<InlinePicture,Error>{parse_inline_picture(s.as_bytes(),Limits::default())}
#[test]
fn parses_extent_unicode_and_both_references(){
    let p=parse(&picture("r:embed='r1' r:link='r2'")).unwrap();
    assert_eq!(p.extent,PictureExtent{cx:63500,cy:127000});
    assert_eq!(p.extent.twips().unwrap(),(100,200));
    assert_eq!(p.document_properties.id,7);
    assert_eq!(p.document_properties.description.as_deref(),Some("科研 & 图"));
    assert_eq!(p.document_properties.title.as_deref(),Some("实验"));
    assert_eq!(p.picture_properties.name,"image.png");
    assert_eq!(p.embed.as_deref(),Some("r1"));
    assert_eq!(p.link.as_deref(),Some("r2"));
}
#[test]
fn strict_namespace_and_drawing_wrapper(){
    let s=picture("r:embed='a'").replace("http://schemas.openxmlformats.org/drawingml/2006/","http://purl.oclc.org/ooxml/drawingml/")
        .replace(R,"http://purl.oclc.org/ooxml/officeDocument/relationships");
    let wrapped=format!("<w:drawing xmlns:w='http://purl.oclc.org/ooxml/wordprocessingml/main'>{s}</w:drawing>");
    assert_eq!(parse(&wrapped).unwrap().extent.twips().unwrap(),(100,200));
    let bad=picture("r:embed='r1'").replace(A,"urn:foreign");
    assert!(parse(&bad).is_err());
}
#[test]
fn malformed_duplicate_and_nonpicture_paths(){
    let s=picture("r:embed='a'");
    for bad in [s.replace("cx='63500'","cx='0'"),s.replace("cy='127000'","cy='-1'"),
        s.replace("cy='127000'","cy='27273042316901'"),
        s.replace("<wp:extent cx='63500' cy='127000'/>","<wp:extent cx='1' cy='1'/><wp:extent cx='2' cy='2'/>"),
        s.replace("r:embed='a'","embed='a'"),s.replace("r:embed='a'","r:embed='a b'"),
        s.replace(&format!("uri='{PIC}'"),"uri='urn:chart'"),s.replace("wp:inline","wp:anchor")]{
        assert!(parse(&bad).is_err(),"{bad}");
    }
}
#[test]
fn xml_and_metadata_budgets(){
    let s=picture("r:embed='a'");
    assert!(parse_inline_picture(s.as_bytes(),Limits{max_part_bytes:1,..Default::default()}).is_err());
    assert!(parse_inline_picture(s.as_bytes(),Limits{max_xml_nodes:1,..Default::default()}).is_err());
    assert!(parse(&format!("<!DOCTYPE wp:inline>{s}")).is_err());
    let long=picture(&format!("r:embed='{}'","x".repeat(1025)));
    assert_eq!(parse(&long),Err(Error::Limit("picture attribute bytes")));
}
#[test]
fn checked_emu_rounding_and_overflow(){
    for (emu,twips) in [(0,0),(317,0),(318,1),(635,1),(952,1),(953,2),(914400,1440)]{
        assert_eq!(emu_to_twips(emu).unwrap(),twips);
    }
    assert_eq!(emu_to_twips(u64::MAX),Err(Error::Limit("picture twips")));
}
fn catalog()->rwcore::MediaCatalog{
    let ct="<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Override PartName='/word/document.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/><Default Extension='png' ContentType='image/png'/></Types>";
    let root=format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='root' Type='{R}/officeDocument' Target='word/document.xml'/></Relationships>");
    let rels=format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='r1' Type='{R}/image' Target='media/a.png'/><Relationship Id='r2' Type='{R}/image' Target='https://example.invalid/a.png' TargetMode='External'/></Relationships>");
    let mut z=ZipWriter::new(Cursor::new(Vec::new()));
    for (n,b) in [("[Content_Types].xml",ct.as_bytes()),("_rels/.rels",root.as_bytes()),
        ("word/document.xml",b"<doc/>".as_slice()),("word/_rels/document.xml.rels",rels.as_bytes()),
        ("word/media/a.png",b"\x89PNG\r\n\x1a\n".as_slice())]{
        z.start_file(n,SimpleFileOptions::default()).unwrap();z.write_all(b).unwrap();
    }
    let bytes=z.finish().unwrap().into_inner();
    load_media(&bytes,Limits::default(),MediaLimits::default()).unwrap()
}
#[test]
fn binding_embedded_and_external_without_fetch(){
    let c=catalog();
    let p=parse(&picture("r:embed='r1' r:link='r2'")).unwrap();
    let b=p.bind(&c,&c.main_part).unwrap();
    assert!(matches!(b.embedded,Some(MediaBinding::Embedded{part_name:"word/media/a.png",..})));
    assert_eq!(b.linked,Some(MediaBinding::External{target:"https://example.invalid/a.png"}));
    assert!(p.bind(&c,"word/header.xml").is_err());
}
#[test]
fn binding_target_mode_and_missing_id_errors(){
    let c=catalog();
    for (refs,error) in [("r:embed='r2'","embedded picture target mode"),
        ("r:link='r1'","linked picture target mode"),("r:embed='unknown'","missing embedded picture resource"),
        ("r:link='unknown'","missing linked picture resource")]{
        let p=parse(&picture(refs)).unwrap();
        assert_eq!(p.bind(&c,&c.main_part),Err(Error::Invalid(error)));
    }
}
