use rwcore::*;
const FONT: &[u8] = include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
fn renderer() -> TrueTypeRenderer<'static> {
    let c = parse_font(FONT, FontLimits::default()).unwrap();
    parse_true_type_renderer(c.face(0).unwrap(), RenderFontLimits::default()).unwrap()
}
#[test]
fn real_font_directory_and_checksums() {
    let c = parse_font(FONT, FontLimits::default()).unwrap();
    let f = c.face(0).unwrap();
    assert_eq!(FONT.len(), 46016);
    assert_eq!(f.tables().len(), 17);
    for t in f.tables() { assert!(f.verify_table_checksum(t.tag).unwrap(), "{:?}", t.tag); }
}
#[test]
fn real_font_nominal_metrics() {
    let r = renderer();
    assert_eq!(r.metrics().head().units_per_em, 1000);
    for (ch,id,advance,bearing) in [('A',1,668,10),('g',86,585,45),('é',81,544,45),('Å',6,668,10),(' ',161,300,0),('O',39,761,50)] {
        let g = r.render_char(ch, MissingGlyphPolicy::Reject, GlyphRenderOptions{ppem_26_6:6400,..Default::default()}).unwrap();
        assert_eq!((g.glyph_id,g.metric.advance_width,g.metric.left_bearing),(id,advance,bearing));
    }
}
#[test]
fn real_font_space_is_empty() {
    let g=renderer().render_char(' ',MissingGlyphPolicy::Reject,GlyphRenderOptions::default()).unwrap();
    assert!(g.bitmap.alpha.is_empty());
    assert!(g.advance_26_6>0);
}
#[test]
fn real_simple_and_composite_glyphs_render() {
    let r=renderer();
    for ch in ['A','g','é','Å','O'] {
        let g=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions::default()).unwrap();
        assert!(g.bitmap.alpha.iter().any(|&a|a>0), "{ch}");
        assert!(g.bitmap.alpha.iter().any(|&a|a<255), "{ch}");
    }
}
#[test]
fn real_font_integer_translation() {
    let r=renderer();
    for ch in ['A','é','g'] {
        let a=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions::default()).unwrap();
        let b=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions{origin_x_26_6:192,origin_y_26_6:-128,..Default::default()}).unwrap();
        assert_eq!(b.bitmap.viewport.left,a.bitmap.viewport.left+3);
        assert_eq!(b.bitmap.viewport.top,a.bitmap.viewport.top-2);
        assert_eq!(a.bitmap.alpha,b.bitmap.alpha);
    }
}
