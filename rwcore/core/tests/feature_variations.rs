use rwcore::*;

fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
fn d(b:&mut Vec<u8>,v:u32){b.extend(v.to_be_bytes());}
fn set32(b:&mut [u8],p:usize,v:usize){b[p..p+4].copy_from_slice(&(v as u32).to_be_bytes());}
fn set16(b:&mut [u8],p:usize,v:u16){b[p..p+2].copy_from_slice(&v.to_be_bytes());}
fn get32(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
type Record<'a>=(Option<&'a [(u16,i16,i16)]>,Option<&'a [(u16,&'a [u16])]>);
fn fixture(spec:&[Record<'_>])->Vec<u8>{
    let mut b=Vec::new();w(&mut b,1);w(&mut b,0);d(&mut b,spec.len() as u32);
    b.resize(8+8*spec.len(),0);
    for (i,(conditions,substitutions)) in spec.iter().enumerate(){
        if let Some(c)=conditions{
            let at=b.len();set32(&mut b,8+8*i,at);w(&mut b,c.len() as u16);
            b.resize(at+2+4*c.len(),0);
            for (j,&(axis,min,max)) in c.iter().enumerate(){
                let rel=b.len()-at;set32(&mut b,at+2+4*j,rel);
                w(&mut b,1);w(&mut b,axis);w(&mut b,min as u16);w(&mut b,max as u16);
            }
        }
        if let Some(s)=substitutions{
            let at=b.len();set32(&mut b,12+8*i,at);w(&mut b,1);w(&mut b,0);w(&mut b,s.len() as u16);
            b.resize(at+6+6*s.len(),0);
            for (j,&(index,lookups)) in s.iter().enumerate(){
                let rel=b.len()-at;set16(&mut b,at+6+6*j,index);set32(&mut b,at+8+6*j,rel);
                w(&mut b,0);w(&mut b,lookups.len() as u16);
                for &id in lookups{w(&mut b,id);}
            }
        }
    }
    b
}
fn parse(b:&[u8])->Result<FeatureVariations,Error>{parse_feature_variations(b,2,3,5,Default::default())}
#[test]fn inclusive_ranges_and_first_matching_record(){
    let b=fixture(&[(Some(&[(0,-8192,8192),(1,0,16384)]),Some(&[(1,&[4,2])])),
        (Some(&[(0,0,16384)]),Some(&[(1,&[3])])),(None,Some(&[(0,&[0])]))]);
    let p=parse(&b).unwrap();assert_eq!(p.axis_count(),2);
    for input in [[-8192,0],[8192,16384],[0,1]]{
        let(i,r)=p.select(&input).unwrap().unwrap();assert_eq!(i,0);
        assert_eq!(r.alternate(1).unwrap().lookups,vec![4,2]);assert!(r.alternate(0).is_none());
    }
    assert_eq!(p.select(&[8193,0]).unwrap().unwrap().0,1);
    assert_eq!(p.select(&[-8193,0]).unwrap().unwrap().0,2);
    assert_eq!(p.select(&[-1,-1]).unwrap().unwrap().0,2);
}
#[test]fn null_and_empty_condition_sets_are_universal(){
    for c in [None,Some(&[][..])]{
        let b=fixture(&[(c,None),(None,Some(&[(0,&[1])]))]);
        let p=parse(&b).unwrap();let(i,r)=p.select(&[0,0]).unwrap().unwrap();
        assert_eq!(i,0);assert_eq!(r.substitutions().unwrap().len(),0);
    }
}
#[test]fn no_match_and_empty_variations(){
    let p=parse(&fixture(&[(Some(&[(0,1,2)]),Some(&[(0,&[1])]))])).unwrap();
    assert!(p.select(&[0,0]).unwrap().is_none());
    assert!(parse(&fixture(&[])).unwrap().select(&[0,0]).unwrap().is_none());
}
#[test]fn unsupported_condition_and_invalid_axis_skip_record(){
    let mut b=fixture(&[(Some(&[(0,-1,1)]),Some(&[(0,&[1])])),
        (Some(&[(2,-1,1)]),Some(&[(0,&[2])])),(None,Some(&[(0,&[3])]))]);
    let c=get32(&b,8);let at=c+get32(&b,c+2);set16(&mut b,at,99);
    let p=parse(&b).unwrap();assert_eq!(p.records()[0].conditions(),&[FeatureVariationCondition::Unsupported(99)]);
    assert_eq!(p.select(&[0,0]).unwrap().unwrap().0,2);
}
#[test]fn unsupported_substitution_version_skips_record(){
    let mut b=fixture(&[(None,Some(&[(0,&[1])])),(None,Some(&[(0,&[2])]))]);
    let p=get32(&b,12);set16(&mut b,p,2);
    let p=parse(&b).unwrap();assert!(p.records()[0].substitutions().is_none());
    assert_eq!(p.select(&[0,0]).unwrap().unwrap().0,1);
}
#[test]fn bad_coordinates_and_intervals(){
    let p=parse(&fixture(&[(None,None)])).unwrap();
    for coords in [&[][..],&[0][..],&[0,0,0][..],&[-16385,0][..],&[16385,0][..]]{
        assert!(matches!(p.select(coords),Err(Error::Invalid(_))));
    }
    for condition in [(0,1,0),(0,-16385,0),(0,0,16385)]{
        assert!(matches!(parse(&fixture(&[(Some(&[condition]),None)])),Err(Error::Invalid(_))));
    }
}
#[test]fn feature_indices_order_and_lookup_domain(){
    for subs in [&[(3,&[0][..])][..],&[(1,&[0][..]),(1,&[2][..])][..],
        &[(2,&[0][..]),(1,&[2][..])][..],&[(0,&[5][..])][..]]{
        assert!(matches!(parse(&fixture(&[(None,Some(subs))])),Err(Error::Invalid(_))));
    }
    let p=parse(&fixture(&[(None,Some(&[(0,&[]),(2,&[4,1,4])]))])).unwrap();
    let r=p.select(&[0,0]).unwrap().unwrap().1;
    assert!(r.alternate(0).unwrap().lookups.is_empty());
    assert_eq!(r.alternate(2).unwrap().lookups,vec![4,1,4]);assert!(r.alternate(1).is_none());
}
#[test]fn every_truncated_prefix_and_bad_offsets(){
    let b=fixture(&[(Some(&[(0,-1,1)]),Some(&[(0,&[1,2])]))]);
    for len in 0..b.len(){assert!(parse(&b[..len]).is_err(),"prefix {len}");}
    let c=get32(&b,8);let s=get32(&b,12);
    for(p,v)in [(8,4),(12,8),(c+2,2),(s+8,6),(8,u32::MAX as usize)]{
        let mut x=b.clone();set32(&mut x,p,v);assert!(parse(&x).is_err());
    }
}
#[test]fn alternate_parameter_offset_is_opaque_and_checked(){
    let mut b=fixture(&[(None,Some(&[(1,&[2])]))]);
    let s=get32(&b,12);let at=s+get32(&b,s+8);
    set16(&mut b,at,6);b.push(42);
    assert_eq!(parse(&b).unwrap().records()[0].alternate(1).unwrap().parameters,Some(at+6));
    set16(&mut b,at,2);assert!(parse(&b).is_err());
}
#[test]fn exact_cumulative_budgets_charge_aliases(){
    let mut b=fixture(&[(Some(&[(0,0,1)]),Some(&[(0,&[1,2])])),(None,None)]);
    let c=get32(&b,8);let s=get32(&b,12);set32(&mut b,16,c);set32(&mut b,20,s);
    let l=FeatureVariationLimits{max_bytes:b.len(),max_axes:2,max_records:2,
        max_conditions:2,max_substitutions:2,max_lookup_refs:4};
    assert!(parse_feature_variations(&b,2,3,5,l).is_ok());
    for bad in [FeatureVariationLimits{max_bytes:b.len()-1,..l},FeatureVariationLimits{max_axes:1,..l},
        FeatureVariationLimits{max_records:1,..l},FeatureVariationLimits{max_conditions:1,..l},
        FeatureVariationLimits{max_substitutions:1,..l},FeatureVariationLimits{max_lookup_refs:3,..l}]{
        assert!(matches!(parse_feature_variations(&b,2,3,5,bad),Err(Error::Limit(_))));
    }
}
#[test]fn gsub_directory_entry_and_absent_variations(){
    // GSUB 1.1, three empty directories, body begins at byte 20.
    let mut b=Vec::new();for v in [1,1,14,16,18]{w(&mut b,v);}d(&mut b,20);
    for _ in 0..3{w(&mut b,0);}b.extend(fixture(&[(None,None)]));
    let p=parse_gsub_feature_variations(&b,0,Default::default(),Default::default()).unwrap().unwrap();
    assert_eq!(p.select(&[]).unwrap().unwrap().0,0);
    set32(&mut b,10,0);
    assert!(parse_gsub_feature_variations(&b,0,Default::default(),Default::default()).unwrap().is_none());
}
#[test]fn exhaustive_small_coordinate_reference(){
    let b=fixture(&[(Some(&[(0,-2,1),(1,0,2)]),Some(&[(0,&[1])])),
        (Some(&[(0,0,2),(1,-2,1)]),Some(&[(0,&[2])])),(None,None)]);
    let p=parse(&b).unwrap();
    for x in -3i16..=3{for y in -3i16..=3{
        let expected=if (-2..=1).contains(&x)&&(0..=2).contains(&y){0}
            else if (0..=2).contains(&x)&&(-2..=1).contains(&y){1}else{2};
        assert_eq!(p.select(&[x,y]).unwrap().unwrap().0,expected);
    }}
}


// Real GSUB 1.1 structure with a synthetic FeatureVariations body.
// DFLT/default has required ccmp(0), optional liga(1), salt(2).
// Lookup 3 is a contextual root whose only action targets lookup 4.
fn dynamic_gsub(body:Option<&[u8]>)->Vec<u8>{
    let mut b=Vec::new();
    for v in [1,1,0,0,0]{w(&mut b,v);}d(&mut b,0);
    let scripts=b.len();set16(&mut b,4,scripts as u16);
    w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0,2,1,2]{w(&mut b,v);}
    let features=b.len();set16(&mut b,6,features as u16);
    w(&mut b,3);
    for (tag,rel) in [(b"ccmp",20),(b"liga",26),(b"salt",32)]{b.extend(tag);w(&mut b,rel);}
    for id in 0..3{for v in [0,1,id]{w(&mut b,v);}}
    let lookups=b.len();set16(&mut b,8,lookups as u16);
    w(&mut b,5);b.resize(lookups+12,0);
    for (id,kind,sub) in [
        (0,1,vec![2,8,1,11,1,1,10]),
        (1,1,vec![2,8,1,12,1,1,11]),
        (2,1,vec![2,8,1,19,1,1,10]),
        (3,5,vec![3,1,1,12,0,4,1,1,10]),
        (4,1,vec![2,8,1,20,1,1,10]),
    ]{
        let at=b.len();set16(&mut b,lookups+2+2*id,(at-lookups) as u16);
        for v in [kind,0,1,8]{w(&mut b,v);}for v in sub{w(&mut b,v);}
    }
    if let Some(body)=body{let at=b.len();set32(&mut b,10,at);b.extend_from_slice(body);}
    b
}
fn dynamic_plan(b:&[u8],tags:&[[u8;4]],coords:&[i16])->GsubVariationPlan{
    build_gsub_variation_plan(b,*b"DFLT",None,tags,coords.len(),coords,
        Default::default(),Default::default(),Default::default()).unwrap().unwrap()
}
fn dynamic_program<'a>(b:&[u8],p:&GsubVariationPlan,coords:&[i16])->Result<CompiledGsubProgram<'a>,Error>{
    compile_gsub_variation_plan_program(b,p,coords.len(),coords,32,None,Default::default(),Default::default())
}
fn dynamic_roots(p:&GsubVariationPlan)->Vec<u16>{p.plan.lookups.iter().map(|l|l.index).collect()}
fn dynamic_execute(b:&[u8],p:&GsubVariationPlan,coords:&[i16])->Vec<u16>{
    let program=dynamic_program(b,p,coords).unwrap();
    execute_gsub_program(&program,&[10],&[],Default::default()).unwrap().expansion.execution.glyphs
}
#[test]fn variation_required_feature_replacement_and_dependency_execution(){
    let body=fixture(&[(Some(&[(0,0,16384)]),Some(&[(0,&[3]),(1,&[3,1,3]),(2,&[4])]))]);
    let b=dynamic_gsub(Some(&body));let p=dynamic_plan(&b,&[*b"liga"],&[0]);
    assert_eq!(p.plan.features,vec![0,1]);assert_eq!(p.replaced_features,vec![0,1]);
    assert_eq!(p.matched_record,Some(0));assert!(p.variations_present);
    assert!(!p.plan.base_only_variations_present);assert_eq!(dynamic_roots(&p),vec![1,3]);
    assert_eq!(p.plan.lookups[1].features,vec![0,1]);
    let program=dynamic_program(&b,&p,&[0]).unwrap();
    assert_eq!(program.roots(),&[1,3]);assert_eq!(program.closure().reachable,vec![1,3,4]);
    assert_eq!(program.dependencies().len(),1);
    assert_eq!(dynamic_execute(&b,&p,&[0]),vec![20]);
    assert!(matches!(compile_gsub_plan_program(&b,&p.plan,32,None,Default::default()),Err(Error::Invalid(_))));
}
#[test]fn variation_no_match_uses_base_and_inclusive_boundary(){
    let b=dynamic_gsub(Some(&fixture(&[(Some(&[(0,1,2)]),Some(&[(0,&[3])]))])));
    for v in [0,3,-1]{
        let p=dynamic_plan(&b,&[*b"liga"],&[v]);
        assert_eq!(p.matched_record,None);assert!(p.replaced_features.is_empty());
        assert_eq!(dynamic_roots(&p),vec![0,1]);assert_eq!(dynamic_execute(&b,&p,&[v]),vec![12]);
    }
    for v in [1,2]{
        let p=dynamic_plan(&b,&[*b"liga"],&[v]);
        assert_eq!(dynamic_roots(&p),vec![1,3]);assert_eq!(dynamic_execute(&b,&p,&[v]),vec![20]);
    }
}
#[test]fn variation_empty_replacement_and_first_match_no_merge(){
    let b=dynamic_gsub(Some(&fixture(&[(None,Some(&[(0,&[])])),(None,Some(&[(1,&[3])]))])));
    let p=dynamic_plan(&b,&[*b"liga"],&[0]);
    assert_eq!(p.matched_record,Some(0));assert_eq!(p.replaced_features,vec![0]);
    assert_eq!(p.plan.required_feature,Some(0));assert_eq!(p.plan.features,vec![0,1]);
    assert_eq!(dynamic_roots(&p),vec![1]);assert_eq!(dynamic_execute(&b,&p,&[0]),vec![10]);
}
#[test]fn variation_null_substitution_stops_and_retains_base(){
    let b=dynamic_gsub(Some(&fixture(&[(None,None),(None,Some(&[(0,&[3])]))])));
    let p=dynamic_plan(&b,&[*b"liga"],&[0]);
    assert_eq!(p.matched_record,Some(0));assert!(p.replaced_features.is_empty());
    assert_eq!(dynamic_roots(&p),vec![0,1]);assert_eq!(dynamic_execute(&b,&p,&[0]),vec![12]);
}
#[test]fn variation_no_table_legacy_plan_parity(){
    let b=dynamic_gsub(None);let p=dynamic_plan(&b,&[*b"liga"],&[0]);
    let layout=parse_gsub_layout(&b,Default::default()).unwrap();
    let base=build_gsub_base_plan(&layout,*b"DFLT",None,&[*b"liga"],Default::default()).unwrap().unwrap();
    assert_eq!(p.plan,base);assert!(!p.variations_present);
    assert_eq!(dynamic_execute(&b,&p,&[0]),vec![12]);
    assert!(compile_gsub_plan_program(&b,&base,32,None,Default::default()).is_ok());
}
#[test]fn variation_fallback_and_unselected_features(){
    let b=dynamic_gsub(Some(&fixture(&[(None,Some(&[(2,&[3])]))])));
    let p=build_gsub_variation_plan(&b,*b"latn",Some(*b"ENG "),&[*b"liga",*b"zzzz"],1,&[0],
        Default::default(),Default::default(),Default::default()).unwrap().unwrap();
    assert_eq!(p.plan.script,*b"DFLT");assert_eq!(p.plan.language,None);
    assert_eq!(p.plan.unavailable_tags,vec![*b"zzzz"]);assert!(p.replaced_features.is_empty());
    assert_eq!(dynamic_execute(&b,&p,&[0]),vec![12]);
}
#[test]fn variation_rejects_stale_coordinates_and_forged_provenance(){
    let b=dynamic_gsub(Some(&fixture(&[(Some(&[(0,0,1)]),Some(&[(0,&[3])]))])));
    let p=dynamic_plan(&b,&[*b"liga"],&[0]);
    assert!(matches!(dynamic_program(&b,&p,&[1]),Err(Error::Invalid(_))));
    let mut bad=p.clone();bad.matched_record=None;
    assert!(matches!(dynamic_program(&b,&bad,&[0]),Err(Error::Invalid(_))));
    let mut bad=p.clone();bad.replaced_features.clear();
    assert!(matches!(dynamic_program(&b,&bad,&[0]),Err(Error::Invalid(_))));
    let mut bad=p.clone();bad.plan.lookups[1].features.clear();
    assert!(matches!(dynamic_program(&b,&bad,&[0]),Err(Error::Invalid(_))));
    let mut bad=p.clone();bad.plan.lookups[1].index=4;
    assert!(matches!(dynamic_program(&b,&bad,&[0]),Err(Error::Invalid(_))));
    let changed=dynamic_gsub(Some(&fixture(&[(Some(&[(0,0,1)]),Some(&[(0,&[2])]))])));
    assert!(matches!(dynamic_program(&changed,&p,&[0]),Err(Error::Invalid(_))));
}
#[test]fn variation_plan_and_compile_limits_use_effective_refs(){
    let b=dynamic_gsub(Some(&fixture(&[(None,Some(&[(0,&[3,3,3])]))])));
    let l=GsubPlanLimits{max_lookup_refs:4,max_lookups:2,..Default::default()};
    let p=build_gsub_variation_plan(&b,*b"DFLT",None,&[*b"liga"],1,&[0],
        Default::default(),Default::default(),l).unwrap().unwrap();
    for bad in [GsubPlanLimits{max_lookup_refs:3,..l},GsubPlanLimits{max_lookups:1,..l}]{
        assert!(matches!(build_gsub_variation_plan(&b,*b"DFLT",None,&[*b"liga"],1,&[0],
            Default::default(),Default::default(),bad),Err(Error::Limit(_))));
    }
    let mut cap=GsubProgramLimits::default();cap.compile.max_plan_refs=10;
    assert!(compile_gsub_variation_plan_program(&b,&p,1,&[0],32,None,cap,Default::default()).is_ok());
    cap.compile.max_plan_refs=9;
    assert!(matches!(compile_gsub_variation_plan_program(&b,&p,1,&[0],32,None,cap,Default::default()),Err(Error::Limit(_))));
}
#[test]fn variation_coordinates_validated_even_without_table(){
    let b=dynamic_gsub(None);
    for (n,coords) in [(1,&[][..]),(1,&[16385][..]),(2,&[0][..])]{
        assert!(matches!(build_gsub_variation_plan(&b,*b"DFLT",None,&[],n,coords,
            Default::default(),Default::default(),Default::default()),Err(Error::Invalid(_))));
    }
    let l=FeatureVariationLimits{max_axes:0,..Default::default()};
    assert!(matches!(build_gsub_variation_plan(&b,*b"DFLT",None,&[],1,&[0],Default::default(),l,Default::default()),Err(Error::Limit(_))));
}
#[test]fn variation_exhaustive_small_plan_to_execution_reference(){
    let b=dynamic_gsub(Some(&fixture(&[(Some(&[(0,-2,1),(1,0,2)]),Some(&[(0,&[3])])),
        (Some(&[(0,0,2),(1,-2,1)]),Some(&[(0,&[])]))])));
    for x in -3i16..=3{for y in -3i16..=3{
        let p=dynamic_plan(&b,&[*b"liga"],&[x,y]);
        let (record,roots,out)=if (-2..=1).contains(&x)&&(0..=2).contains(&y){(Some(0),vec![1,3],20)}
            else if (0..=2).contains(&x)&&(-2..=1).contains(&y){(Some(1),vec![1],10)}
            else{(None,vec![0,1],12)};
        assert_eq!(p.matched_record,record);assert_eq!(dynamic_roots(&p),roots);
        assert_eq!(dynamic_execute(&b,&p,&[x,y]),vec![out]);
    }}
}
