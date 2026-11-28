use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
#[test]fn anchor_coordinate_and_point(){
    let a=parse_gpos_anchor(&words(&[1,32768,32767]),0,Default::default()).unwrap();
    let b=parse_gpos_anchor(&words(&[2,32767,32768,65535]),0,Default::default()).unwrap();
    assert_eq!(a.detail,GposAnchorDetail::Coordinate);assert_eq!(b.detail,GposAnchorDetail::ContourPoint(65535));
    assert_eq!(a.design_offset_from(&b),(-65535,65535));assert_eq!(a.pixel_offset(12,16).unwrap(),(0,0));
    assert!(b.pixel_offset(12,16).is_err());
}
#[test]fn anchor_devices_relative_and_budget(){
    let b=words(&[0,3,10,20,10,10,10,12,1,0x6c00]);
    let l=GposValueLimits{max_bytes:b.len(),max_device_entries:6};
    let a=parse_gpos_anchor(&b,2,l).unwrap();
    assert_eq!(a.pixel_offset(10,11).unwrap(),(1,-2));
    assert!(matches!(parse_gpos_anchor(&b,2,GposValueLimits{max_device_entries:5,..l}),Err(Error::Limit(_))));
    for n in 0..b.len(){assert!(parse_gpos_anchor(&b[..n],2,Default::default()).is_err());}
}
#[test]fn anchor_null_variation_and_invalid_offsets(){
    let b=words(&[3,0,0,0,10,4,7,0x8000]);
    let a=parse_gpos_anchor(&b,0,Default::default()).unwrap();
    assert_eq!(a.detail,GposAnchorDetail::Device{x:None,y:Some(GposDevice::VariationIndex{outer:4,inner:7})});
    assert!(a.pixel_offset(12,12).is_err());
    for offset in [1,8,11,65535]{
        let b=words(&[3,0,0,offset,0,0,0,0]);
        assert!(parse_gpos_anchor(&b,0,Default::default()).is_err());
    }
    assert!(parse_gpos_anchor(&words(&[4,0,0]),0,Default::default()).is_err());
    assert!(parse_gpos_anchor(&[],usize::MAX,Default::default()).is_err());
}
#[test]fn adjustment_checked_all_fields(){
    let a=GposAdjustment{x_placement:10,y_placement:-20,x_advance:30,y_advance:-40};
    assert_eq!(a.checked_add(a).unwrap(),GposAdjustment{x_placement:20,y_placement:-40,x_advance:60,y_advance:-80});
    for v in [GposAdjustment{x_placement:i32::MAX,..Default::default()},GposAdjustment{y_placement:i32::MIN,..Default::default()},GposAdjustment{x_advance:i32::MAX,..Default::default()},GposAdjustment{y_advance:i32::MIN,..Default::default()}]{
        assert!(matches!(a.checked_add(v),Err(Error::Limit(_))));
    }
}
#[test]fn adjustment_pixel_and_design_separation(){
    let d=GposDevice::Pixels{start:10,end:11,deltas:vec![2,-3]};
    let v=GposValueRecord{x_placement:-100,y_advance:50,x_placement_device:Some(d.clone()),y_advance_device:Some(d),..Default::default()};
    assert_eq!(v.design_adjustment(),GposAdjustment{x_placement:-100,y_advance:50,..Default::default()});
    assert_eq!(v.pixel_adjustment(10,11).unwrap(),GposAdjustment{x_placement:2,y_advance:-3,..Default::default()});
    assert_eq!(v.pixel_adjustment(9,12).unwrap(),GposAdjustment::default());
    let v=GposValueRecord{x_advance_device:Some(GposDevice::VariationIndex{outer:0,inner:0}),..v};
    assert!(v.pixel_adjustment(10,11).is_err());
}
