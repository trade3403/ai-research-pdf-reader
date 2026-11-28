use rwcore::*;
fn fixture(kind:u16,flags:u16)->Vec<u8>{
    let mut w=vec![1,0,10,30,44,1,0x4446,0x4c54,8,4,0,0,0xffff,1,0];
    // script list ends at byte 30; feature list ends at 44.
    w.extend([1,0x6b65,0x726e,8,0,1,0]);
    w.extend([1,4,kind,flags,1,if flags&16!=0{10}else{8}]);
    if flags&16!=0{w.push(2);}
    w.push(1);
    w.iter().flat_map(|v|v.to_be_bytes()).collect()
}
#[test]fn gpos_all_nine_lookup_headers(){
    for kind in 1..=9{
        let b=fixture(kind,16);let p=parse_gpos_layout(&b,Default::default()).unwrap();
        assert_eq!(p.lookups[0].kind,kind);assert_eq!(p.lookups[0].mark_filtering_set,Some(2));
        assert_eq!(p.features[0].tag,*b"kern");assert_eq!(p.features[0].lookups,vec![0]);
        assert_eq!(p.scripts[0].default_language.as_ref().unwrap().features,vec![0]);
        assert_eq!(p.lookups[0].subtables,vec![58]);
        assert_eq!(parse_gsub_layout(&b,Default::default()).is_ok(),kind<=8);
    }
}
#[test]fn gpos_invalid_lookup_flags_and_cross_indices(){
    for (kind,flags) in [(0,0),(10,0),(1,32),(1,128)]{
        assert!(parse_gpos_layout(&fixture(kind,flags),Default::default()).is_err());
    }
    let mut b=fixture(1,0);b[42..44].copy_from_slice(&1u16.to_be_bytes());
    assert!(parse_gpos_layout(&b,Default::default()).is_err());
}
#[test]fn gpos_truncation_and_exact_limits(){
    let b=fixture(9,16);
    for n in 0..b.len(){assert!(parse_gpos_layout(&b[..n],Default::default()).is_err(),"prefix {n}");}
    let l=GposLimits{max_records:7,max_bytes:b.len()};
    assert!(parse_gpos_layout(&b,l).is_ok());
    assert!(matches!(parse_gpos_layout(&b,GposLimits{max_records:6,..l}),Err(Error::Limit(_))));
    assert!(matches!(parse_gpos_layout(&b,GposLimits{max_bytes:b.len()-1,..l}),Err(Error::Limit(_))));
}
#[test]fn gpos_version_11_directory(){
    let mut b=fixture(9,0);b.splice(10..10,[0,0,0,0]);b[3]=1;
    for p in [4,6,8]{let v=u16::from_be_bytes([b[p],b[p+1]])+4;b[p..p+2].copy_from_slice(&v.to_be_bytes());}
    let p=parse_gpos_directory(&b,Default::default()).unwrap();assert_eq!(p.minor_version,1);assert_eq!(p.feature_variations,None);
    let at=b.len() as u32;b[10..14].copy_from_slice(&at.to_be_bytes());b.extend([0;8]);
    assert_eq!(parse_gpos_layout(&b,Default::default()).unwrap().directory.feature_variations,Some(at as usize));
}
#[test]fn gpos_native_directory_reference(){
    for font in [include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf").as_slice(),
        include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf").as_slice()]{
        let c=parse_font(font,Default::default()).unwrap();let b=c.face(0).unwrap().table(*b"GPOS").unwrap();
        let p=parse_gpos_layout(b,Default::default()).unwrap();
        let u=|at:usize|u16::from_be_bytes([b[at],b[at+1]]) as usize;
        let base=u(8);assert_eq!(p.lookups.len(),u(base));assert!(!p.lookups.is_empty());
        for (i,l) in p.lookups.iter().enumerate(){
            let at=base+u(base+2+2*i);assert_eq!(l.kind as usize,u(at));assert_eq!(l.flags as usize,u(at+2));
            assert_eq!(l.subtables,(0..u(at+4)).map(|j|at+u(at+6+2*j)).collect::<Vec<_>>());
        }
    }
}
