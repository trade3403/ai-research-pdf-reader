use rwcore::*;
fn w(b:&mut Vec<u8>,v:i16){b.extend(v.to_be_bytes());}
fn p16(b:&mut [u8],i:usize,v:u16){b[i..i+2].copy_from_slice(&v.to_be_bytes());}
fn p32(b:&mut [u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn header(n:i16)->Vec<u8>{let mut b=Vec::new();for v in [n,-500,-500,500,500]{w(&mut b,v);}b}
fn leaf()->Vec<u8>{
    let mut b=header(1);w(&mut b,1);w(&mut b,0);b.extend([1,0]);
    for v in [2,8,4,16]{w(&mut b,v);}b
}
fn composite(records:&[(u16,u16,Vec<u8>)],instructions:&[u8])->Vec<u8>{
    let mut b=header(-1);
    for (i,(flags,id,args)) in records.iter().enumerate(){
        let mut f=*flags;if i+1<records.len(){f|=0x20;}else if !instructions.is_empty(){f|=0x100;}
        b.extend(f.to_be_bytes());b.extend(id.to_be_bytes());b.extend(args);
    }
    if !instructions.is_empty(){b.extend((instructions.len() as u16).to_be_bytes());b.extend(instructions);}b
}
fn args(v:&[i16])->Vec<u8>{v.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn font(glyphs:&[Vec<u8>])->Vec<u8>{
    let mut glyf=Vec::new();let mut loca=Vec::new();
    for g in glyphs{loca.extend((glyf.len() as u32).to_be_bytes());glyf.extend(g);}
    loca.extend((glyf.len() as u32).to_be_bytes());
    let mut head=vec![0;54];p32(&mut head,0,0x10000);p32(&mut head,12,0x5f0f3cf5);p16(&mut head,18,1000);p16(&mut head,50,1);
    let mut maxp=vec![0;32];p32(&mut maxp,0,0x10000);p16(&mut maxp,4,glyphs.len() as u16);
    let mut b=vec![0;76];b[..4].copy_from_slice(&[0,1,0,0]);p16(&mut b,4,4);
    for (i,(tag,data)) in [(*b"head",head),(*b"maxp",maxp),(*b"loca",loca),(*b"glyf",glyf)].into_iter().enumerate(){
        while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);p32(&mut b,p+8,at);p32(&mut b,p+12,data.len() as u32);b.extend(data);
    }b
}
fn parse(b:&[u8])->TrueTypeOutlines<'_>{let c=parse_font(b,FontLimits::default()).unwrap();parse_true_type_outlines(c.face(0).unwrap(),OutlineLimits::default()).unwrap()}
fn assemble(glyphs:&[Vec<u8>],id:u16)->Result<AssembledGlyph,Error>{let b=font(glyphs);assemble_true_type_glyph(&parse(&b),id,CompositeLimits::default())}
fn pt(x:i64,y:i64,on_curve:bool)->FixedOutlinePoint{FixedOutlinePoint{x:x*COMPOSITE_ONE,y:y*COMPOSITE_ONE,on_curve}}
#[test]fn simple_and_empty_assembly(){
    let g=assemble(&[leaf(),vec![]],0).unwrap();assert_eq!(g.points,vec![pt(2,4,true),pt(10,20,false)]);assert_eq!(g.contour_ends,vec![1]);
    assert!(assemble(&[leaf(),vec![]],1).unwrap().points.is_empty());
}
#[test]fn signed_byte_word_offsets_and_contour_order(){
    let c=composite(&[(2,0,vec![255,2]),(3,0,args(&[300,-200]))],&[]);
    let g=assemble(&[leaf(),c],1).unwrap();
    assert_eq!(g.points,vec![pt(1,6,true),pt(9,22,false),pt(302,-196,true),pt(310,-180,false)]);assert_eq!(g.contour_ends,vec![1,3]);
}
#[test]fn scaled_and_unscaled_offsets(){
    for (flags,x,y) in [(0x80b,4,8),(0x100b,7,14),(0x00b,7,14)]{
        let c=composite(&[(flags,0,args(&[6,12,8192]))],&[]);
        assert_eq!(assemble(&[leaf(),c],1).unwrap().points[0],pt(x,y,true));
    }
}
#[test]fn anisotropic_and_two_by_two_transform(){
    let c=composite(&[(0x43,0,args(&[0,0,8192,-16384])),(0x83,0,args(&[0,0,0,16384,-16384,0]))],&[]);
    let g=assemble(&[leaf(),c],1).unwrap();assert_eq!(g.points,vec![pt(1,-4,true),pt(5,-20,false),pt(-4,2,true),pt(-20,10,false)]);
}
#[test]fn fractional_coordinates_preserved(){
    let c=composite(&[(0xb,0,args(&[0,0,4096]))],&[]);
    let g=assemble(&[leaf(),c],1).unwrap();assert_eq!(g.points[0],FixedOutlinePoint{x:32768,y:65536,on_curve:true});
}
#[test]fn real_point_attachment_after_transform(){
    let c=composite(&[(3,0,args(&[100,200])),(9,0,args(&[1,0,8192]))],&[]);
    let g=assemble(&[leaf(),c],1).unwrap();
    assert_eq!(g.points[1].x,g.points[2].x);assert_eq!(g.points[1].y,g.points[2].y);
    assert_eq!(g.points[3],pt(114,228,false));
}
#[test]fn recursion_and_nested_metrics_source(){
    let c=composite(&[(0x203,0,args(&[5,0]))],&[]);
    let d=composite(&[(0x20b,1,args(&[0,10,8192]))],&[]);
    let g=assemble(&[leaf(),c,d],2).unwrap();assert_eq!(g.points[0],FixedOutlinePoint{x:229376,y:786432,on_curve:true});assert_eq!(g.metrics_glyph,0);
}
#[test]fn instructions_and_grid_flag_are_metadata(){
    let c=composite(&[(0x407,0,args(&[1,2]))],&[0xff,0xaa]);let b=font(&[leaf(),c]);let f=parse(&b);
    let g=parse_composite_glyph(&f,1,CompositeLimits::default()).unwrap();assert_eq!(g.instructions,&[0xff,0xaa]);assert!(g.components[0].round_to_grid);
    assert!(assemble_true_type_glyph(&f,1,CompositeLimits::default()).unwrap().overlap);
}
#[test]fn self_and_indirect_cycles(){
    let a=composite(&[(3,0,args(&[0,0]))],&[]);assert!(assemble(&[a],0).is_err());
    let a=composite(&[(3,1,args(&[0,0]))],&[]);let b=composite(&[(3,0,args(&[0,0]))],&[]);assert!(assemble(&[a,b],0).is_err());
}
#[test]fn repeated_siblings_not_cycle_and_budget(){
    let c=composite(&[(3,0,args(&[0,0])),(3,0,args(&[5,5]))],&[]);let b=font(&[leaf(),c]);let f=parse(&b);
    assert!(assemble_true_type_glyph(&f,1,CompositeLimits::default()).is_ok());
    for l in [CompositeLimits{max_depth:0,..Default::default()},CompositeLimits{max_components:1,..Default::default()},
        CompositeLimits{max_visits:2,..Default::default()},CompositeLimits{max_points:3,..Default::default()},CompositeLimits{max_contours:1,..Default::default()}]{
        assert!(assemble_true_type_glyph(&f,1,l).is_err());
    }
}
#[test]fn malformed_transform_reference_and_placement(){
    for (flags,id,a) in [(0x1803,0,args(&[0,0])),(0x4b,0,args(&[0,0,8192,8192])),(3,2,args(&[0,0])),(1,0,args(&[0,0])),(0x8003,0,args(&[0,0]))]{
        assert!(assemble(&[leaf(),composite(&[(flags,id,a)],&[])],1).is_err());
    }
    let c=composite(&[(3,0,args(&[0,0])),(1,0,args(&[9,0]))],&[]);assert!(assemble(&[leaf(),c],1).is_err());
    let c=composite(&[(3,0,args(&[0,0])),(1,0,args(&[0,9]))],&[]);assert!(assemble(&[leaf(),c],1).is_err());
}
#[test]fn truncations_and_instruction_budget(){
    let c=composite(&[(0x83,0,args(&[0,0,16384,0,0,16384]))],&[7,8]);
    for n in 1..c.len(){assert!(assemble(&[leaf(),c[..n].to_vec()],1).is_err(),"prefix {n}");}
    let b=font(&[leaf(),c]);let f=parse(&b);assert!(parse_composite_glyph(&f,1,CompositeLimits{max_instruction_bytes:1,..Default::default()}).is_err());
}
#[test]fn multiple_metrics_and_nonzero_tail_rejected(){
    let c=composite(&[(0x203,0,args(&[0,0])),(0x203,0,args(&[0,0]))],&[]);assert!(assemble(&[leaf(),c],1).is_err());
    let mut c=composite(&[(3,0,args(&[0,0]))],&[]);c.push(9);assert!(assemble(&[leaf(),c],1).is_err());
}
#[test]fn nested_component_budget_counts_all_levels(){
    let c=composite(&[(3,0,args(&[0,0]))],&[]);let d=composite(&[(3,1,args(&[0,0])),(3,1,args(&[0,0]))],&[]);
    let b=font(&[leaf(),c,d]);let f=parse(&b);
    assert!(assemble_true_type_glyph(&f,2,CompositeLimits{max_components:3,..Default::default()}).is_err());
    assert_eq!(assemble_true_type_glyph(&f,2,CompositeLimits::default()).unwrap().points.len(),4);
}
