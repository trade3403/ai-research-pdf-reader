use rwcore::*;
fn fixture(spec:&[(u16,u16,Vec<u16>)])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,spec.len() as u16];
    w.resize(8+spec.len(),0);
    for (i,(kind,effective,sub)) in spec.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        w.extend([*kind,0,1,8]);
        if *kind==7{w.extend([1,*effective,0,8]);}
        w.extend_from_slice(sub);
    }
    w.iter().flat_map(|v|v.to_be_bytes()).collect()
}
fn single(g:u16,v:u16)->Vec<u16>{vec![2,8,1,v,1,1,g]}
fn multiple(g:u16)->Vec<u16>{vec![1,8,1,14,1,1,g,2,11,12]}
fn run(text:&str)->(SourceRun,u32){
    let c=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();
    let cmap=parse_unicode_cmap(c.face(0).unwrap(),Default::default()).unwrap();
    (map_source_run(text,&cmap,false,Default::default()).unwrap(),u32::from(cmap.glyph_count()))
}
#[test]fn single_and_multiple_with_native_seeds(){
    let (run,n)=run("ABA");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    assert_ne!(a,b);
    for kind in [2,7]{
        let bytes=fixture(&[(kind,2,multiple(a)),(1,1,single(11,20))]);
        let p=compile_gsub_program(&bytes,&[0,1],n,None,Default::default()).unwrap();
        let (result,ledger)=execute_gsub_with_provenance(&p,&run,Default::default(),Default::default()).unwrap();
        assert_eq!(result.expansion.execution.glyphs,vec![20,12,if b==11{20}else{b},20,12]);
        assert_eq!(ledger.glyphs().iter().map(|g|g.sources()).collect::<Vec<_>>(),vec![&[0][..],&[0][..],&[1][..],&[2][..],&[2][..]]);
        let input:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
        assert_eq!(result,execute_gsub_program(&p,&input,&[],Default::default()).unwrap());
    }
}
#[test]fn single_wrapped_preserves_sources(){
    let (run,n)=run("AA");let a=run.glyphs()[0].glyph_id;
    let bytes=fixture(&[(7,1,single(a,20))]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    let (result,ledger)=execute_gsub_with_provenance(&p,&run,Default::default(),Default::default()).unwrap();
    assert_eq!(result.expansion.execution.glyphs,vec![20,20]);
    assert_eq!(ledger.glyphs()[0].sources(),&[0]);assert_eq!(ledger.glyphs()[1].sources(),&[1]);
}
#[test]fn provenance_budget_failure_leaves_input(){
    let (run,n)=run("AA");let before=run.clone();
    let bytes=fixture(&[(2,2,multiple(run.glyphs()[0].glyph_id))]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    let l=ProvenanceLimits{max_refs:3,..Default::default()};
    assert!(matches!(execute_gsub_with_provenance(&p,&run,Default::default(),l),Err(Error::Limit(_))));
    assert_eq!(run,before);
}
#[test]fn empty_program_preserves_seed(){
    let (run,n)=run("AB");let bytes=fixture(&[]);
    let p=compile_gsub_program(&bytes,&[],n,None,Default::default()).unwrap();
    let (_,l)=execute_gsub_with_provenance(&p,&run,Default::default(),Default::default()).unwrap();
    assert_eq!(l.glyphs().iter().map(|g|g.glyph_id).collect::<Vec<_>>(),run.glyphs().iter().map(|g|g.glyph_id).collect::<Vec<_>>());
}
#[test]fn other_reachable_kind_rejected_even_empty(){
    let (run,n)=run("");
    let bytes=fixture(&[(3,3,vec![1,8,1,14,1,1,10,2,11,12])]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_with_provenance(&p,&run,Default::default(),Default::default()),Err(Error::Unsupported(_))));
}

fn ligature(a:u16,b:u16,out:u16)->Vec<u16>{
    vec![1,8,1,14,1,1,a,1,4,out,2,b]
}
fn alternate(a:u16)->Vec<u16>{vec![1,8,1,14,1,1,a,2,11,12]}
#[test]fn direct_alternate_explicit_choice(){
    let (run,n)=run("ABA");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=fixture(&[(7,3,alternate(a))]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    let choices=[GsubAlternateSelection{lookup_index:0,candidate_index:1}];
    let (result,l)=execute_gsub_with_direct_provenance(&p,&run,&choices,Default::default(),Default::default()).unwrap();
    assert_eq!(result.expansion.execution.glyphs,vec![12,b,12]);
    assert_eq!(l.glyphs().iter().map(|g|g.sources()).collect::<Vec<_>>(),vec![&[0][..],&[1][..],&[2][..]]);
    assert!(execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),Default::default()).is_err());
    assert!(execute_gsub_with_direct_provenance(&p,&run,&[GsubAlternateSelection{lookup_index:0,candidate_index:2}],Default::default(),Default::default()).is_err());
}
#[test]fn direct_ligature_and_wrapped(){
    let (run,n)=run("ABAB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    for kind in [4,7]{
        let bytes=fixture(&[(kind,4,ligature(a,b,20))]);
        let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
        let (result,l)=execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),Default::default()).unwrap();
        assert_eq!(result.expansion.execution.glyphs,vec![20,20]);
        assert_eq!(l.glyphs()[0].sources(),&[0,1]);assert_eq!(l.glyphs()[1].sources(),&[2,3]);
        assert_eq!(result.ligatures,2);
        assert_eq!(result,execute_gsub_program(&p,&[a,b,a,b],&[],Default::default()).unwrap());
    }
}
#[test]fn direct_expansion_then_ligature_deduplicates(){
    let (run,n)=run("A");let a=run.glyphs()[0].glyph_id;
    let bytes=fixture(&[(2,2,multiple(a)),(4,4,ligature(11,12,20))]);
    let p=compile_gsub_program(&bytes,&[0,1],n,None,Default::default()).unwrap();
    let (result,l)=execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),Default::default()).unwrap();
    assert_eq!(result.expansion.execution.glyphs,vec![20]);assert_eq!(l.glyphs()[0].sources(),&[0]);
}
#[test]fn direct_ligature_budget_atomic(){
    let (run,n)=run("AB");let before=run.clone();
    let bytes=fixture(&[(4,4,ligature(run.glyphs()[0].glyph_id,run.glyphs()[1].glyph_id,20))]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),ProvenanceLimits{max_sources_per_glyph:1,..Default::default()}),Err(Error::Limit(_))));
    assert_eq!(run,before);
}
#[test]fn direct_ligature_ignored_gap(){
    let (run,n)=run("ACB");let a=run.glyphs()[0].glyph_id;let mark=run.glyphs()[1].glyph_id;let b=run.glyphs()[2].glyph_id;
    // Synthetic GDEF 1.0 class definition, format 2, one mark range.
    let gd=[1u16,0,12,0,0,0,2,1,mark,mark,3];
    let gbytes:Vec<_>=gd.iter().flat_map(|v|v.to_be_bytes()).collect();
    let gdef=parse_gdef(&gbytes,n,Default::default()).unwrap();
    let mut bytes=fixture(&[(4,4,ligature(a,b,20))]);
    // First lookup starts at word 9; flag at word 10.
    bytes[20..22].copy_from_slice(&LOOKUP_IGNORE_MARK.to_be_bytes());
    let p=compile_gsub_program(&bytes,&[0],n,Some(&gdef),Default::default()).unwrap();
    let (result,l)=execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),Default::default()).unwrap();
    assert_eq!(result.expansion.execution.glyphs,vec![20,mark]);
    assert_eq!(l.glyphs()[0].sources(),&[0,2]);assert_eq!(l.glyphs()[1].sources(),&[1]);
}

#[test]fn direct_alternate_then_ligature_preserves_sources_and_counters(){
    let (run,n)=run("ABAB");
    let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    for kind in [3,7]{
        let bytes=fixture(&[(kind,3,alternate(a)),(4,4,ligature(12,b,20))]);
        let p=compile_gsub_program(&bytes,&[0,1],n,None,Default::default()).unwrap();
        let choices=[GsubAlternateSelection{lookup_index:0,candidate_index:1}];
        let (result,l)=execute_gsub_with_direct_provenance(&p,&run,&choices,Default::default(),Default::default()).unwrap();
        assert_eq!(result.expansion.execution.glyphs,vec![20,20]);
        assert_eq!(l.glyphs()[0].sources(),&[0,1]);
        assert_eq!(l.glyphs()[1].sources(),&[2,3]);
        assert_eq!(result,execute_gsub_program(&p,&[a,b,a,b],&choices,Default::default()).unwrap());
    }
}
#[test]fn direct_alternate_source_work_budget(){
    let (run,n)=run("A");
    let bytes=fixture(&[(3,3,alternate(run.glyphs()[0].glyph_id))]);
    let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
    let choices=[GsubAlternateSelection{lookup_index:0,candidate_index:0}];
    let exact=ProvenanceLimits{max_work:4,..Default::default()};
    let (_,l)=execute_gsub_with_direct_provenance(&p,&run,&choices,Default::default(),exact).unwrap();
    assert_eq!(l.work(),4);
    assert!(matches!(execute_gsub_with_direct_provenance(&p,&run,&choices,Default::default(),ProvenanceLimits{max_work:3,..exact}),Err(Error::Limit(_))));
}
