use rwcore::*;
fn fixture(flags:u16,ext:bool)->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,1,4,if ext{9}else{3},flags,1,8];
    if ext{w.extend([1,3,0,8]);}
    w.extend([1,14,2,0,22,28,0,1,2,3,7,1,100,20,1,65506,5]);
    w.iter().flat_map(|v|v.to_be_bytes()).collect()
}
fn run(flags:u16,ext:bool,nominal:i32)->GposCursiveRun{
    let b=fixture(flags,ext);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    execute_gpos_cursive_horizontal(&p,&[3,7],&[Default::default();2],&[(nominal,0),(50,0)],&[(0,0);2],GposRunMode::DesignOnly,Default::default()).unwrap()
}
#[test]fn ltr_align_and_advance(){let x=run(0,false,80);assert_eq!(x.adjustments[0].design.x_advance,50);assert_eq!(x.design_origins[1].0,130);assert_eq!(x.adjustments[1].design.y_placement,15);assert_eq!(x.edges[1].unwrap().parent,0);}
#[test]fn rtl_fixed_entry_and_signed_nominal(){let x=run(1,true,-80);assert_eq!(x.adjustments[0].design.x_advance,210);assert_eq!(x.adjustments[0].design.y_placement,-15);assert_eq!(x.edges[0].unwrap().parent,1);}
#[test]fn ignored_mark_span_preserves_advance(){
    let b=fixture(8,false);let g=parse_gdef(&[0,1,0,0,0,12,0,0,0,0,0,0,0,1,0,4,0,1,0,3],32,Default::default()).unwrap();
    let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_cursive_horizontal(&p,&[3,4,7],&[Default::default();3],&[(80,0),(20,0),(50,0)],&[(0,0);3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.adjustments[0].design.x_advance,30);assert_eq!(x.design_origins[2].0,130);assert_eq!(x.adjustments[1],Default::default());assert_eq!(x.edges[2].unwrap().parent,0);
}
#[test]fn missing_anchor_no_match(){let b=fixture(0,false);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();let x=execute_gpos_cursive_horizontal(&p,&[7,3],&[Default::default();2],&[(80,0);2],&[(0,0);2],GposRunMode::DesignOnly,Default::default()).unwrap();assert_eq!(x.edges,vec![None,None]);}
#[test]fn exact_work_limits(){
    let b=fixture(0,false);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let l=GposCursiveRunLimits{max_connections:1,max_span_visits:1,run:GposRunLimits{max_glyphs:2,max_roots:1,max_filter_visits:2,max_subtable_attempts:1},..Default::default()};
    let f=|l|execute_gpos_cursive_horizontal(&p,&[3,7],&[Default::default();2],&[(80,0);2],&[(0,0);2],GposRunMode::DesignOnly,l);
    assert!(f(l).is_ok());assert!(f(GposCursiveRunLimits{max_connections:0,..l}).is_err());assert!(f(GposCursiveRunLimits{max_span_visits:0,..l}).is_err());
}
#[test]fn overflow_atomic_and_length(){
    let b=fixture(0,false);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let prior=[GposRunAdjustment::default();2];
    assert!(execute_gpos_cursive_horizontal(&p,&[3,7],&prior,&[(i32::MIN,0);2],&[(0,0);2],GposRunMode::DesignOnly,Default::default()).is_err());
    assert!(execute_gpos_cursive_horizontal(&p,&[3,7],&prior,&[],&[(0,0);2],GposRunMode::DesignOnly,Default::default()).is_err());
}
#[test]fn design_only_preserves_pixel_values(){
    let b=fixture(0,false);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let mut prior=[GposRunAdjustment::default();2];prior[1].pixels.x_placement=12;
    let x=execute_gpos_cursive_horizontal(&p,&[3,7],&prior,&[(80,0);2],&[(0,0);2],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.adjustments[1].pixels,prior[1].pixels);
}
