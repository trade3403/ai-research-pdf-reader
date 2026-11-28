
use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // 14 header words, then five coverages: nearest backtrack first.
    bytes(&[3,2,28,36,2,42,50,1,60,2,1,4,0,3,
        1,2,9,19, 1,1,8, 1,2,10,20, 2,1,11,12,0, 1,1,13])
}
fn parse(b:&[u8])->Result<CoverageChainContextPositioning,Error>{
    parse_coverage_chain_context_positioning(b,32,5,ContextLimits::default())
}
fn set(b:&mut [u8],i:usize,v:u16){b[2*i..2*i+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn direction_and_action_order(){
    let x=parse(&fixture()).unwrap();
    assert_eq!(x.input_len(),2);assert_eq!(x.backtrack().len(),2);assert_eq!(x.lookahead().len(),1);
    for g in [[8,9,10,11,13],[8,19,20,12,13]]{
        let m=x.match_at(&g,2).unwrap();assert_eq!(m.input_len,2);
        assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:4},ContextAction{sequence_index:0,lookup_index:3}]);
    }
}
#[test]fn mismatches_and_boundaries(){
    let x=parse(&fixture()).unwrap();
    for (g,p) in [(&[9,8,10,11,13][..],2),(&[8,9,10,11],2),(&[9,10,11,13],1),(&[8,9,11,11,13],2),(&[8,9,10,13,13],2),(&[8,9,10,11,14],2),(&[8,32,10,11,13],2)]{
        assert!(x.match_at(g,p).is_none());
    }
    assert!(x.match_at(&[],0).is_none());assert!(x.match_at(&[10],usize::MAX).is_none());
}
#[test]fn empty_context_and_actions(){
    let x=parse(&bytes(&[3,0,1,12,0,0,1,1,10])).unwrap();
    let m=x.match_at(&[10],0).unwrap();assert_eq!(m.input_len,1);assert!(m.actions.is_empty());
    assert!(x.match_at(&[11],0).is_none());
}
#[test]fn empty_coverage_never_matches(){
    let x=parse(&bytes(&[3,0,1,12,0,0,1,0])).unwrap();
    assert!(x.input()[0].is_empty());assert!(x.match_at(&[10],0).is_none());
}
#[test]fn all_prefix_truncations(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
}
#[test]fn invalid_offsets_counts_and_actions(){
    for (i,v) in [(0,2),(1,65535),(2,0),(3,26),(4,0),(5,65535),(6,27),(8,66),(10,2),(11,5),(12,2),(13,5)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn malformed_coverages(){
    for (i,v) in [(14,3),(17,9),(20,32),(23,32),(28,10),(29,1),(32,32)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn cumulative_budgets(){
    let b=fixture();
    for l in [ContextLimits{max_sets:6,..Default::default()},ContextLimits{max_rules:0,..Default::default()},ContextLimits{max_input:4,..Default::default()},ContextLimits{max_total_input:7,..Default::default()},ContextLimits{max_actions:1,..Default::default()},ContextLimits{max_bytes:b.len()-1,..Default::default()}]{
        assert!(parse_coverage_chain_context_positioning(&b,32,5,l).is_err());
    }
    let l=ContextLimits{max_input:5,max_total_input:8,max_sets:7,max_actions:2,max_rules:1,..Default::default()};
    assert!(parse_coverage_chain_context_positioning(&b,32,5,l).is_ok());
}
#[test]fn shared_offsets_charged_per_position(){
    let mut b=fixture();set(&mut b,3,28);
    let l=ContextLimits{max_total_input:8,..Default::default()};
    assert!(parse_coverage_chain_context_positioning(&b,32,5,l).is_err());
    let x=parse(&b).unwrap();assert_eq!(x.backtrack()[0],x.backtrack()[1]);
    assert!(x.match_at(&[19,9,10,11,13],2).is_some());
}
#[test]fn input_relative_indices_and_surrounding_stream(){
    let x=parse(&fixture()).unwrap();
    let m=x.match_at(&[1,8,9,10,11,13,2],3).unwrap();
    assert_eq!(m.input_len,2);assert_eq!(m.actions,x.actions());
}

#[test]fn owning_lookup_domain_and_immutable_run(){
    let b=fixture();let p=parse(&b).unwrap();
    let run=[8,9,10,11,13];let original=run;
    assert_eq!(p.match_at(&run,2).unwrap().input_len,2);
    assert!(p.match_at(&run,run.len()).is_none());
    assert_eq!(run,original);
    assert!(parse_coverage_chain_context_positioning(&b,32,4,Default::default()).is_err());
    assert!(parse_coverage_chain_context_positioning(&b,32,5,Default::default()).is_ok());
}
#[test]fn shared_offsets_across_segments(){
    let mut b=fixture();set(&mut b,8,42);
    let p=parse(&b).unwrap();
    assert_eq!(p.input()[0],p.lookahead()[0]);
    assert!(p.match_at(&[8,9,10,11,20],2).is_some());
    assert!(p.match_at(&[8,9,10,11,13],2).is_none());
    let l=ContextLimits{max_sets:8,max_total_input:9,..Default::default()};
    assert!(parse_coverage_chain_context_positioning(&b,32,5,l).is_ok());
    assert!(parse_coverage_chain_context_positioning(&b,32,5,ContextLimits{max_sets:7,..l}).is_err());
    assert!(parse_coverage_chain_context_positioning(&b,32,5,ContextLimits{max_total_input:8,..l}).is_err());
}
