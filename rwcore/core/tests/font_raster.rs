use rwcore::*;
fn p(x:i64,y:i64)->FixedOutlinePoint{FixedOutlinePoint{x:x*(COMPOSITE_ONE/4),y:-y*(COMPOSITE_ONE/4),on_curve:true}}
fn polygon(contours:&[&[(i64,i64)]])->FlattenedGlyph{
    let mut points=Vec::new();let mut ends=Vec::new();
    for c in contours{points.extend(c.iter().map(|&(x,y)|p(x,y)));ends.push(points.len()-1);}
    let g=AssembledGlyph{points,contour_ends:ends,metrics_glyph:0,overlap:false};
    flatten_glyph_path(&build_glyph_path(&g,PathLimits::default()).unwrap(),
        FlattenOptions{tolerance:1,limits:FlattenLimits::default()}).unwrap()
}
fn rect(x:i64,y:i64,w:i64,h:i64)->[(i64,i64);4]{[(x,y),(x+w,y),(x+w,y+h),(x,y+h)]}
fn options(w:u32,h:u32)->RasterOptions{RasterOptions{
    transform:RasterTransform{units_per_em:16,ppem_26_6:1024,origin_x_26_6:0,origin_y_26_6:0},
    viewport:RasterViewport{left:0,top:0,width:w,height:h},fill:GlyphFillRule::NonZero,
    samples_per_axis:4,limits:RasterLimits::default(),
}}
fn render(g:&FlattenedGlyph,o:RasterOptions)->GlyphBitmap{rasterize_flat_glyph(g,o).unwrap()}
#[test]fn full_pixel_and_top_down_rows(){
    let g=polygon(&[&rect(0,0,4,4)]);let b=render(&g,options(2,2));
    assert_eq!(b.alpha,vec![255,0,0,0]);assert_eq!(b.pixel(0,0),Some(255));assert_eq!(b.pixel(2,0),None);
}
#[test]fn half_pixel_coverage(){
    let g=polygon(&[&rect(2,0,4,4)]);assert_eq!(render(&g,options(2,1)).alpha,vec![128,128]);
}
#[test]fn quarter_pixel_coverage(){
    let g=polygon(&[&rect(2,2,4,4)]);assert_eq!(render(&g,options(2,2)).alpha,vec![64;4]);
}
#[test]fn triangle_boundary_convention(){
    let g=polygon(&[&[(0,0),(4,0),(0,4)]]);
    assert_eq!(render(&g,options(1,1)).alpha,vec![96]);
}
#[test]fn opposite_winding_hole(){
    let outer=rect(0,0,12,12);let mut inner=rect(4,4,4,4);inner.reverse();
    let g=polygon(&[&outer,&inner]);let b=render(&g,options(3,3));
    assert_eq!(b.alpha,vec![255,255,255,255,0,255,255,255,255]);
    let b=render(&g,RasterOptions{fill:GlyphFillRule::EvenOdd,..options(3,3)});assert_eq!(b.pixel(1,1),Some(0));
}
#[test]fn same_winding_distinguishes_rules(){
    let g=polygon(&[&rect(0,0,12,12),&rect(4,4,4,4)]);
    assert_eq!(render(&g,options(3,3)).alpha,vec![255;9]);
    assert_eq!(render(&g,RasterOptions{fill:GlyphFillRule::EvenOdd,..options(3,3)}).pixel(1,1),Some(0));
}
#[test]fn identical_contours_cancel_only_by_rule_or_orientation(){
    let a=rect(0,0,4,4);let g=polygon(&[&a,&a]);
    assert_eq!(render(&g,options(1,1)).alpha,vec![255]);
    assert_eq!(render(&g,RasterOptions{fill:GlyphFillRule::EvenOdd,..options(1,1)}).alpha,vec![0]);
    let mut b=a;b.reverse();let g=polygon(&[&a,&b]);assert_eq!(render(&g,options(1,1)).alpha,vec![0]);
}
#[test]fn viewport_clipping_preserves_left_crossings(){
    let g=polygon(&[&rect(-8,-4,16,8)]);
    let o=RasterOptions{viewport:RasterViewport{left:-1,top:-1,width:4,height:3},..options(4,3)};
    assert_eq!(render(&g,o).alpha,vec![255,255,255,0,255,255,255,0,0,0,0,0]);
}
#[test]fn transform_scale_and_fractional_baseline(){
    let g=polygon(&[&rect(0,0,2,2)]);
    let t=RasterTransform{units_per_em:16,ppem_26_6:2048,origin_x_26_6:32,origin_y_26_6:32};
    assert_eq!(render(&g,RasterOptions{transform:t,..options(2,2)}).alpha,vec![64;4]);
}
#[test]fn empty_and_degenerate_geometry(){
    let g=polygon(&[]);assert_eq!(render(&g,options(2,2)).alpha,vec![0;4]);
    let g=polygon(&[&[(1,1)],&[(0,0),(4,0),(0,0)],&[(1,0),(1,4)]]);
    assert_eq!(render(&g,options(2,2)).alpha,vec![0;4]);
    assert!(render(&g,options(0,2)).alpha.is_empty());
}
#[test]fn all_supported_sample_counts(){
    let g=polygon(&[&rect(0,0,4,4)]);
    for n in [1,2,4,8,16]{assert_eq!(render(&g,RasterOptions{samples_per_axis:n,..options(1,1)}).alpha,vec![255]);}
    let g=polygon(&[&rect(2,0,2,4)]);
    assert_eq!(render(&g,RasterOptions{samples_per_axis:1,..options(1,1)}).alpha,vec![255]);
}
#[test]fn invalid_transform_and_samples_even_for_empty(){
    let g=polygon(&[]);
    for n in [0,3,17]{assert!(rasterize_flat_glyph(&g,RasterOptions{samples_per_axis:n,..options(0,0)}).is_err());}
    for t in [RasterTransform{units_per_em:0,..options(0,0).transform},
        RasterTransform{ppem_26_6:0,..options(0,0).transform},RasterTransform{origin_x_26_6:i64::MAX,..options(0,0).transform}]{
        assert!(rasterize_flat_glyph(&g,RasterOptions{transform:t,..options(0,0)}).is_err());
    }
}
#[test]fn bounded_dimensions_edges_samples_and_scan_work(){
    let g=polygon(&[&rect(0,0,4,4)]);
    for l in [RasterLimits{max_width:1,..Default::default()},RasterLimits{max_height:1,..Default::default()},
        RasterLimits{max_pixels:3,..Default::default()},RasterLimits{max_edges:3,..Default::default()},
        RasterLimits{max_samples:63,..Default::default()},RasterLimits{max_edge_tests:15,..Default::default()}]{
        assert!(rasterize_flat_glyph(&g,RasterOptions{limits:l,..options(2,2)}).is_err());
    }
    let l=RasterLimits{max_pixels:4,max_edges:4,max_samples:64,max_edge_tests:16,..Default::default()};
    assert!(rasterize_flat_glyph(&g,RasterOptions{limits:l,..options(2,2)}).is_ok());
}
#[test]fn extreme_coordinates_rejected_and_deterministic(){
    let g=polygon(&[&rect(0,0,4,4)]);
    let o=RasterOptions{viewport:RasterViewport{left:i32::MAX,..options(2,2).viewport},..options(2,2)};
    assert!(rasterize_flat_glyph(&g,o).is_err());
    let g=polygon(&[&rect(1,1,9,7)]);
    assert_eq!(render(&g,options(3,3)),render(&g,options(3,3)));
}
