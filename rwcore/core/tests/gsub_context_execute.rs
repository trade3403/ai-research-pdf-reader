use rwcore::*;
fn fixture(spec:&[(u16,u16,u16,Vec<Vec<u16>>)]) -> Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,spec.len() as u16];
    w.resize(8+spec.len(),0);
    for (i,(kind,effective,flags,subs)) in spec.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        w.extend([*kind,*flags,subs.len() as u16]);w.resize(at+3+subs.len(),0);
        for (j,sub) in subs.iter().enumerate(){
            w[at+3+j]=((w.len()-at)*2) as u16;
            if *kind==7{w.extend([1,*effective,0,8]);}
            w.extend_from_slice(sub);
        }
    }
    w.iter().flat_map(|v|v.to_be_bytes()).collect()
}
fn single(g:u16,v:u16)->Vec<u16>{vec![2,8,1,v,1,1,g]}
fn ctx1(first:u16,tail:&[u16],actions:&[(u16,u16)])->Vec<u16>{
    let mut w=vec![1,8,1,14,1,1,first,1,4,tail.len() as u16+1,actions.len() as u16];
    w.extend_from_slice(tail);for &(p,l) in actions{w.extend([p,l]);}w
}
fn ctx2(actions:&[(u16,u16)])->Vec<u16>{
    // covered first 10; class 1 for glyphs 10 and 11; class 0 otherwise.
    let mut w=vec![2,12,18,2,0,28,1,1,10,1,10,2,1,1,1,4,2,actions.len() as u16,1];
    for &(p,l) in actions{w.extend([p,l]);}w
}
fn ctx3(input:&[u16],actions:&[(u16,u16)])->Vec<u16>{
    let mut w=vec![3,input.len() as u16,actions.len() as u16];
    w.resize(3+input.len(),0);
    for &(p,l) in actions{w.extend([p,l]);}
    for (i,&g) in input.iter().enumerate(){w[3+i]=(w.len()*2) as u16;w.extend([1,1,g]);}
    w
}
fn gdef()->Gdef{
    let w=[1u16,2,14,0,0,34,48,1,10,7,1,2,3,3,3,4,0,
        1,12,4,1,2,0,0,1,2,0,12,0,20,1,2,10,13,1,2,12,14];
    let b:Vec<_>=w.iter().flat_map(|v|v.to_be_bytes()).collect();
    parse_gdef(&b,32,Default::default()).unwrap()
}
fn run(b:&[u8],roots:&[u16],input:&[u16],g:Option<&Gdef>)->GsubContextExecResult{
    let p=compile_gsub_program(b,roots,32,g,Default::default()).unwrap();
    execute_gsub_context(&p,input,Default::default()).unwrap()
}
#[test]fn context_all_formats_and_wrapped(){
    for (kind,sub) in [(5,ctx1(10,&[11],&[(1,0)])),(5,ctx2(&[(1,0)])),(7,ctx3(&[10,11],&[(1,0)]))]{
        let b=fixture(&[(1,1,0,vec![single(11,20)]),(kind,5,0,vec![sub])]);
        let a=run(&b,&[1],&[10,11,9,11],None);
        assert_eq!(a.glyphs,vec![10,20,9,11]);
        assert_eq!((a.calls,a.actions,a.matches,a.changes,a.peak_depth),(4,1,2,1,2));
    }
}
#[test]fn context_filtered_physical_positions_and_target_filter(){
    let g=gdef();
    let b=fixture(&[(1,1,0,vec![single(11,12)]),(1,1,LOOKUP_IGNORE_MARK,vec![single(12,20)]),
        (5,5,LOOKUP_IGNORE_MARK,vec![ctx3(&[10,11],&[(1,0),(1,1)])])]);
    let a=run(&b,&[2],&[10,13,14,11,9],Some(&g));
    assert_eq!(a.glyphs,vec![10,13,14,12,9]);
    assert_eq!((a.actions,a.matches,a.changes),(2,2,1));
    assert_eq!(run(&b,&[2],&[10,13,11],None).glyphs,vec![10,13,11]);
}
#[test]fn context_action_order_and_dependency_not_root(){
    let b=fixture(&[(1,1,0,vec![single(10,11)]),(1,1,0,vec![single(11,12)]),
        (5,5,0,vec![ctx1(10,&[],&[(0,0),(0,1)])])]);
    let a=run(&b,&[2],&[10,11],None);
    assert_eq!(a.glyphs,vec![12,11]);assert_eq!((a.actions,a.matches,a.changes),(2,3,2));
}
#[test]fn context_nested_at_position_not_global_pass(){
    let b=fixture(&[(7,1,0,vec![single(11,20)]),(5,5,0,vec![ctx3(&[10,11],&[(1,0)])]),
        (5,5,0,vec![ctx1(9,&[10],&[(1,1)])])]);
    let a=run(&b,&[2],&[9,10,11,11],None);
    assert_eq!(a.glyphs,vec![9,10,20,11]);assert_eq!((a.actions,a.peak_depth),(2,3));
}
#[test]fn context_empty_actions_block_fallback_and_advance(){
    let b=fixture(&[(1,1,0,vec![single(10,20)]),
        (5,5,0,vec![ctx1(10,&[10],&[]),ctx1(10,&[],&[(0,0)])])]);
    let a=run(&b,&[1],&[10,10,10],None);
    assert_eq!(a.glyphs,vec![10,10,20]);assert_eq!((a.matches,a.actions,a.changes),(3,1,1));
}
#[test]fn context_active_cycle_fails_but_nonexecuted_cycle_allowed(){
    let b=fixture(&[(5,5,0,vec![ctx1(10,&[],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(p.closure().cycle_witness.is_some());
    assert_eq!(execute_gsub_context(&p,&[9],Default::default()).unwrap().glyphs,vec![9]);
    assert!(matches!(execute_gsub_context(&p,&[10],Default::default()),Err(Error::Invalid(_))));
    let b=fixture(&[(5,5,0,vec![ctx1(10,&[10],&[(1,0)])])]);
    let a=run(&b,&[0],&[10,10,10],None);
    assert_eq!((a.actions,a.peak_depth),(2,3));
}
#[test]fn context_exact_cumulative_limits(){
    let b=fixture(&[(1,1,0,vec![single(11,20)]),(5,5,0,vec![ctx3(&[10,11],&[(1,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    let input=[10,11];
    let l=GsubContextExecLimits{max_glyphs:2,max_roots:1,max_lookups:2,max_depth:2,max_calls:2,
        max_probes:2,max_filter_checks:3,max_rules:1,max_symbols:2,max_actions:1,max_matches:2};
    let a=execute_gsub_context(&p,&input,l).unwrap();
    assert_eq!(a.glyphs,vec![10,20]);
    assert_eq!((a.calls,a.probes,a.filter_checks,a.rules,a.symbols,a.actions,a.matches,a.peak_depth),(2,2,3,1,2,1,2,2));
    for bad in [GsubContextExecLimits{max_glyphs:1,..l},GsubContextExecLimits{max_roots:0,..l},
        GsubContextExecLimits{max_lookups:1,..l},GsubContextExecLimits{max_depth:1,..l},
        GsubContextExecLimits{max_calls:1,..l},GsubContextExecLimits{max_probes:1,..l},
        GsubContextExecLimits{max_filter_checks:2,..l},GsubContextExecLimits{max_rules:0,..l},
        GsubContextExecLimits{max_symbols:1,..l},GsubContextExecLimits{max_actions:0,..l},
        GsubContextExecLimits{max_matches:1,..l}]{
        assert!(matches!(execute_gsub_context(&p,&input,bad),Err(Error::Limit(_))));
        assert_eq!(input,[10,11]);
    }
}
#[test]fn context_hard_depth_cap(){
    let mut spec=Vec::new();
    for i in 0..GSUB_CONTEXT_MAX_DEPTH as u16{spec.push((5,5,0,vec![ctx1(10,&[],&[(0,i+1)])]));}
    spec.push((1,1,0,vec![single(10,11)]));
    let b=fixture(&spec);
    let p=compile_gsub_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_context(&p,&[10],GsubContextExecLimits{max_depth:usize::MAX,..Default::default()}),Err(Error::Limit(_))));
}
#[test]fn context_unsupported_reachable_kind_even_on_empty(){
    let multiple=vec![1,8,1,14,1,1,10,2,11,12];
    let b=fixture(&[(2,2,0,vec![multiple]),(5,5,0,vec![ctx1(10,&[],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_context(&p,&[],Default::default()),Err(Error::Unsupported(_))));
    let p=compile_gsub_program(&b,&[],32,None,Default::default()).unwrap();
    assert_eq!(execute_gsub_context(&p,&[31],Default::default()).unwrap().glyphs,vec![31]);
    assert!(execute_gsub_context(&p,&[32],Default::default()).is_err());
}
#[test]fn context_single_roots_compatibility(){
    let b=fixture(&[(1,1,0,vec![single(10,11),single(11,12)]),(7,1,0,vec![single(11,13)])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let input=[10,11,9];
    let a=execute_gsub_context(&p,&input,Default::default()).unwrap();
    let s=execute_gsub_single(&p,&input,Default::default()).unwrap();
    assert_eq!(a.glyphs,s.glyphs);assert_eq!((a.calls,a.probes,a.matches,a.changes),(s.visits,s.probes,s.matches,s.changes));
}


fn alt(g:u16,seq:&[u16])->Vec<u16>{
    let mut w=vec![1,8,1,14,1,1,g,seq.len() as u16];w.extend_from_slice(seq);w
}
fn choice(lookup_index:u16,candidate_index:usize)->GsubAlternateSelection{
    GsubAlternateSelection{lookup_index,candidate_index}
}
#[test]fn contextual_alternate_all_formats_and_wrapper(){
    for sub in [ctx1(10,&[11],&[(1,0)]),ctx2(&[(1,0)]),ctx3(&[10,11],&[(1,0)])]{
        let b=fixture(&[(7,3,0,vec![alt(11,&[20,21])]),(5,5,0,vec![sub])]);
        let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
        for (i,v) in [20,21].into_iter().enumerate(){
            let a=execute_gsub_context_with_alternates(&p,&[10,11,11],&[choice(0,i)],Default::default()).unwrap();
            assert_eq!(a.glyphs,vec![10,v,11]);assert_eq!((a.actions,a.matches,a.changes,a.peak_depth),(1,2,1,2));
        }
        assert!(matches!(execute_gsub_context(&p,&[],Default::default()),Err(Error::Unsupported(_))));
    }
}
#[test]fn contextual_alternate_reachable_exact_choices(){
    let b=fixture(&[(3,3,0,vec![alt(10,&[11])]),(3,3,0,vec![alt(11,&[12])]),
        (5,5,0,vec![ctx1(10,&[],&[(0,0),(0,1)])]),(3,3,0,vec![alt(12,&[13])])]);
    let p=compile_gsub_program(&b,&[2],32,None,Default::default()).unwrap();
    for bad in [vec![],vec![choice(0,0)],vec![choice(1,0),choice(0,0)],
        vec![choice(0,0),choice(0,0)],vec![choice(0,0),choice(3,0)],
        vec![choice(0,0),choice(1,0),choice(3,0)]]{
        assert!(matches!(execute_gsub_context_with_alternates(&p,&[],&bad,Default::default()),Err(Error::Invalid(_))));
    }
    let a=execute_gsub_context_with_alternates(&p,&[10,11],&[choice(0,0),choice(1,0)],Default::default()).unwrap();
    assert_eq!(a.glyphs,vec![12,11]);assert_eq!((a.actions,a.matches,a.changes),(2,3,2));
}
#[test]fn contextual_alternate_first_match_and_atomic_invalid(){
    let b=fixture(&[(1,1,0,vec![single(10,11)]),(3,3,0,vec![alt(11,&[11]),alt(11,&[20,21])]),
        (5,5,0,vec![ctx1(10,&[],&[(0,0),(0,1)])])]);
    let p=compile_gsub_program(&b,&[2],32,None,Default::default()).unwrap();
    let input=[10,9];
    assert!(matches!(execute_gsub_context_with_alternates(&p,&input,&[choice(1,1)],Default::default()),Err(Error::Invalid(_))));
    assert_eq!(input,[10,9]);
    let a=execute_gsub_context_with_alternates(&p,&input,&[choice(1,0)],Default::default()).unwrap();
    assert_eq!(a.glyphs,vec![11,9]);assert_eq!((a.matches,a.changes),(3,1));
}
#[test]fn contextual_alternate_filter_after_prior_action(){
    let g=gdef();
    let b=fixture(&[(1,1,0,vec![single(11,12)]),(3,3,LOOKUP_IGNORE_MARK,vec![alt(12,&[20])]),
        (5,5,LOOKUP_IGNORE_MARK,vec![ctx3(&[10,11],&[(1,0),(1,1)])])]);
    let p=compile_gsub_program(&b,&[2],32,Some(&g),Default::default()).unwrap();
    let a=execute_gsub_context_with_alternates(&p,&[10,13,11],&[choice(1,usize::MAX)],Default::default()).unwrap();
    assert_eq!(a.glyphs,vec![10,13,12]);assert_eq!((a.actions,a.matches,a.changes),(2,2,1));
}
#[test]fn contextual_alternate_nested_and_root_selection_shared(){
    let b=fixture(&[(3,3,0,vec![alt(10,&[11,12]),alt(12,&[13,14])]),
        (5,5,0,vec![ctx1(9,&[10],&[(1,0)])]),(5,5,0,vec![ctx1(9,&[],&[(0,1)])])]);
    let p=compile_gsub_program(&b,&[0,2],32,None,Default::default()).unwrap();
    // Root 0 changes 10 to 12, so nested context no longer matches.
    let a=execute_gsub_context_with_alternates(&p,&[9,10],&[choice(0,1)],Default::default()).unwrap();
    assert_eq!(a.glyphs,vec![9,12]);assert_eq!(a.actions,1);
    let p=compile_gsub_program(&b,&[2],32,None,Default::default()).unwrap();
    let a=execute_gsub_context_with_alternates(&p,&[9,10,10],&[choice(0,1)],Default::default()).unwrap();
    assert_eq!(a.glyphs,vec![9,12,10]);assert_eq!((a.actions,a.peak_depth),(2,3));
}
#[test]fn contextual_alternate_exact_work_limits(){
    let b=fixture(&[(3,3,0,vec![alt(11,&[20,21])]),(5,5,0,vec![ctx3(&[10,11],&[(1,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    let input=[10,11];let choices=[choice(0,1)];
    let l=GsubContextExecLimits{max_glyphs:2,max_roots:1,max_lookups:2,max_depth:2,max_calls:2,
        max_probes:2,max_filter_checks:3,max_rules:1,max_symbols:2,max_actions:1,max_matches:2};
    assert_eq!(execute_gsub_context_with_alternates(&p,&input,&choices,l).unwrap().glyphs,vec![10,21]);
    for bad in [GsubContextExecLimits{max_calls:1,..l},GsubContextExecLimits{max_probes:1,..l},
        GsubContextExecLimits{max_filter_checks:2,..l},GsubContextExecLimits{max_rules:0,..l},
        GsubContextExecLimits{max_symbols:1,..l},GsubContextExecLimits{max_actions:0,..l},
        GsubContextExecLimits{max_matches:1,..l},GsubContextExecLimits{max_depth:1,..l}]{
        assert!(matches!(execute_gsub_context_with_alternates(&p,&input,&choices,bad),Err(Error::Limit(_))));
    }
}
#[test]fn contextual_alternate_legacy_equivalence_and_unsupported(){
    let b=fixture(&[(1,1,0,vec![single(10,11)]),(5,5,0,vec![ctx1(10,&[],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert_eq!(execute_gsub_context_with_alternates(&p,&[10,9],&[],Default::default()).unwrap(),
        execute_gsub_context(&p,&[10,9],Default::default()).unwrap());
    let b=fixture(&[(2,2,0,vec![alt(10,&[11,12])]),(5,5,0,vec![ctx1(10,&[],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_context_with_alternates(&p,&[],&[],Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn contextual_alternate_exhaustive_small_reference(){
    // Independent finite-domain reference for a filtered two-symbol rule.
    let g=gdef();
    let b=fixture(&[(3,3,0,vec![alt(11,&[20,21])]),(5,5,LOOKUP_IGNORE_MARK,vec![ctx3(&[10,11],&[(1,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,Some(&g),Default::default()).unwrap();
    let alphabet=[9,10,11,12,13];
    for code in 0usize..625{
        let input:Vec<_>=(0..4).map(|i|alphabet[(code/5usize.pow(i))%5]).collect();
        for (index,v) in [20,21].into_iter().enumerate(){
            let mut want=input.clone();
            let visible:Vec<_>=input.iter().enumerate().filter(|(_,g)|**g!=12&&**g!=13).map(|(i,_)|i).collect();
            let mut k=0;
            while k+1<visible.len(){
                if input[visible[k]]==10&&input[visible[k+1]]==11{want[visible[k+1]]=v;k+=2;}else{k+=1;}
            }
            assert_eq!(execute_gsub_context_with_alternates(&p,&input,&[choice(0,index)],Default::default()).unwrap().glyphs,want);
        }
    }
}


fn expansion_run(b:&[u8],roots:&[u16],input:&[u16],g:Option<&Gdef>,choices:&[GsubAlternateSelection])->GsubExpansionExecResult{
    let p=compile_gsub_program(b,roots,32,g,Default::default()).unwrap();
    execute_gsub_context_with_expansion(&p,input,choices,Default::default()).unwrap()
}
#[test]fn context_expansion_rebases_later_and_repeated_actions(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[20,21])]),(1,1,0,vec![single(11,22)]),
        (1,1,0,vec![single(20,23)]),(5,5,0,vec![ctx3(&[10,11],&[(0,0),(1,1),(0,2)])])]);
    let a=expansion_run(&b,&[3],&[10,11,9],None,&[]);
    assert_eq!(a.execution.glyphs,vec![23,21,22,9]);
    assert_eq!((a.execution.actions,a.execution.matches,a.execution.changes),(3,4,3));
    assert_eq!((a.multiple_matches,a.inserted_glyphs),(1,1));
}
#[test]fn context_expansion_all_formats_and_wrapped(){
    for sub in [ctx1(10,&[11],&[(0,0),(1,1)]),ctx2(&[(0,0),(1,1)]),ctx3(&[10,11],&[(0,0),(1,1)])]{
        let b=fixture(&[(7,2,0,vec![alt(10,&[20,21,22])]),(1,1,0,vec![single(11,23)]),
            (7,5,0,vec![sub])]);
        assert_eq!(expansion_run(&b,&[2],&[10,11,11],None,&[]).execution.glyphs,vec![20,21,22,23,11]);
    }
}
#[test]fn context_expansion_rebases_every_active_frame(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[20,21])]),(1,1,0,vec![single(11,22)]),
        (1,1,0,vec![single(15,23)]),(5,5,0,vec![ctx3(&[10,11],&[(0,0),(1,1)])]),
        (5,5,0,vec![ctx3(&[9,10,11,15],&[(1,3),(3,2)])])]);
    let a=expansion_run(&b,&[4],&[9,10,11,15,15],None,&[]);
    assert_eq!(a.execution.glyphs,vec![9,20,21,22,23,15]);
    assert_eq!((a.execution.actions,a.execution.peak_depth,a.inserted_glyphs),(4,3,1));
}
#[test]fn context_expansion_preserves_filtered_gaps_and_rechecks_target(){
    let g=gdef();
    let b=fixture(&[(2,2,0,vec![alt(10,&[12,20])]),(1,1,LOOKUP_IGNORE_MARK,vec![single(12,21)]),
        (1,1,0,vec![single(11,22)]),(5,5,LOOKUP_IGNORE_MARK,vec![ctx3(&[10,11],&[(0,0),(0,1),(1,2)])])]);
    let a=expansion_run(&b,&[3],&[10,13,11,9],Some(&g),&[]);
    assert_eq!(a.execution.glyphs,vec![12,20,13,22,9]);
    assert_eq!((a.execution.actions,a.execution.matches),(3,3));
}
#[test]fn context_expansion_root_skip_and_later_root_visibility(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[10,11]),alt(11,&[20,21])]),
        (1,1,0,vec![single(11,22)])]);
    let a=expansion_run(&b,&[0,1],&[10,11],None,&[]);
    assert_eq!(a.execution.glyphs,vec![10,22,20,21]);
    assert_eq!((a.multiple_matches,a.inserted_glyphs,a.execution.calls),(2,2,6));
}
#[test]fn context_expansion_last_position_extends_root_boundary(){
    let b=fixture(&[(2,2,0,vec![alt(11,&[10,11])]),
        (5,5,0,vec![ctx3(&[10,11],&[(1,0)])])]);
    let a=expansion_run(&b,&[1],&[10,11,10,11],None,&[]);
    assert_eq!(a.execution.glyphs,vec![10,10,11,10,10,11]);
    assert_eq!(a.multiple_matches,2);
}
#[test]fn context_expansion_outside_parent_span_keeps_parent_boundary(){
    let b=fixture(&[(2,2,0,vec![alt(11,&[20,21])]),(1,1,0,vec![single(20,22)]),
        (5,5,0,vec![ctx3(&[10,11],&[(1,0)])]),
        (5,5,0,vec![ctx1(10,&[],&[(0,2)]),ctx1(20,&[],&[(0,1)])])]);
    let a=expansion_run(&b,&[3],&[10,11],None,&[]);
    assert_eq!(a.execution.glyphs,vec![10,22,21]);
    assert_eq!(a.multiple_matches,1);
}
#[test]fn context_expansion_exact_limits_and_immutable_input(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[20,21])]),(5,5,0,vec![ctx3(&[10,11],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    let input=[10,11];
    // two writes, one moved suffix glyph, four parent anchor/boundary
    // inspections and two child anchor/boundary inspections.
    let l=GsubExpansionExecLimits{context:GsubContextExecLimits{max_glyphs:3,max_roots:1,max_lookups:2,
        max_depth:2,max_calls:2,max_probes:2,max_filter_checks:3,max_rules:1,max_symbols:2,max_actions:1,max_matches:2},
        max_inserted:1,max_edit_work:9};
    let a=execute_gsub_context_with_expansion(&p,&input,&[],l).unwrap();
    assert_eq!(a.execution.glyphs,vec![20,21,11]);assert_eq!(a.edit_work,9);
    for bad in [GsubExpansionExecLimits{max_inserted:0,..l},GsubExpansionExecLimits{max_edit_work:8,..l},
        GsubExpansionExecLimits{context:GsubContextExecLimits{max_glyphs:2,..l.context},..l},
        GsubExpansionExecLimits{context:GsubContextExecLimits{max_actions:0,..l.context},..l},
        GsubExpansionExecLimits{context:GsubContextExecLimits{max_matches:1,..l.context},..l}]{
        assert!(matches!(execute_gsub_context_with_expansion(&p,&input,&[],bad),Err(Error::Limit(_))));
        assert_eq!(input,[10,11]);
    }
}
#[test]fn context_expansion_singleton_identity_and_alternate(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[10]),alt(10,&[20,21])]),
        (3,3,0,vec![alt(10,&[22,23])]),(5,5,0,vec![ctx1(10,&[],&[(0,0),(0,1)])])]);
    let a=expansion_run(&b,&[2],&[10],None,&[choice(1,1)]);
    assert_eq!(a.execution.glyphs,vec![23]);
    assert_eq!((a.multiple_matches,a.inserted_glyphs,a.edit_work,a.execution.changes),(1,0,1,1));
}
#[test]fn context_expansion_cycle_and_legacy_rejection(){
    let b=fixture(&[(2,2,0,vec![alt(10,&[10,11])]),(5,5,0,vec![ctx1(10,&[],&[(0,0),(0,1)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_context_with_expansion(&p,&[10],&[],Default::default()),Err(Error::Invalid(_))));
    assert!(matches!(execute_gsub_context_with_alternates(&p,&[],&[],Default::default()),Err(Error::Unsupported(_))));
    let b=fixture(&[(1,1,0,vec![single(10,11)]),(5,5,0,vec![ctx1(10,&[],&[(0,0)])])]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    let a=execute_gsub_context_with_expansion(&p,&[10,9],&[],Default::default()).unwrap();
    assert_eq!(a.execution,execute_gsub_context(&p,&[10,9],Default::default()).unwrap());
    assert_eq!((a.multiple_matches,a.inserted_glyphs,a.edit_work),(0,0,0));
}
