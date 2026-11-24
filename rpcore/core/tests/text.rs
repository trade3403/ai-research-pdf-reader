use rpcore::{Document,Error,text::{Matrix,TextWarning}};
fn fixture(content:&[u8])->Vec<u8>{
    let mut body=format!("<< /Length {} >>\nstream\n",content.len()).into_bytes();body.extend_from_slice(content);body.extend_from_slice(b"\nendstream");
    let objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> /ExtGState << /GS << /ca 0.5 /BM /Multiply >> /Bad << /Font [4 0 R 12] >> >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /uni4E2D /fi /unknown] >> /FontDescriptor << /MissingWidth 500 >> >>".to_vec(),body];
    let mut pdf=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for (i,o) in objects.iter().enumerate(){offsets.push(pdf.len());pdf.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());pdf.extend_from_slice(o);pdf.extend_from_slice(b"\nendobj\n");}
    let x=pdf.len();pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offsets{pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());pdf
}
fn extract(data:&[u8])->rpcore::text::PageText{
    let b=fixture(data);let d=Document::open(&b).unwrap();d.page_text(&d.pages().unwrap()[0],10000).unwrap()
}
fn near(a:[f64;2],b:[f64;2]){assert!((a[0]-b[0]).abs()<1e-9&&(a[1]-b[1]).abs()<1e-9,"{a:?} != {b:?}");}
#[test]
fn text_positions_and_unicode_search(){
    let t=extract(b"BT /F1 10 Tf 1 0 0 1 100 200 Tm 2 Tc 3 Tw 80 Tz 4 Ts [(A) -200 (B C)] TJ ET");
    assert_eq!(t.plain_text(),"中fi \u{fffd}");
    near(t.glyphs[0].origin,[100.,204.]);
    near(t.glyphs[0].advance_end,[105.6,204.]);
    near(t.glyphs[1].origin,[107.2,204.]);
    near(t.glyphs[2].origin,[112.8,204.]);
    near(t.glyphs[3].origin,[120.8,204.]);
    let r=t.search("中f").unwrap();assert_eq!((r[0].glyph_start,r[0].glyph_end),(0,2));
    let r=t.search("i").unwrap();assert_eq!((r[0].glyph_start,r[0].glyph_end),(1,2));
    assert!(t.search("\u{fffd}").unwrap().is_empty());
    assert_eq!(t.select([100.,204.,103.,210.]).unwrap(),vec![0]);
}
#[test]
fn matrix_and_graphics_state(){
    let t=extract(b"q 2 0 0 3 10 20 cm BT /F1 10 Tf 0 1 -1 0 5 7 Tm (AA) Tj ET Q BT /F1 10 Tf 1 0 0 1 2 3 Tm (A) Tj ET");
    near(t.glyphs[0].origin,[20.,41.]);near(t.glyphs[1].origin,[20.,56.]);near(t.glyphs[2].origin,[2.,3.]);
    near(Matrix::translation(1.,2.).compose(Matrix([2.,0.,0.,3.,0.,0.])).unwrap().point([4.,5.]).unwrap(),[9.,17.]);
}
#[test]
fn line_and_quote_operators(){
    let t=extract(b"BT /F1 10 Tf 10 100 Td 20 TL (A) Tj (B) ' 2 1 (A A) \" 5 -30 TD (B) Tj T* (A) Tj ET");
    near(t.glyphs[0].origin,[10.,100.]);near(t.glyphs[1].origin,[10.,80.]);near(t.glyphs[2].origin,[10.,60.]);
    near(t.glyphs[3].origin,[16.,60.]);near(t.glyphs[4].origin,[24.,60.]);near(t.glyphs[5].origin,[15.,30.]);near(t.glyphs[6].origin,[15.,0.]);
}
#[test]
fn unknown_and_invisible_text(){
    let t=extract(b"/Image1 Do BT /F1 10 Tf 3 Tr (A) Tj /Span << /ActualText (real) >> BDC (B) Tj EMC ET");
    assert_eq!(t.glyphs[0].rendering_mode,3);
    assert!(t.warnings.contains(&TextWarning::ExternalObjectNotInterpreted));
    assert!(t.warnings.contains(&TextWarning::ActualTextNotInterpreted));
}
#[test]
fn malformed_text_rejected(){
    for data in [b"BT /F1 10 Tf (A) Tj".as_slice(),b"Q",b"BT BT ET",b"BT (A) Tj ET",b"BT /F1 10 Tf [true] TJ ET",b"BT /F1 10 Tf 8 Tr ET",b"/GS1 gs"]{
        let b=fixture(data);let d=Document::open(&b).unwrap();
        assert!(matches!(d.page_text(&d.pages().unwrap()[0],10000),Err(Error::Invalid(_)|Error::Unsupported(_))));
    }
}

#[test]fn ext_gstate_preserves_text_geometry(){
    let t=extract(b"BT /F1 10 Tf (A) Tj /GS gs (B) Tj ET");
    near(t.glyphs[1].origin,[5.,0.]);
    assert!(t.warnings.contains(&TextWarning::GraphicsStateNotInterpreted));
    let b=fixture(b"/Bad gs");let d=Document::open(&b).unwrap();
    assert!(matches!(d.page_text(&d.pages().unwrap()[0],10000),Err(Error::Unsupported("ExtGState text font"))));
}
