use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // GDEF 1.0, glyph class format 1 at 12, mark attachment format 2 at 26.
    bytes(&[1,0,12,0,0,26,1,10,4,1,2,3,4,2,2,12,12,255,14,15,7])
}
fn parse(b:&[u8])->Result<Gdef,Error>{parse_gdef(b,32,Default::default())}
fn set(b:&mut [u8],i:usize,v:u16){b[2*i..2*i+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn glyph_and_mark_class_domains(){
    let x=parse(&fixture()).unwrap();
    use GdefGlyphClass::*;
    for (g,c) in [(9,Unclassified),(10,Base),(11,Ligature),(12,Mark),(13,Component),(14,Unclassified)]{
        assert_eq!(x.glyph_class(g),Some(c));
    }
    assert_eq!(x.mark_attachment_class(12),Some(255));
    assert_eq!(x.mark_attachment_class(14),Some(7));
    assert_eq!(x.mark_attachment_class(15),Some(7));
    assert_eq!(x.mark_attachment_class(13),Some(0));
    assert_eq!(x.glyph_class(32),None);assert_eq!(x.mark_attachment_class(32),None);
    assert!(x.glyph_classes().is_some());assert!(x.mark_attach_classes().is_some());
}
#[test]fn absent_class_tables(){
    let x=parse(&bytes(&[1,0,0,0,0,0])).unwrap();
    assert!(x.glyph_classes().is_none());assert!(x.mark_attach_classes().is_none());
    assert_eq!(x.glyph_class(31),Some(GdefGlyphClass::Unclassified));
    assert_eq!(x.mark_attachment_class(31),Some(0));
}
#[test]fn gdef_v12_directory(){
    let b=bytes(&[1,2,0,14,18,0,22,0,0,0,0,0,0]);
    let x=parse(&b).unwrap();let d=x.directory();
    assert_eq!(d.minor_version,2);
    assert_eq!(d.attach_list,Some(14));assert_eq!(d.lig_caret_list,Some(18));
    assert_eq!(d.mark_glyph_sets_def,Some(22));assert_eq!(d.item_var_store,None);
}
#[test]fn gdef_v13_large_variation_offset(){
    let mut b=bytes(&[1,3,0,0,0,0,0,0,0]);
    b[14..18].copy_from_slice(&70000u32.to_be_bytes());b.resize(70008,0);
    let x=parse(&b).unwrap();
    assert_eq!(x.directory().item_var_store,Some(70000));
    b.pop();assert!(parse(&b).is_err());
}
#[test]fn gdef_v13_nullable_offsets(){
    let x=parse(&bytes(&[1,3,0,0,0,0,0,0,0])).unwrap();
    assert_eq!(x.directory().minor_version,3);assert_eq!(x.directory().item_var_store,None);
    assert_eq!(x.directory().mark_glyph_sets_def,None);
}
#[test]fn every_prefix_truncation(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
    for b in [bytes(&[1,2,0,0,0,0,0]),bytes(&[1,3,0,0,0,0,0,0,0])]{
        for n in 0..b.len(){assert!(parse(&b[..n]).is_err());}
    }
}
#[test]fn invalid_version_and_directory_offsets(){
    for (i,v) in [(0,2),(1,1),(1,4),(2,10),(2,42),(3,2),(4,41),(5,65535)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
    let mut b=bytes(&[1,3,0,0,0,0,0,0,0]);
    for p in [1u32,17,18,u32::MAX]{
        b[14..18].copy_from_slice(&p.to_be_bytes());assert!(parse(&b).is_err());
    }
}
#[test]fn invalid_class_records_and_domains(){
    for (i,v) in [(6,3),(7,31),(9,5),(15,32),(16,11),(17,256),(18,12),(19,32)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
    let b=fixture();
    for n in [0,65537]{assert!(parse_gdef(&b,n,Default::default()).is_err());}
}
#[test]fn cumulative_class_limits(){
    let b=fixture();
    let l=GdefLimits{max_bytes:b.len(),max_class_records:6,max_assigned_glyphs:7};
    assert!(parse_gdef(&b,32,l).is_ok());
    for l in [GdefLimits{max_bytes:b.len()-1,..l},GdefLimits{max_class_records:5,..l},GdefLimits{max_assigned_glyphs:6,..l}]{
        assert!(parse_gdef(&b,32,l).is_err());
    }
}
#[test]fn aliases_charged_per_reference(){
    let mut b=fixture();set(&mut b,5,12);b.truncate(26);
    let l=GdefLimits{max_class_records:8,max_assigned_glyphs:8,..Default::default()};
    let x=parse_gdef(&b,32,l).unwrap();
    assert_eq!(x.mark_attachment_class(13),Some(4));
    assert!(parse_gdef(&b,32,GdefLimits{max_class_records:7,..l}).is_err());
    assert!(parse_gdef(&b,32,GdefLimits{max_assigned_glyphs:7,..l}).is_err());
}
#[test]fn range_glyph_class_and_dense_mark_class(){
    let b=bytes(&[1,0,12,0,0,28,2,2,5,6,1,7,8,3,1,7,2,2,0]);
    let x=parse(&b).unwrap();
    assert_eq!(x.glyph_class(6),Some(GdefGlyphClass::Base));
    assert_eq!(x.glyph_class(8),Some(GdefGlyphClass::Mark));
    assert_eq!(x.mark_attachment_class(7),Some(2));assert_eq!(x.mark_attachment_class(8),Some(0));
}
#[test]fn deferred_children_only_validate_entry(){
    let b=bytes(&[1,2,0,14,14,0,14,65535,65535]);
    let x=parse(&b).unwrap();
    assert_eq!(x.directory().attach_list,Some(14));
    assert_eq!(x.directory().mark_glyph_sets_def,Some(14));
}
