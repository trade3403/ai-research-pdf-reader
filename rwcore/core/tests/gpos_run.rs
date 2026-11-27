use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture(lookups:&[(u16,u16,Vec<Vec<u16>>)]) -> Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,lookups.len() as u16];w.resize(8+lookups.len(),0);
    for (i,(kind,flags,subs)) in lookups.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;w.extend([*kind,*flags,subs.len() as u16]);w.resize(w.len()+subs.len(),0);
        for (j,s) in subs.iter().enumerate(){w[at+3+j]=((w.len()-at)*2) as u16;w.extend(s);}
    }words(&w)
}
fn single(v:u16)->Vec<u16>{vec![1,8,4,v,1,1,3]}
fn pair(second:bool)->Vec<u16>{
    if second{vec![1,12,4,1,1,18,1,1,3,1,3,10,5]}
    else{vec![1,12,4,0,1,18,1,1,3,1,3,10]}
}
fn run(b:&[u8],roots:&[u16],g:&[u16])->Vec<GposRunAdjustment>{
    let p=compile_gpos_program(b,roots,32,None,Default::default()).unwrap();
    execute_gpos_single_pair(&p,g,&vec![Default::default();g.len()],GposRunMode::DesignOnly,Default::default()).unwrap()
}
#[test]fn first_match_zero_and_root_accumulation(){
    let b=fixture(&[(1,0,vec![single(0),single(20)]),(1,0,vec![single(7)])]);
    let x=run(&b,&[0,1],&[3,4,3]);assert_eq!(x.iter().map(|v|v.design.x_advance).collect::<Vec<_>>(),vec![7,0,7]);
}
#[test]fn pair_second_absent_revisits(){
    let b=fixture(&[(2,0,vec![pair(false)])]);let x=run(&b,&[0],&[3,3,3]);
    assert_eq!(x.iter().map(|v|v.design.x_advance).collect::<Vec<_>>(),vec![10,10,0]);
}
#[test]fn pair_second_present_consumes(){
    let b=fixture(&[(2,0,vec![pair(true)])]);let x=run(&b,&[0],&[3,3,3]);
    assert_eq!(x.iter().map(|v|v.design.x_advance).collect::<Vec<_>>(),vec![10,0,0]);assert_eq!(x[1].design.x_placement,5);
}
#[test]fn class_pair_dispatch(){
    let b=fixture(&[(2,0,vec![vec![2,32,4,1,40,48,2,2,10,11,20,21,30,31,40,41,1,2,3,4,1,3,1,1,2,1,7,8,1]])]);
    let x=run(&b,&[0],&[3,8]);assert_eq!(x[0].design.x_advance,40);assert_eq!(x[1].design.x_placement,41);
}
#[test]fn extension_single_dispatch(){
    let mut s=vec![1,1,0,8];s.extend(single(8));let b=fixture(&[(9,0,vec![s])]);
    assert_eq!(run(&b,&[0],&[3])[0].design.x_advance,8);
}
#[test]fn device_separate_and_overflow_atomic(){
    let b=fixture(&[(1,0,vec![vec![2,12,0x10,2,20,20,1,2,3,4,10,12,1,0x6c00]])]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let mode=GposRunMode::DevicePixels{x_ppem:11,y_ppem:12};
    let x=execute_gpos_single_pair(&p,&[3],&[Default::default()],mode,Default::default()).unwrap();
    assert_eq!(x[0].pixels.x_placement,-2);assert_eq!(x[0].design,GposAdjustment::default());
    let prior=[GposRunAdjustment{pixels:GposAdjustment{x_placement:i32::MIN,..Default::default()},..Default::default()}];
    assert!(execute_gpos_single_pair(&p,&[3],&prior,mode,Default::default()).is_err());assert_eq!(prior[0].pixels.x_placement,i32::MIN);
}
#[test]fn exact_budgets_and_domain(){
    let b=fixture(&[(1,0,vec![single(2)])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let prior=[Default::default();2];let m=GposRunMode::DesignOnly;
    let l=GposRunLimits{max_glyphs:2,max_roots:1,max_filter_visits:2,max_subtable_attempts:2};
    assert!(execute_gpos_single_pair(&p,&[3,4],&prior,m,l).is_ok());
    for l in [GposRunLimits{max_glyphs:1,..l},GposRunLimits{max_roots:0,..l},GposRunLimits{max_filter_visits:1,..l},GposRunLimits{max_subtable_attempts:1,..l}]{assert!(execute_gpos_single_pair(&p,&[3,4],&prior,m,l).is_err());}
    assert!(execute_gpos_single_pair(&p,&[3,32],&prior,m,l).is_err());
    assert!(execute_gpos_single_pair(&p,&[3],&prior,m,l).is_err());
}
#[test]fn unsupported_context_even_empty(){
    let b=fixture(&[(7,0,vec![vec![3,1,0,8,1,1,10]])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(execute_gpos_single_pair(&p,&[],&[],GposRunMode::DesignOnly,Default::default()).is_err());
}
#[test]fn ignored_mark_original_index_mapping(){
    // GDEF GlyphClassDef format 1: glyph 3 base, glyph 4 mark.
    let g=parse_gdef(&words(&[1,0,12,0,0,0,1,3,2,1,3]),32,Default::default()).unwrap();
    let b=fixture(&[(2,8,vec![pair(true)])]);let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_single_pair(&p,&[3,4,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x[0].design.x_advance,10);assert_eq!(x[1],GposRunAdjustment::default());assert_eq!(x[2].design.x_placement,5);
}
