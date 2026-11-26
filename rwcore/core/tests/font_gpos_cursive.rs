use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{words(&[1,14,2,0,22,28,0,1,2,3,7,1,100,20,1,(-30i16) as u16,5])}
#[test]fn cursive_nominal_both_fixed_ends(){
    let p=parse_cursive_positioning(&fixture(),10,Default::default()).unwrap();
    assert_eq!(p.len(),2);
    let origins=[(10,5),(80,7)];
    assert_eq!(p.connect_design(3,7,origins,CursiveFixedEnd::Exit).unwrap(),Some(CursiveConnection{exit_shift:(0,0),entry_shift:(60,13)}));
    assert_eq!(p.connect_design(3,7,origins,CursiveFixedEnd::Entry).unwrap(),Some(CursiveConnection{exit_shift:(-60,-13),entry_shift:(0,0)}));
    for (a,b) in [(7,3),(3,3),(4,7)]{assert!(p.pair(a,b).is_none());}
}
#[test]fn cursive_truncation_and_limits(){
    let b=fixture();for n in 0..b.len(){assert!(parse_cursive_positioning(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=CursivePosLimits{max_records:2,max_coverage_records:2,max_covered:2,values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}};
    assert!(parse_cursive_positioning(&b,10,l).is_ok());
    for bad in [CursivePosLimits{max_records:1,..l},CursivePosLimits{max_covered:1,..l},CursivePosLimits{max_coverage_records:1,..l}]{assert!(matches!(parse_cursive_positioning(&b,10,bad),Err(Error::Limit(_))));}
    for (at,v) in [(0,2),(2,10),(8,12),(10,65535),(16,1)]{
        let mut b=b.clone();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());assert!(parse_cursive_positioning(&b,10,Default::default()).is_err());
    }
}
#[test]fn cursive_checked_geometry(){
    let p=parse_cursive_positioning(&fixture(),10,Default::default()).unwrap();
    assert!(p.connect_design(3,7,[(i32::MAX,0),(i32::MIN,0)],CursiveFixedEnd::Exit).is_err());
    assert_eq!(p.connect_design(3,7,[(i32::MAX,0),(i32::MAX,0)],CursiveFixedEnd::Exit).unwrap().unwrap().entry_shift,(130,15));
}
#[test]fn cursive_shared_device_budget(){
    // Both anchors share format 3 at 22, device relative offset 10.
    let b=words(&[1,14,2,0,22,22,0,1,2,3,7,3,0,0,10,0,10,12,1,0x6c00]);
    let l=CursivePosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()};
    let p=parse_cursive_positioning(&b,10,l).unwrap();
    assert_eq!(p.connect_pixels(3,7,[(2,3),(0,0)],CursiveFixedEnd::Exit,11,12).unwrap().unwrap().entry_shift,(2,3));
    assert!(matches!(parse_cursive_positioning(&b,10,CursivePosLimits{values:GposValueLimits{max_device_entries:5,..l.values},..l}),Err(Error::Limit(_))));
}
#[test]fn cursive_unresolved_point(){
    let mut b=fixture();b[28..30].copy_from_slice(&2u16.to_be_bytes());b.extend([0,1]);
    let p=parse_cursive_positioning(&b,10,Default::default()).unwrap();
    assert!(p.connect_design(3,7,[(0,0);2],CursiveFixedEnd::Exit).unwrap().is_some());
    assert!(p.connect_pixels(3,7,[(0,0);2],CursiveFixedEnd::Exit,12,12).is_err());
}
