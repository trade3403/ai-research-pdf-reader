use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{data(&[1,8,1,14,1,1,3,2,6,14,9,3,4,5,8,2,4])}
fn parse(b:&[u8])->Result<LigatureSubstitution,Error>{parse_ligature_substitution(b,20,LigatureLimits::default())}
fn m(output:u16,consumed:usize)->Option<LigatureMatch>{Some(LigatureMatch{output,consumed})}
#[test]fn ordered_rules(){let r=parse(&fixture()).unwrap();assert_eq!(r.len(),1);assert_eq!(r.rules(3).unwrap(),&[LigatureRule{output:9,remaining:vec![4,5]},LigatureRule{output:8,remaining:vec![4]}]);assert!(r.rules(4).is_none());}
#[test]fn match_and_fallback(){let r=parse(&fixture()).unwrap();assert_eq!(r.match_prefix(&[3,4,5,6]),m(9,3));assert_eq!(r.match_prefix(&[3,4,6]),m(8,2));assert_eq!(r.match_prefix(&[3,4]),m(8,2));for a in [&[][..],&[3],&[3,5],&[4,5]]{assert_eq!(r.match_prefix(a),None);}}
#[test]fn preference_not_longest(){let mut b=fixture();b[16..18].copy_from_slice(&14u16.to_be_bytes());b[18..20].copy_from_slice(&6u16.to_be_bytes());assert_eq!(parse(&b).unwrap().match_prefix(&[3,4,5]),m(8,2));}
#[test]fn replacement_preserves_neighbors(){let r=parse(&fixture()).unwrap();let mut v=vec![1,3,4,5,2];assert_eq!(r.replace_at(&mut v,1).unwrap(),m(9,3));assert_eq!(v,vec![1,9,2]);}
#[test]fn miss_and_error_atomic(){let r=parse(&fixture()).unwrap();let mut v=vec![3,5];let old=v.clone();assert_eq!(r.replace_at(&mut v,0).unwrap(),None);assert!(r.replace_at(&mut v,2).is_err());assert_eq!(v,old);}
#[test]fn empty_table(){assert!(parse(&data(&[1,6,0,1,0])).unwrap().is_empty());}
#[test]fn bad_records(){for (p,v) in [(0usize,2u16),(1,0),(2,2),(3,2),(3,65535),(7,0),(8,2),(8,65535),(10,20),(11,0),(11,1),(12,20)]{let mut b=fixture();b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncations(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [LigatureLimits{max_sets:0,..Default::default()},LigatureLimits{max_rules:1,..Default::default()},LigatureLimits{max_components:2,..Default::default()},LigatureLimits{max_total_components:4,..Default::default()},LigatureLimits{max_bytes:5,..Default::default()}]{assert!(parse_ligature_substitution(&b,20,l).is_err());}}
#[test]fn shared_offsets_charged(){let mut b=fixture();b[18..20].copy_from_slice(&6u16.to_be_bytes());assert_eq!(parse(&b).unwrap().rules(3).unwrap().len(),2);assert!(parse_ligature_substitution(&b,20,LigatureLimits{max_total_components:5,..Default::default()}).is_err());}
