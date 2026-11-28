use rwcore::*;
fn plan(features:Vec<u16>,required:Option<u16>)->GsubPlan{
    GsubPlan{script:*b"latn",language:None,required_feature:required,features,unavailable_tags:vec![],lookups:vec![],base_only_variations_present:false}
}
fn edit(feature:u16,start:usize,end:usize,enabled:bool)->FeatureMaskOverride{FeatureMaskOverride{feature,start,end,enabled}}
#[test]fn mask_overlap_last_wins(){
    let m=ScalarFeatureMasks::build(&plan(vec![2,5,9],Some(2)),4,&[edit(5,0,3,false),edit(5,1,2,true),edit(9,2,4,false)],Default::default()).unwrap();
    assert_eq!(m.scalar_masks(),&[5,7,1,3]);
    assert_eq!(m.lookup_mask(&[5,9]).unwrap(),6);assert!(m.enabled(0,2).unwrap());assert!(!m.enabled(0,5).unwrap());
}
#[test]fn mask_invalid_plan_and_ranges(){
    for p in [plan(vec![2,2],None),plan(vec![5,2],None),plan(vec![2],Some(3))]{
        assert!(ScalarFeatureMasks::build(&p,2,&[],Default::default()).is_err());
    }
    let p=plan(vec![2,5],Some(2));
    for o in [edit(2,0,1,false),edit(7,0,1,true),edit(5,2,1,true),edit(5,0,3,true)]{
        assert!(ScalarFeatureMasks::build(&p,2,&[o],Default::default()).is_err());
    }
}
#[test]fn mask_full_width_and_empty(){
    let p=plan((0..64).collect(),Some(0));
    let m=ScalarFeatureMasks::build(&p,1,&[edit(63,0,1,false)],Default::default()).unwrap();
    assert_eq!(m.scalar_masks(),&[u64::MAX>>1]);assert_eq!(m.lookup_mask(&[63]).unwrap(),1u64<<63);
    assert!(ScalarFeatureMasks::build(&plan((0..65).collect(),None),0,&[],Default::default()).is_err());
    assert_eq!(ScalarFeatureMasks::build(&plan(vec![],None),2,&[],Default::default()).unwrap().scalar_masks(),&[0,0]);
}
#[test]fn mask_exact_budgets(){
    let p=plan(vec![5],None);let o=[edit(5,0,2,false)];
    let l=FeatureMaskLimits{max_scalars:2,max_overrides:1,max_work:5};
    assert!(ScalarFeatureMasks::build(&p,2,&o,l).is_ok());
    for bad in [FeatureMaskLimits{max_scalars:1,..l},FeatureMaskLimits{max_overrides:0,..l},FeatureMaskLimits{max_work:4,..l}]{
        assert!(matches!(ScalarFeatureMasks::build(&p,2,&o,bad),Err(Error::Limit(_))));
    }
}
#[test]fn mask_project_expansion_and_gap_contraction(){
    let c=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();
    let cmap=parse_unicode_cmap(c.face(0).unwrap(),Default::default()).unwrap();
    let run=map_source_run("ABC",&cmap,false,Default::default()).unwrap();
    let mut l=ProvenanceLedger::from_run(&run,u32::from(cmap.glyph_count()),Default::default()).unwrap();
    let m=ScalarFeatureMasks::build(&plan(vec![2,5,9],Some(2)),3,&[edit(5,0,1,false),edit(9,2,3,false)],Default::default()).unwrap();
    assert_eq!(m.project(&l,6).unwrap(),vec![5,7,3]);
    l.expand(0,&[1,2]).unwrap();
    assert_eq!(m.project(&l,8).unwrap(),vec![5,5,7,3]);
    l.contract(&[0,3],1).unwrap();
    assert_eq!(m.project(&l,7).unwrap(),vec![1,5,7]);
    assert!(m.project(&l,6).is_err());
}
#[test]fn mask_exhaustive_reference(){
    let p=plan(vec![1,3,7],Some(1));
    for a in 0..4{for b in a..4{for c in 0..4{for d in c..4{
        let m=ScalarFeatureMasks::build(&p,3,&[edit(3,a,b,false),edit(7,c,d,false)],Default::default()).unwrap();
        for i in 0..3{
            assert_eq!(m.scalar_masks()[i],1+2*u64::from(!(a..b).contains(&i))+4*u64::from(!(c..d).contains(&i)));
        }
    }}}}
}
