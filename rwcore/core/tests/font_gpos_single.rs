use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
#[test]fn single_shared_coverage_and_adjustment(){
    let b=words(&[1,10,5,(-20i16) as u16,30,1,2,3,7]);
    let p=parse_single_positioning(&b,10,Default::default()).unwrap();
    assert!(p.is_shared());assert_eq!(p.len(),2);
    for g in [3,7]{assert_eq!(p.apply_design(g,GposAdjustment{x_advance:100,..Default::default()}).unwrap(),
        Some(GposAdjustment{x_placement:-20,x_advance:130,..Default::default()}));}
    assert_eq!(p.apply_design(4,Default::default()).unwrap(),None);
}
#[test]fn single_indexed_range_coverage(){
    let b=words(&[2,14,2,3,1,(-2i16) as u16,3,2,1,4,6,0]);
    let p=parse_single_positioning(&b,10,Default::default()).unwrap();
    assert!(!p.is_shared());assert_eq!(p.len(),3);
    for (g,y) in [(4,1),(5,-2),(6,3)]{assert_eq!(p.value(g).unwrap().y_placement,y);}
    assert!(p.value(7).is_none());
}
#[test]fn single_device_cumulative_budget_and_atomic_apply(){
    // Two records share one Device target, each decoded against the
    // same subtable-wide budget. Coverage comes before the target.
    let b=words(&[2,12,0x10,2,20,20,1,2,3,4,10,12,1,0x6c00]);
    let l=SinglePosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()};
    let p=parse_single_positioning(&b,10,l).unwrap();
    let (d,px)=p.apply(4,Default::default(),Default::default(),11,12).unwrap().unwrap();
    assert_eq!(d,GposAdjustment::default());assert_eq!(px.x_placement,-2);
    assert!(matches!(parse_single_positioning(&b,10,SinglePosLimits{values:GposValueLimits{max_device_entries:5,..l.values},..l}),Err(Error::Limit(_))));
    assert!(p.apply(4,Default::default(),GposAdjustment{x_placement:i32::MIN,..Default::default()},11,12).is_err());
}
#[test]fn single_malformed_and_exact_limits(){
    let b=words(&[2,12,1,2,10,20,1,2,3,4]);
    for n in 0..b.len(){assert!(parse_single_positioning(&b[..n],10,Default::default()).is_err());}
    let l=SinglePosLimits{max_records:2,max_coverage_records:2,max_covered:2,values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}};
    assert!(parse_single_positioning(&b,10,l).is_ok());
    for bad in [SinglePosLimits{max_records:1,..l},SinglePosLimits{max_coverage_records:1,..l},SinglePosLimits{max_covered:1,..l}]{
        assert!(matches!(parse_single_positioning(&b,10,bad),Err(Error::Limit(_))));
    }
    for (at,v) in [(0,3),(2,8),(4,256),(6,1),(14,1),(18,3)]{
        let mut copy=b.clone();copy[at..at+2].copy_from_slice(& (v as u16).to_be_bytes());
        assert!(parse_single_positioning(&copy,10,Default::default()).is_err());
    }
}
#[test]fn single_zero_format_and_variation_rejection(){
    let b=words(&[2,8,0,2,1,2,3,4]);let p=parse_single_positioning(&b,10,Default::default()).unwrap();
    assert_eq!(p.value(3),Some(&GposValueRecord::default()));
    let b=words(&[1,8,0x10,14,1,1,3,1,2,0x8000]);
    let p=parse_single_positioning(&b,10,Default::default()).unwrap();
    assert!(p.apply(3,Default::default(),Default::default(),12,12).is_err());
    assert_eq!(p.apply_design(3,Default::default()).unwrap(),Some(GposAdjustment::default()));
}
