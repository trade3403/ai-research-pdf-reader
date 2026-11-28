use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf");
fn source(t:&str)->(SourceRun,u32){
    let font=parse_font(FONT,Default::default()).unwrap();
    let c=parse_unicode_cmap(font.face(0).unwrap(),Default::default()).unwrap();
    (map_source_run(t,&c,true,Default::default()).unwrap(),u32::from(c.glyph_count()))
}
fn apply(t:&str,r:&SourceRun,l:&mut ProvenanceLedger,limits:IndicGlyphReorderLimits)->Result<IndicGlyphReorderResult,Error>{
    reorder_devanagari_glyphs(t,r,l,&bundled_indic_properties().unwrap(),&bundled_script_properties().unwrap(),limits)
}
#[test]fn ledger_atomic_permutation_and_inverse(){
    let (r,n)=source("abc");let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let original=l.glyphs().to_vec();let work=l.work();
    l.reorder(&[2,0,1]).unwrap();assert_eq!(l.glyphs(),&[original[2].clone(),original[0].clone(),original[1].clone()]);
    l.reorder(&[1,2,0]).unwrap();assert_eq!(l.glyphs(),original.as_slice());assert_eq!(l.work(),work+24);
    for order in [&[0,0,2][..],&[0,1,3][..],&[0,1][..]]{
        let before=l.clone();assert!(matches!(l.reorder(order),Err(Error::Invalid(_))));assert_eq!(l,before);
    }
}
#[test]fn ledger_reorder_budget_atomic(){
    let (r,n)=source("ab");let mut l=ProvenanceLedger::from_run(&r,n,ProvenanceLimits{max_work:9,..Default::default()}).unwrap();
    let before=l.clone();assert!(matches!(l.reorder(&[1,0]),Err(Error::Limit(_))));assert_eq!(l,before);
}
#[test]fn indic_glyph_simple_native_cmap(){
    let t="कि कि";let (r,n)=source(t);let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let old=l.glyphs().to_vec();let x=apply(t,&r,&mut l,Default::default()).unwrap();
    assert_eq!(x.order,vec![1,0,2,4,3]);assert_eq!(x.inverse,vec![1,0,2,4,3]);
    for (i,&j) in x.order.iter().enumerate(){assert_eq!(l.glyphs()[i],old[j]);}
    assert_eq!(l.reference_count(),5);
}
#[test]fn indic_glyph_expansion_moves_stable_group(){
    let t="किं";let (r,n)=source(t);let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    l.expand(1,&[10,11]).unwrap();let x=apply(t,&r,&mut l,Default::default()).unwrap();
    assert_eq!(x.order,vec![1,2,0,3]);
    assert_eq!(l.glyphs()[0].glyph_id,10);assert_eq!(l.glyphs()[1].glyph_id,11);
    assert_eq!(l.glyphs()[0].sources(),&[1]);assert_eq!(l.glyphs()[1].sources(),&[1]);
}
#[test]fn indic_glyph_contraction_preserves_lineage(){
    let t="क़ि";let (r,n)=source(t);let mut l=ProvenanceLedger::from_run_with_ligature_trace(&r,n,Default::default(),Default::default()).unwrap();
    l.contract(&[0,1],10).unwrap();let parent=l.glyphs()[0].clone();let stats=l.ligature_trace_stats();
    let x=apply(t,&r,&mut l,Default::default()).unwrap();assert_eq!(x.order,vec![1,0]);
    assert_eq!(l.glyphs()[1],parent);assert_eq!(l.glyphs()[1].ligature_lineage().unwrap().components(),&[vec![0],vec![1]]);
    assert_eq!(l.ligature_trace_stats(),stats);
}
#[test]fn indic_glyph_reject_mixed_and_cross_cluster_atomically(){
    for (t,positions) in [("कि",vec![0,1]),("कक",vec![0,1])]{
        let (r,n)=source(t);let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
        l.contract(&positions,10).unwrap();let before=l.clone();
        assert!(matches!(apply(t,&r,&mut l,Default::default()),Err(Error::Unsupported(_))));assert_eq!(l,before);
    }
}
#[test]fn indic_glyph_reject_deferred_identity_and_order(){
    for t in ["र्कि","क्षि","ि"]{
        let (r,n)=source(t);let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();let before=l.clone();
        assert!(matches!(apply(t,&r,&mut l,Default::default()),Err(Error::Unsupported(_))));assert_eq!(l,before);
    }
    let (r,n)=source("कक");let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let before=l.clone();assert!(matches!(apply("खक",&r,&mut l,Default::default()),Err(Error::Invalid(_))));assert_eq!(l,before);
    l.reorder(&[1,0]).unwrap();let before=l.clone();
    assert!(matches!(apply("कक",&r,&mut l,Default::default()),Err(Error::Invalid(_))));assert_eq!(l,before);
}
#[test]fn indic_glyph_work_and_empty(){
    let (r,n)=source("कि");let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();let before=l.clone();
    for limits in [IndicGlyphReorderLimits{max_glyphs:1,..Default::default()},IndicGlyphReorderLimits{max_work:21,..Default::default()}]{
        assert!(matches!(apply("कि",&r,&mut l,limits),Err(Error::Limit(_))));assert_eq!(l,before);
    }
    assert!(apply("कि",&r,&mut l,IndicGlyphReorderLimits{max_work:22,..Default::default()}).is_ok());
    let (r,n)=source("");let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    assert!(apply("",&r,&mut l,IndicGlyphReorderLimits{max_work:0,max_glyphs:0,..Default::default()}).unwrap().order.is_empty());
}
