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

fn context3(input:&[u16],actions:&[(u16,u16)])->Vec<u16>{
    let mut w=vec![3,input.len() as u16,actions.len() as u16];
    w.resize(3+input.len(),0);
    for &(p,l) in actions{w.extend([p,l]);}
    for (i,&g) in input.iter().enumerate(){w[3+i]=(w.len()*2) as u16;w.extend([1,1,g]);}
    w
}
fn chain3(back:&[u16],input:&[u16],ahead:&[u16],actions:&[(u16,u16)])->Vec<u16>{
    let mut w=vec![3];let mut slots=Vec::new();
    for seq in [back,input,ahead]{w.push(seq.len() as u16);for &g in seq{slots.push((w.len(),g));w.push(0);}}
    w.push(actions.len() as u16);
    for &(p,l) in actions{w.extend([p,l]);}
    for (i,g) in slots{w[i]=(w.len()*2) as u16;w.extend([1,1,g]);}
    w
}
fn reverse8(g:u16,out:u16,back:&[u16],ahead:&[u16])->Vec<u16>{
    let mut w=vec![1,0,back.len() as u16];let mut slots=Vec::new();
    for &g in back{slots.push((w.len(),g));w.push(0);}
    w.push(ahead.len() as u16);
    for &g in ahead{slots.push((w.len(),g));w.push(0);}
    w.extend([1,out]);w[1]=(w.len()*2) as u16;w.extend([1,1,g]);
    for (i,g) in slots{w[i]=(w.len()*2) as u16;w.extend([1,1,g]);}
    w
}
fn full_tracked(p:&CompiledGsubProgram<'_>,run:&SourceRun)->(GsubProgramExecResult,ProvenanceLedger){
    let (result,l)=execute_gsub_program_with_provenance(p,run,&[],Default::default(),Default::default()).unwrap();
    let input:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    assert_eq!(result,execute_gsub_program(p,&input,&[],Default::default()).unwrap());
    (result,l)
}
fn source_sets(l:&ProvenanceLedger)->Vec<Vec<usize>>{l.glyphs().iter().map(|g|g.sources().to_vec()).collect()}
#[test]fn nested_growth_rebases_sources(){
    let (run,n)=run("AB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=fixture(&[(2,2,multiple(a)),(1,1,single(b,20)),(5,5,context3(&[a,b],&[(0,0),(1,1)]))]);
    let p=compile_gsub_program(&bytes,&[2],n,None,Default::default()).unwrap();
    let (result,l)=full_tracked(&p,&run);
    assert_eq!(result.expansion.execution.glyphs,vec![11,12,20]);
    assert_eq!(source_sets(&l),vec![vec![0],vec![0],vec![1]]);
    assert!(execute_gsub_with_direct_provenance(&p,&run,&[],Default::default(),Default::default()).is_err());
}
#[test]fn nested_contraction_skips_consumed_sources(){
    let (run,n)=run("ABC");let g:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let bytes=fixture(&[(4,4,ligature(g[0],g[1],20)),(1,1,single(g[2],21)),(5,5,context3(&g,&[(0,0),(1,1),(2,1)]))]);
    let p=compile_gsub_program(&bytes,&[2],n,None,Default::default()).unwrap();
    let (result,l)=full_tracked(&p,&run);
    assert_eq!(result.expansion.execution.glyphs,vec![20,21]);assert_eq!(result.skipped_actions,1);
    assert_eq!(source_sets(&l),vec![vec![0,1],vec![2]]);
}
#[test]fn chained_conditions_do_not_contribute_sources(){
    let (run,n)=run("ABC");let g:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let bytes=fixture(&[(2,2,multiple(g[1])),(7,6,chain3(&g[..1],&g[1..2],&g[2..],&[(0,0)]))]);
    let p=compile_gsub_program(&bytes,&[1],n,None,Default::default()).unwrap();
    let (result,l)=full_tracked(&p,&run);
    assert_eq!(result.expansion.execution.glyphs,vec![g[0],11,12,g[2]]);
    assert_eq!(source_sets(&l),vec![vec![0],vec![1],vec![1],vec![2]]);
}
#[test]fn reverse_root_cascade_keeps_each_source(){
    let (run,n)=run("AAAB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[3].glyph_id;
    for kind in [8,7]{
        let bytes=fixture(&[(kind,8,reverse8(a,b,&[],&[b]))]);
        let p=compile_gsub_program(&bytes,&[0],n,None,Default::default()).unwrap();
        let (result,l)=full_tracked(&p,&run);
        assert_eq!(result.expansion.execution.glyphs,vec![b,b,b,b]);
        assert_eq!(source_sets(&l),vec![vec![0],vec![1],vec![2],vec![3]]);
    }
}
#[test]fn reverse_nested_action_after_growth(){
    let (run,n)=run("ABC");let g:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let bytes=fixture(&[(2,2,multiple(g[0])),(8,8,reverse8(g[1],20,&[12],&[g[2]])),(5,5,context3(&g,&[(0,0),(1,1)]))]);
    let p=compile_gsub_program(&bytes,&[2],n,None,Default::default()).unwrap();
    let (result,l)=full_tracked(&p,&run);
    assert_eq!(result.expansion.execution.glyphs,vec![11,12,20,g[2]]);
    assert_eq!(source_sets(&l),vec![vec![0],vec![0],vec![1],vec![2]]);
}
#[test]fn nested_provenance_budget_atomic(){
    let (run,n)=run("AB");let before=run.clone();let a=run.glyphs()[0].glyph_id;
    let bytes=fixture(&[(2,2,multiple(a)),(5,5,context3(&[a],&[(0,0)]))]);
    let p=compile_gsub_program(&bytes,&[1],n,None,Default::default()).unwrap();
    assert!(matches!(execute_gsub_program_with_provenance(&p,&run,&[],Default::default(),ProvenanceLimits{max_refs:2,..Default::default()}),Err(Error::Limit(_))));
    assert_eq!(run,before);
}

fn masked_fixture(spec:&[(u16,u16,Vec<u16>)],features:&[(&[u8;4],&[u16])])->Vec<u8>{
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,65535,features.len() as u16]{w(&mut b,v);}
    for i in 0..features.len(){w(&mut b,i as u16);}
    let at=b.len();set(&mut b,6,at);w(&mut b,features.len() as u16);
    let mut slots=Vec::new();
    for (tag,_) in features{b.extend(*tag);slots.push(b.len());w(&mut b,0);}
    for ((_,ids),slot) in features.iter().zip(slots){let rel=b.len()-at;set(&mut b,slot,rel);w(&mut b,0);w(&mut b,ids.len() as u16);for &id in *ids{w(&mut b,id);}}
    let at=b.len();set(&mut b,8,at);let old=fixture(spec);b.extend_from_slice(&old[14..]);b
}
fn strict_plan(bytes:&[u8])->GsubPlan{
    let layout=parse_gsub_layout(bytes,Default::default()).unwrap();
    build_gsub_base_plan(&layout,*b"DFLT",None,&[*b"liga",*b"salt"],Default::default()).unwrap().unwrap()
}
#[test]fn strict_ligature_requires_common_feature(){
    let (run,n)=run("AB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=masked_fixture(&[(4,4,ligature(a,b,20))],&[(b"liga",&[0]),(b"salt",&[0])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let edits=[FeatureMaskOverride{feature:0,start:1,end:2,enabled:false},FeatureMaskOverride{feature:1,start:0,end:1,enabled:false}];
    let masks=ScalarFeatureMasks::build(&plan,2,&edits,Default::default()).unwrap();
    let (result,blocked)=execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
    assert_eq!(result.execution.expansion.execution.glyphs,vec![a,b]);assert!(blocked>0);
    let legacy=execute_gsub_program_with_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
    assert_eq!(legacy.execution.expansion.execution.glyphs,vec![20]);
}
#[test]fn strict_nested_input_and_live_expansion(){
    let (run,n)=run("AB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=masked_fixture(&[(2,2,multiple(a)),(1,1,single(b,20)),(5,5,context3(&[a,b],&[(0,0),(1,1)]))],&[(b"liga",&[2])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    for enabled in [false,true]{
        let edits=[FeatureMaskOverride{feature:0,start:1,end:2,enabled}];
        let masks=ScalarFeatureMasks::build(&plan,2,&edits,Default::default()).unwrap();
        let (result,blocked)=execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
        assert_eq!(result.execution.expansion.execution.glyphs,if enabled{vec![11,12,20]}else{vec![a,b]});
        assert_eq!(blocked>0,!enabled);
        assert!(execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),result.mask_work-1).is_err());
    }
}
#[test]fn strict_chain_conditions_not_gated(){
    let (run,n)=run("ABC");let g:Vec<_>=run.glyphs().iter().map(|g|g.glyph_id).collect();
    let bytes=masked_fixture(&[(1,1,single(g[1],20)),(6,6,chain3(&g[..1],&g[1..2],&g[2..],&[(0,0)]))],&[(b"liga",&[1])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan,3,&[FeatureMaskOverride{feature:0,start:0,end:1,enabled:false},FeatureMaskOverride{feature:0,start:2,end:3,enabled:false}],Default::default()).unwrap();
    let (result,blocked)=execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
    assert_eq!(result.execution.expansion.execution.glyphs,vec![g[0],20,g[2]]);assert_eq!(blocked,0);
}

#[test]fn strict_shared_ligature_exhaustive_boolean_oracle(){
    let (run,n)=run("ABAB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=masked_fixture(&[(4,4,ligature(a,b,20))],&[(b"liga",&[0]),(b"salt",&[0])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    for pattern in 0u16..256{
        let mut edits=Vec::new();
        let mut enabled=[[false;2];4];
        for i in 0..4{for f in 0..2{
            enabled[i][f]=pattern&(1<<(2*i+f))!=0;
            edits.push(FeatureMaskOverride{feature:f as u16,start:i,end:i+1,enabled:enabled[i][f]});
        }}
        let masks=ScalarFeatureMasks::build(&plan,4,&edits,Default::default()).unwrap();
        let (result,_)=execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
        let mut glyphs=Vec::new();let mut sources=Vec::new();let mut expected_masks=Vec::new();
        for i in [0,2]{
            let common:Vec<_>=(0..2).filter(|&f|enabled[i][f]&&enabled[i+1][f]).collect();
            if !common.is_empty(){
                glyphs.push(20);sources.push(vec![i,i+1]);expected_masks.push(common.iter().map(|&f|1u64<<f).sum());
            }else{
                for j in i..i+2{
                    glyphs.push(if j%2==0{a}else{b});sources.push(vec![j]);
                    expected_masks.push((0..2).filter(|&f|enabled[j][f]).map(|f|1u64<<f).sum());
                }
            }
        }
        assert_eq!(result.execution.expansion.execution.glyphs,glyphs);
        assert_eq!(source_sets(&result.provenance),sources);
        assert_eq!(result.glyph_masks,expected_masks);
    }
}
#[test]fn strict_reverse_mask_barrier_reference(){
    let (run,n)=run("AAAB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[3].glyph_id;
    let bytes=masked_fixture(&[(8,8,reverse8(a,b,&[],&[b]))],&[(b"liga",&[0])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    for pattern in 0..16{
        let edits:Vec<_>=(0..4).map(|i|FeatureMaskOverride{feature:0,start:i,end:i+1,enabled:pattern&(1<<i)!=0}).collect();
        let masks=ScalarFeatureMasks::build(&plan,4,&edits,Default::default()).unwrap();
        let (result,blocked)=execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap();
        let mut expected=vec![a,a,a,b];
        for i in (0..3).rev(){if pattern&(1<<i)!=0&&expected[i+1]==b{expected[i]=b;}}
        assert_eq!(result.execution.expansion.execution.glyphs,expected);assert_eq!(blocked,0);
        assert_eq!(source_sets(&result.provenance),vec![vec![0],vec![1],vec![2],vec![3]]);
    }
}

fn stage(ids:&[u16])->ShapeStageSpec{ShapeStageSpec{features:ids.to_vec()}}
#[test]fn staged_order_and_persistent_growth_contraction(){
    let (run,n)=run("A");let a=run.glyphs()[0].glyph_id;
    let bytes=masked_fixture(&[(4,4,ligature(11,12,20)),(2,2,multiple(a))],&[(b"liga",&[0]),(b"salt",&[1])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan,1,&[],Default::default()).unwrap();
    let (_,r,_)=execute_staged_gsub(&p,&run,&[],&masks,&[stage(&[1]),stage(&[0])],Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!(r.execution.expansion.execution.glyphs,vec![20]);
    assert_eq!(source_sets(&r.provenance),vec![vec![0]]);
    assert_eq!(r.glyph_masks,vec![3]);assert_eq!(r.execution.expansion.inserted_glyphs,1);assert_eq!(r.execution.removed_glyphs,1);
    let (_,reverse,_)=execute_staged_gsub(&p,&run,&[],&masks,&[stage(&[0]),stage(&[1])],Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!(reverse.execution.expansion.execution.glyphs,vec![11,12]);
}
#[test]fn staged_shared_root_uses_stage_specific_masks(){
    let (run,n)=run("AA");let a=run.glyphs()[0].glyph_id;
    let bytes=masked_fixture(&[(1,1,single(a,20))],&[(b"liga",&[0]),(b"salt",&[0])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let edits=[FeatureMaskOverride{feature:0,start:1,end:2,enabled:false},FeatureMaskOverride{feature:1,start:0,end:1,enabled:false}];
    let masks=ScalarFeatureMasks::build(&plan,2,&edits,Default::default()).unwrap();
    let (schedule,r,blocked)=execute_staged_gsub(&p,&run,&[],&masks,&[stage(&[0]),stage(&[1])],Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!(schedule.root_count(),2);assert_eq!(r.execution.expansion.execution.glyphs,vec![20,20]);
    assert_eq!(r.skipped_roots,2);assert_eq!(blocked,0);assert_eq!(r.glyph_masks,vec![1,2]);
}
#[test]fn staged_single_stage_parity_and_cumulative_limits(){
    let (run,n)=run("AB");let a=run.glyphs()[0].glyph_id;let b=run.glyphs()[1].glyph_id;
    let bytes=masked_fixture(&[(1,1,single(a,20)),(1,1,single(b,21))],&[(b"liga",&[0]),(b"salt",&[1])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan,2,&[],Default::default()).unwrap();
    let (_,r,k)=execute_staged_gsub(&p,&run,&[],&masks,&[stage(&[0,1])],Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!((r,k),execute_gsub_program_with_strict_masks(&p,&run,&[],&masks,Default::default(),Default::default(),1000).unwrap());
    let specs=[stage(&[0]),stage(&[1])];
    let (_,r,_)=execute_staged_gsub(&p,&run,&[],&masks,&specs,Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert!(execute_staged_gsub(&p,&run,&[],&masks,&specs,Default::default(),Default::default(),Default::default(),r.mask_work-1).is_err());
    let mut limits=GsubProgramExecLimits::default();limits.expansion.context.max_calls=3;
    assert!(matches!(execute_staged_gsub(&p,&run,&[],&masks,&specs,Default::default(),limits,Default::default(),1000),Err(Error::Limit(_))));
    assert!(execute_staged_gsub(&p,&run,&[],&masks,&[stage(&[0])],Default::default(),Default::default(),Default::default(),1000).is_err());
}

#[test]fn staged_trace_positive_two_stages(){
    let (run,n)=run("A");let a=run.glyphs()[0].glyph_id;
    let bytes=masked_fixture(&[(4,4,ligature(11,12,20)),(2,2,multiple(a))],&[(b"liga",&[0]),(b"salt",&[1])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan,1,&[],Default::default()).unwrap();
    let specs=[stage(&[1]),stage(&[0])];
    let (schedule,result,blocked,trace)=execute_staged_gsub_traced(&p,&run,&[],&masks,&specs,Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!(trace,vec![ShapeStageTrace{stage_index:0,root_count:1,changes:1},ShapeStageTrace{stage_index:1,root_count:1,changes:1}]);
    assert_eq!(result.execution.expansion.execution.glyphs,vec![20]);
    assert_eq!((schedule,result,blocked),execute_staged_gsub(&p,&run,&[],&masks,&specs,Default::default(),Default::default(),Default::default(),1000).unwrap());
}
#[test]fn staged_trace_counts_disabled_and_noop(){
    let (run,n)=run("A");let a=run.glyphs()[0].glyph_id;
    let bytes=masked_fixture(&[(1,1,single(a,a)),(1,1,single(a,20))],&[(b"liga",&[0]),(b"salt",&[1])]);
    let plan=strict_plan(&bytes);let p=compile_gsub_plan_program(&bytes,&plan,n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan,1,&[FeatureMaskOverride{feature:1,start:0,end:1,enabled:false}],Default::default()).unwrap();
    let (_,result,_,trace)=execute_staged_gsub_traced(&p,&run,&[],&masks,&[stage(&[0]),stage(&[1])],Default::default(),Default::default(),Default::default(),1000).unwrap();
    assert_eq!(trace.iter().map(|x|x.changes).collect::<Vec<_>>(),vec![0,0]);
    assert_eq!(result.execution.expansion.execution.matches,1);assert_eq!(result.skipped_roots,1);
}
