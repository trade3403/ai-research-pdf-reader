use rwcore::*;
fn fixture(targets:&[Option<u16>])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,targets.len() as u16];
    w.resize(8+targets.len(),0);
    for (i,t) in targets.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        if let Some(t)=t{w.extend([7,0,1,8,3,1,1,12,0,*t,1,1,10]);}
        else{w.extend([1,0,1,8,1,8,1,20,1,1,10]);}
    }
    w.iter().flat_map(|x|x.to_be_bytes()).collect()
}
#[test]fn transitive_only_targets_not_scheduled(){
    let b=fixture(&[None,Some(0),Some(1),None]);
    let p=compile_gpos_program(&b,&[2],32,None,Default::default()).unwrap();
    assert_eq!(p.roots(),&[2]);assert_eq!(p.closure().reachable,vec![0,1,2]);
    assert!(p.lookup(3).is_none());assert!(p.lookup(0).is_some());
    assert_eq!(p.dependencies().iter().map(|e|e.site.lookup_index).collect::<Vec<_>>(),vec![1,2]);
}
#[test]fn cycles_compile_once_and_remain_diagnostic(){
    let b=fixture(&[Some(1),Some(0)]);
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    assert_eq!(p.lookups().len(),2);assert_eq!(p.closure().cycle_witness,Some(vec![0,1,0]));
}
#[test]fn shared_targets_compile_once(){
    let b=fixture(&[None,Some(0),Some(0)]);
    let p=compile_gpos_program(&b,&[1,2],32,None,Default::default()).unwrap();
    assert_eq!(p.lookups().len(),3);assert_eq!(p.dependencies().len(),2);
}
#[test]fn cumulative_budgets_across_dependencies(){
    let b=fixture(&[None,Some(0),Some(1)]);
    let p=compile_gpos_program(&b,&[2],32,None,Default::default()).unwrap();
    let bytes=p.lookups().iter().flat_map(|l|l.subtables()).map(|s|b.len()-s.target_offset()).sum();
    let l=GposProgramLimits{compile:GposCompileLimits{max_lookups:3,max_total_subtables:3,max_total_target_bytes:bytes,..Default::default()},
        dependencies:GposDependencyLimits{max_scan_records:8,max_actions:2},..Default::default()};
    assert!(compile_gpos_program(&b,&[2],32,None,l).is_ok());
    for l in [GposProgramLimits{compile:GposCompileLimits{max_lookups:2,..l.compile},..l},
        GposProgramLimits{compile:GposCompileLimits{max_total_subtables:2,..l.compile},..l},
        GposProgramLimits{compile:GposCompileLimits{max_total_target_bytes:bytes-1,..l.compile},..l},
        GposProgramLimits{dependencies:GposDependencyLimits{max_scan_records:7,..l.dependencies},..l},
        GposProgramLimits{dependencies:GposDependencyLimits{max_actions:1,..l.dependencies},..l},
        GposProgramLimits{closure:GposClosureLimits{max_stack_depth:2,..l.closure},..l}]{
        assert!(matches!(compile_gpos_program(&b,&[2],32,None,l),Err(Error::Limit(_))));
    }
}
#[test]fn invalid_roots_and_empty_program(){
    let b=fixture(&[None,None]);
    for r in [&[1,0][..],&[0,0],&[2]]{assert!(compile_gpos_program(&b,r,32,None,Default::default()).is_err());}
    assert!(compile_gpos_program(&b,&[],32,None,Default::default()).unwrap().lookups().is_empty());
}
#[test]fn invalid_nested_reference_rejected(){
    let b=fixture(&[Some(1)]);
    assert!(compile_gpos_program(&b,&[0],32,None,Default::default()).is_err());
}
#[test]fn real_plan_program_preserves_roots(){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),FontLimits::default()).unwrap();
    let face=f.face(0).unwrap();let b=face.table(*b"GPOS").unwrap();
    let g=parse_gdef(face.table(*b"GDEF").unwrap(),268,Default::default()).unwrap();
    let layout=parse_gpos_layout(b,Default::default()).unwrap();
    for s in &layout.scripts{
        let Some(lang)=&s.default_language else{continue;};
        let tags:Vec<_>=lang.features.iter().map(|&i|layout.features[i as usize].tag).collect();
        let plan=build_gpos_base_plan(&layout,s.tag,None,&tags,Default::default()).unwrap().unwrap();
        let p=compile_gpos_plan_program(b,&plan,268,Some(&g),Default::default()).unwrap();
        assert_eq!(p.roots(),plan.lookups.iter().map(|r|r.index).collect::<Vec<_>>());
        assert!(p.closure().is_complete());
        let mut bad=plan.clone();bad.script=*b"zzzz";
        assert!(compile_gpos_plan_program(b,&bad,268,Some(&g),Default::default()).is_err());
    }
}

#[test]fn extension_dependency_and_wrapper_budget(){
    // lookup 0: Type 9 -> Type 7 format 3 -> lookup 1; lookup 1: single.
    let mut v:Vec<u16>=vec![1,0,10,12,14,0,0,2,6,0,9,0,1,8,1,7,0,8,3,1,1,12,0,1,1,1,10];
    v[9]=((v.len()-7)*2) as u16;
    v.extend([1,0,1,8,1,8,1,20,1,1,10]);
    let b:Vec<u8>=v.iter().flat_map(|x|x.to_be_bytes()).collect();
    let p=compile_gpos_program(&b,&[0],32,None,Default::default()).unwrap();
    assert_eq!(p.roots(),&[0]);assert_eq!(p.closure().reachable,vec![0,1]);
    assert_eq!(p.lookup(0).unwrap().declared_kind(),9);
    let cost:usize=p.lookups().iter().flat_map(|l|l.subtables().iter().map(move |s|(l.declared_kind(),s))).map(|(k,s)|b.len()-s.target_offset()+if k==9{8}else{0}).sum();
    let l=GposProgramLimits{compile:GposCompileLimits{max_total_target_bytes:cost,..Default::default()},..Default::default()};
    assert!(compile_gpos_program(&b,&[0],32,None,l).is_ok());
    assert!(compile_gpos_program(&b,&[0],32,None,GposProgramLimits{compile:GposCompileLimits{max_total_target_bytes:cost-1,..l.compile},..l}).is_err());
}
