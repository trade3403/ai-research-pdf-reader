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

#[test]fn cff_builtin_encodings_and_expert_charsets(){
    let c=Cff::parse(fixture::encoded_font(3,[34,35],0,&[])).unwrap();
    assert_eq!(c.glyph_for_code(b'A'),Some(1));assert_eq!(c.glyph_for_code(b'B'),Some(2));assert_eq!(c.glyph_for_code(b'C'),None);
    for (cs,code,name) in [(1,33,b"exclamsmall".as_slice()),(2,36,b"dollaroldstyle".as_slice())]{
        let c=Cff::parse(fixture::encoded_font(cs,[0,0],1,&[])).unwrap();
        assert_eq!(c.glyph_for_code(32),Some(1));assert_eq!(c.glyph_for_code(code),Some(2));assert_eq!(c.glyph_for_name(name),Some(2));
    }
    let c=Cff::parse(fixture::encoded_font(0,[0,0],0,&[])).unwrap();assert_eq!(c.glyph_for_code(32),Some(1));assert_eq!(c.glyph_for_code(33),Some(2));
}
#[test]fn custom_encoding_formats_supplements_and_bounds(){
    for enc in [vec![0,2,65,66],vec![1,1,65,1],vec![0x80,1,65,1,66,0,35]]{
        let c=Cff::parse(fixture::encoded_font(3,[34,35],2,&enc)).unwrap();
        assert_eq!(c.glyph_for_code(65),Some(1));assert_eq!(c.glyph_for_code(66),Some(2));
    }
    for enc in [vec![0,2,65,65],vec![1,1,255,1],vec![0,3,65,66,67],vec![0x80,1,65,1,66,0,90],vec![0x80,1,65,1,65,0,35]]{
        assert!(Cff::parse(fixture::encoded_font(3,[34,35],2,&enc)).is_err());
    }
}
#[test]fn pdf_encoding_precedence_and_native_pixels(){
    // Built-in encoding swaps A/B. PDF named base replaces it; Differences
    // override one code and keep the selected base for all other codes.
    for (enc,hole) in [
        ("",false),
        ("/Encoding /StandardEncoding",true),
        ("/Encoding /WinAnsiEncoding",true),
        ("/Encoding /MacRomanEncoding",true),
        ("/Encoding << /BaseEncoding /StandardEncoding /Differences [65 /B] >>",false),
        ("/Encoding << /Differences [65 /A] >>",true),
    ]{
        let f=fixture::encoded_font(3,[34,35],2,&[0,2,66,65]);
        let p=fixture::pdf_with_font(false,b"BT /F1 20 Tf 10 20 Td (AB) Tj ET",f,enc);
        let d=Document::open(&p).unwrap();let b=d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap();
        let at=(64*b.width+15)*4;
        assert_eq!(&b.rgba[at..at+4],if hole{&[255,255,255,255]}else{&[0,0,0,255]},"{enc}");
    }
    let f=fixture::encoded_font(3,[34,35],0,&[]);
    let p=fixture::pdf_with_font(false,b"BT /F1 20 Tf (A) Tj ET",f,"/Encoding << /Differences [65 /Missing] >>");
    let d=Document::open(&p).unwrap();assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(Error::Unsupported("missing CFF glyph mapping"))));
}
#[test]fn encoding_names_are_not_unicode_indices(){
    use rpcore::fonts::cff::predefined_name;
    assert_eq!(predefined_name(b"StandardEncoding",39),Some(b"quoteright".as_slice()));
    assert_eq!(predefined_name(b"WinAnsiEncoding",39),Some(b"quotesingle".as_slice()));
    assert_eq!(predefined_name(b"WinAnsiEncoding",128),Some(b"Euro".as_slice()));
    assert_eq!(predefined_name(b"MacExpertEncoding",35),Some(b"centoldstyle".as_slice()));
    assert_eq!(predefined_name(b"ExpertEncoding",35),None);
}

#[test]fn explicit_standard_font_substitution(){
    use rpcore::{fonts::standard::StandardFont,raster::StandardFontSet};
    let p=fixture::standard_pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET","Helvetica","");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    assert!(d.render_page(&pages[0],Options::default(),10000).is_err());
    let mut fonts=StandardFontSet::new();
    fonts.insert(StandardFont::Helvetica,fixture::encoded_font(3,[34,35],0,&[])).unwrap();
    let out=d.render_page_with_standard_fonts(&pages[0],Options::default(),10000,&fonts).unwrap();
    assert_eq!(out.substituted_fonts,vec![StandardFont::Helvetica]);assert!(!out.bitmap.missing_text);
    let b=out.bitmap;let pixel=|x:usize,y:usize|{let i=(y*b.width+x)*4;&b.rgba[i..i+4]};
    assert_eq!(pixel(11,65),[0,0,0,255]);assert_eq!(pixel(15,64),[255,255,255,255]);assert_eq!(pixel(29,67),[0,0,0,255]);
    let p=fixture::standard_pdf(b"BT /F1 20 Tf (C) Tj ET","Helvetica","");
    let d=Document::open(&p).unwrap();assert!(d.render_page_with_standard_fonts(&d.pages().unwrap()[0],Options::default(),10000,&fonts).is_err());
    let p=fixture::standard_pdf(b"BT /F1 20 Tf (A) Tj ET","UnknownFont","");
    let d=Document::open(&p).unwrap();assert!(d.render_page_with_standard_fonts(&d.pages().unwrap()[0],Options::default(),10000,&fonts).is_err());
    assert!(fonts.insert(StandardFont::TimesRoman,vec![1,2,3]).is_err());
}
#[path="support/type1_fixture.rs"] mod type1_fixture;
#[test]fn explicit_type1_standard_substitution(){
    use rpcore::{fonts::standard::StandardFont,raster::StandardFontSet};
    let p=fixture::standard_pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET","Helvetica","");
    let d=Document::open(&p).unwrap();
    for bytes in [type1_fixture::pfa(),type1_fixture::pfb()]{
        let mut fonts=StandardFontSet::new();fonts.insert(StandardFont::Helvetica,bytes).unwrap();
        let out=d.render_page_with_standard_fonts(&d.pages().unwrap()[0],Options::default(),10000,&fonts).unwrap();
        assert_eq!(out.substituted_fonts,vec![StandardFont::Helvetica]);assert!(!out.bitmap.missing_text);
        let b=out.bitmap;let i=(65*b.width+11)*4;assert_eq!(&b.rgba[i..i+4],&[0,0,0,255]);
    }
}

fn otto(cff:&[u8])->Vec<u8>{
    let mut b=b"OTTO".to_vec();b.extend([0,1,0,16,0,0,0,0]);
    b.extend(b"CFF ");b.extend([0;4]);b.extend(28u32.to_be_bytes());b.extend((cff.len()as u32).to_be_bytes());b.extend(cff);b
}
#[test]fn opentype_cff_container_and_substitution(){
    use rpcore::{fonts::standard::StandardFont,raster::StandardFontSet};
    let data=fixture::encoded_font(3,[34,35],0,&[]);let wrapped=otto(&data);
    let c=Cff::parse_opentype(&wrapped).unwrap();assert_eq!(c.glyph_for_code(65),Some(1));
    let mut set=StandardFontSet::new();set.insert(StandardFont::Helvetica,wrapped.clone()).unwrap();
    let p=fixture::standard_pdf(b"BT /F1 20 Tf 10 20 Td (AB) Tj ET","Helvetica","");
    let d=Document::open(&p).unwrap();let out=d.render_page_with_standard_fonts(&d.pages().unwrap()[0],Options::default(),10000,&set).unwrap();
    assert_eq!(out.substituted_fonts,vec![StandardFont::Helvetica]);assert!(!out.bitmap.missing_text);
    assert_eq!(&out.bitmap.rgba[(65*out.bitmap.width+11)*4..(65*out.bitmap.width+11)*4+4],&[0,0,0,255]);
    for bad in [0u32,27,u32::MAX]{let mut b=wrapped.clone();b[20..24].copy_from_slice(&bad.to_be_bytes());assert!(Cff::parse_opentype(&b).is_err());}
    let mut b=wrapped.clone();b[12..16].copy_from_slice(b"CFF2");assert!(Cff::parse_opentype(&b).is_err());
    let mut b=wrapped.clone();b.pop();assert!(Cff::parse_opentype(&b).is_err());
}

#[test]fn native_cff_geometry_uses_resolved_names_not_codes(){
    let f=fixture::encoded_font(3,[34,35],2,&[0,2,66,65]);
    let p=fixture::pdf_with_font(false,b"BT /F1 20 Tf 10 20 Td (AB) Tj ET",f,"");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let g=d.page_glyph_geometry(&pages[0],10000,None).unwrap();
    assert_eq!(g.glyphs[0].unicode.as_deref(),Some("B"));
    assert_eq!(g.glyphs[1].unicode.as_deref(),Some("A"));
    assert_eq!(g.glyphs[0].embedded_glyph_name.as_deref(),Some(b"B".as_slice()));
    assert!(g.glyphs.iter().all(|x|x.unicode_from_embedded_name));
    let f=fixture::encoded_font(3,[34,35],2,&[0,2,66,65]);
    let p=fixture::pdf_with_font(false,b"BT /F1 20 Tf 10 20 Td (AB) Tj ET",f,"/Encoding /StandardEncoding");
    let d=Document::open(&p).unwrap();
    let g=d.page_glyph_geometry(&d.pages().unwrap()[0],10000,None).unwrap();
    assert_eq!(g.glyphs[0].unicode.as_deref(),Some("A"));
    assert!(!g.glyphs[0].unicode_from_embedded_name);
    let p=fixture::pdf(true,b"BT /F1 20 Tf 10 20 Td <002a> Tj ET");
    let d=Document::open(&p).unwrap();let g=d.page_glyph_geometry(&d.pages().unwrap()[0],10000,None).unwrap();
    assert_eq!(g.glyphs[0].unicode,None);assert_eq!(g.glyphs[0].embedded_glyph_name,None);
}
