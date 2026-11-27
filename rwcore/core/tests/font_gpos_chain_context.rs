use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    bytes(&[1,8,1,14, 1,1,10, 2,6,30,
        2,9,8,2,11,1,12,2,1,4,0,3,
        0,1,0,0])
}
fn parse(b:&[u8])->Result<ChainContextPositioning,Error>{
    parse_chain_context_positioning(b,32,5,ContextLimits::default())
}
fn set(b:&mut [u8],word:usize,val:u16){b[2*word..2*word+2].copy_from_slice(&val.to_be_bytes());}
#[test]fn chain_direction_and_action_order(){
    let x=parse(&fixture()).unwrap();
    let r=&x.rules(10).unwrap()[0];
    assert_eq!(r.backtrack,vec![9,8]);
    assert_eq!(r.remaining,vec![11]);assert_eq!(r.lookahead,vec![12]);
    let m=x.match_at(&[7,8,9,10,11,12,13],3).unwrap();
    assert_eq!(m.input_len,2);
    assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:4},ContextAction{sequence_index:0,lookup_index:3}]);
}
#[test]fn chain_fallback_and_context_mismatch(){
    let x=parse(&fixture()).unwrap();
    for (g,p) in [(&[9,8,10,11,12][..],2),(&[8,9,10,11],2),(&[9,10,11,12],1),(&[8,9,10,12,12],2),(&[8,9,10,11,13],2)]{
        let m=x.match_at(g,p).unwrap();assert_eq!(m.input_len,1);assert!(m.actions.is_empty());
    }
}
#[test]fn chain_position_and_coverage_miss(){
    let x=parse(&fixture()).unwrap();
    assert!(x.match_at(&[],0).is_none());assert!(x.match_at(&[10],usize::MAX).is_none());
    assert!(x.match_at(&[9,11,12],1).is_none());assert!(x.rules(11).is_none());
}
#[test]fn chain_null_set(){
    let mut b=fixture();set(&mut b,3,0);b.truncate(14);
    let x=parse(&b).unwrap();assert_eq!(x.len(),1);assert!(x.rules(10).is_none());
}
#[test]fn chain_all_prefix_truncations(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
}
#[test]fn chain_bad_offsets_and_counts(){
    for (i,v) in [(0,2),(1,2),(2,2),(3,6),(8,2),(9,65535),(13,0)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn chain_bad_glyphs_and_action_indices(){
    for (i,v) in [(11,32),(12,32),(14,32),(16,32),(18,2),(19,5),(20,2),(21,5)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn chain_cumulative_budgets(){
    let b=fixture();
    for l in [ContextLimits{max_sets:0,..Default::default()},ContextLimits{max_rules:1,..Default::default()},
        ContextLimits{max_input:4,..Default::default()},ContextLimits{max_total_input:5,..Default::default()},
        ContextLimits{max_actions:1,..Default::default()},ContextLimits{max_bytes:b.len()-1,..Default::default()}]{
        assert!(parse_chain_context_positioning(&b,32,5,l).is_err());
    }
    let l=ContextLimits{max_input:5,max_total_input:6,max_actions:2,max_rules:2,..Default::default()};
    assert!(parse_chain_context_positioning(&b,32,5,l).is_ok());
}
#[test]fn chain_coverage_range_and_empty_set(){
    let b=bytes(&[1,8,1,18,2,1,10,10,0,0]);
    let x=parse(&b).unwrap();assert_eq!(x.rules(10),Some(&[][..]));assert!(x.match_at(&[10],0).is_none());
}
#[test]fn chain_shared_rules_charged_per_reference(){
    let mut b=fixture();set(&mut b,9,6);
    let l=ContextLimits{max_total_input:9,..Default::default()};
    assert!(parse_chain_context_positioning(&b,32,5,l).is_err());
    let x=parse(&b).unwrap();assert_eq!(x.rules(10).unwrap()[0],x.rules(10).unwrap()[1]);
}

#[test]fn owning_positioning_lookup_domain(){
    let b=fixture();
    assert!(parse_chain_context_positioning(&b,32,4,Default::default()).is_err());
    assert!(parse_chain_context_positioning(&b,32,5,Default::default()).is_ok());
}
#[test]fn immutable_run_and_input_only_action_index(){
    let p=parse(&fixture()).unwrap();
    let run=[8,9,10,11,12];let original=run;
    assert_eq!(p.match_at(&run,2).unwrap().input_len,2);
    assert!(p.match_at(&run,run.len()).is_none());
    assert_eq!(run,original);
    // Index 2 refers to lookahead, not the two input positions.
    let mut b=fixture();set(&mut b,18,2);
    assert!(parse(&b).is_err());
}
