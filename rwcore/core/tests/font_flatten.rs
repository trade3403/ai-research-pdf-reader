use rwcore::*;
fn p(x:i64,y:i64,on_curve:bool)->FixedOutlinePoint{FixedOutlinePoint{x,y,on_curve}}
fn q(x:i64,y:i64)->PathPoint{PathPoint{x,y}}
fn path(points:Vec<FixedOutlinePoint>,ends:Vec<usize>)->GlyphPath{
    build_glyph_path(&AssembledGlyph{points,contour_ends:ends,metrics_glyph:9,overlap:true},PathLimits::default()).unwrap()
}
fn opts(tolerance:i64)->FlattenOptions{FlattenOptions{tolerance,limits:FlattenLimits::default()}}
fn flat(points:Vec<FixedOutlinePoint>,ends:Vec<usize>,t:i64)->FlattenedGlyph{flatten_glyph_path(&path(points,ends),opts(t)).unwrap()}
fn arch()->GlyphPath{path(vec![p(0,0,true),p(4,8,false),p(8,0,true)],vec![2])}
#[test]fn empty_and_metadata(){
    let f=flat(vec![],vec![],1);assert!(f.contours().is_empty());assert_eq!(f.bounds(),None);
    assert_eq!(f.vertex_count(),0);assert_eq!(f.metrics_glyph(),9);assert!(f.overlap());assert_eq!(f.tolerance(),1);
}
#[test]fn straight_polygon_closure_and_area(){
    let f=flat(vec![p(0,0,true),p(5,0,true),p(5,4,true),p(0,4,true)],vec![3],1);
    let c=&f.contours()[0];assert_eq!(c.vertices(),&[q(0,0),q(10,0),q(10,8),q(0,8)]);
    assert_eq!(c.edges().last(),Some((q(0,8),q(0,0))));assert_eq!(c.signed_area2(),160);
    assert_eq!(c.orientation(),ContourOrientation::CounterClockwise);
}
#[test]fn reverse_contour_and_signed_hole(){
    let f=flat(vec![p(0,0,true),p(10,0,true),p(10,10,true),p(0,10,true),
        p(2,2,true),p(2,8,true),p(8,8,true),p(8,2,true)],vec![3,7],1);
    assert_eq!(f.contours()[0].signed_area2(),800);assert_eq!(f.contours()[1].signed_area2(),-288);
    assert_eq!(f.contours()[1].orientation(),ContourOrientation::Clockwise);assert_eq!(f.vertex_count(),8);
}
#[test]fn dyadic_subdivision_and_exact_endpoints(){
    let f=flatten_glyph_path(&arch(),opts(3)).unwrap();
    assert_eq!(f.contours()[0].vertices(),&[q(0,0),q(8,8),q(16,0)]);
    let f=flatten_glyph_path(&arch(),opts(1)).unwrap();
    assert_eq!(f.contours()[0].vertices(),&[q(0,0),q(4,6),q(8,8),q(12,6),q(16,0)]);
}
#[test]fn loose_tolerance_and_linear_quadratic(){
    let f=flatten_glyph_path(&arch(),opts(8)).unwrap();assert_eq!(f.vertex_count(),2);
    let f=flat(vec![p(0,0,true),p(5,5,false),p(10,10,true)],vec![2],1);
    assert_eq!(f.contours()[0].vertices(),&[q(0,0),q(20,20)]);
}
#[test]fn collinear_backtracking_is_not_dropped_as_flat(){
    let f=flat(vec![p(0,0,true),p(16,0,false),p(0,0,true)],vec![2],1);
    assert!(f.vertex_count()>2);assert_eq!(f.bounds().unwrap().x_max,16);
}
#[test]fn negative_fractional_rounding_and_midpoints(){
    let f=flat(vec![p(0,0,true),p(-3,4,false),p(-1,0,true)],vec![2],1);
    // d=16; N=2 is insufficient after rounding, N=4.
    assert_eq!(f.contours()[0].vertices(),&[q(0,0),q(-2,3),q(-4,4),q(-3,3),q(-2,0)]);
}
#[test]fn degenerate_single_point_and_repeated_endpoint(){
    let f=flat(vec![p(2,3,true)],vec![0],1);let c=&f.contours()[0];
    assert_eq!(c.edges().collect::<Vec<_>>(),vec![(q(4,6),q(4,6))]);assert_eq!(c.orientation(),ContourOrientation::ZeroSignedArea);
    let f=flat(vec![p(0,0,true),p(1,1,true),p(0,0,true)],vec![2],1);assert_eq!(f.vertex_count(),3);
}
#[test]fn all_off_curve_contour_and_closing_quadratic(){
    let f=flat(vec![p(0,0,false),p(4,8,false),p(8,0,false)],vec![2],1);
    let v=f.contours()[0].vertices();assert_eq!(v.first(),Some(&q(8,0)));assert_eq!(v.first(),v.last());assert!(v.len()>4);
}
#[test]fn bounds_are_polygon_not_control_or_curve_bounds(){
    let f=flatten_glyph_path(&arch(),opts(8)).unwrap();let b=f.bounds().unwrap();
    assert_eq!((b.x_min,b.y_min,b.x_max,b.y_max),(0,0,16,0));assert_eq!(arch().bounds().unwrap().y_max,8);
}
#[test]fn resource_limits_and_invalid_tolerance(){
    let g=arch();for t in [0,-1,i64::MIN]{assert!(flatten_glyph_path(&g,opts(t)).is_err());}
    for limits in [FlattenLimits{max_depth:1,..Default::default()},FlattenLimits{max_vertices:4,..Default::default()},FlattenLimits{max_contours:0,..Default::default()}]{
        assert!(flatten_glyph_path(&g,FlattenOptions{tolerance:1,limits}).is_err());
    }
    assert!(flatten_glyph_path(&g,FlattenOptions{tolerance:1,limits:FlattenLimits{max_vertices:5,..Default::default()}}).is_ok());
}
#[test]fn large_coordinates_and_hard_depth(){
    let v=1i64<<46;let g=path(vec![p(-v,-v,true),p(v,v,false),p(-v,-v,true)],vec![2]);
    let f=flatten_glyph_path(&g,opts(i64::MAX)).unwrap();assert_eq!(f.vertex_count(),2);
    assert!(flatten_glyph_path(&g,FlattenOptions{tolerance:1,limits:FlattenLimits{max_depth:0,..Default::default()}}).is_err());
}
#[test]fn sampled_error_bound_for_multiple_tolerances(){
    let g=path(vec![p(-11,5,true),p(23,-31,false),p(7,18,true)],vec![2]);
    for t in [1,2,3,7,20,100]{
        let f=flatten_glyph_path(&g,opts(t)).unwrap();let v=f.contours()[0].vertices();let n=(v.len()-1) as f64;
        for (i,pair) in v.windows(2).enumerate(){for s in [0.0,0.125,0.5,0.875,1.0]{
            let k=(i as f64+s)/n;let j=1.0-k;
            let x=-22.0*j*j+92.0*j*k+14.0*k*k;let y=10.0*j*j-124.0*j*k+36.0*k*k;
            let lx=pair[0].x as f64*(1.0-s)+pair[1].x as f64*s;
            let ly=pair[0].y as f64*(1.0-s)+pair[1].y as f64*s;
            assert!((x-lx).abs()<=t as f64+1e-10&&(y-ly).abs()<=t as f64+1e-10);
        }}
    }
}
#[test]fn total_budget_across_multiple_contours(){
    let g=path(vec![p(0,0,true),p(4,8,false),p(8,0,true),p(0,0,true),p(4,8,false),p(8,0,true)],vec![2,5]);
    let f=flatten_glyph_path(&g,opts(1)).unwrap();assert_eq!(f.vertex_count(),10);
    assert!(flatten_glyph_path(&g,FlattenOptions{tolerance:1,limits:FlattenLimits{max_vertices:9,..Default::default()}}).is_err());
}
