use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{data(&[1,10,2,18,0,1,2,3,7,2,6,18,3,1,4,5,1,2,1,0])}
fn parse(b:&[u8])->Result<ContextPositioning,Error>{parse_context_positioning(b,20,4,ContextLimits::default())}
fn change(b:&mut [u8],p:usize,v:u16){b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn rule_and_lookup_reference(){let r=parse(&fixture()).unwrap();assert_eq!(r.len(),2);let m=r.match_prefix(&[3,4,5,6]).unwrap();assert_eq!(m.input_len,3);assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:2}]);}
#[test]fn empty_action_is_match(){let r=parse(&fixture()).unwrap();let m=r.match_prefix(&[3,4]).unwrap();assert_eq!(m.input_len,1);assert!(m.actions.is_empty());}
#[test]fn null_set_and_misses(){let r=parse(&fixture()).unwrap();assert!(r.rules(7).is_none());assert!(r.match_prefix(&[7,4,5]).is_none());assert!(r.match_prefix(&[2]).is_none());assert!(r.match_prefix(&[]).is_none());}
#[test]fn font_order_priority(){let mut b=fixture();change(&mut b,10,18);change(&mut b,11,6);let r=parse(&b).unwrap();assert_eq!(r.match_prefix(&[3,4,5]).unwrap().input_len,1);}
#[test]fn empty_table(){assert!(parse(&data(&[1,6,0,1,0])).unwrap().is_empty());}
#[test]fn bad_records(){for (p,v) in [(0,2),(1,0),(2,1),(3,2),(3,65535),(10,0),(10,65535),(12,0),(14,20),(16,3),(17,4)]{let mut b=fixture();change(&mut b,p,v);assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncation(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [ContextLimits{max_sets:1,..Default::default()},ContextLimits{max_rules:1,..Default::default()},ContextLimits{max_input:2,..Default::default()},ContextLimits{max_total_input:3,..Default::default()},ContextLimits{max_actions:0,..Default::default()},ContextLimits{max_bytes:5,..Default::default()}]{assert!(parse_context_positioning(&b,20,4,l).is_err());}}
#[test]fn shared_rules_charged(){let mut b=fixture();change(&mut b,11,6);assert_eq!(parse(&b).unwrap().rules(3).unwrap().len(),2);assert!(parse_context_positioning(&b,20,4,ContextLimits{max_total_input:5,..Default::default()}).is_err());}
#[test]fn action_order_preserved(){let b=data(&[1,8,1,14,1,1,3,1,4,2,2,4,1,2,0,1]);let r=parse(&b).unwrap();assert_eq!(r.match_prefix(&[3,4]).unwrap().actions,&[ContextAction{sequence_index:1,lookup_index:2},ContextAction{sequence_index:0,lookup_index:1}]);}

#[test]fn explicit_position_and_immutable_input(){
    let p=parse(&fixture()).unwrap();
    let run=[9,3,4,5,6];let original=run;
    assert!(p.match_at(&run,0).is_none());
    assert_eq!(p.match_at(&run,1).unwrap().input_len,3);
    assert!(p.match_at(&run,run.len()).is_none());
    assert!(p.match_at(&run,usize::MAX).is_none());
    assert_eq!(run,original);
}
#[test]fn owning_gpos_lookup_domain(){
    let b=fixture();
    assert!(parse_context_positioning(&b,20,2,Default::default()).is_err());
    assert!(parse_context_positioning(&b,20,3,Default::default()).is_ok());
}
