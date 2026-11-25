use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf");
fn check(id:u16,ends:&[usize],points:&[(i16,i16,bool)]){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let o=parse_true_type_outlines(c.face(0).unwrap(),OutlineLimits::default()).unwrap();
    let g=o.simple_glyph(id).unwrap();
    assert_eq!(g.contour_ends(),ends);
    assert_eq!(g.points().iter().map(|p|(p.x,p.y,p.on_curve)).collect::<Vec<_>>(),points);
}
#[test] fn italic_a_points(){check(1,&[7,10],&[(-28,37,true),(356,700,true),(463,700,true),(616,11,true),(525,-10,true),(483,190,true),(159,190,true),(48,-8,true),(204,270,true),(466,270,true),(395,607,true)]);}
#[test] fn italic_g_points(){check(86,&[29,45],&[(367,532,true),(426,532,false),(516,508,false),(545,496,true),(458,-6,true),(446,-75,false),(397,-163,false),(332,-212,false),(260,-232,false),(225,-232,true),(157,-232,false),(62,-176,false),(32,-139,true),(102,-87,true),(151,-154,false),(231,-154,true),(281,-154,false),(357,-80,false),(373,9,true),(390,107,true),(362,62,false),(279,0,false),(227,0,true),(179,0,false),(92,48,false),(37,140,false),(37,207,true),(37,304,false),(123,450,false),(273,532,false),(129,217,true),(129,172,false),(164,111,false),(220,80,false),(251,80,true),(289,80,false),(357,134,false),(408,232,false),(420,298,true),(446,443,true),(427,449,false),(385,456,false),(357,456,true),(290,456,false),(187,393,false),(129,285,false)]);}
#[test] fn italic_o_points(){check(39,&[17,35],&[(339,-12,true),(245,-12,false),(115,70,false),(47,209,false),(47,295,true),(47,375,false),(97,524,false),(194,643,false),(334,712,false),(424,712,true),(518,712,false),(648,630,false),(716,491,false),(716,405,true),(716,325,false),(666,176,false),(569,57,false),(429,-12,false),(416,630,true),(352,630,false),(251,576,false),(181,482,false),(144,364,false),(144,301,true),(144,237,false),(190,132,false),(281,70,false),(347,70,true),(411,70,false),(512,124,false),(582,218,false),(619,336,false),(619,399,true),(619,463,false),(573,568,false),(483,630,false)]);}
#[test] fn italic_metrics_and_render(){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let r=parse_true_type_renderer(c.face(0).unwrap(),RenderFontLimits::default()).unwrap();
    for (ch,id,advance,bearing) in [('A',1,663,-28),('g',86,577,32),('O',39,738,47),('é',81,518,37),(' ',161,300,0)]{
        let g=r.render_char(ch,MissingGlyphPolicy::Reject,GlyphRenderOptions::default()).unwrap();
        assert_eq!((g.glyph_id,g.metric.advance_width,g.metric.left_bearing),(id,advance,bearing));
        assert_eq!(g.bitmap.alpha.iter().any(|&a|a>0),ch!=' ');
    }
}
