use rwcore::*;
fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
fn fixture()->Vec<u8>{
    let mut b=Vec::new();for v in [1,0,10,22,34]{w(&mut b,v);}
    w(&mut b,1);b.extend(b"latn");w(&mut b,8);b.extend([0;4]);
    w(&mut b,1);b.extend(b"liga");w(&mut b,8);b.extend([0;4]);
    w(&mut b,1);w(&mut b,4);b.extend([0;6]);b
}
fn parse(b:&[u8])->Result<GsubDirectory,Error>{parse_gsub_directory(b,GsubLimits::default())}
#[test]fn directory_offsets(){let g=parse(&fixture()).unwrap();assert_eq!(g.scripts,vec![GsubRecord{tag:*b"latn",offset:18}]);assert_eq!(g.features,vec![GsubRecord{tag:*b"liga",offset:30}]);assert_eq!(g.lookups,vec![38]);assert_eq!(g.feature_variations,None);}
#[test]fn every_truncated_prefix(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn unsupported_version(){let mut b=fixture();b[1]=2;assert!(parse(&b).is_err());}
#[test]fn invalid_top_offsets(){for p in [4,6,8]{let mut b=fixture();b[p..p+2].copy_from_slice(&0u16.to_be_bytes());assert!(parse(&b).is_err());}}
#[test]fn invalid_child_offsets(){for p in [16,28,36]{let mut b=fixture();b[p..p+2].copy_from_slice(&2u16.to_be_bytes());assert!(parse(&b).is_err());}}
#[test]fn cumulative_budget(){let b=fixture();assert!(parse_gsub_directory(&b,GsubLimits{max_records:2,..Default::default()}).is_err());assert!(parse_gsub_directory(&b,GsubLimits{max_records:3,..Default::default()}).is_ok());}
#[test]fn byte_budget(){assert!(parse_gsub_directory(&fixture(),GsubLimits{max_bytes:10,..Default::default()}).is_err());}
#[test]fn version_11_variation_offset(){let mut b=Vec::new();for v in [1,1,14,16,18,0,20,0,0,0]{w(&mut b,v);}b.extend([0;8]);assert_eq!(parse(&b).unwrap().feature_variations,Some(20));b[13]=0;assert_eq!(parse(&b).unwrap().feature_variations,None);}
