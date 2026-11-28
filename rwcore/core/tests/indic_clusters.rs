use rwcore::*;
use IndicClusterKind as K;
fn run(t:&str)->IndicClusterRun{
    analyze_devanagari_clusters(t,&bundled_indic_properties().unwrap(),&bundled_script_properties().unwrap(),Default::default()).unwrap()
}
fn parts<'a>(t:&'a str,r:&IndicClusterRun)->Vec<&'a str>{r.clusters.iter().map(|c|&t[c.byte_start..c.byte_end]).collect()}
#[test]fn devanagari_conjunct_and_matra(){
    let t="क्षिक";let r=run(t);assert_eq!(parts(t,&r),vec!["क्षि","क"]);
    let c=r.clusters[0];assert_eq!(c.kind,K::Consonant);assert_eq!(c.consonants,2);
    assert_eq!(c.base_candidate,Some(2));assert!(!c.terminal_virama);
}
#[test]fn devanagari_nukta_and_terminal_halant(){
    let t="क़्त्क्";let r=run(t);assert_eq!(parts(t,&r),vec![t]);
    let c=r.clusters[0];assert_eq!(c.consonants,3);assert_eq!(c.base_candidate,Some(5));assert!(c.terminal_virama);
}
#[test]fn devanagari_join_controls(){
    let t="क्\u{200d}षक्\u{200c}ष";let r=run(t);
    assert_eq!(parts(t,&r),vec!["क्\u{200d}ष","क्\u{200c}","ष"]);
    assert_eq!(r.clusters[0].consonants,2);assert!(r.clusters[1].ends_with_non_joiner&&r.clusters[1].terminal_virama);
    let r=run("क्\u{200d}");assert!(r.clusters[0].terminal_virama);
}
#[test]fn devanagari_vowel_and_placeholder(){
    let t="अं◌िक";let r=run(t);assert_eq!(parts(t,&r),vec!["अं","◌ि","क"]);
    assert_eq!(r.clusters.iter().map(|c|c.kind).collect::<Vec<_>>(),vec![K::Vowel,K::Placeholder,K::Consonant]);
    assert_eq!(r.clusters[1].base_candidate,Some(2));
}
#[test]fn devanagari_broken_marks_no_fabricated_base(){
    let t="ि्\u{200d}काि";let r=run(t);assert_eq!(parts(t,&r),vec!["ि्\u{200d}","का","ि"]);
    assert_eq!(r.clusters[0].kind,K::Broken);assert_eq!(r.clusters[2].kind,K::Broken);
    assert_eq!(r.clusters[0].base_candidate,None);assert_eq!(r.clusters[2].base_candidate,None);
}
#[test]fn devanagari_foreign_boundaries(){
    let t="क्বাংলাA😀क";let r=run(t);
    assert_eq!(parts(t,&r).concat(),t);
    assert_eq!(r.clusters[0].kind,K::Consonant);assert!(r.clusters[0].terminal_virama);
    assert!(r.clusters[1..r.clusters.len()-1].iter().all(|c|c.kind==K::Other));
}
#[test]fn devanagari_offsets_and_partition(){
    let t="Aक्षि अं क्\u{200c}😀";let r=run(t);
    let mut next=0;let mut byte=0;
    for c in &r.clusters{
        assert_eq!((c.scalar_start,c.byte_start),(next,byte));
        assert_eq!(t[c.byte_start..c.byte_end].chars().count(),c.scalar_end-c.scalar_start);
        if let Some(i)=c.base_candidate{assert!((c.scalar_start..c.scalar_end).contains(&i));}
        next=c.scalar_end;byte=c.byte_end;
    }
    assert_eq!((next,byte),(t.chars().count(),t.len()));
    assert_eq!(r.scalars.iter().map(|s|s.character).collect::<String>(),t);
}
#[test]fn devanagari_cluster_limits_and_empty(){
    let p=bundled_indic_properties().unwrap();let s=bundled_script_properties().unwrap();
    let t="क्षक";
    for l in [IndicClusterLimits{max_clusters:1,..Default::default()},
        IndicClusterLimits{max_cluster_scalars:2,..Default::default()},IndicClusterLimits{max_work:15,..Default::default()},
        IndicClusterLimits{source:SourceLimits{max_scalars:3,..Default::default()},..Default::default()}]{
        assert!(matches!(analyze_devanagari_clusters(t,&p,&s,l),Err(Error::Limit(_))));
    }
    assert!(analyze_devanagari_clusters(t,&p,&s,IndicClusterLimits{max_work:16,max_clusters:2,max_cluster_scalars:3,..Default::default()}).is_ok());
    assert!(analyze_devanagari_clusters("",&p,&s,IndicClusterLimits{source:SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0},max_clusters:0,max_cluster_scalars:0,max_work:0}).unwrap().clusters.is_empty());
}
