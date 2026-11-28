use rwcore::*;
fn p16(b:&mut [u8],i:usize,n:u16){b[i..i+2].copy_from_slice(&n.to_be_bytes());}
fn p32(b:&mut [u8],i:usize,n:u32){b[i..i+4].copy_from_slice(&n.to_be_bytes());}
fn w(b:&mut Vec<u8>,n:i16){b.extend(n.to_be_bytes());}
fn header(n:i16)->Vec<u8>{let mut b=Vec::new();for v in [n,-500,-500,500,500]{w(&mut b,v);}b}
fn simple(points:&[(i16,i16,bool)],ends:&[u16],instructions:&[u8])->Vec<u8>{
    let mut b=header(ends.len() as i16);
    for &e in ends{b.extend(e.to_be_bytes());}
    b.extend((instructions.len() as u16).to_be_bytes());b.extend(instructions);
    for p in points{b.push(u8::from(p.2));}
    for axis in 0..2{let mut prev=0;for p in points{let v=if axis==0{p.0}else{p.1};w(&mut b,v-prev);prev=v;}}
    b
}
fn font(glyphs:&[Vec<u8>],short:bool)->Vec<u8>{
    let mut glyf=Vec::new();let mut loca=Vec::new();
    for g in glyphs.iter().map(Some).chain(std::iter::once(None)){
        let at=glyf.len();if short{loca.extend(((at/2) as u16).to_be_bytes());}else{loca.extend((at as u32).to_be_bytes());}
        if let Some(g)=g{glyf.extend(g);if short&&glyf.len()%2!=0{glyf.push(0);}}
    }
    let mut head=vec![0;54];p32(&mut head,0,0x10000);p32(&mut head,12,0x5f0f3cf5);p16(&mut head,18,1000);p16(&mut head,50,if short{0}else{1});
    let mut maxp=vec![0;32];p32(&mut maxp,0,0x10000);p16(&mut maxp,4,glyphs.len() as u16);
    let tables=[(*b"head",head),(*b"maxp",maxp),(*b"loca",loca),(*b"glyf",glyf)];
    let mut b=vec![0;76];b[..4].copy_from_slice(&[0,1,0,0]);p16(&mut b,4,4);
    for (i,(tag,data)) in tables.into_iter().enumerate(){while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);p32(&mut b,p+8,at);p32(&mut b,p+12,data.len() as u32);b.extend(data);}
    b
}
fn parse(b:&[u8],l:OutlineLimits)->Result<TrueTypeOutlines<'_>,Error>{let c=parse_font(b,FontLimits::default())?;parse_true_type_outlines(c.face(0).unwrap(),l)}
fn hp(x2:i32,y2:i32)->HalfPoint{HalfPoint{x2,y2}}
fn square()->Vec<u8>{simple(&[(0,0,true),(100,0,true),(100,100,true),(0,100,true)],&[3],&[])}
fn one(g:Vec<u8>)->Vec<u8>{font(&[g],false)}
#[test]fn short_and_long_loca_with_empty_glyph(){
    for short in [false,true]{let b=font(&[vec![],square(),vec![]],short);let f=parse(&b,OutlineLimits::default()).unwrap();
        assert_eq!(f.glyph_count(),3);assert_eq!(f.glyph_kind(0).unwrap(),GlyphOutlineKind::Empty);
        assert_eq!(f.glyph_kind(1).unwrap(),GlyphOutlineKind::Simple);assert_eq!(f.glyph_kind(2).unwrap(),GlyphOutlineKind::Empty);
        assert!(f.simple_glyph(0).unwrap().path().unwrap().is_empty());assert!(f.glyph_bytes(3).is_err());}
}
#[test]fn square_path_and_point_bounds(){
    let b=one(square());let f=parse(&b,OutlineLimits::default()).unwrap();let g=f.simple_glyph(0).unwrap();
    assert_eq!(g.contour_ends(),&[3]);assert_eq!(g.point_bounds(),Some(FontBounds{x_min:0,y_min:0,x_max:100,y_max:100}));
    assert_eq!(g.path().unwrap(),vec![OutlineCommand::MoveTo(hp(0,0)),OutlineCommand::LineTo(hp(200,0)),OutlineCommand::LineTo(hp(200,200)),OutlineCommand::LineTo(hp(0,200)),OutlineCommand::Close]);
}
#[test]fn quadratic_and_implied_half_unit_midpoint(){
    let b=one(simple(&[(0,0,true),(3,5,false),(8,6,false),(12,0,true)],&[3],&[]));
    let f=parse(&b,OutlineLimits::default()).unwrap();let g=f.simple_glyph(0).unwrap();
    assert_eq!(g.path().unwrap(),vec![OutlineCommand::MoveTo(hp(0,0)),OutlineCommand::QuadTo{control:hp(6,10),end:hp(11,11)},OutlineCommand::QuadTo{control:hp(16,12),end:hp(24,0)},OutlineCommand::Close]);
}
#[test]fn off_curve_first_and_last_on_curve(){
    let b=one(simple(&[(2,5,false),(8,1,true),(0,0,true)],&[2],&[]));
    let f=parse(&b,OutlineLimits::default()).unwrap();
    assert_eq!(f.simple_glyph(0).unwrap().path().unwrap(),vec![OutlineCommand::MoveTo(hp(0,0)),OutlineCommand::QuadTo{control:hp(4,10),end:hp(16,2)},OutlineCommand::Close]);
}
#[test]fn all_off_curve_and_single_point(){
    let b=one(simple(&[(1,3,false),(4,8,false)],&[1],&[]));let f=parse(&b,OutlineLimits::default()).unwrap();
    assert_eq!(f.simple_glyph(0).unwrap().path().unwrap(),vec![OutlineCommand::MoveTo(hp(5,11)),OutlineCommand::QuadTo{control:hp(2,6),end:hp(5,11)},OutlineCommand::QuadTo{control:hp(8,16),end:hp(5,11)},OutlineCommand::Close]);
    let b=one(simple(&[(1,3,true)],&[0],&[]));let f=parse(&b,OutlineLimits::default()).unwrap();
    assert_eq!(f.simple_glyph(0).unwrap().path().unwrap(),vec![OutlineCommand::MoveTo(hp(2,6)),OutlineCommand::Close]);
}
#[test]fn multiple_contours_and_inert_instructions(){
    let b=one(simple(&[(1,2,true),(3,4,true),(9,8,true)],&[1,2],&[0xff,0xaa]));
    let f=parse(&b,OutlineLimits::default()).unwrap();let g=f.simple_glyph(0).unwrap();
    assert_eq!(g.instructions(),&[0xff,0xaa]);assert_eq!(g.contour_ends(),&[1,2]);
    assert_eq!(g.path().unwrap().iter().filter(|c|matches!(c,OutlineCommand::Close)).count(),2);
}
#[test]fn compressed_flags_repeats_and_signed_short_deltas(){
    let mut g=header(1);w(&mut g,3);w(&mut g,0);
    // repeated on-curve, x positive short, y negative short; last x unchanged, y positive short
    g.extend([0x5f,2,0x35|4]);g.extend([10,20,30]);g.extend([1,2,3,4]);
    let b=one(g);let f=parse(&b,OutlineLimits::default()).unwrap();let g=f.simple_glyph(0).unwrap();
    assert!(g.overlap_simple());assert_eq!(g.points(),&[OutlinePoint{x:10,y:-1,on_curve:true},OutlinePoint{x:30,y:-3,on_curve:true},OutlinePoint{x:60,y:-6,on_curve:true},OutlinePoint{x:60,y:-2,on_curve:true}]);
}
#[test]fn zero_contour_header_only_and_composite_identified(){
    let b=font(&[header(0),header(-1)],false);let f=parse(&b,OutlineLimits::default()).unwrap();
    assert!(f.simple_glyph(0).unwrap().points().is_empty());assert_eq!(f.glyph_kind(1).unwrap(),GlyphOutlineKind::Composite);
    assert!(matches!(f.simple_glyph(1),Err(Error::Unsupported(_))));
}
#[test]fn malformed_loca_lengths_offsets_and_first(){
    for values in [[1,4],[0,u32::MAX],[4,2]]{
        let mut b=one(square());let at=u32::from_be_bytes(b[52..56].try_into().unwrap()) as usize;
        p32(&mut b,at,values[0]);p32(&mut b,at+4,values[1]);assert!(parse(&b,OutlineLimits::default()).is_err());
    }
    let mut b=one(square());p32(&mut b,56,6);assert!(parse(&b,OutlineLimits::default()).is_err());
}
#[test]fn truncation_rejected_at_all_nonempty_prefixes(){
    let g=square();for n in 1..g.len(){let b=one(g[..n].to_vec());let f=parse(&b,OutlineLimits::default()).unwrap();assert!(f.simple_glyph(0).is_err(),"prefix {n}");}
}
#[test]fn rejects_bad_contours_flags_and_tail(){
    let mut g=simple(&[(0,0,true),(1,1,true)],&[0,1],&[]);p16(&mut g,12,0);
    let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
    let mut g=square();g[14]=0x80;let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
    let mut g=square();g[14]=8;g[15]=5;let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
    let mut g=square();g.push(1);let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
}
#[test]fn coordinate_overflow_rejected(){
    let mut g=header(1);w(&mut g,1);w(&mut g,0);g.extend([1,1]);w(&mut g,32767);w(&mut g,1);w(&mut g,0);w(&mut g,0);
    let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
}
#[test]fn budgets_for_glyph_contour_point_instruction_and_path(){
    let b=one(simple(&[(0,0,true),(1,1,true)],&[1],&[1]));
    assert!(parse(&b,OutlineLimits{max_glyphs:0,..Default::default()}).is_err());
    for l in [OutlineLimits{max_glyph_bytes:1,..Default::default()},OutlineLimits{max_contours:0,..Default::default()},
        OutlineLimits{max_points:1,..Default::default()},OutlineLimits{max_instruction_bytes:0,..Default::default()}]{
        assert!(parse(&b,l).unwrap().simple_glyph(0).is_err());
    }
    let f=parse(&b,OutlineLimits{max_path_commands:2,..Default::default()}).unwrap();assert!(f.simple_glyph(0).unwrap().path().is_err());
}
#[test]fn declared_bounds_advisory_and_padding(){
    let mut g=square();g.extend([0,0]);let b=one(g);let f=parse(&b,OutlineLimits::default()).unwrap();let g=f.simple_glyph(0).unwrap();
    assert_ne!(g.bounds(),g.point_bounds());assert_eq!(g.points().len(),4);
    let mut g=square();p16(&mut g,2,600);let b=one(g);assert!(parse(&b,OutlineLimits::default()).unwrap().simple_glyph(0).is_err());
}
