//! Independent raw SFNT/GSUB reference for base-feature lookup plans.
//! No script-specific shaping stages, execution, or external shaper comparison.
use rwcore::*;

type Tag=[u8;4];
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn tag(b:&[u8],p:usize)->Tag{b[p..p+4].try_into().unwrap()}
fn table<'a>(b:&'a [u8],wanted:Tag)->Option<&'a [u8]>{
    for i in 0..u(b,4){
        let p=12+16*i;
        if tag(b,p)==wanted{let s=d(b,p+8);return Some(&b[s..s+d(b,p+12)]);}
    }
    None
}
#[derive(Debug)]
struct RawLang{required:Option<u16>,features:Vec<u16>}
#[derive(Debug)]
struct RawScript{tag:Tag,default:Option<RawLang>,languages:Vec<(Tag,RawLang)>}
#[derive(Debug)]
struct RawFeature{tag:Tag,lookups:Vec<u16>}
fn words(b:&[u8],p:usize,n:usize)->Vec<u16>{
    (0..n).map(|i|u(b,p+2*i) as u16).collect()
}
fn raw_lang(b:&[u8],p:usize)->RawLang{
    assert_eq!(u(b,p),0);
    let r=u(b,p+2) as u16;
    RawLang{required:(r!=65535).then_some(r),features:words(b,p+6,u(b,p+4))}
}
fn raw_scripts(b:&[u8])->Vec<RawScript>{
    let base=u(b,4);
    (0..u(b,base)).map(|i|{
        let rec=base+2+6*i;
        let p=base+u(b,rec+4);
        let default=u(b,p);
        let languages=(0..u(b,p+2)).map(|j|{
            let q=p+4+6*j;
            (tag(b,q),raw_lang(b,p+u(b,q+4)))
        }).collect();
        RawScript{tag:tag(b,rec),default:(default!=0).then(||raw_lang(b,p+default)),languages}
    }).collect()
}
fn raw_features(b:&[u8])->Vec<RawFeature>{
    let base=u(b,6);
    (0..u(b,base)).map(|i|{
        let rec=base+2+6*i;
        let p=base+u(b,rec+4);
        RawFeature{tag:tag(b,rec),lookups:words(b,p+4,u(b,p+2))}
    }).collect()
}
// Dense masks indexed by FeatureList and LookupList, rather than the
// production planner's ordered sets/maps. All font-defined references
// are independently traversed; duplicate feature tags are not collapsed.
fn expected(s:Tag,language:Option<Tag>,lang:&RawLang,features:&[RawFeature],lookup_count:usize,
            enabled:&[Tag],variations:bool)->GsubPlan{
    let mut available=Vec::new();
    let mut chosen=vec![false;features.len()];
    for id in lang.required.into_iter().chain(lang.features.iter().copied()){
        let f=&features[id as usize];
        available.push(f.tag);
        if Some(id)==lang.required||enabled.contains(&f.tag){chosen[id as usize]=true;}
    }
    let selected:Vec<u16>=chosen.iter().enumerate().filter_map(|(i,&v)|v.then_some(i as u16)).collect();
    let mut provenance=vec![Vec::new();lookup_count];
    for &i in &selected{
        for &j in &features[i as usize].lookups{
            let v=&mut provenance[j as usize];
            if !v.contains(&i){v.push(i);}
        }
    }
    let lookups=provenance.into_iter().enumerate().filter_map(|(i,f)|{
        if f.is_empty(){None}else{Some(GsubPlannedLookup{index:i as u16,features:f})}
    }).collect();
    let mut unavailable_tags:Vec<_>=enabled.iter().copied().filter(|t|!available.contains(t)).collect();
    unavailable_tags.sort_unstable();unavailable_tags.dedup();
    GsubPlan{script:s,language,required_feature:lang.required,features:selected,
        unavailable_tags,lookups,base_only_variations_present:variations}
}
fn check(layout:&GsubLayout,script:Tag,language:Option<Tag>,enabled:&[Tag],want:GsubPlan){
    let actual=build_gsub_base_plan(layout,script,language,enabled,Default::default()).unwrap().unwrap();
    assert_eq!(actual,want,"script {script:?}, language {language:?}, enabled {enabled:?}");
}
fn verify(font:&[u8],counts:[usize;8]){
    let b=table(font,*b"GSUB").unwrap();
    let parsed=parse_font(font,FontLimits::default()).unwrap();
    assert_eq!(parsed.face(0).unwrap().table(*b"GSUB").unwrap(),b);
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    let scripts=raw_scripts(b);let features=raw_features(b);let lookup_count=u(b,u(b,8));
    let variations=u(b,2)==1&&d(b,10)!=0;
    let mut unique=features.iter().map(|f|f.tag).collect::<Vec<_>>();
    unique.sort_unstable();unique.dedup();
    let mut langs=0;
    let mut defaults=0;
    let mut required=0;
    let mut refs=0;
    let mut cases=0;
    let missing=*b"zzzz";
    assert!(!unique.contains(&missing));
    for script in &scripts{
        let systems=script.default.iter().map(|l|(None,l))
            .chain(script.languages.iter().map(|(t,l)|(Some(*t),l)));
        for (language,lang) in systems{
            langs+=1;defaults+=usize::from(language.is_none());
            required+=usize::from(lang.required.is_some());refs+=lang.features.len();
            let mut tags:Vec<_>=lang.required.into_iter().chain(lang.features.iter().copied())
                .map(|i|features[i as usize].tag).collect();
            tags.sort_unstable();tags.dedup();
            let mut mixed:Vec<_>=tags.iter().step_by(2).rev().copied().collect();
            mixed.extend([missing,missing]);
            if let Some(&t)=tags.first(){mixed.push(t);}
            let mut selections=vec![Vec::new(),tags.clone(),mixed,vec![missing]];
            selections.extend(tags.iter().map(|&t|vec![t]));
            for enabled in selections{
                let want=expected(script.tag,language,lang,&features,lookup_count,&enabled,variations);
                check(&layout,script.tag,language,&enabled,want);cases+=1;
            }
        }
        assert!(script.languages.iter().all(|(t,_)|*t!=missing));
        if let Some(lang)=&script.default{
            check(&layout,script.tag,Some(missing),&unique,
                expected(script.tag,None,lang,&features,lookup_count,&unique,variations));
            cases+=1;
        }else{
            assert!(build_gsub_base_plan(&layout,script.tag,Some(missing),&unique,Default::default()).unwrap().is_none());
        }
    }
    assert!(scripts.iter().all(|s|s.tag!=missing));
    if let Some(s)=scripts.iter().find(|s|s.tag==*b"DFLT"){
        if let Some(lang)=&s.default{
            check(&layout,missing,Some(missing),&unique,
                expected(s.tag,None,lang,&features,lookup_count,&unique,variations));
            cases+=1;
        }
    }else{
        assert!(build_gsub_base_plan(&layout,missing,None,&unique,Default::default()).unwrap().is_none());
    }
    assert_eq!([scripts.len(),langs,defaults,required,refs,features.len(),
        features.iter().map(|f|f.lookups.len()).sum(),features.len()-unique.len()],counts);
    assert_eq!(layout.lookups.len(),lookup_count);
    assert!(cases>=langs*4);
    assert!(!variations,"fixture inventory changed: variation body reference is separate");
}
#[test]fn abeezee_regular_plan_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),[2,3,2,0,13,7,8,0]);
}
#[test]fn abeezee_italic_plan_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),[2,3,2,0,13,7,8,0]);
}
#[test]fn alegreya_plan_reference(){
    verify(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),[3,17,3,0,344,344,497,321]);
}
#[test]fn aref_plan_reference(){
    verify(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),[3,7,3,0,97,25,55,7]);
}
#[test]fn padauk_plan_reference(){
    verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),[3,27,3,0,155,37,259,11]);
}
#[test]fn devanagari_plan_reference(){
    verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),[6,25,6,0,269,34,158,19]);
}
#[test]fn nastaliq_plan_reference(){
    verify(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),[5,21,5,0,170,20,63,11]);
}
#[test]fn abel_no_gsub_plan_source(){
    let b=include_bytes!("fixtures/abel/Abel-Regular.ttf");
    assert!(table(b,*b"GSUB").is_none());
    let f=parse_font(b,FontLimits::default()).unwrap();
    assert!(f.face(0).unwrap().table(*b"GSUB").is_none());
}
