#[path="support/tt_fixture.rs"] mod fixture;
use rpcore::{Document,Error,fonts::truetype::TrueType,graphics::Segment,raster::{Options,Bitmap}};
fn pixel(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
#[test]
fn font_metrics_and_contours(){
    let f=TrueType::parse(fixture::font(false)).unwrap();
    assert_eq!(f.units_per_em(),1000);assert_eq!(f.glyph_count(),4);assert_eq!(f.horizontal_metrics(3).unwrap(),(650,100));
    assert!(f.outline(0).unwrap().is_empty());
    let p=f.outline(1).unwrap();assert_eq!(p.len(),10);assert_eq!(p[0],Segment::Move([0.,0.]));
    let p=f.outline(2).unwrap();assert_eq!(p[1],Segment::Cubic([0.,400.],[100.,600.],[300.,600.]));
    assert_eq!(p[2],Segment::Cubic([500.,600.],[600.,400.],[600.,0.]));
    let p=f.outline(3).unwrap();assert_eq!(p[0],Segment::Move([100.,50.]));assert_eq!(p[1],Segment::Line([400.,50.]));
    assert!(f.outline(4).is_err());
}
#[test]
fn malformed_tables_and_compound_cycle(){
    let f=TrueType::parse(fixture::font(true)).unwrap();assert!(matches!(f.outline(3),Err(Error::Invalid(_))));
    let mut b=fixture::font(false);b[20..24].copy_from_slice(&u32::MAX.to_be_bytes());assert!(TrueType::parse(b).is_err());
    let b=fixture::font(false);assert!(TrueType::parse(b[..b.len()-8].to_vec()).is_err());
}
#[test]
fn embedded_glyph_pixels_and_search(){
    let p=fixture::pdf(b"BT /F1 20 Tf 10 20 Td [<0001> -100 <0002> <0003>] TJ ET");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page(&pages[0],Options::default(),10000).unwrap();
    assert!(!b.missing_text);assert_eq!((b.width,b.height),(140,90));
    assert_eq!(pixel(&b,11,65),[0,0,0,255]);assert_eq!(pixel(&b,15,64),[255,255,255,255]);
    assert_eq!(pixel(&b,23,65),[255,255,255,255]);assert_eq!(pixel(&b,30,65),[0,0,0,255]);
    let t=d.page_text(&pages[0],10000).unwrap();assert_eq!(t.plain_text(),"ODo");assert_eq!(t.glyphs[1].origin,[25.,20.]);
}
#[test]
fn text_color_order_invisible_and_preview(){
    let p=fixture::pdf(b"1 0 0 rg BT /F1 20 Tf 10 20 Td <0001> Tj 3 Tr <0001> Tj ET 0 0 1 rg 10 20 2 2 re f");
    let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    let b=d.render_page(&pages[0],Options::default(),10000).unwrap();
    assert_eq!(pixel(&b,10,69),[0,0,255,255]);assert_eq!(pixel(&b,11,65),[255,0,0,255]);assert_eq!(pixel(&b,24,65),[255,255,255,255]);
    let preview=d.render_page(&pages[0],Options{scale:1.,allow_missing_text:true},10000).unwrap();assert!(preview.missing_text);assert_eq!(pixel(&preview,11,65),[255,255,255,255]);
}
