#[path="support/cff_fixture.rs"] mod fixture;
use rpcore::{Document,Error,fonts::cff::Cff,graphics::Segment,raster::Options};
#[test]
fn name_and_cid_charsets_subroutines(){
    for cid in [false,true]{
        let c=Cff::parse(fixture::font(cid,false)).unwrap();
        assert_eq!(c.is_cid(),cid);assert_eq!(c.glyph_count(),3);
        let gid=if cid{c.glyph_for_cid(42)}else{c.glyph_for_name(b"outlineA")};assert_eq!(gid,Some(1));
        let p=c.outline(1).unwrap();assert_eq!(p[0],Segment::Move([0.,0.]));assert_eq!(p[1],Segment::Line([600.,0.]));
        assert_eq!(p.iter().filter(|s|matches!(s,Segment::Close)).count(),2);
        let p=c.outline(2).unwrap();assert_eq!(p[1],Segment::Cubic([0.,700.],[600.,700.],[600.,0.]));
        assert!(c.outline(0).unwrap().is_empty());
    }
}
#[test]
fn bounded_recursive_and_truncated(){
    let c=Cff::parse(fixture::font(false,true)).unwrap();
    assert!(matches!(c.outline(1),Err(Error::Limit(_))));
    let f=fixture::font(false,false);
    assert!(Cff::parse(f[..f.len()-2].to_vec()).is_err());
}
#[test]
fn native_cff_pdf_pixels(){
    for cid in [false,true]{
        let content=if cid{b"BT /F1 20 Tf 10 20 Td <002a004d> Tj ET".as_slice()}else{b"BT /F1 20 Tf 10 20 Td (AB) Tj ET"};
        let p=fixture::pdf(cid,content);
        let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
        let b=d.render_page(&pages[0],Options::default(),10000).unwrap();
        let pixel=|x:usize,y:usize|{let i=(y*b.width+x)*4;&b.rgba[i..i+4]};
        assert!(!b.missing_text);assert_eq!(pixel(11,65),[0,0,0,255]);assert_eq!(pixel(15,64),[255,255,255,255]);
        assert_eq!(pixel(29,67),[0,0,0,255]);
    }
}
