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
#[test]fn unicode_to_square_bitmap(){
    let b=font();let r=parse(&b);let g=r.render_char('A',MissingGlyphPolicy::Reject,opts()).unwrap();
    assert_eq!(g.glyph_id,1);assert_eq!(g.bitmap.viewport,vp(0,-10,10,10));
    assert_eq!(g.bitmap.alpha,vec![255;100]);assert_eq!(g.advance_26_6,896);assert_eq!(g.left_bearing_26_6,-64);
}
#[test]fn supplementary_scalar_and_direct_id(){
    let b=font();let r=parse(&b);let a=r.render_glyph(1,opts()).unwrap();
    assert_eq!(a,r.render_char('😀',MissingGlyphPolicy::Reject,opts()).unwrap());
}
#[test]fn composite_bitmap_and_metric_source(){
    let b=font();let r=parse(&b);let g=r.render_char('B',MissingGlyphPolicy::Reject,opts()).unwrap();
    assert_eq!(g.bitmap.viewport,vp(2,-7,5,5));assert_eq!(g.bitmap.alpha,vec![255;25]);
    assert_eq!(g.metrics_source,1);assert_eq!(g.glyph_id,3);assert_eq!(g.advance_26_6,576);
}
#[test]fn missing_policy_and_notdef(){
    let b=font();let r=parse(&b);assert!(r.render_char('Z',MissingGlyphPolicy::Reject,opts()).is_err());
    let a=r.render_char('Z',MissingGlyphPolicy::Notdef,opts()).unwrap();
    assert!(a.missing);assert_eq!(a.glyph_id,0);assert_eq!(a.bitmap,r.render_glyph(0,opts()).unwrap().bitmap);
}
#[test]fn empty_space_keeps_advance_and_baseline(){
    let b=font();let r=parse(&b);
    let o=GlyphRenderOptions{origin_x_26_6:-1,origin_y_26_6:95,..opts()};
    let a=r.render_char(' ',MissingGlyphPolicy::Reject,o).unwrap();
    assert_eq!(a.bitmap.viewport,vp(-1,1,0,0));assert!(a.bitmap.alpha.is_empty());assert_eq!(a.advance_26_6,384);
}
#[test]fn fractional_baseline_outward_pixel_bounds(){
    let b=font();let r=parse(&b);
    let a=r.render_glyph(1,GlyphRenderOptions{origin_x_26_6:32,origin_y_26_6:32,..opts()}).unwrap();
    assert_eq!(a.bitmap.viewport,vp(0,-10,11,11));assert_eq!(a.bitmap.pixel(0,0),Some(64));
    assert_eq!(a.bitmap.pixel(1,1),Some(255));assert_eq!(a.bitmap.pixel(10,10),Some(64));
}
#[test]fn negative_fractional_origin(){
    let b=font();let r=parse(&b);
    let a=r.render_glyph(1,GlyphRenderOptions{origin_x_26_6:-32,origin_y_26_6:-32,..opts()}).unwrap();
    assert_eq!(a.bitmap.viewport,vp(-1,-11,11,11));assert_eq!(a.bitmap.pixel(0,0),Some(64));
}
#[test]fn integer_translation_preserves_pixels(){
    let b=font();let r=parse(&b);let a=r.render_glyph(1,opts()).unwrap();
    let c=r.render_glyph(1,GlyphRenderOptions{origin_x_26_6:128,origin_y_26_6:-192,..opts()}).unwrap();
    assert_eq!(c.bitmap.viewport,vp(2,-13,10,10));assert_eq!(a.bitmap.alpha,c.bitmap.alpha);
}
#[test]fn size_changes_bitmap_and_metrics(){
    let b=font();let r=parse(&b);
    let a=r.render_glyph(1,GlyphRenderOptions{ppem_26_6:640,..opts()}).unwrap();
    assert_eq!(a.bitmap.viewport,vp(0,-5,5,5));assert_eq!(a.advance_26_6,448);assert_eq!(a.left_bearing_26_6,-32);
}
#[test]fn pixel_tolerance_conservatively_converted(){
    let b=font();let r=parse(&b);
    let a=r.render_glyph(1,opts()).unwrap();assert_eq!(a.flatten_tolerance_path_units,409550);
    let c=r.render_glyph(1,GlyphRenderOptions{tolerance_16_16:2048,..opts()}).unwrap();
    assert!(c.flatten_tolerance_path_units<a.flatten_tolerance_path_units);
    let t=i128::from(c.flatten_tolerance_path_units);
    assert!(2*t*1280*1024+1000*i128::from(PATH_ONE)<=4096*1000*i128::from(PATH_ONE));
}
#[test]fn bad_scale_origin_tolerance_and_id(){
    let b=font();let r=parse(&b);
    for o in [GlyphRenderOptions{ppem_26_6:0,..opts()},GlyphRenderOptions{tolerance_16_16:0,..opts()},
        GlyphRenderOptions{samples_per_axis:3,..opts()},GlyphRenderOptions{origin_x_26_6:i64::MAX,..opts()},
        GlyphRenderOptions{ppem_26_6:u32::MAX,tolerance_16_16:1,..opts()}]{
        assert!(r.render_glyph(2,o).is_err());
    }
    assert!(r.render_glyph(4,opts()).is_err());
}
#[test]fn stage_budgets_propagated(){
    let b=font();let r=parse(&b);
    for l in [GlyphRenderLimits{assembly:CompositeLimits{max_depth:0,..Default::default()},..Default::default()},
        GlyphRenderLimits{path:PathLimits{max_commands:1,..Default::default()},..Default::default()},
        GlyphRenderLimits{flatten:FlattenLimits{max_vertices:1,..Default::default()},..Default::default()},
        GlyphRenderLimits{raster:RasterLimits{max_pixels:24,..Default::default()},..Default::default()},
        GlyphRenderLimits{raster:RasterLimits{max_samples:399,..Default::default()},..Default::default()}]{
        assert!(r.render_glyph(3,GlyphRenderOptions{limits:l,..opts()}).is_err());
    }
}
#[test]fn same_face_parse_limits(){
    let b=font();let c=parse_font(&b,FontLimits::default()).unwrap();let f=c.face(0).unwrap();
    for l in [RenderFontLimits{metrics:FontMetricLimits{max_glyphs:3},..Default::default()},
        RenderFontLimits{cmap:CmapLimits{max_groups:3,..Default::default()},..Default::default()},
        RenderFontLimits{outlines:OutlineLimits{max_glyphs:3,..Default::default()},..Default::default()}]{
        assert!(parse_true_type_renderer(f,l).is_err());
    }
}
#[test]fn fill_rule_and_sampling_forwarded(){
    let b=font();let r=parse(&b);
    let a=r.render_glyph(1,GlyphRenderOptions{samples_per_axis:1,fill:GlyphFillRule::EvenOdd,..opts()}).unwrap();
    assert_eq!(a.bitmap.alpha,vec![255;100]);
    assert_eq!(a,r.render_glyph(1,GlyphRenderOptions{samples_per_axis:1,fill:GlyphFillRule::EvenOdd,..opts()}).unwrap());
}
