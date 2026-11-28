use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn parse(w:&[u16],n:u32)->Result<SingleSubstitution,Error>{parse_single_substitution(&data(w),n,SingleSubstLimits::default())}
#[test]fn positive_delta(){let r=parse(&[1,6,3,1,2,4,8],20).unwrap();assert_eq!(r.substitute(4),Some(7));assert_eq!(r.substitute(8),Some(11));assert_eq!(r.substitute(5),None);}
#[test]fn negative_delta(){let r=parse(&[1,6,65534,2,1,4,7,0],20).unwrap();assert_eq!(r.substitute(4),Some(2));assert_eq!(r.substitute(7),Some(5));}
#[test]fn modulo_wrap(){let r=parse(&[1,6,2,1,1,65535],65536).unwrap();assert_eq!(r.substitute(65535),Some(1));}
#[test]fn explicit_replacements(){let r=parse(&[2,12,3,8,0,5,1,3,1,4,7],10).unwrap();assert_eq!(r.len(),3);assert_eq!(r.substitute(1),Some(8));assert_eq!(r.substitute(4),Some(0));assert_eq!(r.substitute(7),Some(5));}
#[test]fn empty_and_identity(){assert!(parse(&[2,6,0,1,0],20).unwrap().is_empty());assert_eq!(parse(&[1,6,0,1,1,3],20).unwrap().substitute(3),Some(3));}
#[test]fn invalid_outputs(){assert!(parse(&[1,6,2,1,1,9],10).is_err());assert!(parse(&[1,6,65535,1,1,0],10).is_err());assert!(parse(&[2,8,1,10,1,1,2],10).is_err());}
#[test]fn malformed_counts_and_offsets(){for w in [&[2,8,1,3,1,2,1,2][..],&[2,6,1,1,0],&[1,0,0,1,0],&[3,6,0,1,0]]{assert!(parse(w,20).is_err());}}
#[test]fn truncated_prefixes(){let b=data(&[2,12,3,8,0,5,1,3,1,4,7]);for n in 0..b.len(){assert!(parse_single_substitution(&b[..n],10,SingleSubstLimits::default()).is_err());}}
#[test]fn resource_limits(){let b=data(&[1,6,0,2,1,0,9,0]);for l in [SingleSubstLimits{max_records:0,..Default::default()},SingleSubstLimits{max_glyphs:9,..Default::default()},SingleSubstLimits{max_bytes:5,..Default::default()}]{assert!(parse_single_substitution(&b,10,l).is_err());}}
