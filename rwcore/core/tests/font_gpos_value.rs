use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
#[test]fn device_signed_packed_all_formats(){
    for (format,bits) in [(1,2usize),(2,4),(3,8)]{
        let n=1usize<<bits;let per=16/bits;
        let mut w=vec![8,8+n as u16-1,format];
        for i in (0..n).step_by(per){
            let mut packed=0;
            for j in 0..per{if i+j<n{packed|=((i+j) as u16)<<(16-bits*(j+1));}}
            w.push(packed);
        }
        let b=words(&w);let d=parse_gpos_device(&b,0,Default::default()).unwrap();
        for i in 0..n{
            let want=if i>=n/2{i as i16-n as i16}else{i as i16};
            assert_eq!(d.pixel_delta(8+i as u16),Some(want as i8));
        }
        assert_eq!(d.pixel_delta(7),Some(0));assert_eq!(d.pixel_delta(8+n as u16),Some(0));
    }
}
#[test]fn device_partial_word_and_truncation(){
    let b=words(&[10,12,1,0x6c00]);
    let d=parse_gpos_device(&b,0,Default::default()).unwrap();
    assert_eq!((d.pixel_delta(10),d.pixel_delta(11),d.pixel_delta(12)),(Some(1),Some(-2),Some(-1)));
    for n in 0..b.len(){assert!(parse_gpos_device(&b[..n],0,Default::default()).is_err());}
    assert!(parse_gpos_device(&words(&[12,10,1,0]),0,Default::default()).is_err());
    assert!(parse_gpos_device(&words(&[10,12,4,0]),0,Default::default()).is_err());
}
#[test]fn variation_index_distinct_from_pixels(){
    let b=words(&[65535,3,0x8000]);
    let d=parse_gpos_device(&b,0,GposValueLimits{max_device_entries:0,..Default::default()}).unwrap();
    assert_eq!(d,GposDevice::VariationIndex{outer:65535,inner:3});assert_eq!(d.pixel_delta(16),None);
}
#[test]fn value_record_all_formats_and_signed_values(){
    for format in 0u16..=255{
        let mut w=Vec::new();
        for i in 0..4{if format&(1<<i)!=0{w.push((-10-i as i16) as u16);}}
        for i in 0..4{if format&(16<<i)!=0{w.push(0);}}
        let b=words(&w);let (v,end)=parse_gpos_value_record(&b,0,0,b.len(),format,Default::default()).unwrap();
        assert_eq!(end,b.len());assert_eq!(gpos_value_record_size(format).unwrap(),b.len());
        let vals=[v.x_placement,v.y_placement,v.x_advance,v.y_advance];
        for i in 0..4{assert_eq!(vals[i],if format&(1<<i)!=0{-10-i as i16}else{0});}
        assert_eq!([v.x_placement_device,v.y_placement_device,v.x_advance_device,v.y_advance_device],[None,None,None,None]);
    }
    assert!(gpos_value_record_size(256).is_err());
}
#[test]fn value_parent_relative_and_shared_budget(){
    // Parent at byte 2; record at byte 4; both offsets target byte 8.
    let b=words(&[0,0,6,6,10,12,1,0x6c00]);
    let l=GposValueLimits{max_bytes:b.len(),max_device_entries:6};
    let (v,end)=parse_gpos_value_record(&b,4,2,8,0x30,l).unwrap();
    assert_eq!(end,8);assert_eq!(v.x_placement_device,v.y_placement_device);
    assert!(matches!(parse_gpos_value_record(&b,4,2,8,0x30,GposValueLimits{max_device_entries:5,..l}),Err(Error::Limit(_))));
    assert!(parse_gpos_value_record(&b,4,2,10,0x30,l).is_err());
    assert!(parse_gpos_value_record(&b,4,5,8,0x30,l).is_err());
    assert!(parse_gpos_value_record(&b,4,2,8,0x30,GposValueLimits{max_bytes:b.len()-1,..l}).is_err());
}
#[test]fn value_device_and_variation_combined(){
    let b=words(&[(-42i16) as u16,6,14,12,12,3,0x8000,2,9,0x8000]);
    let (v,end)=parse_gpos_value_record(&b,0,0,6,0x51,Default::default()).unwrap();
    assert_eq!(end,6);assert_eq!(v.x_placement,-42);
    assert_eq!(v.x_placement_device.unwrap().pixel_delta(12),Some(-128));
    assert_eq!(v.x_advance_device,Some(GposDevice::VariationIndex{outer:2,inner:9}));
}
