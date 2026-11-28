use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
fn ledger(text:&str,l:ProvenanceLimits)->ProvenanceLedger{
    let c=parse_font(FONT,Default::default()).unwrap();
    let cmap=parse_unicode_cmap(c.face(0).unwrap(),Default::default()).unwrap();
    let run=map_source_run(text,&cmap,true,Default::default()).unwrap();
    ProvenanceLedger::from_run(&run,u32::from(cmap.glyph_count()),l).unwrap()
}
fn sets(l:&ProvenanceLedger)->Vec<Vec<usize>>{
    l.glyphs().iter().map(|g|g.sources().to_vec()).collect()
}
#[test]fn seed_empty_and_unicode(){
    let l=ledger("",Default::default());assert!(l.glyphs().is_empty());
    let l=ledger("Aé😀",Default::default());
    assert_eq!(sets(&l),vec![vec![0],vec![1],vec![2]]);
    assert_eq!(l.scalar_count(),3);assert_eq!(l.reference_count(),3);
}
#[test]fn expand_and_single(){
    let mut l=ledger("ABC",Default::default());l.expand(1,&[1,2,3]).unwrap();
    assert_eq!(sets(&l),vec![vec![0],vec![1],vec![1],vec![1],vec![2]]);
    l.expand(0,&[2]).unwrap();assert_eq!(l.glyphs()[0].glyph_id,2);
    assert_eq!(l.reference_count(),5);
}
#[test]fn contract_preserves_gaps(){
    let mut l=ledger("ABCDE",Default::default());l.contract(&[0,2,4],1).unwrap();
    assert_eq!(sets(&l),vec![vec![0,2,4],vec![1],vec![3]]);
    assert_eq!(l.reference_count(),5);
}
#[test]fn union_deduplicates_expansions(){
    let mut l=ledger("ABC",Default::default());l.expand(1,&[1,2,3]).unwrap();
    l.contract(&[0,1,3,4],1).unwrap();
    assert_eq!(sets(&l),vec![vec![0,1,2],vec![1]]);
    assert_eq!(l.reference_count(),4);
    l.expand(0,&[1,2]).unwrap();assert_eq!(l.reference_count(),7);
}
#[test]fn invalid_edits_are_atomic(){
    let mut l=ledger("ABC",Default::default());let before=l.clone();
    for p in [&[][..],&[0][..],&[1,1][..],&[2,0][..],&[0,3][..]]{
        assert!(l.contract(p,1).is_err());assert_eq!(l,before);
    }
    for (p,ids) in [(3,&[1][..]),(0,&[][..]),(0,&[65535][..])]{assert!(l.expand(p,ids).is_err());assert_eq!(l,before);}
}
#[test]fn glyph_and_reference_caps(){
    let mut l=ledger("AB",ProvenanceLimits{max_glyphs:3,max_refs:3,..Default::default()});
    l.expand(0,&[1,2]).unwrap();let before=l.clone();
    assert!(l.expand(0,&[1,2]).is_err());assert_eq!(l,before);
}
#[test]fn source_set_cap_atomic(){
    let mut l=ledger("ABC",ProvenanceLimits{max_sources_per_glyph:2,..Default::default()});
    let before=l.clone();assert!(l.contract(&[0,1,2],1).is_err());assert_eq!(l,before);
    l.contract(&[0,2],1).unwrap();assert_eq!(sets(&l),vec![vec![0,2],vec![1]]);
}
#[test]fn cumulative_work_cap(){
    let mut l=ledger("AB",ProvenanceLimits{max_work:6,..Default::default()});
    l.expand(0,&[1]).unwrap();assert_eq!(l.work(),6);let before=l.clone();
    assert!(l.expand(0,&[2]).is_err());assert_eq!(l,before);
}
#[test]fn independent_set_reference(){
    use std::collections::BTreeSet;
    for mask in 1u32..32{
        let p:Vec<_>=(0..5).filter(|i|mask&(1<<i)!=0).collect();
        if p.len()<2{continue;}
        let mut l=ledger("ABCDE",Default::default());
        let mut expected=Vec::new();
        let union:BTreeSet<_>=p.iter().copied().collect();
        for i in 0..5{
            if i==p[0]{expected.push(union.iter().copied().collect::<Vec<_>>());}
            else if !union.contains(&i){expected.push(vec![i]);}
        }
        l.contract(&p,1).unwrap();assert_eq!(sets(&l),expected);
    }
}
