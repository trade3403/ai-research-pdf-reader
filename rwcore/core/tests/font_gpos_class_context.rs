use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
// header 14; coverage 14..20; classDef 20..34; set 34..38; rule 38..50.
fn fixture()->Vec<u8>{data(&[2,14,20,3,0,34,0,1,1,3,1,3,4,1,2,2,0,1,4,3,1,2,0,1,2])}
fn parse(b:&[u8])->Result<ClassContextPositioning,Error>{parse_class_context_positioning(b,20,4,ContextLimits::default())}
fn change(b:&mut [u8],p:usize,v:u16){b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn class_match(){let r=parse(&fixture()).unwrap();let m=r.match_prefix(&[3,4,8]).unwrap();assert_eq!(m.input_len,3);assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:2}]);assert!(r.match_prefix(&[3,4,19]).is_some());}
#[test]fn coverage_gate_and_null_set(){let r=parse(&fixture()).unwrap();assert!(r.match_prefix(&[4,4,8]).is_none());assert!(r.match_prefix(&[3,3,8]).is_none());assert!(r.match_prefix(&[3,4,20]).is_none());assert!(r.match_prefix(&[3,4]).is_none());assert!(r.match_prefix(&[]).is_none());}
#[test]fn malformed(){for (p,v) in [(0,1),(1,2),(2,2),(3,0),(5,65535),(13,3),(18,2),(19,0),(21,3),(23,3),(24,4)]{let mut b=fixture();change(&mut b,p,v);assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncation(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [ContextLimits{max_sets:2,..Default::default()},ContextLimits{max_rules:0,..Default::default()},ContextLimits{max_input:2,..Default::default()},ContextLimits{max_total_input:2,..Default::default()},ContextLimits{max_actions:0,..Default::default()}]{assert!(parse_class_context_positioning(&b,20,4,l).is_err());}}

#[test]fn explicit_position_and_lookup_domain(){
    let b=fixture();let p=parse(&b).unwrap();
    let run=[9,3,4,8,2];let original=run;
    assert_eq!(p.match_at(&run,1).unwrap().input_len,3);
    assert!(p.match_at(&run,0).is_none());
    assert!(p.match_at(&run,usize::MAX).is_none());
    assert!(p.match_at(&run,run.len()).is_none());
    assert_eq!(run,original);
    assert!(parse_class_context_positioning(&b,20,2,Default::default()).is_err());
    assert!(parse_class_context_positioning(&b,20,3,Default::default()).is_ok());
}
#[test]fn class_zero_initial_and_empty_action(){
    // coverage glyph 8, empty ClassDef format 1, class-zero rule, no actions.
    let b=data(&[2,10,16,1,22,1,1,8,1,0,0,1,4,1,0]);
    let p=parse(&b).unwrap();
    assert_eq!(p.rule_sets().len(),1);
    assert!(p.rules(7).is_none());
    let m=p.match_prefix(&[8,9]).unwrap();
    assert_eq!(m.input_len,1);assert!(m.actions.is_empty());
}

#[test]fn trailing_only_class_without_rule_set(){
    // Native Gujarati pattern: only class 1 can start, class 2 follows.
    // Coverage {3}; ClassDef assigns 3->1 and 4->2; only two set slots.
    let b=data(&[2,12,18,2,0,28,1,1,3,1,3,2,1,2,1,4,2,1,2,1,2]);
    let p=parse(&b).unwrap();
    assert_eq!(p.match_prefix(&[3,4]).unwrap().actions,&[ContextAction{sequence_index:1,lookup_index:2}]);
    assert!(p.match_prefix(&[4,3]).is_none());
    assert!(p.match_prefix(&[3,3]).is_none());
    let mut bad=b.clone();change(&mut bad,13,3);
    assert!(parse(&bad).is_err()); // rule references undefined class 2
    let mut bad=b.clone();change(&mut bad,8,4);
    assert!(parse(&bad).is_err()); // initial coverage cannot index omitted set
}
