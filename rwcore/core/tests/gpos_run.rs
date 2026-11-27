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

fn ctx(target:u16,sequence:u16)->Vec<u16>{
    // Two input positions both cover glyph 3, single action.
    vec![3,2,1,14,14,sequence,target,1,1,3]
}
fn context_run(b:&[u8],roots:&[u16],g:&[u16],l:GposContextRunLimits)->Result<Vec<GposRunAdjustment>,Error>{
    let p=compile_gpos_program(b,roots,32,None,Default::default()).unwrap();
    execute_gpos_context(&p,g,&vec![Default::default();g.len()],GposRunMode::DesignOnly,l)
}
#[test]fn nested_action_sequence_and_root_consumption(){
    let b=fixture(&[(7,0,vec![ctx(1,1)]),(1,0,vec![single(9)])]);
    let x=context_run(&b,&[0],&[3,3,3,3],Default::default()).unwrap();
    assert_eq!(x.iter().map(|v|v.design.x_advance).collect::<Vec<_>>(),vec![0,9,0,9]);
}
#[test]fn nested_multiple_levels_and_extension(){
    let mut ext=vec![1,7,0,8];ext.extend(ctx(2,0));
    let b=fixture(&[(7,0,vec![ctx(1,0)]),(9,0,vec![ext]),(1,0,vec![single(11)])]);
    let x=context_run(&b,&[0],&[3,3],Default::default()).unwrap();
    assert_eq!(x[0].design.x_advance,11);assert_eq!(x[1].design.x_advance,0);
}
#[test]fn actual_cycle_rejected_but_nonmatching_cycle_allowed(){
    let b=fixture(&[(7,0,vec![ctx(0,0)])]);
    assert!(context_run(&b,&[0],&[3,3],Default::default()).is_err());
    assert!(context_run(&b,&[0],&[4,4],Default::default()).is_ok());
}
#[test]fn context_exact_work_limits(){
    let b=fixture(&[(7,0,vec![ctx(1,1)]),(1,0,vec![single(9)])]);
    let l=GposContextRunLimits{max_depth:2,max_calls:2,max_actions:1,max_context_work:2,
        run:GposRunLimits{max_glyphs:2,max_roots:1,max_filter_visits:4,max_subtable_attempts:2}};
    assert!(context_run(&b,&[0],&[3,3],l).is_ok());
    for l in [GposContextRunLimits{max_depth:1,..l},GposContextRunLimits{max_calls:1,..l},
        GposContextRunLimits{max_actions:0,..l},GposContextRunLimits{max_context_work:1,..l},
        GposContextRunLimits{run:GposRunLimits{max_filter_visits:3,..l.run},..l}]{
        assert!(context_run(&b,&[0],&[3,3],l).is_err());
    }
}
#[test]fn parent_filter_maps_original_and_child_filter_skips(){
    let g=parse_gdef(&words(&[1,0,12,0,0,0,1,3,2,1,3]),32,Default::default()).unwrap();
    let b=fixture(&[(7,8,vec![ctx(1,1)]),(1,0,vec![single(9)])]);
    let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_context(&p,&[3,4,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x[1],GposRunAdjustment::default());assert_eq!(x[2].design.x_advance,9);
    let b=fixture(&[(7,8,vec![ctx(1,1)]),(1,2,vec![single(9)])]);
    let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_context(&p,&[3,4,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x,vec![GposRunAdjustment::default();3]);
}
#[test]fn empty_action_match_blocks_later_subtable(){
    let b=fixture(&[(7,0,vec![vec![3,1,0,8,1,1,3],ctx(1,0)]),(1,0,vec![single(9)])]);
    assert_eq!(context_run(&b,&[0],&[3,3],Default::default()).unwrap(),vec![GposRunAdjustment::default();2]);
}
#[test]fn chained_context_action(){
    // Backtrack 4, input 3, lookahead 5, one action targeting input.
    let chain=vec![3,1,24,1,30,1,36,1,0,1,0,0,1,1,4,1,1,3,1,1,5];
    let b=fixture(&[(8,0,vec![chain]),(1,0,vec![single(9)])]);
    let x=context_run(&b,&[0],&[4,3,5],Default::default()).unwrap();
    assert_eq!(x[1].design.x_advance,9);
    assert_eq!(context_run(&b,&[0],&[5,3,4],Default::default()).unwrap(),vec![GposRunAdjustment::default();3]);
}
#[test]fn nested_pair_applies_at_target_only(){
    let b=fixture(&[(7,0,vec![ctx(1,0)]),(2,0,vec![pair(true)])]);
    let x=context_run(&b,&[0],&[3,3,3],Default::default()).unwrap();
    assert_eq!(x[0].design.x_advance,10);assert_eq!(x[1].design.x_placement,5);assert_eq!(x[2],GposRunAdjustment::default());
}

fn mark_base()->Vec<u16>{vec![1,12,20,2,26,48,1,2,3,4,1,1,7,2,0,10,1,16,1,10,20,1,30,40,1,6,0,1,100,200]}
fn mark_mark()->Vec<u16>{vec![1,12,20,2,28,50,1,2,3,4,1,2,3,7,2,0,10,1,16,1,10,20,1,30,40,2,10,0,16,22,1,100,200,1,65526,30,1,50,65516]}
fn mark_gdef()->Gdef{parse_gdef(&words(&[1,0,12,0,0,0,1,3,5,3,3,0,0,1]),32,Default::default()).unwrap()}
#[test]fn mark_base_skips_classified_mark_and_resolves(){
    let g=mark_gdef();let b=fixture(&[(4,0,vec![mark_base()])]);let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_marks(&p,&[7,4,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.edges[2].unwrap().parent,0);assert!(x.edges[1].is_none());
    let v=x.resolve(&[(0,0),(100,0),(100,0)],&[(0,0);3],Default::default()).unwrap();
    assert_eq!((v[2].design.x_placement,v[2].design.y_placement),(-10,180));
}
#[test]fn mark_to_mark_chain_and_later_residual(){
    let g=mark_gdef();let b=fixture(&[(4,0,vec![mark_base()]),(6,0,vec![mark_mark()]),(1,0,vec![single(9)])]);
    let p=compile_gpos_program(&b,&[0,1,2],32,Some(&g),Default::default()).unwrap();
    let x=execute_gpos_marks(&p,&[7,3,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.edges[1].unwrap().parent,0);assert_eq!(x.edges[2].unwrap().parent,1);
    let v=x.resolve(&[(0,0),(100,0),(109,0)],&[(0,0);3],Default::default()).unwrap();
    assert_eq!((v[2].design.x_placement,v[2].design.y_placement),(71,360));assert_eq!(v[2].design.x_advance,9);
}
#[test]fn mark_parent_barrier_and_missing_gdef(){
    let b=fixture(&[(4,0,vec![mark_base()])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let x=execute_gpos_marks(&p,&[7,5,3],&[Default::default();3],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert!(x.edges[2].is_none());
    let x=execute_gpos_marks(&p,&[7,3],&[Default::default();2],GposRunMode::DesignOnly,Default::default()).unwrap();assert_eq!(x.edges[1].unwrap().parent,0);
}
#[test]fn mark_nested_extension_and_budget(){
    let g=mark_gdef();let mut ext=vec![1,4,0,8];ext.extend(mark_base());
    let c=vec![3,1,1,12,0,1,1,1,3];
    let b=fixture(&[(7,0,vec![c]),(9,0,vec![ext])]);let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let prior=[Default::default();2];let mode=GposRunMode::DesignOnly;
    let l=GposMarkRunLimits{max_parent_visits:1,max_attachment_updates:1,..Default::default()};
    assert_eq!(execute_gpos_marks(&p,&[7,3],&prior,mode,l).unwrap().edges[1].unwrap().parent,0);
    assert!(execute_gpos_marks(&p,&[7,3],&prior,mode,GposMarkRunLimits{max_parent_visits:0,..l}).is_err());
    assert!(execute_gpos_marks(&p,&[7,3],&prior,mode,GposMarkRunLimits{max_attachment_updates:0,..l}).is_err());
    assert!(execute_gpos_context(&p,&[7,3],&prior,mode,Default::default()).is_err());
}
#[test]fn mark_attachment_replaces_prior_placement(){
    let b=fixture(&[(4,0,vec![mark_base()])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let mut prior=[GposRunAdjustment::default();2];prior[1].design.x_placement=55;prior[1].design.x_advance=4;
    let x=execute_gpos_marks(&p,&[7,3],&prior,GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.residuals[1].design.x_placement,0);assert_eq!(x.residuals[1].design.x_advance,4);assert_eq!(prior[1].design.x_placement,55);
}

fn mark_ligature()->Vec<u16>{vec![1,12,20,2,26,48,1,2,3,4,1,1,7,2,0,10,1,16,1,10,20,1,30,40,1,4,2,10,0,16,22,1,100,200,1,300,400,1,500,600]}
#[test]fn ligature_component_occurrence_mapping(){
    use GposLigatureSource::*;
    let b=fixture(&[(5,0,vec![mark_ligature()])]);let g=mark_gdef();
    let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let sources=[Ligature{id:1,components:2},Mark{id:1,component:0},Mark{id:1,component:1}];
    let x=execute_gpos_ligature_marks(&p,&[7,3,4],&[Default::default();3],&sources,GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.edges[1].unwrap().design_delta,(90,180));
    assert_eq!(x.edges[2].unwrap().design_delta,(470,560));
    assert_eq!(x.edges[2].unwrap().parent,0);
}
#[test]fn ligature_fallback_and_null_anchor(){
    use GposLigatureSource::*;
    let b=fixture(&[(5,0,vec![mark_ligature()])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let prior=[Default::default();2];let m=GposRunMode::DesignOnly;
    let x=execute_gpos_ligature_marks(&p,&[7,4],&prior,&[Unmapped,Unmapped],m,Default::default()).unwrap();
    assert_eq!(x.edges[1].unwrap().design_delta,(470,560));
    let x=execute_gpos_ligature_marks(&p,&[7,4],&prior,&[Ligature{id:1,components:2},Mark{id:1,component:0}],m,Default::default()).unwrap();
    assert!(x.edges[1].is_none());
    assert!(execute_gpos_marks(&p,&[7,4],&prior,m,Default::default()).is_err());
}
#[test]fn ligature_invalid_sources_rejected(){
    use GposLigatureSource::*;
    let b=fixture(&[(5,0,vec![mark_ligature()])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    for sources in [[Ligature{id:1,components:2},Mark{id:2,component:0}],
        [Ligature{id:1,components:2},Mark{id:1,component:2}],
        [Ligature{id:1,components:3},Mark{id:1,component:0}],
        [Ligature{id:1,components:2},Ligature{id:1,components:2}]]{
        assert!(execute_gpos_ligature_marks(&p,&[7,3],&[Default::default();2],&sources,GposRunMode::DesignOnly,Default::default()).is_err());
    }
}
#[test]fn mark_source_identity_compatibility(){
    use GposLigatureSource::*;
    assert!(gpos_mark_sources_compatible(Unmapped,Unmapped));
    assert!(gpos_mark_sources_compatible(Mark{id:1,component:0},Mark{id:1,component:0}));
    assert!(!gpos_mark_sources_compatible(Mark{id:1,component:0},Mark{id:1,component:1}));
    assert!(!gpos_mark_sources_compatible(Mark{id:1,component:0},Mark{id:2,component:0}));
    assert!(!gpos_mark_sources_compatible(Unmapped,Mark{id:1,component:0}));
}
#[test]fn ligature_foreign_identity_uses_last_component(){
    use GposLigatureSource::*;
    assert_eq!(select_gpos_ligature_component(Ligature{id:1,components:2},Mark{id:2,component:0},2).unwrap(),1);
    assert!(select_gpos_ligature_component(Ligature{id:1,components:2},Mark{id:1,component:2},2).is_err());
}

 
fn mixed_cursive()->Vec<u16>{vec![1,14,2,0,22,28,0,1,2,3,7,1,100,20,1,65506,5]}
fn mixed(p:&CompiledGposProgram<'_>,glyphs:&[u16],l:GposMixedRunLimits)->Result<GposCursiveRun,Error>{
    let n=glyphs.len();
    execute_gpos_mixed_horizontal(p,glyphs,&vec![Default::default();n],&vec![(80,0);n],
        &vec![(0,0);n],&vec![GposLigatureSource::Unmapped;n],GposRunMode::DesignOnly,l)
}

#[test]fn mixed_cursive_then_mark_root(){
    let b=fixture(&[(3,0,vec![mixed_cursive()]),(4,0,vec![mark_base()])]);
    let p=compile_gpos_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let x=mixed(&p,&[3,7,3],Default::default()).unwrap();
    assert_eq!(x.edges[1].unwrap().parent,0);
    assert_eq!(x.edges[2].unwrap().parent,1);
    assert_eq!(x.design_origins[1].0,130);
    assert_eq!(x.adjustments[1].design.y_placement,15);
    assert_eq!(x.adjustments[2].design.y_placement,195);
    assert_eq!(x.design_origins[2].0+x.adjustments[2].design.x_placement,220);
}
#[test]fn mixed_nested_cursive_extension_and_order(){
    let context=vec![3,1,2,16,0,1,0,2,1,1,3];
    let mut ext=vec![1,3,0,8];ext.extend(mixed_cursive());
    let b=fixture(&[(7,0,vec![context]),(9,0,vec![ext]),(1,0,vec![single(9)])]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let x=mixed(&p,&[3,7],Default::default()).unwrap();
    assert_eq!(x.edges[1].unwrap().parent,0);
    assert_eq!(x.adjustments[0].design.x_advance,59);
    assert_eq!(x.design_origins[1].0,139);
    assert_eq!(x.adjustments[1].design.x_placement,-9);
    assert_eq!(x.adjustments[1].design.y_placement,15);
}
#[test]fn mixed_shared_budgets_and_repeated_parent(){
    let b=fixture(&[(3,0,vec![mixed_cursive()]),(3,0,vec![mixed_cursive()])]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(mixed(&p,&[3,7],Default::default()).is_ok());
    for l in [GposMixedRunLimits{max_connections:0,..Default::default()},
        GposMixedRunLimits{max_span_visits:0,..Default::default()},
        GposMixedRunLimits{marks:GposMarkRunLimits{max_attachment_updates:0,..Default::default()},..Default::default()}]{
        assert!(mixed(&p,&[3,7],l).is_err());
    }
    let p=compile_gpos_program(&b,&[0,1],32,None,Default::default()).unwrap();
    assert!(mixed(&p,&[3,7],Default::default()).is_err());
}
#[test]fn mixed_design_only_ignores_pixel_overflow(){
    let b=fixture(&[(3,1,vec![mixed_cursive()])]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let mut prior=[GposRunAdjustment::default();2];
    prior[0].pixels.x_advance=i32::MAX;prior[1].pixels.y_placement=i32::MIN;
    let x=execute_gpos_mixed_horizontal(&p,&[3,7],&prior,&[(80,0);2],&[(i32::MAX,0);2],
        &[GposLigatureSource::Unmapped;2],GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.edges[0].unwrap().parent,1);
    assert_eq!(x.adjustments[0].pixels,prior[0].pixels);
    assert_eq!(x.adjustments[1].pixels,prior[1].pixels);
    assert_eq!(x.pixel_origins,vec![(0,0);2]);
}
#[test]fn mixed_ligature_path_matches_legacy(){
    use GposLigatureSource::*;
    let b=fixture(&[(5,0,vec![mark_ligature()]),(1,0,vec![single(9)])]);
    let p=compile_gpos_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let sources=[Ligature{id:1,components:2},Mark{id:1,component:0}];
    let prior=[Default::default();2];
    let x=execute_gpos_mixed_horizontal(&p,&[7,3],&prior,&[(80,0);2],&[(0,0);2],
        &sources,GposRunMode::DesignOnly,Default::default()).unwrap();
    let old=execute_gpos_ligature_marks(&p,&[7,3],&prior,&sources,GposRunMode::DesignOnly,Default::default()).unwrap();
    assert_eq!(x.edges,old.edges);
    assert_eq!(x.adjustments,old.resolve(&x.design_origins,&x.pixel_origins,Default::default()).unwrap());
}

fn directional_cursive(rtl:bool)->Vec<u16>{
    let mut c=mixed_cursive();
    if rtl{c[12]=(-30i16) as u16;c[15]=100;}
    c
}
fn directed(p:&CompiledGposProgram<'_>,glyphs:&[u16],prior:&[GposRunAdjustment],nominal:&[(i32,i32)],flow:GposHorizontalFlow)->Result<GposCursiveRun,Error>{
    let n=glyphs.len();
    execute_gpos_mixed_directional(p,glyphs,prior,nominal,&vec![(0,0);n],
        &vec![GposLigatureSource::Unmapped;n],GposRunMode::DesignOnly,flow,Default::default())
}
#[test]fn directed_ltr_and_rtl_independent_lookup_flag(){
    use GposHorizontalFlow::*;
    for (flow,rtl) in [(LeftToRight,false),(RightToLeft,true)]{
        for flag in [0,1]{
            let b=fixture(&[(3,flag,vec![directional_cursive(rtl)])]);
            let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
            let x=directed(&p,&[3,7],&[Default::default();2],&[(80,0);2],flow).unwrap();
            let expected=if rtl{[0,50]}else{[50,0]};
            assert_eq!([x.adjustments[0].design.x_advance,x.adjustments[1].design.x_advance],expected);
            assert_eq!(x.design_origins,if rtl{vec![(-80,0),(-210,0)]}else{vec![(0,0),(130,0)]});
            let child=if flag==0{1}else{0};
            assert_eq!(x.edges[child].unwrap().parent,1-child);
            assert_eq!(x.adjustments[child].design.y_placement,if flag==0{15}else{-15});
        }
    }
}
#[test]fn directed_rtl_skipped_mark_preserves_middle_width(){
    let g=parse_gdef(&words(&[1,0,12,0,0,0,1,3,2,1,3]),32,Default::default()).unwrap();
    let b=fixture(&[(3,8,vec![directional_cursive(true)])]);
    let p=compile_gpos_program(&b,&[0],32,Some(&g),Default::default()).unwrap();
    let mut prior=[GposRunAdjustment::default();3];prior[1].design.x_advance=5;
    let x=directed(&p,&[3,4,7],&prior,&[(80,0),(20,0),(80,0)],GposHorizontalFlow::RightToLeft).unwrap();
    assert_eq!(x.design_origins,vec![(-80,0),(-105,0),(-210,0)]);
    assert_eq!(x.adjustments[2].design.x_advance,25);
    assert_eq!(x.adjustments[1],prior[1]);assert!(x.edges[1].is_none());
    assert_eq!(x.edges[2].unwrap().parent,0);
}
#[test]fn directed_math_reference_grid(){
    // Independent reference: explicitly search a small width domain for
    // coincident physical anchor coordinates, using the untouched fixed
    // endpoint placement and zero child placement. No production helper.
    use GposHorizontalFlow::*;
    for (flow,rtl) in [(LeftToRight,false),(RightToLeft,true)]{
        for flag in [0,1]{
            let b=fixture(&[(3,flag,vec![directional_cursive(rtl)])]);
            let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
            let (exit,entry)=if rtl{(-30,100)}else{(100,-30)};
            for pa in [-8,0,11]{for pb in [-6,0,13]{for na in [0,30,80]{for nb in [0,40,90]{
                let mut prior=[GposRunAdjustment::default();2];prior[0].design.x_placement=pa;prior[1].design.x_placement=pb;
                let fixed_a=if flag==0{pa}else{0};let fixed_b=if flag==0{0}else{pb};
                let mut candidates=Vec::new();
                for width in 0..=200{
                    let (xa,xb)=if rtl{(-na,-na-width)}else{(0,width)};
                    if xa+fixed_a+exit==xb+fixed_b+entry{candidates.push(width);}
                }
                assert_eq!(candidates.len(),1);
                let x=directed(&p,&[3,7],&prior,&[(na,0),(nb,0)],flow).unwrap();
                let (owner,nominal)=if rtl{(1,nb)}else{(0,na)};
                assert_eq!(nominal+x.adjustments[owner].design.x_advance,candidates[0]);
                assert_eq!(x.adjustments[1-owner].design.x_advance,0);
                assert_eq!(x.adjustments[0].design.x_placement,fixed_a);
                assert_eq!(x.adjustments[1].design.x_placement,fixed_b);
                assert_eq!(x.design_origins[0].0+fixed_a+exit,x.design_origins[1].0+fixed_b+entry);
            }}}}
        }
    }
}
#[test]fn directed_device_correction_independent(){
    let c=vec![1,14,2,0,22,40,0,1,2,3,7,3,(-30i16) as u16,20,10,0,10,12,1,0x6c00,1,100,5];
    let b=fixture(&[(3,0,vec![c])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let x=execute_gpos_mixed_directional(&p,&[3,7],&[Default::default();2],&[(80,0);2],&[(2,0),(3,0)],
        &[GposLigatureSource::Unmapped;2],GposRunMode::DevicePixels{x_ppem:10,y_ppem:10},GposHorizontalFlow::RightToLeft,Default::default()).unwrap();
    assert_eq!(x.adjustments[1].design.x_advance,50);
    assert_eq!(x.adjustments[1].pixels.x_advance,-4);
    assert_eq!(x.pixel_origins,vec![(-2,0),(-1,0)]);
    assert_eq!(x.edges[1].unwrap().pixel_delta,(1,0));
    assert_eq!(x.adjustments[1].pixels.x_placement,0);
}
#[test]fn directed_terminal_overflow_and_atomic_input(){
    use GposHorizontalFlow::*;
    let b=fixture(&[(1,0,vec![single(9)])]);let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let prior=[GposRunAdjustment::default()];
    for flow in [LeftToRight,RightToLeft]{
        assert!(directed(&p,&[3],&prior,&[(i32::MAX,0)],flow).is_err());
        assert_eq!(prior,[GposRunAdjustment::default()]);
    }
    assert!(directed(&p,&[3],&prior,&[],RightToLeft).is_err());
}
#[test]fn directed_nested_extension_keeps_logical_index(){
    let context=vec![3,1,1,12,0,1,1,1,3];
    let mut ext=vec![1,3,0,8];ext.extend(directional_cursive(true));
    let b=fixture(&[(7,0,vec![context]),(9,1,vec![ext])]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    let x=directed(&p,&[3,7],&[Default::default();2],&[(80,0);2],GposHorizontalFlow::RightToLeft).unwrap();
    assert_eq!(x.edges[0].unwrap().parent,1);
    assert_eq!(x.design_origins,vec![(-80,0),(-210,0)]);
    assert_eq!(x.adjustments[1].design.x_advance,50);
}
#[test]fn directed_mark_chain_resolves_physical_origins(){
    let b=fixture(&[(3,0,vec![directional_cursive(true)]),(4,0,vec![mark_base()])]);
    let p=compile_gpos_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let x=directed(&p,&[3,7,3],&[Default::default();3],&[(80,0),(80,0),(0,0)],GposHorizontalFlow::RightToLeft).unwrap();
    assert_eq!(x.edges[2].unwrap().parent,1);
    assert_eq!(x.design_origins,vec![(-80,0),(-210,0),(-210,0)]);
    assert_eq!(x.adjustments[2].design.x_placement,90);
    assert_eq!(x.adjustments[2].design.y_placement,195);
}
