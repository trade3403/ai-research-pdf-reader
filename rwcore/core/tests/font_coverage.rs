use rwcore::*;
fn data(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn parse(w:&[u16])->Result<GlyphCoverage,Error>{parse_glyph_coverage(&data(w),100,100,100)}
#[test] fn format_one_sparse(){let c=parse(&[1,3,2,5,9]).unwrap();assert_eq!(c.len(),3);assert_eq!(c.index(2),Some(0));assert_eq!(c.index(5),Some(1));assert_eq!(c.index(9),Some(2));assert_eq!(c.index(8),None);}
#[test] fn format_two_ranges(){let c=parse(&[2,2,3,5,0,8,10,3]).unwrap();assert_eq!(c.len(),6);assert_eq!(c.index(5),Some(2));assert_eq!(c.index(8),Some(3));assert_eq!(c.index(10),Some(5));assert_eq!(c.index(6),None);}
#[test] fn empty_coverage(){for f in [1,2]{let c=parse(&[f,0]).unwrap();assert!(c.is_empty());assert_eq!(c.index(0),None);}}
#[test] fn unordered_and_overlapping(){for w in [&[1,2,5,5][..],&[1,2,5,4],&[2,2,1,5,0,5,8,5],&[2,1,8,2,0]]{assert!(parse(w).is_err());}}
#[test] fn invalid_indices_and_glyph_bounds(){for w in [&[2,1,1,3,1][..],&[2,2,1,3,0,5,7,2],&[1,1,100],&[2,1,90,100,0]]{assert!(parse(w).is_err());}}
#[test] fn truncation(){let b=data(&[2,2,3,5,0,8,10,3]);for n in 0..b.len(){assert!(parse_glyph_coverage(&b[..n],100,100,100).is_err());}}
#[test] fn limits(){let b=data(&[2,1,0,99,0]);assert!(parse_glyph_coverage(&b,100,0,100).is_err());assert!(parse_glyph_coverage(&b,100,1,99).is_err());assert!(parse_glyph_coverage(&b,100,1,100).is_ok());assert!(parse(&[3,0]).is_err());}
#[test] fn full_u16_domain(){let c=parse_glyph_coverage(&data(&[2,1,0,65535,0]),65536,1,65536).unwrap();assert_eq!(c.len(),65536);assert_eq!(c.index(65535),Some(65535));}
