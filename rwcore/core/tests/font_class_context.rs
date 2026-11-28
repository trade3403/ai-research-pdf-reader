use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
// header 14; coverage 14..20; classDef 20..34; set 34..38; rule 38..50.
fn fixture()->Vec<u8>{data(&[2,14,20,3,0,34,0,1,1,3,1,3,4,1,2,2,0,1,4,3,1,2,0,1,2])}
fn parse(b:&[u8])->Result<ClassContextSubstitution,Error>{parse_class_context_substitution(b,20,4,ContextLimits::default())}
fn change(b:&mut [u8],p:usize,v:u16){b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn class_match(){let r=parse(&fixture()).unwrap();let m=r.match_prefix(&[3,4,8]).unwrap();assert_eq!(m.input_len,3);assert_eq!(m.actions,&[ContextAction{sequence_index:1,lookup_index:2}]);assert!(r.match_prefix(&[3,4,19]).is_some());}
#[test]fn coverage_gate_and_null_set(){let r=parse(&fixture()).unwrap();assert!(r.match_prefix(&[4,4,8]).is_none());assert!(r.match_prefix(&[3,3,8]).is_none());assert!(r.match_prefix(&[3,4,20]).is_none());assert!(r.match_prefix(&[3,4]).is_none());assert!(r.match_prefix(&[]).is_none());}
#[test]fn malformed(){for (p,v) in [(0,1),(1,2),(2,2),(3,0),(5,65535),(13,3),(18,2),(19,0),(21,3),(23,3),(24,4)]{let mut b=fixture();change(&mut b,p,v);assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncation(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [ContextLimits{max_sets:2,..Default::default()},ContextLimits{max_rules:0,..Default::default()},ContextLimits{max_input:2,..Default::default()},ContextLimits{max_total_input:2,..Default::default()},ContextLimits{max_actions:0,..Default::default()}]{assert!(parse_class_context_substitution(&b,20,4,l).is_err());}}
