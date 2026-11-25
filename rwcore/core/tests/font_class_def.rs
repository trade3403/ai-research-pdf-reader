use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn parse(w:&[u16])->Result<GlyphClassDef,Error>{parse_glyph_class_def(&data(w),20,4,20,20)}
#[test]fn format_one(){let r=parse(&[1,3,3,1,0,2]).unwrap();assert_eq!(r.class(2),Some(0));assert_eq!(r.class(3),Some(1));assert_eq!(r.class(4),Some(0));assert_eq!(r.class(5),Some(2));assert_eq!(r.class(20),None);}
#[test]fn format_two(){let r=parse(&[2,2,2,4,3,7,9,1]).unwrap();for i in 0..20{assert_eq!(r.class(i),Some(if (2..=4).contains(&i){3}else if (7..=9).contains(&i){1}else{0}));}}
#[test]fn full_domain(){let r=parse_glyph_class_def(&data(&[2,1,0,65535,65535]),65536,65536,1,65536).unwrap();assert_eq!(r.class(65535),Some(65535));}
#[test]fn malformed(){for w in [&[1,19,2,1,2][..],&[1,0,1,4],&[2,1,5,4,1],&[2,2,1,4,1,4,6,2],&[2,1,1,20,1],&[3,0]]{assert!(parse(w).is_err());}}
#[test]fn budgets(){let b=data(&[2,1,1,9,1]);assert!(parse_glyph_class_def(&b,20,4,0,20).is_err());assert!(parse_glyph_class_def(&b,20,4,1,8).is_err());assert!(parse_glyph_class_def(&b,20,0,1,20).is_err());}
#[test]fn truncation(){let b=data(&[2,1,1,9,1]);for n in 0..b.len(){assert!(parse_glyph_class_def(&b[..n],20,4,20,20).is_err());}}
