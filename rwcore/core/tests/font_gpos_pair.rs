use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // Coverage [3,7], first set has two records, second set one.
    words(&[1,14,4,1,2,22,36,1,2,3,7,2,4,(-20i16) as u16,5,9,10,(-3i16) as u16,1,4,30,0])
}
#[test]fn pair_exact_matching_and_adjustment(){
    let b=fixture();let p=parse_pair_positioning_format1(&b,10,Default::default()).unwrap();
    assert_eq!(p.len(),2);assert_eq!(p.second_format(),1);
    assert_eq!(p.pair(3,4).unwrap().first_value.x_advance,-20);
    assert_eq!(p.pair(3,9).unwrap().second_value.x_placement,-3);
    assert_eq!(p.pair(7,4).unwrap().first_value.x_advance,30);
    for (a,b) in [(4,3),(3,5),(7,9)]{assert!(p.pair(a,b).is_none());}
    let prior=[GposAdjustment{x_advance:100,..Default::default()},GposAdjustment{x_placement:10,..Default::default()}];
    assert_eq!(p.apply_design(3,4,prior).unwrap(),Some([GposAdjustment{x_advance:80,..Default::default()},GposAdjustment{x_placement:15,..Default::default()}]));
}
#[test]fn pair_truncation_and_exact_limits(){
    let b=fixture();
    for n in 0..b.len(){assert!(parse_pair_positioning_format1(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=PairPosLimits{max_sets:2,max_pairs:3,max_coverage_records:2,max_covered:2,values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}};
    assert!(parse_pair_positioning_format1(&b,10,l).is_ok());
    for bad in [PairPosLimits{max_sets:1,..l},PairPosLimits{max_pairs:2,..l},PairPosLimits{max_covered:1,..l}]{
        assert!(matches!(parse_pair_positioning_format1(&b,10,bad),Err(Error::Limit(_))));
    }
}
#[test]fn pair_invalid_order_offsets_and_indices(){
    for (at,v) in [(0,2),(2,12),(4,256),(10,12),(16,1),(30,4),(30,10)]{
        let mut b=fixture();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());
        assert!(parse_pair_positioning_format1(&b,10,Default::default()).is_err(),"at {at}");
    }
}
#[test]fn pair_device_relative_and_atomic(){
    // PairSet at 18, record at 20, Device at 26 (relative 8).
    let b=words(&[1,12,0x40,0x10,1,18,1,1,3,1,4,8,8,10,12,1,0x6c00]);
    let l=PairPosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()};
    let p=parse_pair_positioning_format1(&b,10,l).unwrap();
    let (d,px)=p.apply(3,4,[Default::default();2],[Default::default();2],11,12).unwrap().unwrap();
    assert_eq!(d,[Default::default();2]);assert_eq!(px[0].x_advance,-2);assert_eq!(px[1].x_placement,-2);
    assert!(matches!(parse_pair_positioning_format1(&b,10,PairPosLimits{values:GposValueLimits{max_device_entries:5,..l.values},..l}),Err(Error::Limit(_))));
    assert!(p.apply(3,4,[Default::default();2],[GposAdjustment{x_advance:i32::MIN,..Default::default()},Default::default()],11,12).is_err());
}
#[test]fn pair_zero_format_and_second_overflow(){
    let b=words(&[1,12,0,0,1,18,1,1,3,1,4]);
    let p=parse_pair_positioning_format1(&b,10,Default::default()).unwrap();
    assert_eq!(p.apply_design(3,4,[Default::default();2]).unwrap(),Some([Default::default();2]));
    let p=parse_pair_positioning_format1(&fixture(),10,Default::default()).unwrap();
    assert!(p.apply_design(3,4,[Default::default(),GposAdjustment{x_placement:i32::MAX,..Default::default()}]).is_err());
}
