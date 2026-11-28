use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abel/Abel-Regular.ttf");
#[test] fn abel_directory_checksums(){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let f=c.face(0).unwrap();
    assert_eq!(FONT.len(),35220);
    assert_eq!(f.tables().len(),14);
    for t in f.tables(){assert!(f.verify_table_checksum(t.tag).unwrap(),"{:?}",t.tag);}
}
#[test] fn abel_independent_metrics(){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let r=parse_true_type_renderer(c.face(0).unwrap(),RenderFontLimits::default()).unwrap();
    assert_eq!(r.metrics().head().units_per_em,2048);
    for (ch,id,advance,bearing) in [('A',36,950,20),('g',74,930,123),('O',50,1094,143),('é',170,930,123),(' ',3,471,0)] {
        let g=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions::default()).unwrap();
        assert_eq!((g.glyph_id,g.metric.advance_width,g.metric.left_bearing),(id,advance,bearing));
        assert_eq!(g.bitmap.alpha.iter().any(|&a|a>0),ch!=' ');
    }
}
