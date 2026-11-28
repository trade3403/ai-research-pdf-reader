use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{data(&[3,2,2,18,26,1,2,0,1,1,2,3,4,2,1,7,9,0])}
fn parse(b:&[u8])->Result<CoverageContextPositioning,Error>{parse_coverage_context_positioning(b,20,4,ContextLimits::default())}
fn change(b:&mut [u8],p:usize,v:u16){b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn mixed_coverage_formats(){let r=parse(&fixture()).unwrap();assert_eq!(r.input_len(),2);assert_eq!(r.coverages()[0].len(),2);assert_eq!(r.coverages()[1].len(),3);for a in 3..=4{for b in 7..=9{assert_eq!(r.match_prefix(&[a,b,19]).unwrap().input_len,2);}}}
#[test]fn misses(){let r=parse(&fixture()).unwrap();for g in [&[][..],&[3],&[7,3],&[2,8],&[3,6],&[3,20]]{assert!(r.match_prefix(g).is_none());}}
#[test]fn action_order(){let r=parse(&fixture()).unwrap();let a=[ContextAction{sequence_index:1,lookup_index:2},ContextAction{sequence_index:0,lookup_index:1}];assert_eq!(r.actions(),&a);assert_eq!(r.match_prefix(&[3,8]).unwrap().actions,&a);}
#[test]fn zero_actions(){let r=parse(&data(&[3,1,0,8,1,1,3])).unwrap();assert!(r.match_prefix(&[3]).unwrap().actions.is_empty());}
#[test]fn empty_coverage_never_matches(){let r=parse(&data(&[3,1,0,8,1,0])).unwrap();assert!(r.match_prefix(&[3]).is_none());}
#[test]fn malformed(){for (p,v) in [(0,2),(1,0),(2,65535),(3,0),(3,16),(4,65535),(5,2),(6,4),(12,20),(15,10),(17,1)]{let mut b=fixture();change(&mut b,p,v);assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncated_prefixes(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [ContextLimits{max_sets:2,..Default::default()},ContextLimits{max_rules:0,..Default::default()},ContextLimits{max_input:1,..Default::default()},ContextLimits{max_total_input:4,..Default::default()},ContextLimits{max_actions:1,..Default::default()},ContextLimits{max_bytes:5,..Default::default()}]{assert!(parse_coverage_context_positioning(&b,20,4,l).is_err());}}
#[test]fn shared_coverage_charged(){let mut b=fixture();change(&mut b,4,18);let r=parse(&b).unwrap();assert!(r.match_prefix(&[3,4]).is_some());assert!(parse_coverage_context_positioning(&b,20,4,ContextLimits{max_total_input:3,..Default::default()}).is_err());}

#[test]fn explicit_position_and_lookup_domain(){
    let b=fixture();let p=parse(&b).unwrap();
    let run=[9,3,8,2];let original=run;
    assert_eq!(p.match_at(&run,1).unwrap().input_len,2);
    assert!(p.match_at(&run,0).is_none());
    assert!(p.match_at(&run,usize::MAX).is_none());
    assert!(p.match_at(&run,run.len()).is_none());
    assert_eq!(run,original);
    assert!(parse_coverage_context_positioning(&b,20,2,Default::default()).is_err());
    assert!(parse_coverage_context_positioning(&b,20,3,Default::default()).is_ok());
}
#[test]fn exact_cumulative_budgets(){
    let b=fixture();
    let l=ContextLimits{max_sets:3,max_rules:1,max_input:2,max_total_input:5,max_actions:2,max_bytes:b.len()};
    assert!(parse_coverage_context_positioning(&b,20,4,l).is_ok());
    let mut b=fixture();change(&mut b,4,18);
    let l=ContextLimits{max_sets:4,max_total_input:4,..l};
    assert!(parse_coverage_context_positioning(&b,20,4,l).is_ok());
    assert!(parse_coverage_context_positioning(&b,20,4,ContextLimits{max_sets:3,..l}).is_err());
}
