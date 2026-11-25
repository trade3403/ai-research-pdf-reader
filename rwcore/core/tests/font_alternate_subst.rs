use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{data(&[1,10,2,18,24,1,2,3,7,2,9,8,3,6,4,6])}
fn parse(b:&[u8])->Result<AlternateSubstitution,Error>{parse_alternate_substitution(b,20,AlternateSubstLimits::default())}
#[test]fn ordered_candidates(){let r=parse(&fixture()).unwrap();assert_eq!(r.len(),2);assert_eq!(r.alternates(3),Some(&[9,8][..]));assert_eq!(r.alternates(7),Some(&[6,4,6][..]));assert_eq!(r.alternates(4),None);}
#[test]fn explicit_selection(){let r=parse(&fixture()).unwrap();assert_eq!(r.select(3,0).unwrap(),Some(9));assert_eq!(r.select(3,1).unwrap(),Some(8));assert!(r.select(3,2).is_err());assert_eq!(r.select(4,usize::MAX).unwrap(),None);}
#[test]fn replacement_preserves_neighbors(){let r=parse(&fixture()).unwrap();let mut v=vec![1,3,7,2];assert!(r.replace_at(&mut v,1,1).unwrap());assert_eq!(v,vec![1,8,7,2]);}
#[test]fn failure_and_miss_are_atomic(){let r=parse(&fixture()).unwrap();let mut v=vec![3,2];let old=v.clone();assert!(r.replace_at(&mut v,0,2).is_err());assert!(r.replace_at(&mut v,2,0).is_err());assert!(!r.replace_at(&mut v,1,0).unwrap());assert_eq!(v,old);}
#[test]fn empty_table(){assert!(parse(&data(&[1,6,0,1,0])).unwrap().is_empty());}
#[test]fn bad_records(){for (p,v) in [(0usize,2u16),(1,0),(2,1),(3,2),(3,65535),(9,0),(10,20)]{let mut b=fixture();b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncations(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [AlternateSubstLimits{max_records:1,..Default::default()},AlternateSubstLimits{max_covered:1,..Default::default()},AlternateSubstLimits{max_alternates:2,..Default::default()},AlternateSubstLimits{max_total_glyphs:4,..Default::default()},AlternateSubstLimits{max_bytes:5,..Default::default()}]{assert!(parse_alternate_substitution(&b,20,l).is_err());}}
#[test]fn shared_offsets_charged(){let mut b=fixture();b[8..10].copy_from_slice(&18u16.to_be_bytes());let r=parse(&b).unwrap();assert_eq!(r.alternates(3),r.alternates(7));assert!(parse_alternate_substitution(&b,20,AlternateSubstLimits{max_total_glyphs:3,..Default::default()}).is_err());}
#[test]fn coverage_format_two(){let b=data(&[1,10,2,20,24,2,1,3,4,0,1,9,2,8,7]);let r=parse(&b).unwrap();assert_eq!(r.alternates(3),Some(&[9][..]));assert_eq!(r.alternates(4),Some(&[8,7][..]));}
