use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{data(&[1,10,2,18,24,1,2,3,7,2,8,9,3,4,5,6])}
fn parse(b:&[u8])->Result<MultipleSubstitution,Error>{parse_multiple_substitution(b,20,MultipleSubstLimits::default())}
#[test]fn sequences(){let r=parse(&fixture()).unwrap();assert_eq!(r.len(),2);assert_eq!(r.substitute(3),Some(&[8,9][..]));assert_eq!(r.substitute(7),Some(&[4,5,6][..]));assert_eq!(r.substitute(4),None);}
#[test]fn replacement_preserves_neighbors(){let r=parse(&fixture()).unwrap();let mut v=vec![1,3,7,2];assert_eq!(r.replace_at(&mut v,1,6).unwrap(),Some(2));assert_eq!(v,vec![1,8,9,7,2]);}
#[test]fn failure_is_atomic(){let r=parse(&fixture()).unwrap();let mut v=vec![3,7];let old=v.clone();assert!(r.replace_at(&mut v,0,2).is_err());assert_eq!(v,old);assert!(r.replace_at(&mut v,2,10).is_err());assert_eq!(v,old);}
#[test]fn miss_unchanged(){let r=parse(&fixture()).unwrap();let mut v=vec![2];assert_eq!(r.replace_at(&mut v,0,1).unwrap(),None);assert_eq!(v,vec![2]);}
#[test]fn empty_table(){let r=parse(&data(&[1,6,0,1,0])).unwrap();assert!(r.is_empty());}
#[test]fn bad_records(){for (p,v) in [(0usize,2u16),(1,0),(2,1),(3,2),(3,65535),(9,0),(10,20)]{let mut b=fixture();b[2*p..2*p+2].copy_from_slice(&v.to_be_bytes());assert!(parse(&b).is_err(),"word {p}");}}
#[test]fn truncations(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn budgets(){let b=fixture();for l in [MultipleSubstLimits{max_records:1,..Default::default()},MultipleSubstLimits{max_covered:1,..Default::default()},MultipleSubstLimits{max_sequence:2,..Default::default()},MultipleSubstLimits{max_total_glyphs:4,..Default::default()},MultipleSubstLimits{max_bytes:5,..Default::default()}]{assert!(parse_multiple_substitution(&b,20,l).is_err());}}
#[test]fn shared_offsets_charged(){let mut b=fixture();b[8..10].copy_from_slice(&18u16.to_be_bytes());let r=parse(&b).unwrap();assert_eq!(r.substitute(3),r.substitute(7));assert!(parse_multiple_substitution(&b,20,MultipleSubstLimits{max_total_glyphs:3,..Default::default()}).is_err());}
