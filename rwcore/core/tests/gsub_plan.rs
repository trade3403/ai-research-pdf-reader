use rwcore::*;
fn lang(required_feature:Option<u16>,features:&[u16])->GsubLangSys{
    GsubLangSys{required_feature,features:features.to_vec()}
}
fn feature(tag:[u8;4],lookups:&[u16])->GsubFeature{
    GsubFeature{tag,parameters:None,lookups:lookups.to_vec()}
}
fn fixture()->GsubLayout{
    GsubLayout{
        directory:GsubDirectory{minor_version:0,scripts:vec![],features:vec![],lookups:vec![],feature_variations:None},
        scripts:vec![
            GsubScript{tag:*b"DFLT",default_language:Some(lang(None,&[4])),languages:vec![]},
            GsubScript{tag:*b"latn",default_language:Some(lang(Some(2),&[3,0,1,2,0])),
                languages:vec![(*b"TRK ",lang(Some(3),&[1,4]))]},
            GsubScript{tag:*b"arab",default_language:None,languages:vec![(*b"URD ",lang(None,&[4]))]},
        ],
        features:vec![feature(*b"liga",&[4,1,4]),feature(*b"liga",&[2,1]),feature(*b"rlig",&[3,1]),
            feature(*b"locl",&[0]),feature(*b"calt",&[5])],
        lookups:(0..6).map(|_|GsubLookup{kind:1,flags:0,subtables:vec![8],mark_filtering_set:None}).collect(),
    }
}
fn plan(x:&GsubLayout,s:[u8;4],lang:Option<[u8;4]>,tags:&[[u8;4]])->Option<GsubPlan>{
    build_gsub_base_plan(x,s,lang,tags,Default::default()).unwrap()
}
fn indices(p:&GsubPlan)->Vec<u16>{p.lookups.iter().map(|l|l.index).collect()}
#[test]fn exact_script_default_language_required(){
    let x=fixture();let p=plan(&x,*b"latn",None,&[]).unwrap();
    assert_eq!(p.script,*b"latn");assert_eq!(p.language,None);
    assert_eq!(p.required_feature,Some(2));assert_eq!(p.features,vec![2]);assert_eq!(indices(&p),vec![1,3]);
}
#[test]fn exact_language_overrides_default(){
    let x=fixture();let p=plan(&x,*b"latn",Some(*b"TRK "),&[*b"liga",*b"calt"]).unwrap();
    assert_eq!(p.language,Some(*b"TRK "));assert_eq!(p.required_feature,Some(3));
    assert_eq!(p.features,vec![1,3,4]);assert_eq!(indices(&p),vec![0,1,2,5]);
}
#[test]fn absent_language_falls_back_to_selected_default(){
    let x=fixture();let p=plan(&x,*b"latn",Some(*b"ENG "),&[]).unwrap();
    assert_eq!(p.language,None);assert_eq!(p.required_feature,Some(2));
}
#[test]fn absent_script_uses_dflt(){
    let x=fixture();let p=plan(&x,*b"cyrl",Some(*b"SRB "),&[*b"calt"]).unwrap();
    assert_eq!(p.script,*b"DFLT");assert_eq!(p.language,None);assert_eq!(indices(&p),vec![5]);
}
#[test]fn no_arbitrary_fallback(){
    let mut x=fixture();
    assert!(plan(&x,*b"arab",None,&[]).is_none());
    assert!(plan(&x,*b"arab",Some(*b"ARA "),&[]).is_none());
    assert_eq!(plan(&x,*b"arab",Some(*b"URD "),&[*b"calt"]).unwrap().language,Some(*b"URD "));
    x.scripts.remove(0);assert!(plan(&x,*b"cyrl",None,&[]).is_none());
}
#[test]fn duplicate_tags_and_lookup_provenance(){
    let x=fixture();let p=plan(&x,*b"latn",None,&[*b"liga",*b"liga",*b"rlig"]).unwrap();
    assert_eq!(p.features,vec![0,1,2]);assert_eq!(indices(&p),vec![1,2,3,4]);
    assert_eq!(p.lookups[0].features,vec![0,1,2]);
    assert_eq!(p.lookups[1].features,vec![1]);assert_eq!(p.lookups[3].features,vec![0]);
}
#[test]fn unavailable_features_and_empty_plan(){
    let x=fixture();let p=plan(&x,*b"latn",None,&[*b"zzzz",*b"calt",*b"zzzz"]).unwrap();
    assert_eq!(p.unavailable_tags,vec![*b"calt",*b"zzzz"]);
    let p=plan(&x,*b"DFLT",None,&[]).unwrap();
    assert!(p.lookups.is_empty());assert!(p.features.is_empty());assert_eq!(p.required_feature,None);
}
#[test]fn variations_explicit_base_only(){
    let mut x=fixture();x.directory.minor_version=1;x.directory.feature_variations=Some(100);
    assert!(plan(&x,*b"latn",None,&[]).unwrap().base_only_variations_present);
    x.directory.feature_variations=None;
    assert!(!plan(&x,*b"latn",None,&[]).unwrap().base_only_variations_present);
}
#[test]fn invalid_selected_language_feature_references(){
    let mut x=fixture();x.scripts[1].default_language.as_mut().unwrap().features.push(999);
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[],Default::default()).is_err());
    let mut x=fixture();x.scripts[1].default_language.as_mut().unwrap().required_feature=Some(999);
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[],Default::default()).is_err());
}
#[test]fn invalid_selected_lookup_reference(){
    let mut x=fixture();x.features[0].lookups.push(999);
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[*b"liga"],Default::default()).is_err());
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[],Default::default()).is_ok());
}
#[test]fn duplicate_script_and_language_rejected(){
    let mut x=fixture();x.scripts.push(x.scripts[1].clone());
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[],Default::default()).is_err());
    let mut x=fixture();x.scripts[1].languages.push((*b"TRK ",lang(None,&[])));
    assert!(build_gsub_base_plan(&x,*b"latn",None,&[],Default::default()).is_err());
}
#[test]fn exact_cumulative_limits(){
    let x=fixture();let tags=[*b"liga",*b"liga"];
    // 3 scripts + 1 language + 5 optional references + 1 required.
    // Selected features 0,1,2 have 7 raw lookup refs, 4 unique lookups.
    let l=GsubPlanLimits{max_scan_records:10,max_requested_tags:2,max_features:3,max_lookup_refs:7,max_lookups:4};
    assert!(build_gsub_base_plan(&x,*b"latn",None,&tags,l).is_ok());
    for small in [GsubPlanLimits{max_scan_records:9,..l},GsubPlanLimits{max_requested_tags:1,..l},
        GsubPlanLimits{max_features:2,..l},GsubPlanLimits{max_lookup_refs:6,..l},GsubPlanLimits{max_lookups:3,..l}]{
        assert!(build_gsub_base_plan(&x,*b"latn",None,&tags,small).is_err());
    }
}
#[test]fn real_abeezee_parsed_layout_integration(){
    let font=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),FontLimits::default()).unwrap();
    let face=font.face(0).unwrap();
    let layout=parse_gsub_layout(face.table(*b"GSUB").unwrap(),Default::default()).unwrap();
    let s=layout.scripts.iter().find(|s|s.default_language.is_some()).unwrap();
    let lang=s.default_language.as_ref().unwrap();
    let enabled:Vec<_>=lang.features.iter().map(|&i|layout.features[i as usize].tag).collect();
    let p=plan(&layout,s.tag,None,&enabled).unwrap();
    let mut expected:Vec<_>=lang.required_feature.into_iter().chain(lang.features.iter().copied())
        .flat_map(|i|layout.features[i as usize].lookups.iter().copied()).collect();
    expected.sort_unstable();expected.dedup();
    assert_eq!(indices(&p),expected);
}
