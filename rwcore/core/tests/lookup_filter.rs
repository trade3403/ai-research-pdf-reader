use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Gdef{
    // GDEF 1.2, glyph ClassDef at 14, mark attachment ClassDef at 34,
    // MarkGlyphSetsDef at 48. 10 Base,11 Ligature,12/13/14 Mark,
    // 15 Component,16 Unclassified. Mark classes 12=1,13=2,14=0.
    // Set 0 contains 13 and non-mark 10; set 1 contains 12 and 14.
    let b=bytes(&[1,2,14,0,0,34,48,
        1,10,7,1,2,3,3,3,4,0,
        1,12,4,1,2,0,0,
        1,2,0,12,0,20,1,2,10,13,1,2,12,14]);
    parse_gdef(&b,32,Default::default()).unwrap()
}
fn filter<'a>(g:&'a Gdef,flags:u16,set:Option<u16>)->LookupFilter<'a>{
    LookupFilter::new(flags,set,32,Some(g)).unwrap()
}
#[test]fn unfiltered_all_classes(){
    let g=fixture();let f=filter(&g,0,None);
    assert_eq!(g.glyph_count(),32);
    for id in 0..32u16{assert_eq!(f.eligibility(id).unwrap(),GlyphEligibility::Eligible);}
    assert!(f.eligibility(32).is_err());
}
#[test]fn ignore_base_and_ligature_independently(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,LOOKUP_IGNORE_BASE,None);
    assert_eq!(f.eligibility(10).unwrap(),IgnoreBase);
    assert_eq!(f.eligibility(11).unwrap(),Eligible);
    let f=filter(&g,LOOKUP_IGNORE_LIGATURE,None);
    assert_eq!(f.eligibility(10).unwrap(),Eligible);
    assert_eq!(f.eligibility(11).unwrap(),IgnoreLigature);
    assert_eq!(f.eligibility(15).unwrap(),Eligible);
}
#[test]fn ignore_marks_and_combined_flags(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,LOOKUP_IGNORE_BASE|LOOKUP_IGNORE_LIGATURE|LOOKUP_IGNORE_MARK,None);
    for (id,e) in [(10,IgnoreBase),(11,IgnoreLigature),(12,IgnoreMark),(13,IgnoreMark),
        (14,IgnoreMark),(15,Eligible),(16,Eligible),(9,Eligible)]{
        assert_eq!(f.eligibility(id).unwrap(),e);
    }
}
#[test]fn attachment_type_only_affects_marks(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,0x0100,None);
    assert_eq!(f.mark_attachment_type(),1);
    assert_eq!(f.eligibility(12).unwrap(),Eligible);
    assert_eq!(f.eligibility(13).unwrap(),WrongMarkAttachmentClass);
    assert_eq!(f.eligibility(14).unwrap(),WrongMarkAttachmentClass);
    for id in [9,10,11,15,16]{assert_eq!(f.eligibility(id).unwrap(),Eligible);}
}
#[test]fn mark_set_membership_only_affects_marks(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,LOOKUP_USE_MARK_SET,Some(0));
    assert_eq!(f.mark_filtering_set(),Some(0));
    for (id,e) in [(10,Eligible),(11,Eligible),(12,OutsideMarkSet),(13,Eligible),(14,OutsideMarkSet),(15,Eligible)]{
        assert_eq!(f.eligibility(id).unwrap(),e);
    }
    let f=filter(&g,LOOKUP_USE_MARK_SET,Some(1));
    assert_eq!(f.eligibility(12).unwrap(),Eligible);
    assert_eq!(f.eligibility(13).unwrap(),OutsideMarkSet);
    assert_eq!(f.eligibility(14).unwrap(),Eligible);
}
#[test]fn set_overrides_attachment_type(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,0x0100|LOOKUP_USE_MARK_SET,Some(0));
    assert_eq!(f.eligibility(12).unwrap(),OutsideMarkSet);
    assert_eq!(f.eligibility(13).unwrap(),Eligible);
}
#[test]fn ignore_marks_overrides_set_and_attachment(){
    let g=fixture();use GlyphEligibility::*;
    let f=filter(&g,0x0200|LOOKUP_IGNORE_MARK|LOOKUP_USE_MARK_SET,Some(0));
    for id in [12,13,14]{assert_eq!(f.eligibility(id).unwrap(),IgnoreMark);}
    assert!(LookupFilter::new(LOOKUP_IGNORE_MARK|LOOKUP_USE_MARK_SET,Some(2),32,Some(&g)).is_err());
}
#[test]fn right_to_left_does_not_reverse_logical_indices(){
    let g=fixture();
    let f=filter(&g,LOOKUP_RIGHT_TO_LEFT|LOOKUP_IGNORE_MARK,None);
    assert!(f.right_to_left());assert_eq!(f.flags(),9);
    assert_eq!(f.eligible_indices(&[12,10,13,11,14,15],Default::default()).unwrap(),vec![1,3,5]);
}
#[test]fn missing_gdef_does_not_infer_classes(){
    let f=LookupFilter::new(0xff0e,None,32,None).unwrap();
    assert_eq!(f.eligible_indices(&[10,12,31],Default::default()).unwrap(),vec![0,1,2]);
    assert!(LookupFilter::new(LOOKUP_USE_MARK_SET,Some(0),32,None).is_err());
}
#[test]fn missing_attachment_class_defaults_zero(){
    let g=parse_gdef(&bytes(&[1,0,12,0,0,0,1,12,1,3]),32,Default::default()).unwrap();
    assert_eq!(filter(&g,0x0100,None).eligibility(12).unwrap(),GlyphEligibility::WrongMarkAttachmentClass);
    assert_eq!(filter(&g,0,None).eligibility(12).unwrap(),GlyphEligibility::Eligible);
    assert!(LookupFilter::new(LOOKUP_USE_MARK_SET,Some(0),32,Some(&g)).is_err());
}
#[test]fn invalid_flags_domains_and_mark_indices(){
    let g=fixture();
    for (flags,set,n) in [(0x20,None,32),(0x40,None,32),(0x80,None,32),
        (0,Some(0),32),(LOOKUP_USE_MARK_SET,None,32),(LOOKUP_USE_MARK_SET,Some(2),32),
        (0,None,0),(0,None,65537),(0,None,31)]{
        assert!(LookupFilter::new(flags,set,n,Some(&g)).is_err());
    }
    assert!(LookupFilter::new(0,None,65536,None).unwrap().eligibility(65535).unwrap().is_eligible());
}
#[test]fn bounded_indices_and_invalid_glyphs(){
    let g=fixture();let f=filter(&g,LOOKUP_IGNORE_MARK,None);
    let run=[10,12,11,13,15,14];
    let l=LookupFilterLimits{max_run_glyphs:6,max_eligible:3};
    assert_eq!(f.eligible_indices(&run,l).unwrap(),vec![0,2,4]);
    assert!(f.eligible_indices(&run,LookupFilterLimits{max_run_glyphs:5,..l}).is_err());
    assert!(f.eligible_indices(&run,LookupFilterLimits{max_eligible:2,..l}).is_err());
    assert!(f.eligible_indices(&[10,32],l).is_err());
    let zero=LookupFilterLimits{max_run_glyphs:6,max_eligible:0};
    assert!(f.eligible_indices(&[12,13,14],zero).unwrap().is_empty());
    assert!(f.eligible_indices(&[],zero).unwrap().is_empty());
}
#[test]fn gsub_lookup_integration(){
    let g=fixture();
    let mut lookup=GsubLookup{kind:5,flags:LOOKUP_USE_MARK_SET,subtables:vec![20],mark_filtering_set:Some(1)};
    let f=LookupFilter::from_gsub_lookup(&lookup,32,Some(&g)).unwrap();
    assert_eq!(f.eligible_indices(&[12,13,14],Default::default()).unwrap(),vec![0,2]);
    lookup.kind=0;
    assert!(LookupFilter::from_gsub_lookup(&lookup,32,Some(&g)).is_err());
}
