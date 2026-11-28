use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
fn check(id:u16,ends:&[usize],points:&[(i64,i64,bool)]){
    let c=parse_font(FONT,FontLimits::default()).unwrap();
    let o=parse_true_type_outlines(c.face(0).unwrap(),OutlineLimits::default()).unwrap();
    let g=assemble_true_type_glyph(&o,id,CompositeLimits::default()).unwrap();
    assert_eq!(g.contour_ends,ends);
    let expected:Vec<_>=points.iter().map(|&(x,y,on)|(x*COMPOSITE_ONE,y*COMPOSITE_ONE,on)).collect();
    assert_eq!(g.points.iter().map(|p|(p.x,p.y,p.on_curve)).collect::<Vec<_>>(),expected);
}
#[test] fn composite_6(){check(6,&[7,10,24,36],&[(571,-10,true),(493,190,true),(169,190,true),(91,-8,true),(10,25,true),(284,700,true),(388,700,true),(658,25,true),(199,270,true),(462,270,true),(331,606,true),(334,638,true),(282,638,false),(209,706,false),(209,758,true),(209,811,false),(282,878,false),(334,878,true),(368,878,false),(425,847,false),(459,793,false),(459,758,true),(459,723,false),(425,669,false),(368,638,false),(334,700,true),(355,700,false),(382,731,false),(382,758,true),(382,781,false),(357,816,false),(334,816,true),(312,816,false),(286,787,false),(286,758,true),(286,734,false),(312,700,false)]);}
#[test] fn composite_81(){check(81,&[23,30,34],&[(288,-12,true),(176,-12,false),(45,131,false),(45,260,true),(45,338,false),(104,461,false),(210,532,false),(280,532,true),(355,532,false),(454,467,false),(504,357,false),(504,291,true),(504,256,false),(501,223,true),(137,223,true),(146,150,false),(230,68,false),(299,68,true),(344,68,false),(416,87,false),(449,102,true),(470,25,true),(434,9,false),(344,-12,false),(136,296,true),(421,296,true),(421,372,false),(345,454,false),(278,454,true),(220,454,false),(144,373,false),(197,575,true),(166,636,true),(355,761,true),(397,677,true)]);}
