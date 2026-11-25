use rwcore::*;
fn p16(b:&mut [u8],at:usize,n:u16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn p32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_be_bytes());}
fn w(b:&mut Vec<u8>,v:i16){b.extend(v.to_be_bytes());}
fn square()->Vec<u8>{
    let mut b=Vec::new();for v in [1,0,0,500,500,3,0]{w(&mut b,v);}
    b.extend([1,1,1,1]);for v in [0,500,0,-500,0,0,500,0]{w(&mut b,v);}b
}
fn composite()->Vec<u8>{
    let mut b=Vec::new();for v in [-1,100,100,350,350,0x20b,1,100,100,8192]{w(&mut b,v);}b
}
fn font()->Vec<u8>{
    let mut head=vec![0;54];p32(&mut head,0,0x10000);p32(&mut head,12,0x5f0f3cf5);p16(&mut head,18,1000);p16(&mut head,50,1);
    let mut maxp=vec![0;32];p32(&mut maxp,0,0x10000);p16(&mut maxp,4,4);
    let mut hhea=vec![0;36];p32(&mut hhea,0,0x10000);p16(&mut hhea,34,4);
    let mut hmtx=Vec::new();for v in [600,0,700,-50,300,0,450,100]{w(&mut hmtx,v);}
    let groups=[(32u32,32u32,2u32),(65,66,1),(0x1f600,0x1f600,1)];
    // A -> 1; B explicitly mapped to composite ID 3; space -> 2.
    let groups=[groups[0],(65,65,1),(66,66,3),groups[2]];
    let mut cmap=vec![0;28+groups.len()*12];p16(&mut cmap,2,1);p16(&mut cmap,4,3);p16(&mut cmap,6,10);p32(&mut cmap,8,12);
    p16(&mut cmap,12,12);p32(&mut cmap,16,(16+groups.len()*12) as u32);p32(&mut cmap,24,groups.len() as u32);
    for (i,(a,z,g)) in groups.into_iter().enumerate(){p32(&mut cmap,28+i*12,a);p32(&mut cmap,32+i*12,z);p32(&mut cmap,36+i*12,g);}
    let mut glyf=Vec::new();let mut loca=Vec::new();
    for g in [square(),square(),vec![],composite()]{loca.extend((glyf.len() as u32).to_be_bytes());glyf.extend(g);}
    loca.extend((glyf.len() as u32).to_be_bytes());
    let tables=[(*b"head",head),(*b"maxp",maxp),(*b"hhea",hhea),(*b"hmtx",hmtx),(*b"cmap",cmap),(*b"loca",loca),(*b"glyf",glyf)];
    let mut b=vec![0;12+tables.len()*16];b[..4].copy_from_slice(&[0,1,0,0]);p16(&mut b,4,tables.len() as u16);
    for (i,(tag,data)) in tables.into_iter().enumerate(){
        while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);p32(&mut b,p+8,at);p32(&mut b,p+12,data.len() as u32);b.extend(data);
    }b
}
fn parse(b:&[u8])->TrueTypeRenderer<'_>{
    let c=parse_font(b,FontLimits::default()).unwrap();parse_true_type_renderer(c.face(0).unwrap(),RenderFontLimits::default()).unwrap()
}
fn opts()->GlyphRenderOptions{GlyphRenderOptions{ppem_26_6:1280,..Default::default()}}
fn vp(left:i32,top:i32,width:u32,height:u32)->RasterViewport{RasterViewport{left,top,width,height}}

fn text_opts()->TextRenderOptions{TextRenderOptions{glyph:opts(),..Default::default()}}
#[test]fn two_glyphs_and_transparent_gap(){
    let b=font();let r=parse(&b);let t=render_nominal_text(&r,"AA",text_opts()).unwrap();
    assert_eq!(t.bitmap.viewport,vp(0,-10,24,10));assert_eq!(t.advance_26_6,1792);assert_eq!(t.total_units,1400);
    assert_eq!(t.glyphs[1].origin_x_26_6,896);assert_eq!(t.glyphs[1].end_x_26_6,1792);
    for row in t.bitmap.alpha.chunks_exact(24){assert_eq!(&row[..10],&[255;10]);assert_eq!(&row[10..14],&[0;4]);assert_eq!(&row[14..],&[255;10]);}
}
#[test]fn space_and_composite_placement(){
    let b=font();let r=parse(&b);let t=render_nominal_text(&r,"A B",text_opts()).unwrap();
    assert_eq!(t.total_units,1450);assert_eq!(t.advance_26_6,1856);assert_eq!(t.bitmap.viewport,vp(0,-10,27,10));
    assert_eq!(t.glyphs[1].viewport.width,0);assert_eq!(t.glyphs[2].origin_x_26_6,1280);
    assert_eq!(t.glyphs[2].viewport,vp(22,-7,5,5));assert_eq!(t.glyphs[2].metrics_source,1);
    assert_eq!(t.bitmap.pixel(22,3),Some(255));assert_eq!(t.bitmap.pixel(22,2),Some(0));
}
#[test]fn utf8_and_missing_policy(){
    let b=font();let r=parse(&b);assert!(render_nominal_text(&r,"A😀Z",text_opts()).is_err());
    let t=render_nominal_text(&r,"A😀Z",TextRenderOptions{missing:MissingGlyphPolicy::Notdef,..text_opts()}).unwrap();
    assert_eq!(t.glyphs.iter().map(|g|(g.byte_start,g.byte_end,g.glyph_id,g.missing)).collect::<Vec<_>>(),
        vec![(0,1,1,false),(1,5,1,false),(5,6,0,true)]);assert_eq!(t.missing_count,1);
}
#[test]fn cumulative_rounding_not_individual_advance(){
    let b=font();let r=parse(&b);let o=TextRenderOptions{glyph:GlyphRenderOptions{ppem_26_6:64,..opts()},..text_opts()};
    let t=render_nominal_text(&r,"AAAAAAAAAA",o).unwrap();
    assert_eq!(t.advance_26_6,448);assert_eq!(t.glyphs[2].origin_x_26_6,90);
    assert_eq!(t.glyphs[3].origin_x_26_6,134);
    assert!(t.glyphs.windows(2).all(|w|w[0].end_x_26_6==w[1].origin_x_26_6&&w[0].end_units==w[1].start_units));
}
#[test]fn negative_fractional_baseline(){
    let b=font();let r=parse(&b);
    let t=render_nominal_text(&r,"A",TextRenderOptions{glyph:GlyphRenderOptions{origin_x_26_6:-32,origin_y_26_6:-32,..opts()},..text_opts()}).unwrap();
    assert_eq!(t.bitmap.viewport,vp(-1,-11,11,11));assert_eq!(t.bitmap.pixel(0,0),Some(64));
    assert_eq!(t.end_x_26_6,864);assert_eq!(t.advance_26_6,896);
}
#[test]fn empty_text_and_spaces_preserve_advance(){
    let b=font();let r=parse(&b);let o=TextRenderOptions{glyph:GlyphRenderOptions{origin_x_26_6:-1,origin_y_26_6:95,..opts()},..text_opts()};
    let t=render_nominal_text(&r,"",o).unwrap();assert_eq!(t.bitmap.viewport,vp(-1,1,0,0));assert_eq!(t.advance_26_6,0);
    let t=render_nominal_text(&r,"  ",o).unwrap();assert_eq!(t.bitmap.viewport,vp(-1,1,0,0));assert!(t.bitmap.alpha.is_empty());
    assert_eq!(t.advance_26_6,768);assert_eq!(t.glyphs.len(),2);
}
#[test]fn invalid_options_even_for_empty_text(){
    let b=font();let r=parse(&b);
    for glyph in [GlyphRenderOptions{ppem_26_6:0,..opts()},GlyphRenderOptions{samples_per_axis:3,..opts()},
        GlyphRenderOptions{tolerance_16_16:0,..opts()},GlyphRenderOptions{origin_x_26_6:i64::MAX,..opts()}]{
        assert!(render_nominal_text(&r,"",TextRenderOptions{glyph,..text_opts()}).is_err());
    }
}
#[test]fn zero_advance_overlap_source_over(){
    let mut b=font();let at=u32::from_be_bytes(b[68..72].try_into().unwrap()) as usize;
    // hmtx record is fourth (directory offset 60); zero A advance.
    p16(&mut b,at+4,0);
    let r=parse(&b);let o=TextRenderOptions{glyph:GlyphRenderOptions{origin_x_26_6:32,origin_y_26_6:32,..opts()},..text_opts()};
    let t=render_nominal_text(&r,"AA",o).unwrap();assert_eq!(t.advance_26_6,0);
    assert_eq!(t.glyphs[0].viewport,t.glyphs[1].viewport);assert_eq!(t.bitmap.pixel(0,0),Some(112));
    assert_eq!(t.bitmap.pixel(1,0),Some(192));assert_eq!(t.bitmap.pixel(1,1),Some(255));
}
#[test]fn aggregate_pixel_and_sample_budgets(){
    let b=font();let r=parse(&b);
    for limits in [TextRenderLimits{max_glyph_pixels:199,..Default::default()},
        TextRenderLimits{max_total_samples:3199,..Default::default()},
        TextRenderLimits{max_bitmap_width:23,..Default::default()},
        TextRenderLimits{max_bitmap_height:9,..Default::default()},
        TextRenderLimits{max_bitmap_pixels:239,..Default::default()}]{
        assert!(render_nominal_text(&r,"AA",TextRenderOptions{limits,..text_opts()}).is_err());
    }
    let limits=TextRenderLimits{max_glyph_pixels:200,max_total_samples:3200,max_bitmap_width:24,max_bitmap_height:10,max_bitmap_pixels:240,..Default::default()};
    assert!(render_nominal_text(&r,"AA",TextRenderOptions{limits,..text_opts()}).is_ok());
}
#[test]fn bytes_scalars_and_per_glyph_limits(){
    let b=font();let r=parse(&b);
    for limits in [TextRenderLimits{max_text_bytes:3,..Default::default()},TextRenderLimits{max_scalars:0,..Default::default()}]{
        assert!(render_nominal_text(&r,"😀",TextRenderOptions{limits,..text_opts()}).is_err());
    }
    let glyph=GlyphRenderOptions{limits:GlyphRenderLimits{path:PathLimits{max_commands:1,..Default::default()},..Default::default()},..opts()};
    assert!(render_nominal_text(&r,"A",TextRenderOptions{glyph,..text_opts()}).is_err());
}
#[test]fn controls_are_not_nominal_glyphs(){
    let b=font();let r=parse(&b);
    for s in ["A\n","\t","\r","\u{0}","\u{85}","\u{2028}","\u{2029}"]{
        assert!(matches!(render_nominal_text(&r,s,TextRenderOptions{missing:MissingGlyphPolicy::Notdef,..text_opts()}),Err(Error::Unsupported(_))));
    }
}
#[test]fn final_pen_budget_even_with_empty_glyph(){
    let b=font();let r=parse(&b);
    let glyph=GlyphRenderOptions{origin_x_26_6:(1i64<<30)-64,..opts()};
    assert!(render_nominal_text(&r," ",TextRenderOptions{glyph,..text_opts()}).is_err());
}
#[test]fn leading_and_trailing_space_not_bitmap_bounds(){
    let b=font();let r=parse(&b);let t=render_nominal_text(&r," A ",text_opts()).unwrap();
    assert_eq!(t.bitmap.viewport,vp(6,-10,10,10));assert_eq!(t.bitmap.alpha,vec![255;100]);
    assert_eq!(t.advance_26_6,1664);
}
#[test]fn single_glyph_bitmap_matches_and_output_deterministic(){
    let b=font();let r=parse(&b);
    let o=TextRenderOptions{glyph:GlyphRenderOptions{fill:GlyphFillRule::EvenOdd,samples_per_axis:8,..opts()},..text_opts()};
    let t=render_nominal_text(&r,"B",o).unwrap();
    assert_eq!(t.bitmap,r.render_glyph(3,o.glyph).unwrap().bitmap);
    assert_eq!(render_nominal_text(&r,"AB😀",o).unwrap(),render_nominal_text(&r,"AB😀",o).unwrap());
}
