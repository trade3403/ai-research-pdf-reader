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

fn face_options()->FaceShapeOptions<'static>{
    FaceShapeOptions{script:*b"latn",language:None,substitution_tags:&[],
        positioning_tags:&[*b"kern"],alternates:&[],flow:LTR,
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
    let mut o=face_options();o.substitution_tags=&[*b"ZZZZ"];o.positioning_tags=&[*b"kern",*b"ZZZZ"];
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
