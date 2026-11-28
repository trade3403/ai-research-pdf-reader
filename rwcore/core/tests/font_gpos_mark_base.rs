use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // mark coverage 12, base coverage 20, mark array 26,
    // base array 48. Two mark classes, one base, second cell null.
    words(&[1,12,20,2,26,48,1,2,3,4,1,1,7,
        2,0,10,1,16,1,10,20,1,30,40,
        1,6,0,1,100,200])
}
#[test]fn mark_base_class_and_geometry(){
    let p=parse_mark_to_base(&fixture(),10,Default::default()).unwrap();
    assert_eq!(p.dimensions(),(1,2));assert_eq!(p.mark(3).unwrap().class,0);
    assert_eq!(p.mark(4).unwrap().class,1);
    assert_eq!(p.attach_design(7,3,[(10,5),(80,7)]).unwrap(),Some((20,178)));
    assert_eq!(p.attach_pixels(7,3,[(2,1),(0,3)],12,16).unwrap(),Some((2,-2)));
    for (a,b) in [(7,4),(8,3),(7,5)]{assert!(p.pair(a,b).is_none());}
    assert!(p.base_anchor(7,2).is_none());
}
#[test]fn mark_base_exact_limits_and_truncation(){
    let b=fixture();
    for n in 0..b.len(){assert!(parse_mark_to_base(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=MarkPosLimits{max_marks:2,max_bases:1,max_classes:2,max_cells:2,max_coverage_records:2,max_covered:2,
        values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}};
    assert!(parse_mark_to_base(&b,10,l).is_ok());
    for bad in [MarkPosLimits{max_marks:1,..l},MarkPosLimits{max_bases:0,..l},MarkPosLimits{max_classes:1,..l},MarkPosLimits{max_cells:1,..l}]{assert!(matches!(parse_mark_to_base(&b,10,bad),Err(Error::Limit(_))));}
}
#[test]fn mark_base_invalid_offsets_and_classes(){
    for (at,v) in [(0,2),(2,0),(4,10),(6,0),(8,10),(10,65535),(26,1),(28,2),(30,0),(30,8),(48,2),(50,4)]{
        let mut b=fixture();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());
        assert!(parse_mark_to_base(&b,10,Default::default()).is_err(),"at {at}");
    }
}
#[test]fn mark_base_overflow_and_cancellation(){
    let p=parse_mark_to_base(&fixture(),10,Default::default()).unwrap();
    assert!(p.attach_design(7,3,[(i32::MAX,0),(i32::MIN,0)]).is_err());
    assert_eq!(p.attach_design(7,3,[(i32::MAX,0),(i32::MAX,0)]).unwrap(),Some((90,180)));
}
#[test]fn mark_base_device_budget(){
    // mark array 24, base array 30; both anchors target the same
    // physical format 3 anchor at 34, with one 3-entry Device.
    let b=words(&[1,12,18,1,24,30,1,1,3,1,1,7,1,0,10,1,4,3,10,20,10,0,10,12,1,0x6c00]);
    let l=MarkPosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()};
    let p=parse_mark_to_base(&b,10,l).unwrap();
    assert_eq!(p.attach_pixels(7,3,[(2,1),(0,3)],11,12).unwrap(),Some((2,-2)));
    assert!(matches!(parse_mark_to_base(&b,10,MarkPosLimits{values:GposValueLimits{max_device_entries:5,..l.values},..l}),Err(Error::Limit(_))));
}
