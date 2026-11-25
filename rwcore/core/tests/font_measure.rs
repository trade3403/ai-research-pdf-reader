use rwcore::*;
fn p16(b:&mut [u8],at:usize,n:u16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn pi16(b:&mut [u8],at:usize,n:i16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn p32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_be_bytes());}
fn font(cff:bool,with_cmap:bool)->Vec<u8>{
    let mut head=vec![0;54];p32(&mut head,0,0x10000);p32(&mut head,12,0x5f0f3cf5);p16(&mut head,18,1000);
    let mut maxp=vec![0;if cff{6}else{32}];p32(&mut maxp,0,if cff{0x5000}else{0x10000});p16(&mut maxp,4,7);
    let mut hhea=vec![0;36];p32(&mut hhea,0,0x10000);pi16(&mut hhea,4,800);pi16(&mut hhea,6,-200);pi16(&mut hhea,8,50);p16(&mut hhea,34,6);
    let mut hmtx=vec![0;26];
    for (i,(w,l)) in [(400,-20),(333,-10),(500,12),(250,0),(1000,-25),(0,5)].into_iter().enumerate(){
        p16(&mut hmtx,i*4,w);pi16(&mut hmtx,i*4+2,l);
    }
    pi16(&mut hmtx,24,-17);
    let groups=[(0x41,0x43,1),(0x301,0x301,5),(0x4e2d,0x4e2d,4),(0x1f600,0x1f600,6)];
    let mut cmap=vec![0;12+16+groups.len()*12];p16(&mut cmap,2,1);p16(&mut cmap,4,3);p16(&mut cmap,6,10);p32(&mut cmap,8,12);
    p16(&mut cmap,12,12);p32(&mut cmap,16,(16+groups.len()*12) as u32);p32(&mut cmap,24,groups.len() as u32);
    for (i,(a,z,g)) in groups.into_iter().enumerate(){p32(&mut cmap,28+i*12,a);p32(&mut cmap,32+i*12,z);p32(&mut cmap,36+i*12,g);}
    let mut tables=vec![(*b"head",head),(*b"maxp",maxp),(*b"hhea",hhea),(*b"hmtx",hmtx)];
    if with_cmap{tables.push((*b"cmap",cmap));}
    let mut b=vec![0;12+tables.len()*16];b[..4].copy_from_slice(&if cff{*b"OTTO"}else{[0,1,0,0]});p16(&mut b,4,tables.len() as u16);
    for (i,(tag,data)) in tables.into_iter().enumerate(){
        while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);p32(&mut b,p+8,at);p32(&mut b,p+12,data.len() as u32);b.extend(data);
    }
    b
}
fn parse(b:&[u8])->Result<NominalFont<'_>,Error>{
    let c=parse_font(b,FontLimits::default())?;
    parse_nominal_font(c.face(0).unwrap(),FontMetricLimits::default(),CmapLimits::default())
}
fn opts()->MeasureOptions{MeasureOptions{font_size_twips:240,unit:MeasureUnit::Twips,missing:MissingGlyphPolicy::Reject,limits:MeasureLimits::default()}}
#[test]fn mixed_unicode_utf8_offsets_and_glyph_ids(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure("A中😀B",opts()).unwrap();
    assert_eq!(m.glyphs.iter().map(|g|(g.glyph_id,g.byte_start,g.byte_end)).collect::<Vec<_>>(),vec![(1,0,1),(4,1,4),(6,4,8),(2,8,9)]);
    assert_eq!(m.total_units,1833);assert_eq!(m.width,440);assert_eq!(m.missing_count,0);
}
#[test]fn cumulative_rounding_prevents_drift(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure(&"A".repeat(100),MeasureOptions{font_size_twips:20,..opts()}).unwrap();
    assert_eq!(m.total_units,33300);assert_eq!(m.width,666);
    assert_eq!(m.glyphs.iter().map(|g|g.end-g.start).sum::<i64>(),666);
    assert!(m.glyphs.windows(2).all(|p|p[0].end==p[1].start&&p[0].end_units==p[1].start_units));
}
#[test]fn fixed_pixel_scaling_without_twip_rounding(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure("A",MeasureOptions{font_size_twips:20,unit:MeasureUnit::Pixels26_6{dpi:96},..opts()}).unwrap();
    assert_eq!(m.width,28);assert_eq!(m.glyphs[0].left_bearing,-1);
    assert_eq!(m.unit,MeasureUnit::Pixels26_6{dpi:96});
}
#[test]fn glyph_metrics_and_repeated_final_advance(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure("A😀",opts()).unwrap();
    assert_eq!(m.glyphs[0].metric,GlyphHorizontalMetric{advance_width:333,left_bearing:-10});
    assert_eq!(m.glyphs[0].left_bearing,-2);
    assert_eq!(m.glyphs[1].metric,GlyphHorizontalMetric{advance_width:0,left_bearing:-17});
    assert_eq!(m.glyphs[1].start,m.glyphs[1].end);
}
#[test]fn missing_policy_reject_or_notdef(){
    let b=font(false,true);let f=parse(&b).unwrap();assert!(f.measure("AZ",opts()).is_err());
    let m=f.measure("AZ?",MeasureOptions{missing:MissingGlyphPolicy::Notdef,..opts()}).unwrap();
    assert_eq!(m.missing_count,2);assert_eq!(m.total_units,1133);
    assert_eq!(m.glyphs.iter().map(|g|(g.glyph_id,g.missing)).collect::<Vec<_>>(),vec![(1,false),(0,true),(0,true)]);
}
#[test]fn zero_advance_combining_scalar_not_shaping(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure("A\u{301}",opts()).unwrap();
    assert_eq!(m.glyphs.len(),2);assert_eq!(m.glyphs[1].glyph_id,5);
    assert_eq!((m.glyphs[1].byte_start,m.glyphs[1].byte_end),(1,3));assert_eq!(m.glyphs[1].start,m.glyphs[1].end);
}
#[test]fn line_metrics_signed_and_scaled(){
    let b=font(false,true);let f=parse(&b).unwrap();let m=f.measure("",opts()).unwrap();
    assert_eq!(m.line,NominalLineMetrics{ascender:192,descender:-48,line_gap:12,line_height:252});
    assert_eq!(m.width,0);assert_eq!(m.total_units,0);assert!(m.glyphs.is_empty());
}
#[test]fn empty_input_still_validates_scale(){
    let b=font(false,true);let f=parse(&b).unwrap();
    assert!(f.measure("",MeasureOptions{font_size_twips:0,..opts()}).is_err());
    assert!(f.measure("",MeasureOptions{unit:MeasureUnit::Pixels26_6{dpi:0},..opts()}).is_err());
}
#[test]fn rejects_control_and_line_separators(){
    let b=font(false,true);let f=parse(&b).unwrap();
    for s in ["A\tB","A\nB","\r","\u{0}","\u{85}","\u{2028}","\u{2029}"]{
        assert!(matches!(f.measure(s,MeasureOptions{missing:MissingGlyphPolicy::Notdef,..opts()}),Err(Error::Unsupported(_))));
    }
}
#[test]fn byte_and_scalar_budgets(){
    let b=font(false,true);let f=parse(&b).unwrap();
    assert!(f.measure("中",MeasureOptions{limits:MeasureLimits{max_text_bytes:2,max_scalars:1},..opts()}).is_err());
    assert!(f.measure("AB",MeasureOptions{limits:MeasureLimits{max_text_bytes:2,max_scalars:1},..opts()}).is_err());
    assert!(f.measure("中",MeasureOptions{limits:MeasureLimits{max_text_bytes:3,max_scalars:1},..opts()}).is_ok());
    assert!(f.measure("",MeasureOptions{limits:MeasureLimits{max_text_bytes:0,max_scalars:0},..opts()}).is_ok());
}
#[test]fn cff_uses_same_nominal_measurement(){
    let a=font(false,true);let b=font(true,true);
    assert_eq!(parse(&a).unwrap().measure("ABC中😀",opts()).unwrap(),parse(&b).unwrap().measure("ABC中😀",opts()).unwrap());
}
#[test]fn requires_both_mapping_and_metrics(){
    let b=font(false,false);assert!(parse(&b).is_err());
    let b=font(false,true);let c=parse_font(&b,FontLimits::default()).unwrap();
    assert!(parse_nominal_font(c.face(0).unwrap(),FontMetricLimits{max_glyphs:6},CmapLimits::default()).is_err());
    assert!(parse_nominal_font(c.face(0).unwrap(),FontMetricLimits::default(),CmapLimits{max_groups:3,..Default::default()}).is_err());
}
#[test]fn checked_device_scale_overflow(){
    let b=font(false,true);let f=parse(&b).unwrap();
    assert!(f.measure(&"中".repeat(12),MeasureOptions{font_size_twips:u32::MAX,unit:MeasureUnit::Pixels26_6{dpi:u32::MAX},..opts()}).is_err());
}
#[test]fn logical_order_and_no_implicit_normalization(){
    let b=font(false,true);let f=parse(&b).unwrap();
    let m=f.measure("中BA",opts()).unwrap();assert_eq!(m.glyphs.iter().map(|g|g.character).collect::<String>(),"中BA");
    assert!(f.measure("Á",opts()).is_err());assert!(f.measure("A\u{301}",opts()).is_ok());
}
