use rwcore::*;
fn p16(b:&mut [u8],at:usize,n:u16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn pi16(b:&mut [u8],at:usize,n:i16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn p32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_be_bytes());}
fn tables(cff:bool)->Vec<([u8;4],Vec<u8>)>{
    let mut head=vec![0;54];p32(&mut head,0,0x10000);p32(&mut head,4,0x18000);p32(&mut head,12,0x5f0f3cf5);
    p16(&mut head,16,3);p16(&mut head,18,1000);pi16(&mut head,36,-120);pi16(&mut head,38,-250);
    pi16(&mut head,40,1100);pi16(&mut head,42,850);p16(&mut head,44,1);p16(&mut head,46,8);pi16(&mut head,50,if cff{0}else{1});
    let mut maxp=vec![0;if cff{6}else{32}];p32(&mut maxp,0,if cff{0x5000}else{0x10000});p16(&mut maxp,4,4);
    if !cff{for i in 0..13{p16(&mut maxp,6+i*2,(i+1) as u16);}}
    let mut hhea=vec![0;36];p32(&mut hhea,0,0x10000);pi16(&mut hhea,4,800);pi16(&mut hhea,6,-200);
    pi16(&mut hhea,8,50);p16(&mut hhea,10,700);pi16(&mut hhea,12,-20);pi16(&mut hhea,14,-10);
    pi16(&mut hhea,16,680);pi16(&mut hhea,18,1);pi16(&mut hhea,20,0);pi16(&mut hhea,22,-2);p16(&mut hhea,34,2);
    let mut hmtx=vec![0;12];p16(&mut hmtx,0,500);pi16(&mut hmtx,2,-30);p16(&mut hmtx,4,700);
    pi16(&mut hmtx,6,12);pi16(&mut hmtx,8,-7);pi16(&mut hmtx,10,90);
    vec![(*b"head",head),(*b"maxp",maxp),(*b"hhea",hhea),(*b"hmtx",hmtx)]
}
fn build(cff:bool,t:&[([u8;4],Vec<u8>)])->Vec<u8>{
    let mut b=vec![0;12+t.len()*16];b[..4].copy_from_slice(&if cff{*b"OTTO"}else{[0,1,0,0]});
    p16(&mut b,4,t.len() as u16);
    for (i,(tag,data)) in t.iter().enumerate(){
        while b.len()%4!=0{b.push(0);}
        let at=b.len();let r=12+i*16;b[r..r+4].copy_from_slice(tag);p32(&mut b,r+8,at as u32);p32(&mut b,r+12,data.len() as u32);b.extend(data);
    }
    b
}
fn parse(b:&[u8])->Result<FontMetrics<'_>,Error>{
    let c=parse_font(b,FontLimits::default())?;
    parse_font_metrics(c.face(0).unwrap(),FontMetricLimits::default())
}
fn valid(cff:bool)->Vec<u8>{build(cff,&tables(cff))}
#[test]fn head_and_bounds(){
    let b=valid(false);let m=parse(&b).unwrap();let h=m.head();
    assert_eq!(h.units_per_em,1000);assert_eq!(h.revision_raw,0x18000);assert_eq!(h.loca_format,1);
    assert_eq!(h.bounds,FontBounds{x_min:-120,y_min:-250,x_max:1100,y_max:850});
    assert_eq!((h.flags,h.mac_style,h.lowest_rec_ppem),(3,1,8));
}
#[test]fn maxp_true_type_all_fields(){
    let b=valid(false);let m=parse(&b).unwrap();let x=m.maxp().true_type.unwrap();
    assert_eq!(m.maxp().glyph_count,4);assert_eq!((x.points,x.contours,x.composite_points,x.composite_contours),(1,2,3,4));
    assert_eq!((x.zones,x.twilight_points,x.storage,x.function_defs,x.instruction_defs,x.stack_elements,x.instruction_bytes,x.component_elements,x.component_depth),(5,6,7,8,9,10,11,12,13));
}
#[test]fn cff_maxp_half_version(){
    let b=valid(true);let m=parse(&b).unwrap();assert_eq!(m.maxp().glyph_count,4);
    assert!(m.maxp().true_type.is_none());assert_eq!(m.head().loca_format,0);
}
#[test]fn horizontal_header_signed_metrics(){
    let b=valid(false);let h=parse(&b).unwrap().horizontal_header();
    assert_eq!((h.ascender,h.descender,h.line_gap,h.advance_width_max),(800,-200,50,700));
    assert_eq!((h.min_left_bearing,h.min_right_bearing,h.x_max_extent),(-20,-10,680));
    assert_eq!((h.caret_slope_rise,h.caret_slope_run,h.caret_offset),(1,0,-2));assert_eq!(h.line_height_units(),1050);
}
#[test]fn long_and_short_metrics(){
    let b=valid(false);let m=parse(&b).unwrap();
    for (g,(w,l)) in [(500,-30),(700,12),(700,-7),(700,90)].into_iter().enumerate(){
        assert_eq!(m.glyph(g as u16).unwrap(),GlyphHorizontalMetric{advance_width:w,left_bearing:l});
    }
    assert!(m.glyph(4).is_err());assert!(m.glyph(u16::MAX).is_err());
}
#[test]fn all_long_and_single_long_metrics(){
    let mut t=tables(false);p16(&mut t[2].1,34,4);t[3].1.resize(16,0);
    p16(&mut t[3].1,8,600);pi16(&mut t[3].1,10,-9);p16(&mut t[3].1,12,800);pi16(&mut t[3].1,14,-10);
    let b=build(false,&t);let m=parse(&b).unwrap();assert_eq!(m.glyph(3).unwrap().advance_width,800);
    p16(&mut t[2].1,34,1);t[3].1.truncate(10);pi16(&mut t[3].1,8,-123);
    let b=build(false,&t);let m=parse(&b).unwrap();assert_eq!(m.glyph(3).unwrap(),GlyphHorizontalMetric{advance_width:500,left_bearing:-123});
}
#[test]fn exact_integer_scaling_twips_and_pixels(){
    let b=valid(false);let s=parse(&b).unwrap().scale(240).unwrap();
    assert_eq!(s.font_size_twips(),240);assert_eq!(s.units_per_em(),1000);
    assert_eq!(s.twips(1000).unwrap(),240);assert_eq!(s.twips(500).unwrap(),120);
    assert_eq!(s.twips(-30).unwrap(),-7);assert_eq!(s.pixels_26_6(1000,96).unwrap(),1024);
    assert_eq!(s.pixels_26_6(1,96).unwrap(),1);assert_eq!(s.twips(1).unwrap(),0);
}
#[test]fn rounding_ties_away_and_checked_scale(){
    let s=FontScale::new(1000,500).unwrap();assert_eq!(s.twips(1).unwrap(),1);assert_eq!(s.twips(-1).unwrap(),-1);
    let s=FontScale::new(128,20).unwrap();assert_eq!(s.pixels_26_6(1,72).unwrap(),1);assert_eq!(s.pixels_26_6(-1,72).unwrap(),-1);
    assert!(s.pixels_26_6(1,0).is_err());assert!(FontScale::new(15,240).is_err());assert!(FontScale::new(1000,0).is_err());
    let s=FontScale::new(16,u32::MAX).unwrap();assert!(s.twips(i64::MAX).is_err());assert!(s.pixels_26_6(i64::MIN,u32::MAX).is_err());
}
#[test]fn rejects_missing_required_tables(){
    for i in 0..4{let mut t=tables(false);t.remove(i);assert!(parse(&build(false,&t)).is_err());}
}
#[test]fn rejects_bad_head_fields(){
    for (at,value) in [(18,15),(18,16385),(50,2),(52,1)]{
        let mut t=tables(false);p16(&mut t[0].1,at,value);assert!(parse(&build(false,&t)).is_err());
    }
    let mut t=tables(false);p32(&mut t[0].1,12,0);assert!(parse(&build(false,&t)).is_err());
    let mut t=tables(false);pi16(&mut t[0].1,36,1200);assert!(parse(&build(false,&t)).is_err());
    let mut t=tables(true);pi16(&mut t[0].1,50,1);assert!(parse(&build(true,&t)).is_err());
}
#[test]fn rejects_flavor_versions_and_glyph_count(){
    let mut t=tables(false);p32(&mut t[1].1,0,0x5000);assert!(parse(&build(false,&t)).is_err());
    let mut t=tables(true);p32(&mut t[1].1,0,0x10000);assert!(parse(&build(true,&t)).is_err());
    let mut t=tables(false);p16(&mut t[1].1,4,0);assert!(parse(&build(false,&t)).is_err());
    let b=valid(false);let c=parse_font(&b,FontLimits::default()).unwrap();
    assert!(parse_font_metrics(c.face(0).unwrap(),FontMetricLimits{max_glyphs:3}).is_err());
}
#[test]fn rejects_bad_hhea_count_and_format(){
    for (at,v) in [(34,0),(34,5),(32,1)]{let mut t=tables(false);p16(&mut t[2].1,at,v);assert!(parse(&build(false,&t)).is_err());}
    let mut t=tables(false);p32(&mut t[2].1,0,0x20000);assert!(parse(&build(false,&t)).is_err());
}
#[test]fn rejects_every_truncated_metric_table_and_extra_byte(){
    for i in 0..4{
        let full=tables(false);for len in 0..full[i].1.len(){let mut t=full.clone();t[i].1.truncate(len);assert!(parse(&build(false,&t)).is_err(),"table {i}, len {len}");}
        let mut t=full;t[i].1.push(0);assert!(parse(&build(false,&t)).is_err());
    }
}
#[test]fn non_power_of_two_units_and_extreme_signed_metric(){
    for n in [16,2048,16384,999]{
        let mut t=tables(false);p16(&mut t[0].1,18,n);pi16(&mut t[3].1,2,i16::MIN);p16(&mut t[3].1,0,u16::MAX);
        let b=build(false,&t);let m=parse(&b).unwrap();assert_eq!(m.head().units_per_em,n);
        assert_eq!(m.glyph(0).unwrap(),GlyphHorizontalMetric{advance_width:u16::MAX,left_bearing:i16::MIN});
    }
}
