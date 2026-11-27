use rwcore::*;
fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
fn fixture(id:u16)->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,2,0,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,2);
    for (tag,rel) in [(b"kern",14),(b"mark",20)]{b.extend(tag);w(&mut b,rel);}
    for v in [0,1,0,0,1,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,8,at);for v in [2,6,28]{w(&mut b,v);}
    for delta in [7,13]{for v in [1,0,1,8,1,8,4,delta,1,1,id]{w(&mut b,v);}}
    b
}
fn inputs(text:&str)->(SourceRun,u32){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();
    let cmap=parse_unicode_cmap(f.face(0).unwrap(),Default::default()).unwrap();
    (map_source_run(text,&cmap,false,Default::default()).unwrap(),u32::from(cmap.glyph_count()))
}
fn program(b:&[u8],n:u32)->CompiledGposProgram<'_>{
    let l=parse_gpos_layout(b,Default::default()).unwrap();
    let p=build_gpos_base_plan(&l,*b"DFLT",None,&[*b"kern",*b"mark"],Default::default()).unwrap().unwrap();
    compile_gpos_plan_program(b,&p,n,None,Default::default()).unwrap()
}
fn specs()->[ShapeStageSpec;2]{[ShapeStageSpec{features:vec![0]},ShapeStageSpec{features:vec![1]}]}
fn execute(p:&CompiledGposProgram<'_>,l:&ProvenanceLedger,m:&GposScalarFeatureMasks,s:&[ShapeStageSpec],work:usize)
    ->Result<(ShapeStageSchedule,GposCursiveRun),Error>{
    let ids:Vec<_>=l.glyphs().iter().map(|g|g.glyph_id).collect();let n=ids.len();
    execute_staged_gpos_joint(p,l,m,s,Default::default(),work,&ids,&vec![GposRunAdjustment::default();n],
        &vec![(100,0);n],&vec![(0,0);n],&vec![GposLigatureSource::Unmapped;n],GposRunMode::DesignOnly,
        GposHorizontalFlow::LeftToRight,Default::default(),Default::default(),false)
}
#[test]fn gpos_stage_all_enabled_and_range_mask(){
    let (r,n)=inputs("AA");let b=fixture(r.glyphs()[0].glyph_id);let p=program(&b,n);
    let l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let masks=GposScalarFeatureMasks::build(p.validated_plan().unwrap(),2,&[],Default::default()).unwrap();
    let (s,x)=execute(&p,&l,&masks,&specs(),100).unwrap();
    assert_eq!(s.root_count(),2);
    assert_eq!(x.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![20,20]);
    assert_eq!(x.design_origins,vec![(0,0),(120,0)]);
    let m=GposScalarFeatureMasks::build(p.validated_plan().unwrap(),2,&[
        FeatureMaskOverride{feature:1,start:1,end:2,enabled:false}],Default::default()).unwrap();
    let (_,x)=execute(&p,&l,&m,&specs(),100).unwrap();
    assert_eq!(x.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![20,7]);
}
#[test]fn gpos_mask_projection_contraction_intersection(){
    let (r,n)=inputs("AA");let id=r.glyphs()[0].glyph_id;let b=fixture(id);let p=program(&b,n);
    let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    l.contract(&[0,1],id).unwrap();
    let m=GposScalarFeatureMasks::build(p.validated_plan().unwrap(),2,&[
        FeatureMaskOverride{feature:0,start:1,end:2,enabled:false}],Default::default()).unwrap();
    assert_eq!(m.project(&l,100).unwrap(),vec![2]);
    assert_eq!(execute(&p,&l,&m,&specs(),100).unwrap().1.adjustments[0].design.x_advance,13);
}
#[test]fn gpos_stage_invalid_partition_and_projection_budget(){
    let (r,n)=inputs("AA");let b=fixture(r.glyphs()[0].glyph_id);let p=program(&b,n);
    let l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let m=GposScalarFeatureMasks::build(p.validated_plan().unwrap(),2,&[],Default::default()).unwrap();
    assert!(matches!(execute(&p,&l,&m,&specs()[..1],100),Err(Error::Invalid(_))));
    assert!(matches!(execute(&p,&l,&m,&specs(),3),Err(Error::Limit(_))));
    assert!(execute(&p,&l,&m,&specs(),4).is_ok());
}
#[test]fn gpos_stage_native_kern_parity(){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();let face=f.face(0).unwrap();
    let b=face.table(*b"GPOS").unwrap();let (r,n)=inputs("AVATAR");
    let gd=parse_gdef(face.table(*b"GDEF").unwrap(),n,Default::default()).unwrap();
    let layout=parse_gpos_layout(b,Default::default()).unwrap();
    let plan=build_gpos_base_plan(&layout,*b"latn",None,&[*b"kern"],Default::default()).unwrap().unwrap();
    let p=compile_gpos_plan_program(b,&plan,n,Some(&gd),Default::default()).unwrap();
    let l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    let m=GposScalarFeatureMasks::build(&plan,r.scalars().len(),&[],Default::default()).unwrap();
    let specs=[ShapeStageSpec{features:plan.features.clone()}];
    let (_,x)=execute(&p,&l,&m,&specs,100).unwrap();
    let ids:Vec<_>=l.glyphs().iter().map(|g|g.glyph_id).collect();let n=ids.len();
    let y=execute_gpos_mixed_joint(&p,&ids,&vec![GposRunAdjustment::default();n],&vec![(100,0);n],&vec![(0,0);n],
        &vec![GposLigatureSource::Unmapped;n],GposRunMode::DesignOnly,GposHorizontalFlow::LeftToRight,Default::default(),Default::default()).unwrap();
    assert_eq!(x,y);assert!(x.adjustments.iter().any(|a|a.design.x_advance!=0));
}

// Explicit two-feature plan, with dependency-only lookups and shared roots.
fn planned_fixture(features:[&[u16];2],lookups:&[(u16,u16,Vec<u16>)])->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,2,0,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,2);
    b.extend(b"kern");w(&mut b,14);b.extend(b"mark");w(&mut b,(18+2*features[0].len()) as u16);
    for ids in features{w(&mut b,0);w(&mut b,ids.len() as u16);for &id in ids{w(&mut b,id);}}
    let at=b.len();set(&mut b,8,at);w(&mut b,lookups.len() as u16);
    for _ in lookups{w(&mut b,0);}
    for (i,(kind,flag,sub)) in lookups.iter().enumerate(){
        let rel=b.len()-at;set(&mut b,at+2+2*i,rel);
        for v in [*kind,*flag,1,8]{w(&mut b,v);}for &v in sub{w(&mut b,v);}
    }
    b
}
fn ledger(ids:&[u16])->(ProvenanceLedger,u32){
    let (r,n)=inputs(&"A".repeat(ids.len()));
    let mut l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    for (i,&id) in ids.iter().enumerate(){l.expand(i,&[id]).unwrap();}
    (l,n)
}
fn gdef_bytes(n:u32)->Gdef{
    let bytes:Vec<_>=[1u16,0,12,0,0,0,1,3,5,3,3,0,0,1].iter().flat_map(|v|v.to_be_bytes()).collect();
    parse_gdef(&bytes,n,Default::default()).unwrap()
}
fn planned<'a>(b:&[u8],n:u32,gd:Option<&'a Gdef>)->CompiledGposProgram<'a>{
    let l=parse_gpos_layout(b,Default::default()).unwrap();
    let p=build_gpos_base_plan(&l,*b"DFLT",None,&[*b"kern",*b"mark"],Default::default()).unwrap().unwrap();
    compile_gpos_plan_program(b,&p,n,gd,Default::default()).unwrap()
}
fn disable(feature:u16,start:usize,end:usize)->FeatureMaskOverride{
    FeatureMaskOverride{feature,start,end,enabled:false}
}
fn mask_for(p:&CompiledGposProgram<'_>,l:&ProvenanceLedger,edits:&[FeatureMaskOverride])->GposScalarFeatureMasks{
    GposScalarFeatureMasks::build(p.validated_plan().unwrap(),l.scalar_count(),edits,Default::default()).unwrap()
}
fn pair_sub()->Vec<u16>{vec![1,12,4,1,1,18,1,1,3,1,3,10,5]}
fn single_sub()->Vec<u16>{vec![1,8,4,7,1,1,3]}
fn base_sub()->Vec<u16>{vec![1,12,20,2,26,48,1,2,3,4,1,1,7,2,0,10,1,16,1,10,20,1,30,40,1,6,0,1,100,200]}
fn stack_sub()->Vec<u16>{vec![1,12,20,2,28,50,1,2,3,4,1,2,3,7,2,0,10,1,16,1,10,20,1,30,40,2,10,0,16,22,1,100,200,1,65526,30,1,50,65516]}
#[test]fn masked_pair_barrier_does_not_skip_disabled_glyph(){
    let (l,n)=ledger(&[3,3,3]);let b=planned_fixture([&[0],&[1]],&[(2,0,pair_sub()),(1,0,single_sub())]);
    let p=planned(&b,n,None);
    let all=mask_for(&p,&l,&[]);
    let x=execute(&p,&l,&all,&specs(),100).unwrap().1;
    assert_eq!(x.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![17,7,7]);
    assert_eq!(x.adjustments[1].design.x_placement,5);
    let m=mask_for(&p,&l,&[disable(0,1,2)]);
    let x=execute(&p,&l,&m,&specs(),100).unwrap().1;
    assert_eq!(x.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![7,7,7]);
    assert!(x.adjustments.iter().all(|a|a.design.x_placement==0));
}
#[test]fn masked_pair_gdef_ignored_gap_is_not_participant(){
    let (l,n)=ledger(&[3,4,3]);
    // Explicit class definition with 3 base, 4 mark for the pair.
    let bytes:Vec<_>=[1u16,0,12,0,0,0,1,3,2,1,3].iter().flat_map(|v|v.to_be_bytes()).collect();
    let pair_gd=parse_gdef(&bytes,n,Default::default()).unwrap();
    let b=planned_fixture([&[0],&[1]],&[(2,LOOKUP_IGNORE_MARK,pair_sub()),(1,0,single_sub())]);
    let p=planned(&b,n,Some(&pair_gd));
    let m=mask_for(&p,&l,&[disable(0,1,2)]);
    let x=execute(&p,&l,&m,&specs(),100).unwrap().1;
    assert_eq!(x.adjustments[0].design.x_advance,17);
    assert_eq!(x.adjustments[1],GposRunAdjustment::default());
    assert_eq!(x.adjustments[2].design.x_placement,5);
}
#[test]fn masked_nested_pair_inherits_common_context_feature(){
    let (l,n)=ledger(&[3,3,3]);
    let ctx=vec![3,2,1,14,14,1,1,1,1,3];
    let b=planned_fixture([&[0],&[0]],&[(7,0,ctx),(2,0,pair_sub())]);
    let p=planned(&b,n,None);
    let one=[ShapeStageSpec{features:vec![0,1]}];
    let all=mask_for(&p,&l,&[]);
    let x=execute(&p,&l,&all,&one,100).unwrap().1;
    assert_eq!(x.adjustments[1].design.x_advance,10);
    assert_eq!(x.adjustments[2].design.x_placement,5);
    // Outer input 0,1 has common feature 0; nested pair 1,2
    // only has feature 1. The nested action must not switch features.
    let m=mask_for(&p,&l,&[disable(1,0,1),disable(0,2,3)]);
    let x=execute(&p,&l,&m,&one,100).unwrap().1;
    assert_eq!(x.adjustments,vec![GposRunAdjustment::default();3]);
    let (schedule,x)=execute(&p,&l,&all,&specs(),100).unwrap();
    assert_eq!(schedule.root_count(),2);
    assert_eq!(x.adjustments[1].design.x_advance,20);
    assert_eq!(x.adjustments[2].design.x_placement,10);
}
#[test]fn masked_chain_backtrack_lookahead_are_conditions_only(){
    let (l,n)=ledger(&[4,3,5]);
    let chain=vec![3,1,24,1,30,1,36,1,0,1,0,0,1,1,4,1,1,3,1,1,5];
    let b=planned_fixture([&[0],&[1]],&[(8,0,chain),(1,0,single_sub())]);
    let p=planned(&b,n,None);
    let m=mask_for(&p,&l,&[disable(0,0,1),disable(0,2,3),disable(1,0,3)]);
    let x=execute(&p,&l,&m,&specs(),100).unwrap().1;
    assert_eq!(x.adjustments[1].design.x_advance,7);
    let m=mask_for(&p,&l,&[disable(0,1,2),disable(1,0,3)]);
    assert_eq!(execute(&p,&l,&m,&specs(),100).unwrap().1.adjustments,vec![GposRunAdjustment::default();3]);
}
#[test]fn masked_mark_chain_shared_state_and_stage_reversal(){
    let (l,n)=ledger(&[7,3,3]);let gd=gdef_bytes(n);
    let b=planned_fixture([&[0],&[1]],&[(4,0,base_sub()),(6,0,stack_sub())]);let p=planned(&b,n,Some(&gd));
    let all=mask_for(&p,&l,&[]);
    let x=execute(&p,&l,&all,&specs(),100).unwrap().1;
    assert_eq!(x.edges[1].unwrap().parent,0);assert_eq!(x.edges[2].unwrap().parent,1);
    assert_eq!(x.adjustments[2].design.y_placement,360);
    assert_eq!(x.design_origins[2].0+x.adjustments[2].design.x_placement,180);
    // Disabled mark-to-mark parent cannot be jumped over. Earlier
    // mark-to-base edge remains intact instead of being cleared.
    let m=mask_for(&p,&l,&[disable(1,1,2)]);
    let x=execute(&p,&l,&m,&specs(),100).unwrap().1;
    assert_eq!(x.edges[2].unwrap().parent,0);assert_eq!(x.adjustments[2].design.y_placement,180);
    let mut rev=specs();rev.reverse();
    let x=execute(&p,&l,&all,&rev,100).unwrap().1;
    assert_eq!(x.edges[2].unwrap().parent,0);
    let m=mask_for(&p,&l,&[disable(0,0,1),disable(1,0,3)]);
    assert!(execute(&p,&l,&m,&specs(),100).unwrap().1.edges.iter().all(Option::is_none));
}
#[test]fn masked_cursive_mark_joint_coordinates_and_budget(){
    let (l,n)=ledger(&[3,7,3]);let gd=gdef_bytes(n);
    let curs=vec![1,14,2,0,22,28,0,1,2,3,7,1,100,20,1,65506,5];
    let b=planned_fixture([&[0],&[1]],&[(3,0,curs),(4,0,base_sub())]);let p=planned(&b,n,Some(&gd));
    let all=mask_for(&p,&l,&[]);
    let x=execute(&p,&l,&all,&specs(),100).unwrap().1;
    assert_eq!(x.edges[1].unwrap().parent,0);assert_eq!(x.edges[2].unwrap().parent,1);
    assert_eq!(x.design_origins[1].0,130);assert_eq!(x.adjustments[2].design.y_placement,195);
    assert_eq!(x.design_origins[2].0+x.adjustments[2].design.x_placement,220);
    let m=mask_for(&p,&l,&[disable(0,1,2)]);
    let x=execute(&p,&l,&m,&specs(),100).unwrap().1;
    assert!(x.edges[1].is_none());assert_eq!(x.edges[2].unwrap().parent,1);
    assert_eq!(x.design_origins[1].0,100);assert_eq!(x.adjustments[2].design.y_placement,180);
    let ids=[3,7,3];let prior=[GposRunAdjustment::default();3];
    let limits=GposMixedRunLimits{marks:GposMarkRunLimits{context:GposContextRunLimits{max_context_work:0,..Default::default()},..Default::default()},..Default::default()};
    assert!(matches!(execute_staged_gpos_joint(&p,&l,&all,&specs(),Default::default(),100,&ids,&prior,
        &[(100,0);3],&[(0,0);3],&[GposLigatureSource::Unmapped;3],GposRunMode::DesignOnly,
        GposHorizontalFlow::LeftToRight,limits,Default::default(),false),Err(Error::Limit(_))));
    assert_eq!(prior,[GposRunAdjustment::default();3]);
}
