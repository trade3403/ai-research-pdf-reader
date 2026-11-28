use rwcore::*;
use IndicReorderStatus as K;
use IndicRephCandidate as R;
fn plan(t:&str)->IndicReorderPlan{
    plan_devanagari_reorder(t,&bundled_indic_properties().unwrap(),&bundled_script_properties().unwrap(),Default::default()).unwrap()
}
#[test]fn indic_simple_left_matra_plan(){
    let p=plan("कि");assert_eq!(p.candidate_order,vec![1,0]);assert_eq!(p.candidate_inverse,vec![1,0]);
    let c=&p.clusters[0];assert_eq!(c.status,K::SimplePrebaseCandidate);assert_eq!(c.base_candidate,Some(0));assert_eq!(c.prebase_matra,Some(1));
    assert_eq!(p.logical.scalars.iter().map(|s|s.character).collect::<String>(),"कि");
}
#[test]fn indic_prebase_with_nukta_and_tail(){
    let p=plan("क़िं");assert_eq!(p.candidate_order,vec![2,0,1,3]);
    assert_eq!(p.clusters[0].prebase_matra,Some(2));
    assert_eq!(p.clusters[0].status,K::SimplePrebaseCandidate);
}
#[test]fn indic_reph_candidate_deferred(){
    let p=plan("र्कि");let c=&p.clusters[0];
    assert_eq!(c.reph,R::Prefix{ra:0,virama:1});assert_eq!(c.base_candidate,Some(2));
    assert_eq!(c.prebase_matra,Some(3));assert_eq!(c.status,K::NeedsFont);
    assert_eq!(p.candidate_order,vec![0,1,2,3]);
}
#[test]fn indic_explicit_join_controls_deferred(){
    for t in ["र्\u{200d}क","र्\u{200c}क"]{
        let p=plan(t);assert_eq!(p.clusters[0].reph,R::JoinControl);
        assert_eq!(p.clusters[0].status,K::NeedsFont);
        assert_eq!(p.candidate_order,(0..t.chars().count()).collect::<Vec<_>>());
    }
}
#[test]fn indic_conjunct_and_terminal_virama_deferred(){
    let p=plan("क्षिक्");assert_eq!(p.clusters.len(),2);
    assert_eq!(p.clusters[0].status,K::NeedsFont);assert_eq!(p.clusters[0].base_candidate,Some(2));
    assert_eq!(p.clusters[0].prebase_matra,Some(3));assert_eq!(p.clusters[1].status,K::NeedsFont);
    assert_eq!(p.candidate_order,vec![0,1,2,3,4,5]);
}
#[test]fn indic_identity_broken_placeholder_and_other(){
    let p=plan("िअंकूA◌ि");assert_eq!(p.clusters.iter().map(|c|c.status).collect::<Vec<_>>(),
        vec![K::Broken,K::Identity,K::Identity,K::Other,K::SimplePrebaseCandidate]);
    assert_eq!(p.clusters[0].base_candidate,None);assert_eq!(p.candidate_order,vec![0,1,2,3,4,5,7,6]);
}
#[test]fn indic_candidate_permutation_and_utf8_partition(){
    let t="Aक़िं र्कि कि😀";let p=plan(t);
    for (at,&i) in p.candidate_order.iter().enumerate(){assert_eq!(p.candidate_inverse[i],at);}
    for c in &p.clusters{let mut ids=p.candidate_order[c.scalar_start..c.scalar_end].to_vec();ids.sort_unstable();
        assert_eq!(ids,(c.scalar_start..c.scalar_end).collect::<Vec<_>>());}
    for (s,(at,ch)) in p.logical.scalars.iter().zip(t.char_indices()){
        assert_eq!((s.character,s.byte_start,s.byte_end),(ch,at,at+ch.len_utf8()));
    }
}
#[test]fn indic_reorder_limits_and_empty(){
    let p=bundled_indic_properties().unwrap();let s=bundled_script_properties().unwrap();
    for l in [IndicReorderLimits{max_scalars:1,..Default::default()},IndicReorderLimits{max_work:7,..Default::default()},
        IndicReorderLimits{clusters:IndicClusterLimits{max_clusters:0,..Default::default()},..Default::default()}]{
        assert!(matches!(plan_devanagari_reorder("कि",&p,&s,l),Err(Error::Limit(_))));
    }
    assert!(plan_devanagari_reorder("कि",&p,&s,IndicReorderLimits{max_scalars:2,max_work:8,..Default::default()}).is_ok());
    let r=plan_devanagari_reorder("",&p,&s,IndicReorderLimits{max_scalars:0,max_work:0,..Default::default()}).unwrap();
    assert!(r.clusters.is_empty()&&r.candidate_order.is_empty()&&r.candidate_inverse.is_empty());
}
