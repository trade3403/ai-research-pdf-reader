use rwcore::{load_media,read_docx,ImageFormat,MediaBinding,MediaLimits,Limits,Error};
use std::io::{Cursor,Write};
use zip::{ZipWriter,write::SimpleFileOptions};
const OFFICE:&str="http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const PNG:&[u8]=b"\x89PNG\r\n\x1a\nopaque-test";
const JPEG:&[u8]=b"\xff\xd8\xffopaque-test";
fn rel(id:&str,target:&str,external:bool)->String{
    format!("<Relationship Id='{id}' Type='{OFFICE}image' Target='{target}' {}/>",if external{"TargetMode='External'"}else{""})
}
fn rels(body:&str)->String{
    format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'>{body}</Relationships>")
}
fn fixture(main_rels:&str,header_rels:&str,mimes:&str,extras:&[(&str,&[u8])])->Vec<u8>{
    let ct=format!("<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Override PartName='/manuscript/main.xml' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/>{mimes}</Types>");
    let root=rels(&format!("<Relationship Id='root' Type='{OFFICE}officeDocument' Target='manuscript/main.xml'/>"));
    let main=rels(main_rels);
    let header=rels(header_rels);
    let doc=b"<w:document xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'><w:body><w:p/></w:body></w:document>";
    let mut entries:Vec<(&str,&[u8])>=vec![("[Content_Types].xml",ct.as_bytes()),("_rels/.rels",root.as_bytes()),
        ("manuscript/main.xml",doc),("manuscript/_rels/main.xml.rels",main.as_bytes()),
        ("manuscript/header.xml",b"<header/>"),("manuscript/_rels/header.xml.rels",header.as_bytes())];
    entries.extend_from_slice(extras);
    let mut z=ZipWriter::new(Cursor::new(Vec::new()));
    for (name,data) in entries{
        z.start_file(name,SimpleFileOptions::default()).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap().into_inner()
}
const MIME:&str="<Default Extension='png' ContentType='image/png'/><Default Extension='jpg' ContentType='image/jpeg'/>";
fn load(bytes:&[u8])->Result<rwcore::MediaCatalog,Error>{load_media(bytes,Limits::default(),MediaLimits::default())}
#[test]
fn canonical_deduplication_and_source_scoped_ids(){
    let main=format!("{}{}",rel("r1","../media/a.png",false),rel("r2","/media/a.png",false));
    let h=rel("r1","../media/b.jpg",false);
    let bytes=fixture(&main,&h,MIME,&[("media/a.png",PNG),("media/b.jpg",JPEG)]);
    let c=load(&bytes).unwrap();
    assert_eq!(c.main_part,"manuscript/main.xml");
    assert_eq!(c.resources().len(),2);
    assert_eq!(c.relationship_count(),3);
    assert_eq!(c.total_bytes(),PNG.len()+JPEG.len());
    let Some(MediaBinding::Embedded{resource:a,part_name})=c.resolve(&c.main_part,"r1") else{panic!("embedded")};
    let Some(MediaBinding::Embedded{resource:b,..})=c.resolve(&c.main_part,"r2") else{panic!("embedded")};
    assert_eq!(part_name,"media/a.png");
    assert!(std::ptr::eq(a,b));
    assert_eq!(a.bytes,PNG);
    assert!(matches!(c.resolve("manuscript/header.xml","r1"),Some(MediaBinding::Embedded{resource,..}) if resource.format==ImageFormat::Jpeg));
    assert!(c.resolve("unknown.xml","r1").is_none());
}
#[test]
fn external_is_inert_and_unreferenced_bytes_not_loaded(){
    let bytes=fixture(&rel("remote","https://example.invalid/image.png",true),"",MIME,&[("media/bad.png",b"not png")]);
    let c=load(&bytes).unwrap();
    assert_eq!(c.resolve(&c.main_part,"remote"),Some(MediaBinding::External{target:"https://example.invalid/image.png"}));
    assert!(c.resources().is_empty());
    assert_eq!(c.total_bytes(),0);
}
#[test]
fn signature_mismatch_and_unsupported_mime_fail_opt_in_only(){
    let main=rel("a","../media/a.png",false);
    let bytes=fixture(&main,"",MIME,&[("media/a.png",JPEG)]);
    assert_eq!(load(&bytes),Err(Error::Invalid("image signature mismatch")));
    assert!(read_docx(&bytes,Limits::default()).is_ok());
    let bytes=fixture(&main,"","<Default Extension='png' ContentType='image/svg+xml'/>",&[("media/a.png",PNG)]);
    assert_eq!(load(&bytes),Err(Error::Unsupported("image content type")));
    let bytes=fixture(&main,"","",&[("media/a.png",PNG)]);
    assert_eq!(load(&bytes),Err(Error::Invalid("missing image content type")));
}
#[test]
fn all_supported_signatures_and_truncations(){
    for (format,signature) in [
        (ImageFormat::Png,b"\x89PNG\r\n\x1a\n".as_slice()),
        (ImageFormat::Jpeg,b"\xff\xd8\xff".as_slice()),
        (ImageFormat::Gif,b"GIF89a".as_slice()),
        (ImageFormat::Bmp,b"BM".as_slice()),
        (ImageFormat::Tiff,b"MM\x00\x2a".as_slice()),
        (ImageFormat::WebP,b"RIFF\x00\x00\x00\x00WEBP".as_slice())]{
        assert!(format.matches_signature(signature));
        assert!(!format.matches_signature(&signature[..signature.len()-1]));
        assert!(!format.matches_signature(b""));
        assert!(format.mime().starts_with("image/"));
    }
}
#[test]
fn per_resource_and_aggregate_bytes_are_bounded(){
    let main=format!("{}{}",rel("a","../media/a.png",false),rel("b","../media/b.jpg",false));
    let bytes=fixture(&main,"",MIME,&[("media/a.png",PNG),("media/b.jpg",JPEG)]);
    assert_eq!(load_media(&bytes,Limits::default(),MediaLimits{max_resource_bytes:PNG.len()-1,..MediaLimits::default()}),Err(Error::Limit("media resource bytes")));
    assert_eq!(load_media(&bytes,Limits::default(),MediaLimits{max_total_bytes:PNG.len()+JPEG.len()-1,..MediaLimits::default()}),Err(Error::Limit("total media bytes")));
    assert!(load_media(&bytes,Limits::default(),MediaLimits{max_total_bytes:PNG.len()+JPEG.len(),..MediaLimits::default()}).is_ok());
}
#[test]
fn duplicate_refs_only_consume_one_resource_budget(){
    let main=format!("{}{}",rel("a","../media/a.png",false),rel("b","../media/a.png",false));
    let bytes=fixture(&main,"",MIME,&[("media/a.png",PNG)]);
    let l=MediaLimits{max_resources:1,max_total_bytes:PNG.len(),max_relationships:2,..MediaLimits::default()};
    assert!(load_media(&bytes,Limits::default(),l).is_ok());
    assert_eq!(load_media(&bytes,Limits::default(),MediaLimits{max_relationships:1,..l}),Err(Error::Limit("media relationships")));
    assert_eq!(load_media(&bytes,Limits::default(),MediaLimits{max_resources:0,..l}),Err(Error::Limit("media resources")));
}
#[test]
fn graph_safety_missing_and_escaping_targets(){
    let bytes=fixture(&rel("a","../media/missing.png",false),"",MIME,&[]);
    assert_eq!(load(&bytes),Err(Error::MissingNamedPart("media/missing.png".into())));
    let bytes=fixture(&rel("a","../../escape.png",false),"",MIME,&[]);
    assert_eq!(load(&bytes),Err(Error::Invalid("target escapes package")));
    let r=rel("a","../media/a.png",false);
    let bytes=fixture(&format!("{r}{r}"),"",MIME,&[("media/a.png",PNG)]);
    assert_eq!(load(&bytes),Err(Error::Invalid("duplicate relationship ID")));
}
