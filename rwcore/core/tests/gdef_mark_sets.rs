use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // Three sets: format 1, format 2, empty format 1.
    // Header 16 bytes; coverage offsets 16,24,34.
    bytes(&[1,3,0,16,0,24,0,34,1,2,10,12,2,1,12,15,0,1,0])
}
fn parse(b:&[u8])->Result<MarkGlyphSets,Error>{parse_mark_glyph_sets(b,32,Default::default())}
fn set(b:&mut [u8],i:usize,v:u16){b[2*i..2*i+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn membership_and_independent_sets(){
    let x=parse(&fixture()).unwrap();
    assert_eq!(x.len(),3);assert!(!x.is_empty());
    for g in 0..32u16{
        assert_eq!(x.contains(0,g),Some(g==10||g==12));
        assert_eq!(x.contains(1,g),Some((12..=15).contains(&g)));
        assert_eq!(x.contains(2,g),Some(false));
    }
    assert_eq!(x.coverage(1).unwrap().index(15),Some(3));
    assert_eq!(x.contains(3,10),None);assert_eq!(x.contains(0,32),None);
    assert!(x.coverage(usize::MAX).is_none());
}
#[test]fn empty_set_array(){
    let x=parse(&bytes(&[1,0])).unwrap();
    assert!(x.is_empty());assert_eq!(x.contains(0,0),None);
}
#[test]fn offset_beyond_u16(){
    let mut b=bytes(&[1,1,1,4464]); // 70000
    b.resize(70000,0);b.extend(bytes(&[1,1,12]));
    let x=parse(&b).unwrap();assert_eq!(x.contains(0,12),Some(true));
}
#[test]fn nonmonotonic_offsets_and_aliases(){
    let mut b=fixture();set(&mut b,3,24);set(&mut b,5,16);set(&mut b,7,24);
    let x=parse(&b).unwrap();
    assert_eq!(x.coverage(0),x.coverage(2));
    assert_eq!(x.contains(1,10),Some(true));assert_eq!(x.contains(0,10),Some(false));
}
#[test]fn all_prefix_truncations(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
}
#[test]fn invalid_headers_and_offsets(){
    for (i,v) in [(0,2),(1,65535),(2,65535),(3,0),(3,15),(3,38),(3,37),(5,39),(7,36)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn malformed_coverages(){
    // Empty Coverage Format 2 is legal: zero ranges, no range records.
    let mut valid=fixture();set(&mut valid,17,2);
    let x=parse(&valid).unwrap();
    assert!(x.coverage(2).unwrap().is_empty());
    for glyph in 0..32u16{assert_eq!(x.contains(2,glyph),Some(false));}
    for (i,v) in [(8,3),(9,65535),(10,32),(11,10),(12,3),(14,16),(15,32),(16,1),(17,3),(18,1)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn cumulative_limits(){
    let b=fixture();
    let l=MarkGlyphSetLimits{max_bytes:b.len(),max_sets:3,max_records:3,max_covered:6};
    assert!(parse_mark_glyph_sets(&b,32,l).is_ok());
    for l in [MarkGlyphSetLimits{max_bytes:b.len()-1,..l},MarkGlyphSetLimits{max_sets:2,..l},
        MarkGlyphSetLimits{max_records:2,..l},MarkGlyphSetLimits{max_covered:5,..l}]{
        assert!(parse_mark_glyph_sets(&b,32,l).is_err());
    }
}
#[test]fn aliases_charge_each_reference(){
    let mut b=fixture();set(&mut b,7,24);
    let l=MarkGlyphSetLimits{max_records:4,max_covered:10,..Default::default()};
    assert!(parse_mark_glyph_sets(&b,32,l).is_ok());
    assert!(parse_mark_glyph_sets(&b,32,MarkGlyphSetLimits{max_records:3,..l}).is_err());
    assert!(parse_mark_glyph_sets(&b,32,MarkGlyphSetLimits{max_covered:9,..l}).is_err());
}
#[test]fn face_domain_validation(){
    let b=fixture();
    for n in [0,15,65537]{assert!(parse_mark_glyph_sets(&b,n,Default::default()).is_err());}
    let b=bytes(&[1,1,0,8,1,1,65535]);
    let x=parse_mark_glyph_sets(&b,65536,Default::default()).unwrap();
    assert_eq!(x.contains(0,65535),Some(true));
}
#[test]fn gdef_v12_and_v13_integration(){
    for (minor,header) in [(2,14),(3,18)]{
        let mut b=bytes(&[1,minor,0,0,0,0,header]);
        b.resize(header as usize,0);b.extend(fixture());
        let x=parse_gdef(&b,32,Default::default()).unwrap();
        assert_eq!(x.directory().mark_glyph_sets_def,Some(header as usize));
        assert_eq!(x.mark_sets().unwrap().contains(1,15),Some(true));
        assert_eq!(x.glyph_class(15),Some(GdefGlyphClass::Unclassified));
        assert!(parse_gdef_with_mark_set_limits(&b,32,Default::default(),MarkGlyphSetLimits{max_sets:2,..Default::default()}).is_err());
    }
}
#[test]fn absent_and_invalid_gdef_mark_set_children(){
    let b=bytes(&[1,0,0,0,0,0]);
    assert!(parse_gdef(&b,32,Default::default()).unwrap().mark_sets().is_none());
    let mut b=bytes(&[1,2,0,0,0,0,14]);b.extend(fixture());
    b[15]=2;
    assert!(parse_gdef_directory(&b,Default::default()).is_ok());
    assert!(parse_gdef(&b,32,Default::default()).is_err());
}
