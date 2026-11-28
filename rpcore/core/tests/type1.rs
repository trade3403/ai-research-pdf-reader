#[path="support/type1_fixture.rs"] mod fixture;
use rpcore::{Document,Error,fonts::type1::Type1,graphics::Segment,raster::Options};
#[test]
fn encrypted_and_plain_charstrings(){
    for iv in [-1,0,4]{
        let (mut c,e,t)=fixture::sections(iv,false);let (l1,l2,l3)=(c.len(),e.len(),t.len());c.extend(e);c.extend(t);
        let f=Type1::parse_pdf(&c,l1,l2,l3).unwrap();
        assert_eq!(f.glyph_count(),3);assert_eq!(f.glyph_name(65),Some(b"A".as_slice()));
        let p=f.outline(b"A").unwrap();assert_eq!(p[0],Segment::Move([20.,0.]));assert_eq!(p[1],Segment::Line([620.,0.]));
        assert_eq!(p.iter().filter(|x|matches!(x,Segment::Close)).count(),2);
        let p=f.outline(b"B").unwrap();assert_eq!(p[1],Segment::Cubic([20.,700.],[620.,700.],[620.,0.]));
        assert!(f.outline(b".notdef").unwrap().is_empty());
        assert!(Type1::parse_pdf(&c,l1,l2,l3-1).is_err());
    }
}
#[test]
fn pfa_and_pfb_containers(){
    let a=Type1::parse_pfa(&fixture::pfa()).unwrap();
    let b=Type1::parse_pfb(&fixture::pfb()).unwrap();
    assert_eq!(a.outline(b"A").unwrap(),b.outline(b"A").unwrap());
    let mut truncated=fixture::pfb();truncated.pop();assert!(Type1::parse_pfb(&truncated).is_err());
}
#[test]
fn recursive_subr_and_truncated_sections(){
    let (mut c,e,t)=fixture::sections(4,true);let (l1,l2,l3)=(c.len(),e.len(),t.len());c.extend(e);c.extend(t);
    let f=Type1::parse_pdf(&c,l1,l2,l3).unwrap();assert!(matches!(f.outline(b"A"),Err(Error::Limit(_))));
    let (mut c,e,_)=fixture::sections(4,false);let l1=c.len();c.extend(&e[..e.len()/2]);assert!(Type1::parse_pdf(&c,l1,e.len()/2,0).is_err());
}
#[test]
fn native_type1_render_and_differences(){
    for enc in ["","/Encoding /StandardEncoding","/Encoding << /BaseEncoding /StandardEncoding /Differences [65 /B /A] >>"]{
        let pdf=fixture::pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET",enc);
        let doc=Document::open(&pdf).unwrap();let pages=doc.pages().unwrap();
        let b=doc.render_page(&pages[0],Options::default(),10000).unwrap();
        let pixel=|x:usize,y:usize|{let i=(y*b.width+x)*4;&b.rgba[i..i+4]};
        assert!(!b.missing_text);assert_eq!(pixel(15,64),if enc.contains("Differences"){[0,0,0,255]}else{[255,255,255,255]});
        assert_eq!(pixel(28,64),if enc.contains("Differences"){[255,255,255,255]}else{[0,0,0,255]});
    }
}

#[test]fn native_type1_geometry_recovers_builtin_unicode(){
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET","");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let g=d.page_glyph_geometry(&pages[0],10000,None).unwrap();
    assert_eq!(g.glyphs[0].unicode.as_deref(),Some("A"));
    assert_eq!(g.glyphs[1].unicode.as_deref(),Some("B"));
    assert!(g.glyphs.iter().all(|x|x.unicode_from_embedded_name));
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET","/Encoding << /BaseEncoding /StandardEncoding /Differences [65 /B /A] >>");
    let d=Document::open(&p).unwrap();
    let g=d.page_glyph_geometry(&d.pages().unwrap()[0],10000,None).unwrap();
    assert_eq!(g.glyphs[0].unicode.as_deref(),Some("B"));
    assert_eq!(g.glyphs[0].embedded_glyph_name.as_deref(),Some(b"B".as_slice()));
    assert!(!g.glyphs[0].unicode_from_embedded_name);
}
