use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
fn fixture(kind:u16,sub:&[u16])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,1,4,kind,0,1,8];w.extend_from_slice(sub);
    w.iter().flat_map(|v|v.to_be_bytes()).collect()
}
fn empty()->Vec<u8>{[1u16,0,10,12,14,0,0,0].iter().flat_map(|v|v.to_be_bytes()).collect()}
fn single(a:u16,z:u16)->Vec<u8>{fixture(1,&[2,8,1,z,1,1,a])}
fn multiple(a:u16)->Vec<u8>{fixture(2,&[1,8,1,14,1,1,a,2,11,12])}
fn ligature(a:u16,b:u16)->Vec<u8>{fixture(4,&[1,8,1,14,1,1,a,1,4,20,2,b])}
fn position(id:u16,delta:i16)->Vec<u8>{fixture(1,&[1,8,4,delta as u16,1,1,id])}
fn inputs(text:&str)->(SourceRun,FontMetrics<'static>,u32){
    let f=parse_font(FONT,Default::default()).unwrap();
    let face=f.face(0).unwrap();
    let metrics=parse_font_metrics(face,Default::default()).unwrap();
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    (map_source_run(text,&cmap,false,Default::default()).unwrap(),metrics,u32::from(cmap.glyph_count()))
}
fn run(s:&[u8],p:&[u8],input:&SourceRun,m:&FontMetrics<'_>,n:u32,flow:GposHorizontalFlow,l:JointShapeLimits)->Result<JointShapeResult,Error>{
    let sr=if s.len()==16{&[][..]}else{&[0u16][..]};
    let pr=if p.len()==16{&[][..]}else{&[0u16][..]};
    let gs=compile_gsub_program(s,sr,n,None,Default::default()).unwrap();
    let gp=compile_gpos_program(p,pr,n,None,Default::default()).unwrap();
    execute_joint_shape(&gs,&gp,input,m,&[],flow,l)
}
const LTR:GposHorizontalFlow=GposHorizontalFlow::LeftToRight;
const RTL:GposHorizontalFlow=GposHorizontalFlow::RightToLeft;
#[test]fn replacement_uses_new_metrics_and_positioning(){
    let (r,m,n)=inputs("AA");let a=r.glyphs()[0].glyph_id;
    let z=86;assert_ne!(a,z);
    let out=run(&single(a,z),&position(z,17),&r,&m,n,LTR,Default::default()).unwrap();
    assert_eq!(out.substitution.expansion.execution.glyphs,vec![z,z]);
    assert_eq!(out.design_advances,vec![(585,0);2]);
    assert_eq!(out.positioning.design_origins,vec![(0,0),(602,0)]);
    assert_eq!(out.positioning.adjustments[1].design.x_advance,17);
    assert_eq!(out.provenance.glyphs()[1].sources(),&[1]);
}
#[test]fn expansion_provenance_and_post_metrics(){
    let (r,m,n)=inputs("A");let a=r.glyphs()[0].glyph_id;
    let out=run(&multiple(a),&position(12,-9),&r,&m,n,LTR,Default::default()).unwrap();
    assert_eq!(out.substitution.expansion.execution.glyphs,vec![11,12]);
    assert_eq!(out.design_advances,vec![(i32::from(m.glyph(11).unwrap().advance_width),0),(i32::from(m.glyph(12).unwrap().advance_width),0)]);
    assert_eq!(out.provenance.glyphs()[0].sources(),&[0]);
    assert_eq!(out.provenance.glyphs()[1].sources(),&[0]);
    assert_eq!(out.positioning.adjustments[1].design.x_advance,-9);
}
#[test]fn contraction_provenance_and_rtl(){
    let (r,m,n)=inputs("ABAB");let a=r.glyphs()[0].glyph_id;let b=r.glyphs()[1].glyph_id;
    let out=run(&ligature(a,b),&position(20,10),&r,&m,n,RTL,Default::default()).unwrap();
    let w=i32::from(m.glyph(20).unwrap().advance_width)+10;
    assert_eq!(out.substitution.expansion.execution.glyphs,vec![20,20]);
    assert_eq!(out.provenance.glyphs()[0].sources(),&[0,1]);
    assert_eq!(out.provenance.glyphs()[1].sources(),&[2,3]);
    assert_eq!(out.positioning.design_origins,vec![(-w,0),(-2*w,0)]);
}
#[test]fn empty_roots_preserve_source(){
    let (r,m,n)=inputs("A");let out=run(&empty(),&empty(),&r,&m,n,LTR,Default::default()).unwrap();
    assert_eq!(out.design_advances,vec![(668,0)]);
    assert_eq!(out.provenance.glyphs()[0].sources(),&[0]);
    assert_eq!(out.positioning.adjustments,vec![GposRunAdjustment::default()]);
}
#[test]fn empty_text(){
    let (r,m,n)=inputs("");let out=run(&empty(),&empty(),&r,&m,n,LTR,Default::default()).unwrap();
    assert!(out.positioning.adjustments.is_empty());assert!(out.provenance.glyphs().is_empty());
}
#[test]fn domain_mismatch(){
    let (r,m,n)=inputs("A");let b=empty();
    let gs=compile_gsub_program(&b,&[],n,None,Default::default()).unwrap();
    let gp=compile_gpos_program(&b,&[],n-1,None,Default::default()).unwrap();
    assert!(matches!(execute_joint_shape(&gs,&gp,&r,&m,&[],LTR,Default::default()),Err(Error::Invalid(_))));
}
#[test]fn output_cap_rejects_expansion_atomically(){
    let (r,m,n)=inputs("A");let before=r.clone();
    let l=JointShapeLimits{max_output_glyphs:1,..Default::default()};
    assert!(matches!(run(&multiple(r.glyphs()[0].glyph_id),&empty(),&r,&m,n,LTR,l),Err(Error::Limit(_))));
    assert_eq!(r,before);
}
#[test]fn native_font_feature_pipeline(){
    let (r,m,n)=inputs("AVATAR");let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let sb=face.table(*b"GSUB").unwrap();let pb=face.table(*b"GPOS").unwrap();
    let gd=face.table(*b"GDEF").map(|x|parse_gdef(x,n,Default::default()).unwrap());
    let sl=parse_gsub_layout(sb,Default::default()).unwrap();let pl=parse_gpos_layout(pb,Default::default()).unwrap();
    let sp=build_gsub_base_plan(&sl,*b"latn",None,&[*b"liga"],Default::default()).unwrap().unwrap();
    let pp=build_gpos_base_plan(&pl,*b"latn",None,&[*b"kern"],Default::default()).unwrap().unwrap();
    let gs=compile_gsub_plan_program(sb,&sp,n,gd.as_ref(),Default::default()).unwrap();
    let gp=compile_gpos_plan_program(pb,&pp,n,gd.as_ref(),Default::default()).unwrap();
    let out=execute_joint_shape(&gs,&gp,&r,&m,&[],LTR,Default::default()).unwrap();
    assert_eq!(out.positioning.adjustments.len(),out.provenance.glyphs().len());
    assert!(out.positioning.adjustments.iter().any(|a|a.design.x_advance!=0));
}

static FACE_KERN_TAGS:[[u8;4];1]=[*b"kern"];
static FACE_UNKNOWN_TAGS:[[u8;4];1]=[*b"ZZZZ"];
static FACE_KERN_UNKNOWN_TAGS:[[u8;4];2]=[*b"kern",*b"ZZZZ"];
fn face_options()->FaceShapeOptions<'static>{
    FaceShapeOptions{script:*b"latn",language:None,substitution_tags:&[],
        positioning_tags:&FACE_KERN_TAGS,alternates:&[],flow:LTR,
        allow_missing_glyphs:false,require_requested_features:true}
}
#[test]fn same_face_native_text_to_positioned_run(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let out=shape_font_face(face,"AVATAR",face_options(),Default::default()).unwrap();
    assert_eq!(out.source.scalars().len(),6);
    assert_eq!(out.units_per_em,parse_font_metrics(face,Default::default()).unwrap().head().units_per_em);
    assert_eq!(out.positioning_plan.script,*b"latn");
    assert!(out.positioning_plan.unavailable_tags.is_empty());
    assert!(out.shaped.positioning.adjustments.iter().any(|a|a.design.x_advance!=0));
    let m=parse_font_metrics(face,Default::default()).unwrap();
    for (g,w) in out.shaped.provenance.glyphs().iter().zip(&out.shaped.design_advances){
        assert_eq!(*w,(i32::from(m.glyph(g.glyph_id).unwrap().advance_width),0));
    }
}
#[test]fn same_face_reports_unavailable_tags_or_rejects(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let mut o=face_options();o.substitution_tags=&FACE_UNKNOWN_TAGS;o.positioning_tags=&FACE_KERN_UNKNOWN_TAGS;
    assert!(matches!(shape_font_face(face,"AV",o,Default::default()),Err(Error::Unsupported(_))));
    o.require_requested_features=false;
    let out=shape_font_face(face,"AV",o,Default::default()).unwrap();
    assert_eq!(out.substitution_plan.unavailable_tags,vec![*b"ZZZZ"]);
    assert_eq!(out.positioning_plan.unavailable_tags,vec![*b"ZZZZ"]);
}
#[test]fn same_face_missing_glyph_policy(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let mut o=face_options();let text="A\u{10ffff}";
    assert!(matches!(shape_font_face(face,text,o,Default::default()),Err(Error::Invalid(_))));
    o.allow_missing_glyphs=true;
    let out=shape_font_face(face,text,o,Default::default()).unwrap();
    assert!(out.source.glyphs()[1].missing);
    assert_eq!(out.source.glyphs()[1].glyph_id,0);
    assert_eq!(out.source.byte_range(1,2).unwrap(),1..5);
}
#[test]fn same_face_early_source_and_tag_limits(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let mut l=FaceShapeLimits::default();l.source.max_bytes=1;
    assert!(matches!(shape_font_face(face,"AV",face_options(),l),Err(Error::Limit(_))));
    let mut l=FaceShapeLimits::default();l.positioning_plan.max_requested_tags=0;
    assert!(matches!(shape_font_face(face,"A",face_options(),l),Err(Error::Limit(_))));
    let mut l=FaceShapeLimits::default();l.joint.max_output_glyphs=1;
    assert!(matches!(shape_font_face(face,"AV",face_options(),l),Err(Error::Limit(_))));
}
#[test]fn same_face_empty_text_and_empty_selection(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let mut o=face_options();o.positioning_tags=&[];
    let out=shape_font_face(face,"",o,Default::default()).unwrap();
    assert!(out.source.glyphs().is_empty());
    assert!(out.shaped.positioning.adjustments.is_empty());
}
#[test]fn same_face_rtl_nominal_pen_without_optional_features(){
    let font=parse_font(FONT,Default::default()).unwrap();let face=font.face(0).unwrap();
    let mut o=face_options();o.positioning_tags=&[];o.flow=RTL;
    let out=shape_font_face(face,"AA",o,Default::default()).unwrap();
    let mut x=0;
    for ((w,a),origin) in out.shaped.design_advances.iter().zip(&out.shaped.positioning.adjustments).zip(&out.shaped.positioning.design_origins){
        x-=w.0+a.design.x_advance;assert_eq!(origin.0,x);
    }
}

// Two independent GSUB features, each one direct lookup: A -> 11 -> 12.
// Build actual ScriptList/FeatureList/LookupList bytes with two stages.
fn stage_fixture(a:u16)->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,2,0,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,2);
    for (tag,rel) in [(b"ccmp",14),(b"liga",20)]{b.extend(tag);w(&mut b,rel);}
    for v in [0,1,0,0,1,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,8,at);for v in [2,6,28]{w(&mut b,v);}
    for (from,to) in [(a,11),(11,12)]{
        for v in [1,0,1,8,2,8,1,to,1,1,from]{w(&mut b,v);}
    }
    b
}
fn stage_program(b:&[u8],n:u32)->CompiledGsubProgram<'_>{
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    let p=build_gsub_base_plan(&layout,*b"DFLT",None,&[*b"ccmp",*b"liga"],Default::default()).unwrap().unwrap();
    compile_gsub_plan_program(b,&p,n,None,Default::default()).unwrap()
}
fn stages()->[ShapeStageSpec;2]{[ShapeStageSpec{features:vec![0]},ShapeStageSpec{features:vec![1]}]}
#[test]fn staged_joint_order_and_final_metrics(){
    let (r,m,n)=inputs("AA");let sb=stage_fixture(r.glyphs()[0].glyph_id);let gs=stage_program(&sb,n);
    let pb=position(12,17);let gp=compile_gpos_program(&pb,&[0],n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(gs.validated_plan().unwrap(),2,&[],Default::default()).unwrap();
    let x=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&stages(),LTR,Default::default(),Default::default(),None).unwrap();
    assert_eq!(x.shaped.substitution.expansion.execution.glyphs,vec![12,12]);
    assert_eq!(x.trace.iter().map(|t|t.changes).collect::<Vec<_>>(),vec![2,2]);
    assert_eq!(x.shaped.design_advances,vec![(i32::from(m.glyph(12).unwrap().advance_width),0);2]);
    assert!(x.shaped.positioning.adjustments.iter().all(|a|a.design.x_advance==17));
    let mut rev=stages();rev.reverse();
    let y=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&rev,LTR,Default::default(),Default::default(),None).unwrap();
    assert_eq!(y.shaped.substitution.expansion.execution.glyphs,vec![11,11]);
    assert!(y.shaped.positioning.adjustments.iter().all(|a|a.design.x_advance==0));
}
#[test]fn staged_joint_mask_range_and_gpos_namespace(){
    let (r,m,n)=inputs("AA");let sb=stage_fixture(r.glyphs()[0].glyph_id);let gs=stage_program(&sb,n);
    let pb=position(11,19);let gp=compile_gpos_program(&pb,&[0],n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(gs.validated_plan().unwrap(),2,
        &[FeatureMaskOverride{feature:1,start:1,end:2,enabled:false}],Default::default()).unwrap();
    let x=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&stages(),RTL,Default::default(),Default::default(),None).unwrap();
    assert_eq!(x.shaped.substitution.expansion.execution.glyphs,vec![12,11]);
    assert_eq!(x.glyph_masks,vec![3,1]);assert!(x.skipped_roots>0);
    assert_eq!(x.shaped.positioning.adjustments[1].design.x_advance,19);
    assert_eq!(x.shaped.provenance.glyphs()[1].sources(),&[1]);
}
#[test]fn staged_joint_limits_and_partition(){
    let (r,m,n)=inputs("AA");let sb=stage_fixture(r.glyphs()[0].glyph_id);let gs=stage_program(&sb,n);
    let pb=empty();let gp=compile_gpos_program(&pb,&[],n,None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(gs.validated_plan().unwrap(),2,&[],Default::default()).unwrap();
    let x=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&stages(),LTR,Default::default(),Default::default(),None).unwrap();
    let before=r.clone();
    assert!(matches!(execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&stages(),LTR,Default::default(),
        JointStageLimits{max_mask_work:x.mask_work-1,..Default::default()},None),Err(Error::Limit(_))));
    assert!(matches!(execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&stages()[..1],LTR,Default::default(),Default::default(),None),Err(Error::Invalid(_))));
    assert_eq!(r,before);
}
#[test]fn staged_joint_native_trace_opt_in(){
    let (r,m,n)=inputs("AVATAR");let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let sb=face.table(*b"GSUB").unwrap();let pb=face.table(*b"GPOS").unwrap();
    let gd=parse_gdef(face.table(*b"GDEF").unwrap(),n,Default::default()).unwrap();
    let sl=parse_gsub_layout(sb,Default::default()).unwrap();let pl=parse_gpos_layout(pb,Default::default()).unwrap();
    let sp=build_gsub_base_plan(&sl,*b"latn",None,&[*b"liga"],Default::default()).unwrap().unwrap();
    let pp=build_gpos_base_plan(&pl,*b"latn",None,&[*b"kern"],Default::default()).unwrap().unwrap();
    let gs=compile_gsub_plan_program(sb,&sp,n,Some(&gd),Default::default()).unwrap();
    let gp=compile_gpos_plan_program(pb,&pp,n,Some(&gd),Default::default()).unwrap();
    let specs=if sp.features.is_empty(){vec![]}else{vec![ShapeStageSpec{features:sp.features.clone()}]};
    let masks=ScalarFeatureMasks::build(&sp,r.scalars().len(),&[],Default::default()).unwrap();
    let x=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&masks,&specs,LTR,Default::default(),Default::default(),Some((&gd,Default::default()))).unwrap();
    let y=execute_joint_shape_with_ligatures(&gs,&gp,&r,&m,&gd,&[],LTR,Default::default(),Default::default()).unwrap();
    assert_eq!(x.shaped.positioning,y.positioning);
    assert_eq!(x.shaped.substitution,y.substitution);
    assert_eq!(x.shaped.provenance.ligature_trace_stats().unwrap().0,0);
    assert_eq!(x.trace.iter().map(|t|t.changes).sum::<usize>(),x.shaped.substitution.expansion.execution.changes);
}

fn dual_gpos_fixture(a:u16)->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,2,0,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,2);
    for (tag,rel) in [(b"kern",14),(b"mark",20)]{b.extend(tag);w(&mut b,rel);}
    for v in [0,1,0,0,1,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,8,at);for v in [2,6,28]{w(&mut b,v);}
    for (id,delta) in [(a,7),(11,13)]{for v in [1,0,1,8,1,8,4,delta,1,1,id]{w(&mut b,v);}}
    b
}
fn dual_gp(b:&[u8],n:u32)->CompiledGposProgram<'_>{
    let l=parse_gpos_layout(b,Default::default()).unwrap();
    let p=build_gpos_base_plan(&l,*b"DFLT",None,&[*b"kern",*b"mark"],Default::default()).unwrap().unwrap();
    compile_gpos_plan_program(b,&p,n,None,Default::default()).unwrap()
}
fn dual_run(r:&SourceRun,m:&FontMetrics<'_>,n:u32,se:&[FeatureMaskOverride],pe:&[FeatureMaskOverride],
    ps:&[ShapeStageSpec],pl:JointStageLimits)->Result<DualStageShapeResult,Error>{
    let sb=stage_fixture(r.glyphs()[0].glyph_id);let gs=stage_program(&sb,n);
    let pb=dual_gpos_fixture(12);let gp=dual_gp(&pb,n);
    let sm=ScalarFeatureMasks::build(gs.validated_plan().unwrap(),r.scalars().len(),se,Default::default())?;
    let pm=GposScalarFeatureMasks::build(gp.validated_plan().unwrap(),r.scalars().len(),pe,Default::default())?;
    execute_dual_stage_joint_shape(&gs,&gp,r,m,&[],&sm,&stages(),LTR,Default::default(),Default::default(),
        None,JointPositionStages{masks:&pm,specs:ps,limits:pl})
}
fn dual_disable(feature:u16,start:usize,end:usize)->FeatureMaskOverride{
    FeatureMaskOverride{feature,start,end,enabled:false}
}
#[test]fn dual_stage_independent_masks_post_substitution(){
    let (r,m,n)=inputs("AAA");
    let x=dual_run(&r,&m,n,&[dual_disable(1,1,2)],&[dual_disable(0,2,3)],&stages(),Default::default()).unwrap();
    assert_eq!(x.staged.shaped.substitution.expansion.execution.glyphs,vec![12,11,12]);
    assert_eq!(x.staged.glyph_masks,vec![3,1,3]);
    assert_eq!(x.staged.shaped.positioning.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![7,13,0]);
    assert_eq!(x.positioning_schedule.root_count(),2);
    assert_eq!(x.staged.shaped.design_advances.iter().map(|p|p.0).collect::<Vec<_>>(),
        [12,11,12].iter().map(|id|i32::from(m.glyph(*id).unwrap().advance_width)).collect::<Vec<_>>());
}
#[test]fn dual_stage_position_budget_partition_and_atomicity(){
    let (r,m,n)=inputs("AA");let before=r.clone();
    assert!(matches!(dual_run(&r,&m,n,&[],&[],&stages()[..1],Default::default()),Err(Error::Invalid(_))));
    assert!(matches!(dual_run(&r,&m,n,&[],&[],&stages(),JointStageLimits{max_mask_work:3,..Default::default()}),Err(Error::Limit(_))));
    assert!(dual_run(&r,&m,n,&[],&[],&stages(),JointStageLimits{max_mask_work:4,..Default::default()}).is_ok());
    assert_eq!(r,before);
}
#[test]fn dual_stage_reordered_position_stages(){
    let (r,m,n)=inputs("AA");let mut ps=stages();ps.reverse();
    let x=dual_run(&r,&m,n,&[],&[],&ps,Default::default()).unwrap();
    assert_eq!(x.positioning_schedule.stages()[0].features(),&[1]);
    assert_eq!(x.staged.schedule.stages()[0].features(),&[0]);
    assert_eq!(x.staged.shaped.positioning.adjustments[0].design.x_advance,7);
}
fn face_stage_request(specs:&[ShapeStageSpec])->FaceStageRequest<'_>{
    FaceStageRequest{specs,overrides:&[],masks:Default::default(),stages:Default::default()}
}
#[test]fn dual_stage_same_face_native_strict_parity(){
    let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let o=face_options();let base=shape_font_face_with_ligatures(face,"AVATAR",o,Default::default(),Default::default()).unwrap();
    let ss=if base.substitution_plan.features.is_empty(){vec![]}else{vec![ShapeStageSpec{features:base.substitution_plan.features.clone()}]};
    let ps=vec![ShapeStageSpec{features:base.positioning_plan.features.clone()}];
    let x=shape_font_face_dual_stage(face,"AVATAR",o,Default::default(),face_stage_request(&ss),face_stage_request(&ps),Some(Default::default())).unwrap();
    assert_eq!(x.source,base.source);
    assert_eq!(x.shaped.staged.shaped.positioning,base.shaped.positioning);
    assert_eq!(x.shaped.staged.shaped.substitution,base.shaped.substitution);
    assert!(x.shaped.staged.shaped.positioning.adjustments.iter().any(|a|a.design.x_advance!=0));
    assert_eq!(x.shaped.staged.shaped.provenance.ligature_trace_stats().unwrap().0,0);
}
#[test]fn dual_stage_same_face_empty_plans(){
    let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let mut o=face_options();o.positioning_tags=&[];
    let base=shape_font_face(face,"",o,Default::default()).unwrap();
    let specs=|f:&[u16]|if f.is_empty(){vec![]}else{vec![ShapeStageSpec{features:f.to_vec()}]};
    let ss=specs(&base.substitution_plan.features);let ps=specs(&base.positioning_plan.features);
    let x=shape_font_face_dual_stage(face,"",o,Default::default(),face_stage_request(&ss),face_stage_request(&ps),None).unwrap();
    assert!(x.shaped.staged.shaped.positioning.adjustments.is_empty());
    assert!(x.shaped.staged.shaped.provenance.glyphs().is_empty());
}

// Explicit one-feature plan around the existing independently tested
// lookup fixtures. Feature indices deliberately coincide across tables.
fn dual_one_feature(tag:[u8;4],kind:u16,flag:u16,sub:&[u16])->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,1,0]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,1);b.extend(tag);w(&mut b,8);
    for v in [0,1,0]{w(&mut b,v);}
    let at=b.len();set(&mut b,8,at);
    for v in [1,4,kind,flag,1,8]{w(&mut b,v);}
    for &v in sub{w(&mut b,v);}
    b
}
fn dual_edit_run(sb:&[u8],pb:&[u8],r:&SourceRun,m:&FontMetrics<'_>,n:u32,
    se:&[FeatureMaskOverride],pe:&[FeatureMaskOverride],gd:Option<&Gdef>,
    flow:GposHorizontalFlow,limits:JointShapeLimits,projection:usize)->Result<DualStageShapeResult,Error>{
    let sl=parse_gsub_layout(sb,Default::default())?;
    let pl=parse_gpos_layout(pb,Default::default())?;
    let sp=build_gsub_base_plan(&sl,*b"DFLT",None,&[*b"liga"],Default::default())?.unwrap();
    let pp=build_gpos_base_plan(&pl,*b"DFLT",None,&[*b"kern"],Default::default())?.unwrap();
    let gs=compile_gsub_plan_program(sb,&sp,n,gd,Default::default())?;
    let gp=compile_gpos_plan_program(pb,&pp,n,gd,Default::default())?;
    let sm=ScalarFeatureMasks::build(&sp,r.scalars().len(),se,Default::default())?;
    let pm=GposScalarFeatureMasks::build(&pp,r.scalars().len(),pe,Default::default())?;
    let ss=[ShapeStageSpec{features:vec![0]}];
    execute_dual_stage_joint_shape(&gs,&gp,r,m,&[],&sm,&ss,flow,limits,Default::default(),
        gd.map(|g|(g,JointLigatureLimits::default())),JointPositionStages{
            masks:&pm,specs:&ss,limits:JointStageLimits{max_mask_work:projection,..Default::default()}})
}
#[test]fn dual_expansion_inherits_position_masks(){
    let (r,m,n)=inputs("AA");let a=r.glyphs()[0].glyph_id;
    let sb=dual_one_feature(*b"liga",2,0,&[1,8,1,14,1,1,a,2,11,12]);
    // Same advance adjustment for both descendants.
    let pb=dual_one_feature(*b"kern",1,0,&[1,8,4,9,1,2,11,12]);
    for flow in [LTR,RTL]{
        let x=dual_edit_run(&sb,&pb,&r,&m,n,&[],&[dual_disable(0,1,2)],None,flow,Default::default(),100).unwrap().staged.shaped;
        assert_eq!(x.substitution.expansion.execution.glyphs,vec![11,12,11,12]);
        assert_eq!(x.provenance.glyphs().iter().map(|g|g.sources()[0]).collect::<Vec<_>>(),vec![0,0,1,1]);
        assert_eq!(x.positioning.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![9,9,0,0]);
        assert_eq!(x.design_advances[1].0,i32::from(m.glyph(12).unwrap().advance_width));
    }
}
#[test]fn dual_contraction_intersects_position_masks(){
    let (r,m,n)=inputs("ABAB");let a=r.glyphs()[0].glyph_id;let b=r.glyphs()[1].glyph_id;
    let sb=dual_one_feature(*b"liga",4,0,&[1,8,1,14,1,1,a,1,4,20,2,b]);
    let pb=dual_one_feature(*b"kern",1,0,&[1,8,4,10,1,1,20]);
    for flow in [LTR,RTL]{
        let x=dual_edit_run(&sb,&pb,&r,&m,n,&[],&[dual_disable(0,1,2)],None,flow,Default::default(),100).unwrap().staged.shaped;
        assert_eq!(x.substitution.expansion.execution.glyphs,vec![20,20]);
        assert_eq!(x.provenance.glyphs()[0].sources(),&[0,1]);
        assert_eq!(x.provenance.glyphs()[1].sources(),&[2,3]);
        assert_eq!(x.positioning.adjustments.iter().map(|a|a.design.x_advance).collect::<Vec<_>>(),vec![0,10]);
        let w=i32::from(m.glyph(20).unwrap().advance_width);
        assert_eq!(x.positioning.design_origins,if flow==LTR{vec![(0,0),(w,0)]}else{vec![(-w,0),(-2*w-10,0)]});
    }
}
#[test]fn dual_contraction_strict_gsub_mask_independent(){
    let (r,m,n)=inputs("ABAB");let a=r.glyphs()[0].glyph_id;let b=r.glyphs()[1].glyph_id;
    let sb=dual_one_feature(*b"liga",4,0,&[1,8,1,14,1,1,a,1,4,20,2,b]);
    let pb=dual_one_feature(*b"kern",1,0,&[1,8,4,10,1,1,20]);
    let x=dual_edit_run(&sb,&pb,&r,&m,n,&[dual_disable(0,1,2)],&[],None,LTR,Default::default(),100).unwrap().staged;
    assert_eq!(x.shaped.substitution.expansion.execution.glyphs,vec![a,b,20]);
    assert_eq!(x.shaped.positioning.adjustments[2].design.x_advance,10);
    assert!(x.blocked_inputs>0);
}
#[test]fn dual_expansion_final_projection_and_output_caps(){
    let (r,m,n)=inputs("A");let a=r.glyphs()[0].glyph_id;let before=r.clone();
    let sb=dual_one_feature(*b"liga",2,0,&[1,8,1,14,1,1,a,2,11,12]);
    let pb=dual_one_feature(*b"kern",1,0,&[1,8,4,9,1,1,12]);
    assert!(matches!(dual_edit_run(&sb,&pb,&r,&m,n,&[],&[],None,LTR,Default::default(),3),Err(Error::Limit(_))));
    assert!(dual_edit_run(&sb,&pb,&r,&m,n,&[],&[],None,LTR,Default::default(),4).is_ok());
    assert!(matches!(dual_edit_run(&sb,&pb,&r,&m,n,&[],&[],None,LTR,
        JointShapeLimits{max_output_glyphs:1,..Default::default()},100),Err(Error::Limit(_))));
    assert_eq!(r,before);
}
#[test]fn dual_strict_component_anchors_and_masked_parent(){
    let (r,m,n)=inputs("ACBC");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;let b=r.glyphs()[2].glyph_id;
    let mut records=[(20u16,2u16),(c,3)];records.sort_unstable();
    let mut w=vec![1u16,0,12,0,0,0,2,2];for (g,k) in records{w.extend([g,g,k]);}
    let bytes:Vec<_>=w.iter().flat_map(|v|v.to_be_bytes()).collect();
    let gd=parse_gdef(&bytes,n,Default::default()).unwrap();
    let sb=dual_one_feature(*b"liga",4,LOOKUP_IGNORE_MARK,&[1,8,1,14,1,1,a,1,4,20,2,b]);
    let pb=dual_one_feature(*b"kern",5,0,&[1,12,18,1,24,36,1,1,c,1,1,20,1,0,6,1,10,20,
        1,4,2,6,12,1,100,200,1,300,400]);
    for flow in [LTR,RTL]{
        let x=dual_edit_run(&sb,&pb,&r,&m,n,&[],&[],Some(&gd),flow,Default::default(),100).unwrap().staged.shaped;
        assert_eq!(x.substitution.expansion.execution.glyphs,vec![20,c,c]);
        assert_eq!(x.provenance.glyphs()[0].ligature_lineage().unwrap().components(),&[vec![0],vec![2]]);
        for (i,delta) in [(1,(90,180)),(2,(290,380))]{
            let edge=x.positioning.edges[i].unwrap();assert_eq!(edge.parent,0);assert_eq!(edge.design_delta,delta);
            let o=x.positioning.design_origins[i];let adj=x.positioning.adjustments[i].design;
            assert_eq!((o.0+adj.x_placement,o.1+adj.y_placement),
                (x.positioning.design_origins[0].0+delta.0,x.positioning.design_origins[0].1+delta.1));
        }
        let y=dual_edit_run(&sb,&pb,&r,&m,n,&[],&[dual_disable(0,2,3)],Some(&gd),flow,Default::default(),100).unwrap().staged.shaped;
        assert!(y.positioning.edges.iter().all(Option::is_none));
    }
}
