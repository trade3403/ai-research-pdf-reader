use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
const SINGLE:&[u16]=&[1,6,1,1,1,10];
const MULTI:&[u16]=&[1,8,1,14,1,1,10,2,11,12];
const LIG:&[u16]=&[1,8,1,14,1,1,10,1,4,12,2,11];
const CTX:&[u16]=&[3,1,0,8,1,1,10];
const CHAIN:&[u16]=&[3,0,1,12,0,0,1,1,10];
const REVERSE:&[u16]=&[1,12,0,0,1,11,1,1,10];
fn wrap(kind:u16,target:&[u16])->Vec<u16>{
    let mut w=vec![1,kind,0,8];w.extend(target);w
}
// A GSUB with empty Script/Feature lists and an actual LookupList.
// Each lookup's subtables are emitted in order. Offsets are byte relative.
fn fixture(lookups:&[(u16,u16,Option<u16>,Vec<Vec<u16>>)]) -> Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,lookups.len() as u16];
    let list=7;
    w.resize(8+lookups.len(),0);
    for (i,(kind,flags,set,subs)) in lookups.iter().enumerate(){
        let at=w.len();w[list+1+i]=((at-list)*2) as u16;
        w.extend([*kind,*flags,subs.len() as u16]);
        w.resize(w.len()+subs.len(),0);
        if flags&16!=0{w.push(set.unwrap());}
        for (j,s) in subs.iter().enumerate(){
            w[at+3+j]=((w.len()-at)*2) as u16;w.extend(s);
        }
    }
    bytes(&w)
}
fn one(kind:u16,sub:&[u16])->Vec<u8>{fixture(&[(kind,0,None,vec![sub.to_vec()])])}
fn compile(b:&[u8],indices:&[u16])->Result<Vec<CompiledGsubLookup<'static>>,Error>{
    compile_gsub_lookups(b,indices,32,None,Default::default())
}
#[test]fn every_direct_kind_dispatch(){
    for (kind,w) in [(1,SINGLE),(2,MULTI),(3,MULTI),(4,LIG),(5,CTX),(6,CHAIN),(8,REVERSE)]{
        let b=one(kind,w);let x=compile(&b,&[0]).unwrap();let l=&x[0];
        assert_eq!(l.index(),0);assert_eq!(l.declared_kind(),kind);assert_eq!(l.effective_kind(),kind);
        let s=&l.subtables()[0];assert_eq!(s.source_offset(),s.target_offset());
        assert_eq!(s.target(),&parse_direct_gsub_target(kind,&bytes(w),32,1,Default::default()).unwrap());
    }
}
#[test]fn every_extension_target_kind_dispatch(){
    for (kind,w) in [(1,SINGLE),(2,MULTI),(3,MULTI),(4,LIG),(5,CTX),(6,CHAIN),(8,REVERSE)]{
        let b=one(7,&wrap(kind,w));let x=compile(&b,&[0]).unwrap();let l=&x[0];
        assert_eq!(l.declared_kind(),7);assert_eq!(l.effective_kind(),kind);
        let s=&l.subtables()[0];assert_eq!(s.target_offset(),s.source_offset()+8);
        assert_eq!(s.target(),&parse_direct_gsub_target(kind,&bytes(w),32,1,Default::default()).unwrap());
    }
}
#[test]fn direct_dispatch_short_and_invalid(){
    for n in 0..2{assert!(parse_direct_gsub_target(1,&[0;2][..n],32,1,Default::default()).is_err());}
    for k in [0,7,9]{assert!(parse_direct_gsub_target(k,&bytes(SINGLE),32,1,Default::default()).is_err());}
}
#[test]fn lookup_order_and_subtable_order(){
    let b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()]),
        (2,0,None,vec![MULTI.to_vec()]),(7,0,None,vec![wrap(8,REVERSE)])]);
    let x=compile(&b,&[0,2]).unwrap();
    assert_eq!(x.iter().map(|l|l.index()).collect::<Vec<_>>(),vec![0,2]);
    assert!(x[0].subtables()[0].source_offset()<x[0].subtables()[1].source_offset());
    assert_eq!(x[2-1].effective_kind(),8);
    for ids in [&[2,0][..],&[0,0],&[3]]{assert!(compile(&b,ids).is_err());}
    assert!(compile(&b,&[]).unwrap().is_empty());
}
#[test]fn mixed_extension_targets_and_empty_lookup_rejected(){
    let b=fixture(&[(7,0,None,vec![wrap(1,SINGLE),wrap(2,MULTI)])]);
    assert!(compile(&b,&[0]).is_err());
    let b=fixture(&[(1,0,None,vec![])]);
    assert!(compile(&b,&[0]).is_err());
}
#[test]fn cumulative_suffix_and_subtable_budgets(){
    let b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()]),(7,0,None,vec![wrap(1,SINGLE)])]);
    let x=compile(&b,&[0,1]).unwrap();
    let total=x.iter().flat_map(|l|l.subtables()).map(|s|b.len()-s.target_offset()+s.target_offset()-s.source_offset()).sum::<usize>();
    let l=GsubCompileLimits{max_lookups:2,max_subtables_per_lookup:2,max_total_subtables:3,max_total_target_bytes:total,..Default::default()};
    assert!(compile_gsub_lookups(&b,&[0,1],32,None,l).is_ok());
    for l in [GsubCompileLimits{max_lookups:1,..l},GsubCompileLimits{max_subtables_per_lookup:1,..l},
        GsubCompileLimits{max_total_subtables:2,..l},GsubCompileLimits{max_total_target_bytes:total-1,..l}]{
        assert!(compile_gsub_lookups(&b,&[0,1],32,None,l).is_err());
    }
}
#[test]fn alias_charged_twice(){
    let mut b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()])]);
    let layout=parse_gsub_layout(&b,Default::default()).unwrap();
    let at=layout.directory.lookups[0];
    let first=[b[at+6],b[at+7]];
    b[at+8..at+10].copy_from_slice(&first);
    let x=compile(&b,&[0]).unwrap();let subs=x[0].subtables();
    assert_eq!(subs[0],subs[1]);
    let cost=2*(b.len()-subs[0].target_offset());
    assert!(compile_gsub_lookups(&b,&[0],32,None,GsubCompileLimits{max_total_target_bytes:cost,..Default::default()}).is_ok());
    assert!(compile_gsub_lookups(&b,&[0],32,None,GsubCompileLimits{max_total_target_bytes:cost-1,..Default::default()}).is_err());
}
#[test]fn invalid_glyph_domain_even_empty_selection(){
    let b=one(1,SINGLE);
    for n in [0,65537]{assert!(compile_gsub_lookups(&b,&[],n,None,Default::default()).is_err());}
    let g=parse_gdef(&bytes(&[1,0,0,0,0,0]),31,Default::default()).unwrap();
    assert!(compile_gsub_lookups(&b,&[],32,Some(&g),Default::default()).is_err());
}
#[test]fn lookup_flags_validated_before_compilation(){
    let b=fixture(&[(1,16,Some(0),vec![SINGLE.to_vec()])]);
    assert!(compile(&b,&[0]).is_err());
    let g=parse_gdef(&bytes(&[1,2,0,0,0,0,14,1,1,0,8,1,1,10]),32,Default::default()).unwrap();
    let x=compile_gsub_lookups(&b,&[0],32,Some(&g),Default::default()).unwrap();
    assert_eq!(x[0].filter().mark_filtering_set(),Some(0));
}
#[test]fn malformed_target_and_nested_reference_rejected(){
    assert!(compile(&one(1,&[1,6,30,1,1,10]),&[0]).is_err());
    assert!(compile(&one(5,&[3,1,1,12,0,1,1,1,10]),&[0]).is_err());
    // Lookup 1 exists but is not selected: retain its index, no recursion.
    let b=fixture(&[(5,0,None,vec![vec![3,1,1,12,0,1,1,1,10]]),(1,0,None,vec![SINGLE.to_vec()])]);
    let x=compile(&b,&[0]).unwrap();
    let ExtensionTarget::CoverageContext(c)=x[0].subtables()[0].target()else{panic!()};
    assert_eq!(c.actions()[0].lookup_index,1);assert_eq!(x.len(),1);
}
#[test]fn per_target_and_layout_limits(){
    let b=one(1,SINGLE);
    let l=GsubCompileLimits{target:ExtensionLimits{single:SingleSubstLimits{max_records:0,..Default::default()},..Default::default()},..Default::default()};
    assert!(compile_gsub_lookups(&b,&[0],32,None,l).is_err());
    assert!(compile_gsub_lookups(&b,&[0],32,None,GsubCompileLimits{layout:GsubLimits{max_bytes:b.len()-1,..Default::default()},..Default::default()}).is_err());
}
#[test]fn real_base_plan_compilation_and_provenance_validation(){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),FontLimits::default()).unwrap();
    let face=f.face(0).unwrap();
    let b=face.table(*b"GSUB").unwrap();
    let g=parse_gdef(face.table(*b"GDEF").unwrap(),268,Default::default()).unwrap();
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    let s=layout.scripts.iter().find(|s|s.default_language.as_ref().is_some_and(|l|!l.features.is_empty())).unwrap();
    let tags:Vec<_>=s.default_language.as_ref().unwrap().features.iter().map(|&i|layout.features[i as usize].tag).collect();
    let p=build_gsub_base_plan(&layout,s.tag,None,&tags,Default::default()).unwrap().unwrap();
    let x=compile_gsub_plan(b,&p,268,Some(&g),Default::default()).unwrap();
    assert!(!x.is_empty());
    assert_eq!(x.iter().map(|l|l.index()).collect::<Vec<_>>(),p.lookups.iter().map(|l|l.index).collect::<Vec<_>>());
    let mut bad=p.clone();bad.lookups[0].features.clear();
    assert!(compile_gsub_plan(b,&bad,268,Some(&g),Default::default()).is_err());
    let mut bad=p.clone();bad.script=*b"zzzz";
    assert!(compile_gsub_plan(b,&bad,268,Some(&g),Default::default()).is_err());
    assert!(compile_gsub_plan(b,&p,268,Some(&g),GsubCompileLimits{max_plan_refs:0,..Default::default()}).is_err());
}
