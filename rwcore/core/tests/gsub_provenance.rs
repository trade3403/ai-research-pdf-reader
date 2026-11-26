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
