use rwcore::*;
const FONT:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
const TAGS:[[u8;4];4]=[*b"fina",*b"init",*b"isol",*b"medi"];
fn fixture(a:u16,required:bool,special:bool)->Vec<u8>{
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"arab");w(&mut b,8);
    for v in [4,0,0,if required{0}else{0xffff},4,0,1,2,3]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,4);
    for (i,tag) in TAGS.iter().enumerate(){b.extend(if i==0&&special{b"fin2"}else{tag});w(&mut b,26+6*i as u16);}
    for i in 0..4{for v in [0,1,i]{w(&mut b,v);}}
    let at=b.len();set(&mut b,8,at);for v in [4,10,32,54,76]{w(&mut b,v);}
    for to in 11..=14{for v in [1,0,1,8,2,8,1,to,1,1,a]{w(&mut b,v);}}
    b
}
fn inputs(t:&str)->(SourceRun,FontMetrics<'static>,u32){
    let f=parse_font(FONT,Default::default()).unwrap();let face=f.face(0).unwrap();
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    (map_source_run(t,&cmap,false,Default::default()).unwrap(),
        parse_font_metrics(face,Default::default()).unwrap(),u32::from(cmap.glyph_count()))
}
fn program(b:&[u8],n:u32,tags:&[[u8;4]])->(GsubLayout,CompiledGsubProgram<'static>){
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    let p=build_gsub_base_plan(&layout,*b"arab",None,tags,Default::default()).unwrap().unwrap();
    let gs=compile_gsub_plan_program(b,&p,n,None,Default::default()).unwrap();
    (layout,gs)
}
fn data()->UnicodeJoiningTypes{parse_joining_types("0041;D\n0042;R",Default::default()).unwrap()}
#[test]fn arabic_masks_and_actual_staged_joint_replacement(){
    let t="AAA A";let (r,m,n)=inputs(t);let a=r.glyphs()[0].glyph_id;let space=r.glyphs()[3].glyph_id;
    let b=fixture(a,false,false);let (layout,gs)=program(&b,n,&TAGS);
    let f=build_arabic_form_plan(&layout,&gs,&r,t,&data(),Default::default()).unwrap();
    assert_eq!(f.masks.scalar_masks(),&[2,8,1,0,4]);
    assert_eq!(f.specs.len(),1);
    let pb:Vec<u8>=[1u16,0,10,12,14,0,0,0].iter().flat_map(|v|v.to_be_bytes()).collect();
    let gp=compile_gpos_program(&pb,&[],n,None,Default::default()).unwrap();
    let out=execute_staged_joint_shape(&gs,&gp,&r,&m,&[],&f.masks,&f.specs,GposHorizontalFlow::RightToLeft,
        Default::default(),Default::default(),None).unwrap();
    assert_eq!(out.shaped.substitution.expansion.execution.glyphs,vec![12,14,11,space,13]);
    assert_eq!(out.glyph_masks,vec![2,8,1,0,4]);
    for (i,g) in out.shaped.provenance.glyphs().iter().enumerate(){assert_eq!(g.sources(),&[i]);}
    assert_eq!(out.shaped.design_advances[0].0,i32::from(m.glyph(12).unwrap().advance_width));
}
#[test]fn arabic_missing_optional_forms_not_synthesized(){
    let t="AAA";let (r,_,n)=inputs(t);let b=fixture(r.glyphs()[0].glyph_id,false,false);
    let (layout,gs)=program(&b,n,&[*b"init"]);
    let f=build_arabic_form_plan(&layout,&gs,&r,t,&data(),Default::default()).unwrap();
    assert_eq!(f.masks.scalar_masks(),&[1,0,0]);
}
#[test]fn arabic_source_identity_and_required_form(){
    let t="AA";let (r,_,n)=inputs(t);let a=r.glyphs()[0].glyph_id;
    let b=fixture(a,false,false);let (layout,gs)=program(&b,n,&TAGS);
    assert!(matches!(build_arabic_form_plan(&layout,&gs,&r,"AB",&data(),Default::default()),Err(Error::Invalid(_))));
    let b=fixture(a,true,false);let (layout,gs)=program(&b,n,&TAGS);
    assert!(matches!(build_arabic_form_plan(&layout,&gs,&r,t,&data(),Default::default()),Err(Error::Invalid(_))));
}
#[test]fn arabic_special_form_rejected(){
    let t="AA";let (r,_,n)=inputs(t);let b=fixture(r.glyphs()[0].glyph_id,false,true);
    let (layout,gs)=program(&b,n,&[*b"fin2"]);
    assert!(matches!(build_arabic_form_plan(&layout,&gs,&r,t,&data(),Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn arabic_form_work_and_override_limits(){
    let t="AAA A";let (r,_,n)=inputs(t);let b=fixture(r.glyphs()[0].glyph_id,false,false);
    let (layout,gs)=program(&b,n,&TAGS);
    for m in [FeatureMaskLimits{max_scalars:4,..Default::default()},
        FeatureMaskLimits{max_work:19,..Default::default()},FeatureMaskLimits{max_overrides:0,..Default::default()}]{
        assert!(matches!(build_arabic_form_plan(&layout,&gs,&r,t,&data(),ArabicFormLimits{masks:m,..Default::default()}),Err(Error::Limit(_))));
    }
    let (r,_,n)=inputs("");let (layout,gs)=program(&b,n,&TAGS);
    let f=build_arabic_form_plan(&layout,&gs,&r,"",&data(),Default::default()).unwrap();
    assert!(f.joining.is_empty()&&f.masks.scalar_masks().is_empty());
}
#[test]fn arabic_native_dejavu_forms_same_face(){
    // Supplemental Ubuntu CI font. Direct presentation-form cmap entries
    // are an independent glyph-ID oracle for these basic beh forms.
    let Some(path)=std::env::var_os("RWCORE_DEJAVU_REFERENCE")else{return;};
    let bytes=std::fs::read(path).unwrap();let f=parse_font(&bytes,Default::default()).unwrap();let face=f.face(0).unwrap();
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    let m=parse_font_metrics(face,Default::default()).unwrap();
    let tags=[*b"init",*b"medi",*b"fina"];
    let o=FaceShapeOptions{script:*b"arab",language:None,substitution_tags:&tags,positioning_tags:&[],
        alternates:&[],flow:GposHorizontalFlow::RightToLeft,allow_missing_glyphs:false,require_requested_features:true};
    let d=bundled_joining_types().unwrap();
    let result=shape_font_face_arabic_forms(face,"ببب",o,Default::default(),&d,Default::default(),
        Default::default(),Default::default(),None).unwrap();
    let expected:Vec<_>=['\u{fe91}','\u{fe92}','\u{fe90}'].iter().map(|&ch|cmap.glyph_for_char(ch).unwrap().unwrap()).collect();
    assert_eq!(result.face.shaped.staged.shaped.substitution.expansion.execution.glyphs,expected);
    assert_eq!(result.joining.iter().map(|s|s.form).collect::<Vec<_>>(),
        vec![ArabicJoiningForm::Initial,ArabicJoiningForm::Medial,ArabicJoiningForm::Final]);
    let shaped=&result.face.shaped.staged.shaped;
    for (i,&g) in expected.iter().enumerate(){assert_eq!(shaped.provenance.glyphs()[i].sources(),&[i]);
        assert_eq!(shaped.design_advances[i].0,i32::from(m.glyph(g).unwrap().advance_width));}
}
