use rpcore::{Document,fonts::{FontKind,WritingMode}};
fn pdf(objects:&[Vec<u8>])->Vec<u8>{
    let mut b=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for (i,o) in objects.iter().enumerate(){offsets.push(b.len());b.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());b.extend(o);b.extend_from_slice(b"\nendobj\n");}
    let x=b.len();b.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offsets{b.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    b.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());b
}
fn stream(data:&[u8])->Vec<u8>{
    let mut b=format!("<< /Length {} >>\nstream\n",data.len()).into_bytes();b.extend(data);b.extend_from_slice(b"\nendstream");b
}
fn fixture(extra:Vec<Vec<u8>>)->Vec<u8>{
    let mut objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> >>".to_vec(),b"<< /Type /Page /Parent 2 0 R >>".to_vec()];
    objects.extend(extra);pdf(&objects)
}
#[test]
fn simple_encoding_and_widths(){
    let b=fixture(vec![b"<< /Type /Font /Subtype /TrueType /BaseFont /Example /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /uni4E2D /fi /unknown] >> /FirstChar 65 /LastChar 67 /Widths [600 500 400] /FontDescriptor << /MissingWidth 250 >> >>".to_vec()]);
    let d=Document::open(&b).unwrap();let p=d.pages().unwrap();
    let f=d.page_font(&p[0],b"F1").unwrap();assert_eq!(f.kind,FontKind::TrueType);
    let g=f.decode(&[65,66,67,128]).unwrap();
    assert_eq!(g[0].unicode.as_deref(),Some("中"));assert_eq!(g[1].unicode.as_deref(),Some("fi"));assert_eq!(g[2].unicode,None);assert_eq!(g[3].unicode.as_deref(),Some("€"));
    assert_eq!(g.iter().map(|g|g.width).collect::<Vec<_>>(),vec![Some(600.),Some(500.),Some(400.),Some(250.)]);
}
#[test]
fn identity_cid_unicode_and_gid(){
    let b=fixture(vec![
        b"<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /DescendantFonts [5 0 R] /ToUnicode 6 0 R >>".to_vec(),
        b"<< /Subtype /CIDFontType2 /DW 1000 /W [1 [500 600] 3 4 750] /CIDToGIDMap 7 0 R >>".to_vec(),
        stream(b"1 begincodespacerange <0000> <ffff> endcodespacerange 2 beginbfchar <0001> <4e2d> <0002> <00660069> endbfchar"),
        stream(&[0,0,0,12,0,13,0,14,0,15]),
    ]);
    let d=Document::open(&b).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();
    assert_eq!(f.kind,FontKind::Type0Cid2);assert_eq!(f.writing_mode,WritingMode::Horizontal);
    let g=f.decode(&[0,1,0,2,0,3,0,5]).unwrap();
    assert_eq!(g[0].cid,Some(1));assert_eq!(g[0].glyph_id,Some(12));assert_eq!(g[0].unicode.as_deref(),Some("中"));assert_eq!(g[1].unicode.as_deref(),Some("fi"));
    assert_eq!(g[2].unicode,None);assert_eq!(g[2].width,Some(750.));assert_eq!(g[3].width,Some(1000.));assert_eq!(g[3].glyph_id,None);
    assert!(f.decode(&[0]).is_err());
}
#[test]
fn custom_cid_cmap_vertical(){
    let b=fixture(vec![
        b"<< /Subtype /Type0 /Encoding 6 0 R /DescendantFonts [5 0 R] /ToUnicode 7 0 R >>".to_vec(),
        b"<< /Subtype /CIDFontType0 /W [10 12 550] >>".to_vec(),
        stream(b"/WMode 1 def 2 begincodespacerange <00> <7f> <8100> <81ff> endcodespacerange 1 begincidchar <41> 10 endcidchar 1 begincidrange <8100> <8101> 11 endcidrange"),
        stream(b"2 begincodespacerange <00> <7f> <8100> <81ff> endcodespacerange 1 beginbfchar <41> <0041> endbfchar 1 beginbfrange <8100> <8101> <4e00> endbfrange"),
    ]);
    let d=Document::open(&b).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();assert_eq!(f.writing_mode,WritingMode::Vertical);
    let g=f.decode(&[65,0x81,1,66]).unwrap();
    assert_eq!(g[1].cid,Some(12));assert_eq!(g[1].unicode.as_deref(),Some("丁"));assert_eq!(g[1].width,Some(550.));assert_eq!(g[2].cid,None);assert_eq!(g[2].width,None);
}
#[test]
fn unknown_encoding_not_guessed(){
    let b=fixture(vec![b"<< /Subtype /Type1 /BaseFont /UnknownExample >>".to_vec()]);
    let d=Document::open(&b).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();
    assert_eq!(f.decode(b"A").unwrap()[0].unicode,None);
}

#[test]fn standard_fourteen_metrics_and_encoding(){
    use rpcore::fonts::standard::StandardFont;
    for face in StandardFont::ALL{
        let name=std::str::from_utf8(face.name()).unwrap();
        let p=fixture(vec![format!("<< /Subtype /Type1 /BaseFont /{name} >>").into_bytes()]);
        let d=Document::open(&p).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();
        assert_eq!(f.standard_face(),Some(face));
        let g=f.decode(b"A ").unwrap();
        assert!(g.iter().all(|g|g.width.is_some()));
        if face==StandardFont::Symbol{assert_eq!(g[0].unicode.as_deref(),Some("Α"));assert_eq!(g[0].width,Some(722.));}
        if face==StandardFont::ZapfDingbats{assert_eq!(g[0].unicode.as_deref(),Some("✡"));}
        if face==StandardFont::Helvetica{assert_eq!(g[0].width,Some(667.));assert_eq!(g[1].width,Some(278.));}
        if face==StandardFont::TimesRoman{assert_eq!(g[0].width,Some(722.));assert_eq!(g[1].width,Some(250.));}
    }
}
#[test]fn standard_differences_and_explicit_width_precedence(){
    for (extra,expected) in [("",1000.),("/FirstChar 65 /LastChar 65 /Widths [123]",123.)]{
        let p=fixture(vec![format!("<< /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /AE] >> {extra} >>").into_bytes()]);
        let d=Document::open(&p).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();
        let g=f.decode(b"A").unwrap();assert_eq!(g[0].width,Some(expected));assert_eq!(g[0].unicode.as_deref(),Some("Æ"));
        assert_eq!(f.simple_glyph_name(65),Some(b"AE".as_slice()));
    }
    let p=fixture(vec![b"<< /Subtype /Type1 /BaseFont /ABCDEF+Helvetica >>".to_vec()]);
    let d=Document::open(&p).unwrap();let f=d.page_font(&d.pages().unwrap()[0],b"F1").unwrap();
    assert_eq!(f.standard_face(),None);assert_eq!(f.decode(b"A").unwrap()[0].width,None);
}
