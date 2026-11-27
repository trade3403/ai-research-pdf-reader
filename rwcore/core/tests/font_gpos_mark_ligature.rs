use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    words(&[1,12,20,2,26,48,1,2,3,4,1,1,7,
        2,0,10,1,16,1,10,20,1,30,40,
        1,4,2,10,0,16,22,1,100,200,1,300,400,1,500,600])
}
#[test]fn component_geometry(){
    let p=parse_mark_to_ligature(&fixture(),10,Default::default()).unwrap();
    assert_eq!(p.component_count(7),Some(2));
    assert_eq!(p.attach_design(7,0,3,[(10,5),(80,7)]).unwrap(),Some((20,178)));
    assert_eq!(p.attach_design(7,1,4,[(0,0);2]).unwrap(),Some((470,560)));
    assert_eq!(p.attach_pixels(7,1,4,[(2,1),(0,3)],12,16).unwrap(),Some((2,-2)));
    assert!(p.pair(7,0,4).is_none());
    assert!(p.pair(7,usize::MAX,3).is_none());
    assert!(p.pair(8,0,3).is_none());
    assert!(p.pair(7,0,5).is_none());
    assert!(p.component_anchor(7,0,2).is_none());
}
#[test]fn component_limits_and_prefixes(){
    let b=fixture();
    for n in 0..b.len(){assert!(parse_mark_to_ligature(&b[..n],10,Default::default()).is_err(),"prefix {n}");}
    let l=MarkLigatureLimits{max_components:2,marks:MarkPosLimits{max_marks:2,max_bases:1,max_classes:2,max_cells:4,max_coverage_records:2,max_covered:2,values:GposValueLimits{max_bytes:b.len(),max_device_entries:0}}};
    assert!(parse_mark_to_ligature(&b,10,l).is_ok());
    assert!(matches!(parse_mark_to_ligature(&b,10,MarkLigatureLimits{max_components:1,..l}),Err(Error::Limit(_))));
    for marks in [MarkPosLimits{max_cells:3,..l.marks},MarkPosLimits{max_bases:0,..l.marks},MarkPosLimits{max_classes:1,..l.marks},MarkPosLimits{max_marks:1,..l.marks}] {
        assert!(matches!(parse_mark_to_ligature(&b,10,MarkLigatureLimits{marks,..l}),Err(Error::Limit(_))));
    }
}
#[test]fn component_offsets(){
    for (at,v) in [(0,2),(2,0),(4,10),(6,0),(8,10),(10,65535),(28,2),(30,0),(48,2),(50,0),(50,2),(52,0),(54,8),(58,65535)]{
        let mut b=fixture();b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());
        assert!(parse_mark_to_ligature(&b,10,Default::default()).is_err(),"at {at}");
    }
}
#[test]fn component_overflow(){
    let p=parse_mark_to_ligature(&fixture(),10,Default::default()).unwrap();
    assert!(p.attach_design(7,0,3,[(i32::MAX,0),(i32::MIN,0)]).is_err());
    assert_eq!(p.attach_design(7,0,3,[(i32::MAX,0);2]).unwrap(),Some((90,180)));
}
#[test]fn component_device_budget(){
    // MarkArray 24, LigatureArray 30, LigatureAttach 34,
    // shared format 3 anchor 38 with three device entries.
    let b=words(&[1,12,18,1,24,30,1,1,3,1,1,7,1,0,14,1,4,1,4,3,10,20,10,0,10,12,1,0x6c00]);
    let l=MarkLigatureLimits{marks:MarkPosLimits{values:GposValueLimits{max_device_entries:6,..Default::default()},..Default::default()},..Default::default()};
    let p=parse_mark_to_ligature(&b,10,l).unwrap();
    assert_eq!(p.attach_pixels(7,0,3,[(2,1),(0,3)],11,12).unwrap(),Some((2,-2)));
    assert!(matches!(parse_mark_to_ligature(&b,10,MarkLigatureLimits{marks:MarkPosLimits{values:GposValueLimits{max_device_entries:5,..l.marks.values},..l.marks},..l}),Err(Error::Limit(_))));
}
