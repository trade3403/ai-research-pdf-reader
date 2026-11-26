use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
type Rule<'a>=(u16,&'a [u16]);
type Lookup<'a>=(u16,u16,u16,Option<u16>,&'a [Rule<'a>]);
// declared kind, effective kind, flags, mark-set, ordered subtables
fn fixture(spec:&[Lookup<'_>])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,spec.len() as u16];
    w.resize(8+spec.len(),0);
    for (i,&(kind,effective,flags,mark,rules)) in spec.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        w.extend([kind,flags,rules.len() as u16]);
        w.resize(at+3+rules.len(),0);
        if let Some(m)=mark{w.push(m);}
        for (j,&(input,seq)) in rules.iter().enumerate(){
            w[at+3+j]=((w.len()-at)*2) as u16;
            if kind==7{w.extend([1,effective,0,8]);}
            match effective{
                1=>{assert_eq!(seq.len(),1);w.extend([2,8,1,seq[0],1,1,input]);}
                2|3=>{w.extend([1,8,1,14,1,1,input,seq.len() as u16]);w.extend_from_slice(seq);}
                _=>panic!("fixture kind")
            }
        }
    }
    words(&w)
}
fn gdef()->Gdef{
    parse_gdef(&words(&[1,2,14,0,0,34,48,
        1,10,7,1,2,3,3,3,4,0,
        1,12,4,1,2,0,0,
        1,2,0,12,0,20,1,2,10,13,1,2,12,14]),32,Default::default()).unwrap()
}
fn run(b:&[u8],roots:&[u16],input:&[u16],g:Option<&Gdef>)->GsubMixedExecResult{
    let p=compile_gsub_program(b,roots,32,g,Default::default()).unwrap();
    execute_gsub_mixed(&p,input,Default::default()).unwrap()
}
#[test]fn expansion_preserves_order_and_does_not_revisit_emitted(){
    let b=fixture(&[(2,2,0,None,&[(10,&[10,11]),(11,&[12,13])])]);
    let a=run(&b,&[0],&[10,9,11,10],None);
    assert_eq!(a.glyphs,vec![10,11,9,12,13,10,11]);
    assert_eq!((a.visits,a.probes,a.matches,a.changes,a.emitted),(4,6,3,3,7));
}
#[test]fn first_match_identity_and_overlapping_subtables(){
    let b=fixture(&[(2,2,0,None,&[(10,&[10]),(10,&[11,12]),(11,&[13,14])])]);
    let a=run(&b,&[0],&[10,11],None);
    assert_eq!(a.glyphs,vec![10,13,14]);
    assert_eq!((a.probes,a.matches,a.changes),(4,2,1));
}
#[test]fn ordered_mixed_roots_and_unselected_lookup(){
    let b=fixture(&[(1,1,0,None,&[(10,&[11])]),(2,2,0,None,&[(11,&[12,13])]),
        (1,1,0,None,&[(12,&[14]),(13,&[15])]),(2,2,0,None,&[(14,&[16,17])])]);
    assert_eq!(run(&b,&[0,1,2],&[10,9],None).glyphs,vec![14,15,9]);
    assert_eq!(run(&b,&[1,2],&[10,11],None).glyphs,vec![10,14,15]);
}
#[test]fn wrapped_multiple_and_single_dispatch(){
    let b=fixture(&[(7,2,0,None,&[(10,&[11,12])]),(7,1,0,None,&[(11,&[13])])]);
    let a=run(&b,&[0,1],&[10,9],None);
    assert_eq!(a.glyphs,vec![13,12,9]);
    assert_eq!((a.visits,a.probes,a.matches,a.changes,a.emitted),(5,5,2,2,6));
}
#[test]fn filtering_recomputed_for_expanded_glyphs(){
    let g=gdef();
    let b=fixture(&[(2,2,LOOKUP_IGNORE_MARK,None,&[(10,&[12,15]),(12,&[20,21])]),
        (2,2,LOOKUP_IGNORE_MARK,None,&[(12,&[22,23]),(15,&[24,25])])]);
    assert_eq!(run(&b,&[0,1],&[10,12],Some(&g)).glyphs,vec![12,24,25,12]);
}
#[test]fn mark_set_filter_and_attachment_precedence(){
    let g=gdef();
    let b=fixture(&[(2,2,LOOKUP_USE_MARK_SET|0x0100,Some(0),&[(12,&[20,21]),(13,&[22,23])])]);
    assert_eq!(run(&b,&[0],&[12,13,14],Some(&g)).glyphs,vec![12,22,23,14]);
}
#[test]fn exact_limits_and_atomic_failure(){
    let b=fixture(&[(2,2,0,None,&[(10,&[11,12])]),(1,1,0,None,&[(11,&[13])])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let input=[10,9];
    let l=GsubMixedExecLimits{max_glyphs:3,max_roots:2,max_visits:5,max_probes:5,max_matches:2,max_emitted:6};
    assert_eq!(execute_gsub_mixed(&p,&input,l).unwrap().glyphs,vec![13,12,9]);
    for bad in [GsubMixedExecLimits{max_glyphs:2,..l},GsubMixedExecLimits{max_roots:1,..l},
        GsubMixedExecLimits{max_visits:4,..l},GsubMixedExecLimits{max_probes:4,..l},
        GsubMixedExecLimits{max_matches:1,..l},GsubMixedExecLimits{max_emitted:5,..l}]{
        assert!(matches!(execute_gsub_mixed(&p,&input,bad),Err(Error::Limit(_))));
        assert_eq!(input,[10,9]);
    }
}
#[test]fn repeated_expansion_hits_intermediate_cap(){
    let b=fixture(&[(2,2,0,None,&[(10,&[10,10])]),(2,2,0,None,&[(10,&[10,10])])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    assert_eq!(execute_gsub_mixed(&p,&[10],Default::default()).unwrap().glyphs,vec![10;4]);
    assert!(matches!(execute_gsub_mixed(&p,&[10],GsubMixedExecLimits{max_glyphs:3,..Default::default()}),Err(Error::Limit(_))));
}
#[test]fn empty_and_invalid_domain(){
    let b=fixture(&[(2,2,0,None,&[(10,&[11,12])])]);
    let p=compile_gsub_program(&b,&[],32,None,Default::default()).unwrap();
    let a=execute_gsub_mixed(&p,&[31],GsubMixedExecLimits{max_emitted:0,..Default::default()}).unwrap();
    assert_eq!(a.glyphs,vec![31]);assert_eq!(a.emitted,0);
    assert!(execute_gsub_mixed(&p,&[32],Default::default()).is_err());
    assert!(run(&b,&[0],&[],None).glyphs.is_empty());
}
#[test]fn unsupported_roots_and_legacy_single_api(){
    let b=words(&[1,0,10,12,14,0,0,2,6,26,
        1,0,1,8,1,6,1,1,1,10,
        5,0,1,8,3,1,1,12,0,0,1,1,10]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_mixed(&p,&[],Default::default()),Err(Error::Unsupported(_))));
    let b=fixture(&[(2,2,0,None,&[(10,&[11,12])])]);
    let p=compile_gsub_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_single(&p,&[10],Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn single_only_matches_legacy_counters(){
    let b=fixture(&[(1,1,0,None,&[(10,&[11]),(12,&[12])]),(1,1,0,None,&[(11,&[13])])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let input=[10,12,9,11];
    let a=execute_gsub_single(&p,&input,Default::default()).unwrap();
    let m=execute_gsub_mixed(&p,&input,Default::default()).unwrap();
    assert_eq!(a.glyphs,m.glyphs);
    assert_eq!((a.visits,a.probes,a.matches,a.changes),(m.visits,m.probes,m.matches,m.changes));
    assert_eq!(m.emitted,8);
}


fn choice(lookup_index:u16,candidate_index:usize)->GsubAlternateSelection{
    GsubAlternateSelection{lookup_index,candidate_index}
}
fn alternate_run(b:&[u8],roots:&[u16],input:&[u16],g:Option<&Gdef>,choices:&[GsubAlternateSelection])->GsubMixedExecResult{
    let p=compile_gsub_program(b,roots,32,g,Default::default()).unwrap();
    execute_gsub_with_alternates(&p,input,choices,Default::default()).unwrap()
}
#[test]fn alternate_explicit_candidate_order(){
    let b=fixture(&[(3,3,0,None,&[(10,&[13,11,12])])]);
    for (i,v) in [13,11,12].into_iter().enumerate(){
        let a=alternate_run(&b,&[0],&[10,9,10],None,&[choice(0,i)]);
        assert_eq!(a.glyphs,vec![v,9,v]);
        assert_eq!((a.visits,a.probes,a.matches,a.changes,a.emitted),(3,3,2,2,3));
    }
}
#[test]fn alternate_first_match_identity_and_duplicate_candidates(){
    let b=fixture(&[(3,3,0,None,&[(10,&[11,10,10]),(10,&[12,13,14]),(11,&[15,16,17])])]);
    let a=alternate_run(&b,&[0],&[10,11],None,&[choice(0,2)]);
    assert_eq!(a.glyphs,vec![10,17]);
    assert_eq!((a.probes,a.matches,a.changes),(4,2,1));
}
#[test]fn alternate_covered_invalid_does_not_fall_through(){
    let b=fixture(&[(3,3,0,None,&[(10,&[11]),(10,&[12,13])])]);
    let p=compile_gsub_program(&b,&[0],32,None,Default::default()).unwrap();
    let input=[9,10];
    assert!(matches!(execute_gsub_with_alternates(&p,&input,&[choice(0,1)],Default::default()),Err(Error::Invalid(_))));
    assert_eq!(input,[9,10]);
    assert_eq!(alternate_run(&b,&[0],&[9],None,&[choice(0,usize::MAX)]).glyphs,vec![9]);
}
#[test]fn alternate_wrapped_mixed_roots(){
    let b=fixture(&[(2,2,0,None,&[(10,&[11,12])]),(7,3,0,None,&[(11,&[13,14]),(12,&[15,16])]),
        (1,1,0,None,&[(14,&[17])]),(3,3,0,None,&[(16,&[18,19])])]);
    let a=alternate_run(&b,&[0,1,2,3],&[10,9],None,&[choice(1,1),choice(3,0)]);
    assert_eq!(a.glyphs,vec![17,18,9]);
    assert_eq!((a.visits,a.matches,a.changes,a.emitted),(11,5,5,12));
}
#[test]fn alternate_current_glyph_filter_and_miss(){
    let g=gdef();
    let b=fixture(&[(2,2,0,None,&[(10,&[12,15])]),
        (3,3,LOOKUP_IGNORE_MARK,None,&[(12,&[20]),(15,&[21,22])])]);
    assert_eq!(alternate_run(&b,&[0,1],&[10],Some(&g),&[choice(1,1)]).glyphs,vec![12,22]);
}
#[test]fn alternate_mark_set_policy(){
    let g=gdef();
    let b=fixture(&[(3,3,LOOKUP_USE_MARK_SET|0x0100,Some(0),&[(12,&[20]),(13,&[21,22])])]);
    assert_eq!(alternate_run(&b,&[0],&[12,13,14],Some(&g),&[choice(0,1)]).glyphs,vec![12,22,14]);
}
#[test]fn alternate_requires_exact_sorted_root_selections(){
    let b=fixture(&[(3,3,0,None,&[(10,&[11])]),(1,1,0,None,&[(11,&[12])]),
        (3,3,0,None,&[(12,&[13])]),(3,3,0,None,&[(13,&[14])])]);
    let p=compile_gsub_program(&b,&[0,1,2],32,None,Default::default()).unwrap();
    for bad in [vec![],vec![choice(0,0)],vec![choice(2,0),choice(0,0)],
        vec![choice(0,0),choice(0,0)],vec![choice(0,0),choice(1,0)],
        vec![choice(0,0),choice(3,0)],vec![choice(0,0),choice(2,0),choice(3,0)]]{
        assert!(matches!(execute_gsub_with_alternates(&p,&[],&bad,Default::default()),Err(Error::Invalid(_))));
    }
    assert!(execute_gsub_with_alternates(&p,&[],&[choice(0,0),choice(2,0)],Default::default()).unwrap().glyphs.is_empty());
    assert!(matches!(execute_gsub_mixed(&p,&[],Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn alternate_exact_work_limits(){
    let b=fixture(&[(3,3,0,None,&[(10,&[11,12])]),(3,3,0,None,&[(12,&[13])])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let choices=[choice(0,1),choice(1,0)];
    let l=GsubMixedExecLimits{max_glyphs:2,max_roots:2,max_visits:4,max_probes:4,max_matches:2,max_emitted:4};
    assert_eq!(execute_gsub_with_alternates(&p,&[10,9],&choices,l).unwrap().glyphs,vec![13,9]);
    for bad in [GsubMixedExecLimits{max_glyphs:1,..l},GsubMixedExecLimits{max_roots:1,..l},
        GsubMixedExecLimits{max_visits:3,..l},GsubMixedExecLimits{max_probes:3,..l},
        GsubMixedExecLimits{max_matches:1,..l},GsubMixedExecLimits{max_emitted:3,..l}]{
        assert!(matches!(execute_gsub_with_alternates(&p,&[10,9],&choices,bad),Err(Error::Limit(_))));
    }
}
#[test]fn alternate_no_same_root_cascade(){
    let b=fixture(&[(3,3,0,None,&[(10,&[11]),(11,&[12])])]);
    assert_eq!(alternate_run(&b,&[0],&[10,11],None,&[choice(0,0)]).glyphs,vec![11,12]);
}
#[test]fn alternate_empty_policy_preserves_mixed_legacy(){
    let b=fixture(&[(2,2,0,None,&[(10,&[11,12])]),(1,1,0,None,&[(11,&[13])])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    assert_eq!(execute_gsub_with_alternates(&p,&[10,9],&[],Default::default()).unwrap(),
        execute_gsub_mixed(&p,&[10,9],Default::default()).unwrap());
    assert!(matches!(execute_gsub_with_alternates(&p,&[10],&[choice(0,0)],Default::default()),Err(Error::Invalid(_))));
    assert!(execute_gsub_with_alternates(&p,&[32],&[],Default::default()).is_err());
}
