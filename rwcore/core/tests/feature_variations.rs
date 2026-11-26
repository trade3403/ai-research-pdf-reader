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
