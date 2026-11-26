use rwcore::*;
fn native()->(&'static [u8], &'static str){
    (include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf"),"Aé office")
}
fn check(disable:bool){
    let (font,text)=native();
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();
    let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"case",*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&[])};
    let plan=f.build_plan(req).unwrap().unwrap();let p=f.compile_plan(req,&plan).unwrap();
    let edits:Vec<_>=plan.plan.features.iter().copied().filter(|id|Some(*id)!=plan.plan.required_feature&&disable).map(|feature|FeatureMaskOverride{feature,start:0,end:run.scalars().len(),enabled:false}).collect();
    let masks=ScalarFeatureMasks::build(&plan.plan,run.scalars().len(),&edits,Default::default()).unwrap();
    let result=execute_gsub_program_with_masks(&p,&run,&[],&masks,Default::default(),Default::default(),100000).unwrap();
    let (old,ledger)=execute_gsub_program_with_provenance(&p,&run,&[],Default::default(),Default::default()).unwrap();
    if !disable{assert_eq!(result.execution,old);assert_eq!(result.provenance,ledger);assert_eq!(result.skipped_roots,0);}
    else{
        assert!(plan.plan.required_feature.is_none());
        assert_eq!(result.execution.expansion.execution.glyphs,run.glyphs().iter().map(|g|g.glyph_id).collect::<Vec<_>>());
        assert!(result.skipped_roots>0);
    }
    assert_eq!(result.glyph_masks,masks.project(&result.provenance,100000).unwrap());
    assert!(execute_gsub_program_with_masks(&p,&run,&[],&masks,Default::default(),Default::default(),result.mask_work-1).is_err());
    assert_eq!(result,execute_gsub_program_with_masks(&p,&run,&[],&masks,Default::default(),Default::default(),result.mask_work).unwrap());
}
#[test]fn masked_native_all_enabled(){check(false);}
#[test]fn masked_native_disabled(){check(true);}
#[test]fn masked_reject_unvalidated_roots(){
    let (font,text)=native();let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&[])};
    let plan=f.build_plan(req).unwrap().unwrap();
    let p=compile_gsub_program(sf.table(*b"GSUB").unwrap(),&[],f.glyph_count(),None,Default::default()).unwrap();
    let masks=ScalarFeatureMasks::build(&plan.plan,run.scalars().len(),&[],Default::default()).unwrap();
    assert!(execute_gsub_program_with_masks(&p,&run,&[],&masks,Default::default(),Default::default(),100000).is_err());
}

#[test]fn strict_native_variable_api_and_budget(){
    let (font,text)=native();let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let axis=f.fvar().axis(*b"wght").unwrap();
    for weight in [axis.min,axis.default,axis.max]{
        let tags=[(*b"wght",weight)];
        let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"case",*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&tags)};
        let (plan,result,blocked)=f.execute_with_strict_masks(req,&run,&[],&[],Default::default(),Default::default(),100000).unwrap().unwrap();
        let (old,ledger)=f.execute_with_provenance(req,&run,&[],Default::default()).unwrap().unwrap();
        assert_eq!(plan,old.plan);assert_eq!(result.execution,old.execution);assert_eq!(result.provenance,ledger);assert_eq!(blocked,0);
        assert_eq!(result.skipped_roots,0);
        let exact=result.mask_work;
        assert!(f.execute_with_strict_masks(req,&run,&[],&[],Default::default(),Default::default(),exact-1).is_err());
        assert_eq!((plan,result,blocked),f.execute_with_strict_masks(req,&run,&[],&[],Default::default(),Default::default(),exact).unwrap().unwrap());
    }
}
#[test]fn strict_native_disable_all_optional(){
    let (font,text)=native();let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"case",*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&[])};
    let plan=f.build_plan(req).unwrap().unwrap();assert!(plan.plan.required_feature.is_none());
    let edits:Vec<_>=plan.plan.features.iter().map(|&feature|FeatureMaskOverride{feature,start:0,end:run.scalars().len(),enabled:false}).collect();
    let (_,result,blocked)=f.execute_with_strict_masks(req,&run,&[],&edits,Default::default(),Default::default(),100000).unwrap().unwrap();
    assert_eq!(result.execution.expansion.execution.glyphs,run.glyphs().iter().map(|g|g.glyph_id).collect::<Vec<_>>());
    assert!(result.skipped_roots>0);assert_eq!(blocked,0);assert!(result.glyph_masks.iter().all(|&x|x==0));
}

#[test]fn native_tag_stage_single_group_parity(){
    let (font,text)=native();let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let axis=f.fvar().axis(*b"wght").unwrap();
    for weight in [axis.min,axis.default,axis.max]{
        let coords=[(*b"wght",weight)];
        let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"case",*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&coords)};
        let groups=[ShapeTagStage{tags:vec![*b"case",*b"rvrn"]}];
        let (plan,schedule,result,blocked)=f.execute_tag_stages(req,&run,&[],&groups,&[],Default::default(),Default::default(),Default::default(),100000).unwrap().unwrap();
        let (old,expected,k)=f.execute_with_strict_masks(req,&run,&[],&[],Default::default(),Default::default(),100000).unwrap().unwrap();
        assert_eq!(plan,old);assert_eq!(result,expected);assert_eq!(blocked,k);
        assert_eq!(schedule.stages().len(),1);
    }
}
#[test]fn native_tag_stage_multiple_groups_disable(){
    let (font,text)=native();let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let cmap=parse_unicode_cmap(sf,Default::default()).unwrap();let run=map_source_run(text,&cmap,false,Default::default()).unwrap();
    let f=parse_variable_gsub_face(sf,Default::default()).unwrap();
    let req=VariableGsubRequest{script:*b"latn",language:None,enabled:&[*b"case",*b"rvrn"],coordinates:VariationUserCoordinates::Tagged(&[])};
    let plan=f.build_plan(req).unwrap().unwrap();
    let groups=[ShapeTagStage{tags:vec![*b"rvrn"]},ShapeTagStage{tags:vec![*b"case"]}];
    let edits:Vec<_>=plan.plan.features.iter().map(|&feature|FeatureMaskOverride{feature,start:0,end:run.scalars().len(),enabled:false}).collect();
    let (_,schedule,result,blocked)=f.execute_tag_stages(req,&run,&[],&groups,&edits,Default::default(),Default::default(),Default::default(),100000).unwrap().unwrap();
    assert_eq!(result.execution.expansion.execution.glyphs,run.glyphs().iter().map(|g|g.glyph_id).collect::<Vec<_>>());
    assert_eq!(result.skipped_roots,schedule.root_count()*run.glyphs().len());assert_eq!(blocked,0);
    assert!(result.glyph_masks.iter().all(|&x|x==0));
}
