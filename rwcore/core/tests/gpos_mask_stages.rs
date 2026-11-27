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
