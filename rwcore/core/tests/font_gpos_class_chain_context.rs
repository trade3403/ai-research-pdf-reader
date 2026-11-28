
use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    bytes(&[2,18,26,42,58,3,0,66,0,
        1,2,10,20,
        2,2,8,8,2,9,9,1,
        2,2,10,11,1,20,21,2,
        1,12,1,7,
        2,6,30,
        2,1,2,2,2,1,7,2,1,4,0,3,
        0,1,0,0])
}
fn parse(b:&[u8])->Result<ClassChainContextPositioning,Error>{
    parse_class_chain_context_positioning(b,32,5,ContextLimits::default())
}
fn set(b:&mut [u8],i:usize,v:u16){b[2*i..2*i+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn independent_classes_and_direction(){
    let x=parse(&fixture()).unwrap();
    let m=x.match_at(&[8,9,10,21,12],2).unwrap();
    assert_eq!(m.input_len,2);
    assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:4},ContextAction{sequence_index:0,lookup_index:3}]);
    assert_eq!(x.rules(10).unwrap()[0].backtrack,vec![1,2]);
}
#[test]fn fallback_and_boundaries(){
    let x=parse(&fixture()).unwrap();
    for (g,p) in [(&[9,8,10,20,12][..],2),(&[8,9,10,20],2),(&[9,10,20,12],1),(&[8,9,10,11,12],2),(&[8,9,10,20,13],2)]{
        let m=x.match_at(g,p).unwrap();assert_eq!(m.input_len,1);assert!(m.actions.is_empty());
    }
}
#[test]fn coverage_gate_and_null_class_set(){
    let x=parse(&fixture()).unwrap();
    assert!(x.rules(11).is_none());assert!(x.rules(20).is_none());
    assert!(x.match_at(&[8,9,11,20,12],2).is_none());
    assert!(x.match_at(&[],0).is_none());assert!(x.match_at(&[10],usize::MAX).is_none());
}
#[test]fn implicit_zero_and_out_of_face(){
    let mut b=fixture();
    set(&mut b,11,7);set(&mut b,6,66);set(&mut b,7,0);
    set(&mut b,37,0);set(&mut b,40,0);set(&mut b,42,0);
    let x=parse(&b).unwrap();
    let m=x.match_at(&[8,6,7,5,4],2).unwrap();assert_eq!(m.input_len,2);
    for g in [[8,32,7,5,4],[8,6,7,32,4],[8,6,7,5,32]]{
        assert_eq!(x.match_at(&g,2).unwrap().input_len,1);
    }
}
#[test]fn truncation(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
}
#[test]fn bad_header_offsets_and_rule_counts(){
    for (i,v) in [(0,1),(1,2),(2,4),(3,0),(4,65535),(5,0),(7,12),(34,2),(35,65535),(39,0)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn bad_input_classes_and_action_indices(){
    for (i,v) in [(25,3),(28,3),(40,3),(44,2),(45,5),(46,2),(47,5)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn cumulative_budgets(){
    let b=fixture();
    for l in [ContextLimits{max_sets:2,..Default::default()},ContextLimits{max_rules:1,..Default::default()},ContextLimits{max_input:4,..Default::default()},ContextLimits{max_total_input:5,..Default::default()},ContextLimits{max_actions:1,..Default::default()},ContextLimits{max_bytes:b.len()-1,..Default::default()}]{
        assert!(parse_class_chain_context_positioning(&b,32,5,l).is_err());
    }
    let l=ContextLimits{max_input:5,max_total_input:6,max_actions:2,max_rules:2,max_sets:3,..Default::default()};
    assert!(parse_class_chain_context_positioning(&b,32,5,l).is_ok());
}
#[test]fn shared_rules_charged(){
    let mut b=fixture();set(&mut b,35,6);
    let l=ContextLimits{max_total_input:9,..Default::default()};
    assert!(parse_class_chain_context_positioning(&b,32,5,l).is_err());
    let x=parse(&b).unwrap();assert_eq!(x.rules(10).unwrap()[0],x.rules(10).unwrap()[1]);
}
#[test]fn high_backtrack_lookahead_classes(){
    let mut b=fixture();set(&mut b,20,65535);set(&mut b,37,65535);
    set(&mut b,32,65534);set(&mut b,42,65534);
    let x=parse(&b).unwrap();assert_eq!(x.match_at(&[8,9,10,20,12],2).unwrap().input_len,2);
}

#[test]fn trailing_only_input_class(){
    let mut b=fixture();
    // Remove glyph 20 from coverage, preserving the other offsets.
    set(&mut b,10,1);
    // Glyphs 20..21 use a trailing-only class 65535, no set at that index.
    set(&mut b,28,65535);set(&mut b,40,65535);
    let p=parse(&b).unwrap();
    assert_eq!(p.match_at(&[8,9,10,21,12],2).unwrap().input_len,2);
    assert!(p.rules(20).is_none());
    let mut bad=b.clone();set(&mut bad,40,65534);
    assert!(parse(&bad).is_err());
    let mut bad=b.clone();set(&mut bad,10,2);
    assert!(parse(&bad).is_err());
    assert!(parse_class_chain_context_substitution(&b,32,5,Default::default()).is_err());
}
#[test]fn owning_lookup_domain_and_immutable_run(){
    let b=fixture();let p=parse(&b).unwrap();
    let run=[8,9,10,21,12];let original=run;
    assert_eq!(p.match_at(&run,2).unwrap().input_len,2);
    assert!(p.match_at(&run,run.len()).is_none());
    assert_eq!(run,original);
    assert!(parse_class_chain_context_positioning(&b,32,4,Default::default()).is_err());
}
