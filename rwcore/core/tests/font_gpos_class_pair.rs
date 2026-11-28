use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // 2x2 matrix, coverage [3,4], class1 format 1 assigns 3 -> 1,
    // class2 format 2 assigns 7..8 -> 1.
    words(&[2,32,4,1,40,48,2,2,10,11,20,21,30,31,40,41,
        1,2,3,4,1,3,1,1,2,1,7,8,1])
}
#[test]fn class_pair_matrix_and_default_zero(){
    let p=parse_pair_positioning_format2(&fixture(),10,Default::default()).unwrap();
    assert_eq!(p.dimensions(),(2,2));assert_eq!(p.second_format(),1);
    for (a,b,c,d,x,y) in [(4,6,0,0,10,11),(4,7,0,1,20,21),(3,6,1,0,30,31),(3,8,1,1,40,41)]{
        assert_eq!(p.classes(a,b),Some((c,d)));
        let got=p.apply_design(a,b,[Default::default();2]).unwrap().unwrap();
        assert_eq!(got[0].x_advance,x);assert_eq!(got[1].x_placement,y);
    }
    assert!(p.pair(5,7).is_none());assert!(p.pair(3,10).is_none());
    assert!(p.apply_design(3,8,[Default::default(),GposAdjustment{x_placement:i32::MAX,..Default::default()}]).is_err());
}
#[test]fn class_pair_exact_caps_and_truncation(){
    let b=fixture();
    for n in 0..b.len(){assert!(parse_pair_positioning_format2(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=ClassPairPosLimits{pair:PairPosLimits{max_pairs:4,max_covered:2,..Default::default()},max_classes:2,max_class_records:1,max_class_assigned:2};
    assert!(parse_pair_positioning_format2(&b,10,l).is_ok());
    for bad in [ClassPairPosLimits{max_classes:1,..l},ClassPairPosLimits{max_class_records:0,..l},ClassPairPosLimits{max_class_assigned:1,..l},ClassPairPosLimits{pair:PairPosLimits{max_pairs:3,..l.pair},..l}]{
        assert!(matches!(parse_pair_positioning_format2(&b,10,bad),Err(Error::Limit(_))));
    }
}
#[test]fn class_pair_invalid_dimensions_and_offsets(){
    for (at,v) in [(0,1),(2,16),(4,256),(8,16),(10,0),(12,0),(14,0),(46,2),(56,2)]{
        let mut b=fixture();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());
        assert!(parse_pair_positioning_format2(&b,10,Default::default()).is_err(),"at {at}");
    }
}
#[test]fn class_pair_zero_stride_is_still_bounded(){
    let b=words(&[2,16,0,0,22,22,2,2,1,1,3,2,0]);
    let p=parse_pair_positioning_format2(&b,10,Default::default()).unwrap();
    assert_eq!(p.apply_design(3,4,[Default::default();2]).unwrap(),Some([Default::default();2]));
    let mut b=b;b[12..14].copy_from_slice(&65535u16.to_be_bytes());b[14..16].copy_from_slice(&65535u16.to_be_bytes());
    assert!(matches!(parse_pair_positioning_format2(&b,10,Default::default()),Err(Error::Limit(_))));
}
#[test]fn class_pair_device_base_and_budget(){
    // One cell, both ValueRecords refer to subtable-relative Device at 30.
    let b=words(&[2,20,0x40,0x10,26,26,1,1,30,30,1,1,3,2,0,10,12,1,0x6c00]);
    let l=ClassPairPosLimits{pair:PairPosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()},..Default::default()};
    let p=parse_pair_positioning_format2(&b,10,l).unwrap();
    let (_,px)=p.apply(3,4,[Default::default();2],[Default::default();2],11,12).unwrap().unwrap();
    assert_eq!(px[0].x_advance,-2);assert_eq!(px[1].x_placement,-2);
    assert!(matches!(parse_pair_positioning_format2(&b,10,ClassPairPosLimits{pair:PairPosLimits{values:GposValueLimits{max_device_entries:5,..l.pair.values},..l.pair},..l}),Err(Error::Limit(_))));
}
