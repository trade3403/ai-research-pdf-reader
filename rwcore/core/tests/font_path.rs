use rwcore::*;
fn fixed(x:i64,y:i64,on_curve:bool)->FixedOutlinePoint{FixedOutlinePoint{x,y,on_curve}}
fn p(x:i64,y:i64,on_curve:bool)->FixedOutlinePoint{fixed(x*COMPOSITE_ONE,y*COMPOSITE_ONE,on_curve)}
fn q(x:i64,y:i64)->PathPoint{PathPoint{x,y}}
fn u(x:i64,y:i64)->PathPoint{q(x*PATH_ONE,y*PATH_ONE)}
fn g(points:Vec<FixedOutlinePoint>,ends:Vec<usize>)->AssembledGlyph{
    AssembledGlyph{points,contour_ends:ends,metrics_glyph:7,overlap:true}
}
fn path(points:Vec<FixedOutlinePoint>,ends:Vec<usize>)->GlyphPath{build_glyph_path(&g(points,ends),PathLimits::default()).unwrap()}
fn bounds(x_min:i64,y_min:i64,x_max:i64,y_max:i64)->GlyphPathBounds{GlyphPathBounds{x_min,y_min,x_max,y_max}}
#[test]fn empty_and_metadata(){
    let x=path(vec![],vec![]);assert!(x.commands().is_empty());assert_eq!(x.bounds(),None);assert_eq!(x.control_bounds(),None);
    assert_eq!(x.metrics_glyph(),7);assert!(x.overlap());
}
#[test]fn straight_contour_closure(){
    let x=path(vec![p(-2,-1,true),p(5,-1,true),p(5,4,true),p(-2,4,true)],vec![3]);
    assert_eq!(x.commands(),&[GlyphPathCommand::MoveTo(u(-2,-1)),GlyphPathCommand::LineTo(u(5,-1)),GlyphPathCommand::LineTo(u(5,4)),GlyphPathCommand::LineTo(u(-2,4)),GlyphPathCommand::Close]);
    assert_eq!(x.bounds(),Some(bounds(-2*PATH_ONE,-PATH_ONE,5*PATH_ONE,4*PATH_ONE)));assert_eq!(x.bounds(),x.control_bounds());
}
#[test]fn implied_midpoint_preserves_half_of_fixed_lsb(){
    let x=path(vec![fixed(0,0,true),fixed(1,3,false),fixed(2,4,false),fixed(5,0,true)],vec![3]);
    assert_eq!(x.commands(),&[GlyphPathCommand::MoveTo(q(0,0)),GlyphPathCommand::QuadTo{control:q(2,6),end:q(3,7)},GlyphPathCommand::QuadTo{control:q(4,8),end:q(10,0)},GlyphPathCommand::Close]);
}
#[test]fn all_off_curve_and_single_point(){
    let x=path(vec![fixed(1,3,false),fixed(2,4,false)],vec![1]);
    assert_eq!(x.commands(),&[GlyphPathCommand::MoveTo(q(3,7)),GlyphPathCommand::QuadTo{control:q(2,6),end:q(3,7)},GlyphPathCommand::QuadTo{control:q(4,8),end:q(3,7)},GlyphPathCommand::Close]);
    let x=path(vec![p(-3,2,true)],vec![0]);assert_eq!(x.bounds(),Some(bounds(-3*PATH_ONE,2*PATH_ONE,-3*PATH_ONE,2*PATH_ONE)));
}
#[test]fn first_off_last_on_and_multiple_contours(){
    let x=path(vec![p(1,4,false),p(6,0,true),p(0,0,true),p(-5,-3,true)],vec![2,3]);
    assert_eq!(x.commands(),&[GlyphPathCommand::MoveTo(u(0,0)),GlyphPathCommand::QuadTo{control:u(1,4),end:u(6,0)},GlyphPathCommand::Close,GlyphPathCommand::MoveTo(u(-5,-3)),GlyphPathCommand::Close]);
    assert_eq!(x.bounds(),Some(bounds(-5*PATH_ONE,-3*PATH_ONE,6*PATH_ONE,2*PATH_ONE)));
}
#[test]fn interior_quadratic_maximum_not_control_hull(){
    let x=path(vec![p(0,0,true),p(5,10,false),p(10,0,true)],vec![2]);
    assert_eq!(x.bounds(),Some(bounds(0,0,10*PATH_ONE,5*PATH_ONE)));
    assert_eq!(x.control_bounds(),Some(bounds(0,0,10*PATH_ONE,10*PATH_ONE)));
}
#[test]fn negative_extremum_and_outward_rounding(){
    // x(t) from 0 via -2 to -1 in path units doubled: min=-8/3.
    let x=path(vec![fixed(0,0,true),fixed(-2,2,false),fixed(-1,1,true)],vec![2]);
    assert_eq!(x.bounds(),Some(bounds(-3,0,0,3)));
}
#[test]fn different_x_and_y_extremum_parameters(){
    let x=path(vec![p(0,0,true),p(8,-6,false),p(4,6,true)],vec![2]);
    // xmax=16/3 and ymin=-2, endpoints give ymax=6.
    assert_eq!(x.bounds(),Some(bounds(0,-2*PATH_ONE,(16*PATH_ONE+2)/3,6*PATH_ONE)));
}
#[test]fn monotone_linear_and_endpoint_extrema(){
    let x=path(vec![p(0,0,true),p(2,0,false),p(4,4,true)],vec![2]);
    assert_eq!(x.bounds(),Some(bounds(0,0,4*PATH_ONE,4*PATH_ONE)));
}
#[test]fn closure_curve_contributes_extrema(){
    let x=path(vec![p(0,0,true),p(4,0,true),p(2,-8,false)],vec![2]);
    assert_eq!(x.bounds(),Some(bounds(0,-4*PATH_ONE,4*PATH_ONE,0)));
}
#[test]fn rejects_invalid_public_geometry(){
    for ends in [vec![],vec![2],vec![0,0]]{assert!(build_glyph_path(&g(vec![p(0,0,true)],ends),PathLimits::default()).is_err());}
    assert!(build_glyph_path(&g(vec![],vec![0]),PathLimits::default()).is_err());
    assert!(build_glyph_path(&g(vec![p(0,0,true),p(1,1,true)],vec![0]),PathLimits::default()).is_err());
}
#[test]fn bounded_coordinates_and_resource_limits(){
    let v=1i64<<46;
    let x=path(vec![fixed(-v,-v,true),fixed(v,v,false),fixed(-v,-v,true)],vec![2]);
    assert_eq!(x.bounds(),Some(bounds(-2*v,-2*v,0,0)));
    for n in [i64::MIN,i64::MAX,v+1]{assert!(build_glyph_path(&g(vec![fixed(n,0,true)],vec![0]),PathLimits::default()).is_err());}
    let x=g(vec![p(0,0,true),p(1,1,true)],vec![1]);
    for l in [PathLimits{max_points:1,..Default::default()},PathLimits{max_contours:0,..Default::default()},PathLimits{max_commands:2,..Default::default()}]{
        assert!(build_glyph_path(&x,l).is_err());
    }
}
fn w(b:&mut Vec<u8>,v:i16){b.extend(v.to_be_bytes());}
fn put16(b:&mut [u8],i:usize,v:u16){b[i..i+2].copy_from_slice(&v.to_be_bytes());}
fn put32(b:&mut [u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn font()->Vec<u8>{
    let mut glyf=Vec::new();for v in [1,0,0,10,10,2,0]{w(&mut glyf,v);}
    glyf.extend([1,0,1]);for v in [0,5,5,0,10,-10]{w(&mut glyf,v);}
    let end=glyf.len() as u32;
    for v in [-1,0,0,20,20,0xb,0,2,3,8192]{w(&mut glyf,v);}
    let mut loca=Vec::new();for v in [0,end,glyf.len() as u32]{loca.extend(v.to_be_bytes());}
    let mut head=vec![0;54];put32(&mut head,0,0x10000);put32(&mut head,12,0x5f0f3cf5);put16(&mut head,18,1000);put16(&mut head,50,1);
    let mut maxp=vec![0;32];put32(&mut maxp,0,0x10000);put16(&mut maxp,4,2);
    let mut b=vec![0;76];b[..4].copy_from_slice(&[0,1,0,0]);put16(&mut b,4,4);
    for (i,(tag,data)) in [(*b"head",head),(*b"maxp",maxp),(*b"loca",loca),(*b"glyf",glyf)].into_iter().enumerate(){
        while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);put32(&mut b,p+8,at);put32(&mut b,p+12,data.len() as u32);b.extend(data);
    }b
}
#[test]fn actual_simple_and_composite_share_path_entry(){
    let b=font();let f=parse_font(&b,FontLimits::default()).unwrap();
    let o=parse_true_type_outlines(f.face(0).unwrap(),OutlineLimits::default()).unwrap();
    let a=true_type_glyph_path(&o,0,CompositeLimits::default(),PathLimits::default()).unwrap();
    let c=true_type_glyph_path(&o,1,CompositeLimits::default(),PathLimits::default()).unwrap();
    assert_eq!(a.bounds(),Some(bounds(0,0,10*PATH_ONE,5*PATH_ONE)));
    assert_eq!(c.bounds(),Some(bounds(2*PATH_ONE,3*PATH_ONE,7*PATH_ONE,11*PATH_ONE/2)));
    assert_eq!(a.commands().len(),c.commands().len());
}
#[test]fn end_to_end_limits_and_invalid_glyph(){
    let b=font();let f=parse_font(&b,FontLimits::default()).unwrap();
    let o=parse_true_type_outlines(f.face(0).unwrap(),OutlineLimits::default()).unwrap();
    assert!(true_type_glyph_path(&o,2,CompositeLimits::default(),PathLimits::default()).is_err());
    assert!(true_type_glyph_path(&o,1,CompositeLimits{max_depth:0,..Default::default()},PathLimits::default()).is_err());
    assert!(true_type_glyph_path(&o,0,CompositeLimits::default(),PathLimits{max_commands:2,..Default::default()}).is_err());
}
