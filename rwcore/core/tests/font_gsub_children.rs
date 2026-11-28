use rwcore::*;
fn fixture()->Vec<u8>{
    let mut b=Vec::new();
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    // ScriptList at 10, script at 18, default LangSys at 22.
    // FeatureList at 30, feature at 38; LookupList at 44, lookup at 48.
    for v in [1,0,10,30,44,1]{w(&mut b,v);}
    b.extend(b"latn");for v in [8,4,0,0,0,1,0]{w(&mut b,v);}
    w(&mut b,1);b.extend(b"liga");for v in [8,0,1,0,1,4,1,0x10,1,10,3,1]{w(&mut b,v);}
    b
}
fn parse(b:&[u8])->Result<GsubLayout,Error>{parse_gsub_layout(b,GsubLimits::default())}
fn changed(p:usize,v:u16)->Vec<u8>{let mut b=fixture();b[p..p+2].copy_from_slice(&v.to_be_bytes());b}
#[test]fn child_graph(){let g=parse(&fixture()).unwrap();let l=g.scripts[0].default_language.as_ref().unwrap();assert_eq!(l.required_feature,Some(0));assert_eq!(l.features,vec![0]);assert_eq!(g.features[0].lookups,vec![0]);assert_eq!(g.lookups[0].subtables,vec![58]);assert_eq!(g.lookups[0].mark_filtering_set,Some(3));}
#[test]fn child_truncation(){let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}}
#[test]fn cross_indices(){for p in [24,28,42]{assert!(parse(&changed(p,1)).is_err());}}
#[test]fn reserved_language(){assert!(parse(&changed(22,1)).is_err());}
#[test]fn child_offsets(){for (p,v) in [(18,2),(38,2),(54,8)]{assert!(parse(&changed(p,v)).is_err());}}
#[test]fn lookup_flags_and_type(){for (p,v) in [(48,0),(48,9),(50,0x20)]{assert!(parse(&changed(p,v)).is_err());}}
#[test]fn optional_required_feature(){assert_eq!(parse(&changed(24,0xffff)).unwrap().scripts[0].default_language.as_ref().unwrap().required_feature,None);}
#[test]fn cumulative_child_budget(){let b=fixture();assert!(parse_gsub_layout(&b,GsubLimits{max_records:6,..Default::default()}).is_err());assert!(parse_gsub_layout(&b,GsubLimits{max_records:7,..Default::default()}).is_ok());}
