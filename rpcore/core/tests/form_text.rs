#[path="support/cff_fixture.rs"] mod fixture;
use rpcore::{Document,Error,graphics::Command,raster::{Options,Bitmap}};
fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut b=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();b.extend_from_slice(data);b.extend_from_slice(b"\nendstream");b
}
fn pdf(page:&[u8],form:&[u8],nested:&[u8],form_resources:&str,bbox:&str)->Vec<u8>{
    let desc=b"<< /Type /FontDescriptor /FontName /RPCore /Flags 4 /FontBBox [0 0 600 700] /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 80 /FontFile3 10 0 R >>".to_vec();
    let font=|enc:&str|format!("<< /Type /Font /Subtype /Type1 /BaseFont /RPCore /Encoding {enc} /FirstChar 65 /LastChar 66 /Widths [650 650] /FontDescriptor 9 0 R >>").into_bytes();
    let objs=[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 80] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 7 0 R >> /XObject << /G 5 0 R /N 6 0 R >> >> >>".to_vec(),
        stream("",page),
        stream(&format!("/Subtype /Form /BBox {bbox} /Matrix [1 0 0 1 25 0] {form_resources}"),form),
        stream("/Subtype /Form /BBox [0 0 20 40] /Matrix [1 0 0 1 20 0]",nested),
        font("/StandardEncoding"),font("<< /BaseEncoding /StandardEncoding /Differences [65 /B] >>"),
        desc,stream("/Subtype /Type1C",&fixture::encoded_font(3,[34,35],0,&[])),
    ];
    let mut b=b"%PDF-1.5\n".to_vec();let mut off=Vec::new();
    for (i,o) in objs.iter().enumerate(){off.push(b.len());b.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());b.extend_from_slice(o);b.extend_from_slice(b"\nendobj\n");}
    let x=b.len();b.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objs.len()+1).as_bytes());
    for o in off{b.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    b.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objs.len()+1).as_bytes());b
}
fn render(p:&[u8])->Bitmap{let d=Document::open(p).unwrap();d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap()}
fn px(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
const TEXT:&[u8]=b"BT /F1 20 Tf 0 20 Td (A) Tj ET";
const OWN:&str="/Resources << /Font << /F1 8 0 R >> /XObject << /N 6 0 R >> >>";
#[test]fn form_font_scope_and_nested_inheritance(){
    let p=pdf(b"BT /F1 20 Tf 0 20 Td (A) Tj ET /G Do BT 70 20 Td (A) Tj ET",b"BT /F1 20 Tf 0 20 Td (A) Tj ET /N Do",TEXT,OWN,"[0 0 45 40]");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();let list=d.page_display_list(&pages[0],10000).unwrap();
    assert!(list.commands.iter().any(|c|matches!(c,Command::Text{scope:Some((6,0)),resource_owner:Some((5,0)),..})));
    let b=render(&p);
    // A has a hole at local x=5,y=6; B is solid. Page F1 remains A after Form.
    for x in [5,75]{assert_eq!(px(&b,x,53),[255,255,255,255]);}
    for x in [30,50]{assert_eq!(px(&b,x,53),[0,0,0,255]);}
    for x in [1,26,46,71]{assert_eq!(px(&b,x,53),[0,0,0,255]);}
}
#[test]fn form_without_resources_inherits_page_and_empty_clip(){
    let p=pdf(b"/G Do",TEXT,b"","","[0 0 20 40]");
    let b=render(&p);assert_eq!(px(&b,26,53),[0,0,0,255]);assert_eq!(px(&b,30,53),[255,255,255,255]);
    let p=pdf(b"/G Do",TEXT,b"",OWN,"[0 0 0 40]");
    let b=render(&p);assert!(b.rgba.chunks_exact(4).all(|p|p==[255,255,255,255]));
}
#[test]fn explicit_form_resources_do_not_fall_through(){
    let p=pdf(b"/G Do",TEXT,b"","/Resources << >>","[0 0 20 40]");
    let d=Document::open(&p).unwrap();assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(Error::Invalid("Font resources"))));
}
#[test]fn same_form_reused_in_two_resource_environments(){
    let p=pdf(b"/N Do /G Do",b"/N Do",TEXT,OWN,"[0 0 45 40]");
    let b=render(&p);assert_eq!(px(&b,25,53),[255,255,255,255]);assert_eq!(px(&b,50,53),[0,0,0,255]);
}
