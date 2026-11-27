use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    // Mark1 {3,4}, Mark2 {3,7}; shared glyph 3 is valid.
    // Mark2 rows: (100,200), null; (-10,30), (50,-20).
    words(&[1,12,20,2,28,50,1,2,3,4,1,2,3,7,
        2,0,10,1,16,1,10,20,1,30,40,
        2,10,0,16,22,1,100,200,1,(-10i16) as u16,30,1,50,(-20i16) as u16])
}
#[test]fn mark_roles_and_matrix(){
    let p=parse_mark_to_mark(&fixture(),10,Default::default()).unwrap();
    assert_eq!(p.dimensions(),(2,2));
    assert_eq!(p.mark1(3).unwrap().class,0);
    assert_eq!(p.mark1(4).unwrap().class,1);
    assert!(p.mark1(7).is_none());
    assert_eq!(p.attach_design(3,3,[(10,5),(80,7)]).unwrap(),Some((20,178)));
    assert_eq!(p.attach_design(7,4,[(0,0);2]).unwrap(),Some((20,-60)));
    assert_eq!(p.attach_design(7,3,[(0,0);2]).unwrap(),Some((-20,10)));
    assert!(p.pair(3,4).is_none());assert!(p.pair(4,3).is_none());
    assert!(p.mark2_anchor(7,2).is_none());
    assert_eq!(p.attach_pixels(7,4,[(2,1),(0,3)],12,16).unwrap(),Some((2,-2)));
}
#[test]fn mark_matrix_limits_and_prefixes(){
    let b=fixture();
    for n in 0..b.len(){assert!(parse_mark_to_mark(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=MarkPosLimits{max_marks:2,max_bases:2,max_classes:2,max_cells:4,max_coverage_records:2,max_covered:2,values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}};
    assert!(parse_mark_to_mark(&b,10,MarkToMarkLimits{matrix:l}).is_ok());
    for matrix in [MarkPosLimits{max_marks:1,..l},MarkPosLimits{max_bases:1,..l},MarkPosLimits{max_cells:3,..l},MarkPosLimits{max_classes:1,..l},MarkPosLimits{values:GposValueLimits{max_bytes:b.len()-1,..l.values},..l}]{
        assert!(matches!(parse_mark_to_mark(&b,10,MarkToMarkLimits{matrix}),Err(Error::Limit(_))));
    }
}
#[test]fn mark_matrix_malformed(){
    for (at,v) in [(0,2),(2,0),(4,65535),(6,0),(8,10),(10,65535),(28,1),(30,2),(32,0),(32,8),(50,1),(52,8),(56,65535)]{
        let mut b=fixture();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());
        assert!(parse_mark_to_mark(&b,10,Default::default()).is_err(),"offset {at}");
    }
    assert!(parse_mark_to_mark(&fixture(),7,Default::default()).is_err());
}
#[test]fn mark_overflow_and_cancellation(){
    let p=parse_mark_to_mark(&fixture(),10,Default::default()).unwrap();
    assert!(p.attach_design(3,3,[(i32::MAX,0),(i32::MIN,0)]).is_err());
    assert!(p.attach_pixels(3,3,[(0,i32::MIN),(0,i32::MAX)],12,16).is_err());
    assert_eq!(p.attach_design(3,3,[(i32::MAX,i32::MIN);2]).unwrap(),Some((90,180)));
}
#[test]fn mark_cumulative_device_budget(){
    // Both arrays refer to same anchor: each reference consumes budget.
    let b=words(&[1,12,18,1,24,30,1,1,3,1,1,7,1,0,10,1,4,3,10,20,10,0,10,12,1,0x6c00]);
    let l=MarkPosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()};
    let p=parse_mark_to_mark(&b,10,MarkToMarkLimits{matrix:l}).unwrap();
    assert_eq!(p.attach_pixels(7,3,[(2,1),(0,3)],11,12).unwrap(),Some((2,-2)));
    let matrix=MarkPosLimits{values:GposValueLimits{max_device_entries:5,..l.values},..l};
    assert!(matches!(parse_mark_to_mark(&b,10,MarkToMarkLimits{matrix}),Err(Error::Limit(_))));
}
#[test]fn mark_contour_unresolved(){
    // Mark1 format 2, Mark2 format 1. MarkArray=24, Mark2Array=38.
    let b=words(&[1,12,18,1,24,38,1,1,3,1,1,7,1,0,6,2,10,20,5,1,4,1,100,200]);
    let p=parse_mark_to_mark(&b,10,Default::default()).unwrap();
    assert_eq!(p.attach_design(7,3,[(0,0);2]).unwrap(),Some((90,180)));
    assert!(matches!(p.attach_pixels(7,3,[(0,0);2],12,16),Err(Error::Unsupported(_))));
}
