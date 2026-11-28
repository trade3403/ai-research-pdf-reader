use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|v|v.to_be_bytes()).collect()}
fn fixture(kind:u16,flag:u16,sub:&[u16])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,1,4,kind,flag,1,8];w.extend_from_slice(sub);words(&w)
}
fn setup(text:&str)->(SourceRun,FontMetrics<'static>,u32){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();
    let face=f.face(0).unwrap();let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    (map_source_run(text,&cmap,false,Default::default()).unwrap(),
     parse_font_metrics(face,Default::default()).unwrap(),u32::from(cmap.glyph_count()))
}
fn gdef(n:u32,lig:u16,mark:u16)->Gdef{
    assert_ne!(lig,mark);
    let mut records=[(lig,2),(mark,3)];records.sort_unstable();
    let mut w=vec![1,0,12,0,0,0,2,2];
    for (g,c) in records{w.extend([g,g,c]);}
    parse_gdef(&words(&w),n,Default::default()).unwrap()
}
fn trace(r:&SourceRun,n:u32,gd:&Gdef,a:u16,b:u16)->ProvenanceLedger{
    let bytes=fixture(4,LOOKUP_IGNORE_MARK,&[1,8,1,14,1,1,a,1,4,20,2,b]);
    let p=compile_gsub_program(&bytes,&[0],n,Some(gd),Default::default()).unwrap();
    execute_gsub_program_with_ligature_trace(&p,r,&[],Default::default(),Default::default(),Default::default()).unwrap().1
}
#[test]fn maps_skipped_and_trailing_marks_to_distinct_components(){
    let (r,_,n)=setup("ACBCACBC");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;let b=r.glyphs()[2].glyph_id;
    let gd=gdef(n,20,c);let l=trace(&r,n,&gd,a,b);
    let got=map_gpos_ligature_sources(&l,&gd,Default::default()).unwrap();
    assert_eq!(got,vec![GposLigatureSource::Ligature{id:1,components:2},
        GposLigatureSource::Mark{id:1,component:0},GposLigatureSource::Mark{id:1,component:1},
        GposLigatureSource::Ligature{id:2,components:2},
        GposLigatureSource::Mark{id:2,component:0},GposLigatureSource::Mark{id:2,component:1}]);
}
#[test]fn contiguous_ligature_and_nonmark_boundary(){
    let (r,_,n)=setup("ABCD");let a=r.glyphs()[0].glyph_id;let b=r.glyphs()[1].glyph_id;let c=r.glyphs()[2].glyph_id;
    let gd=gdef(n,20,c);let l=trace(&r,n,&gd,a,b);
    assert_eq!(map_gpos_ligature_sources(&l,&gd,Default::default()).unwrap(),
        vec![GposLigatureSource::Ligature{id:1,components:2},
             GposLigatureSource::Mark{id:1,component:1},GposLigatureSource::Unmapped]);
}
#[test]fn refuses_legacy_and_untracked_ligature(){
    let (r,_,n)=setup("ACB");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;
    let gd=gdef(n,a,c);
    let l=ProvenanceLedger::from_run(&r,n,Default::default()).unwrap();
    assert!(matches!(map_gpos_ligature_sources(&l,&gd,Default::default()),Err(Error::Invalid(_))));
    let l=ProvenanceLedger::from_run_with_ligature_trace(&r,n,Default::default(),Default::default()).unwrap();
    assert!(matches!(map_gpos_ligature_sources(&l,&gd,Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn refuses_shared_sources_and_nonmark_gap(){
    let (r,_,n)=setup("ACB");let c=r.glyphs()[1].glyph_id;let gd=gdef(n,20,c);
    let mut l=ProvenanceLedger::from_run_with_ligature_trace(&r,n,Default::default(),Default::default()).unwrap();
    l.contract(&[0,2],20).unwrap();
    let other=gdef(n,20,r.glyphs()[2].glyph_id);
    assert!(matches!(map_gpos_ligature_sources(&l,&other,Default::default()),Err(Error::Unsupported(_))));
    l.expand(1,&[c,c]).unwrap();
    assert!(matches!(map_gpos_ligature_sources(&l,&gd,Default::default()),Err(Error::Unsupported(_))));
}
#[test]fn mapping_resource_limits(){
    let (r,_,n)=setup("ACB");let gd=gdef(n,20,r.glyphs()[1].glyph_id);
    let l=trace(&r,n,&gd,r.glyphs()[0].glyph_id,r.glyphs()[2].glyph_id);
    for limit in [LigatureMappingLimits{max_glyphs:1,..Default::default()},
                  LigatureMappingLimits{max_scalars:2,..Default::default()},
                  LigatureMappingLimits{max_work:1,..Default::default()}]{
        assert!(matches!(map_gpos_ligature_sources(&l,&gd,limit),Err(Error::Limit(_))));
    }
}
#[test]fn mapped_gsub_to_gpos_component_geometry(){
    let (r,m,n)=setup("ACBC");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;let b=r.glyphs()[2].glyph_id;
    let gd=gdef(n,20,c);let l=trace(&r,n,&gd,a,b);
    let sources=map_gpos_ligature_sources(&l,&gd,Default::default()).unwrap();
    let glyphs:Vec<_>=l.glyphs().iter().map(|g|g.glyph_id).collect();
    // MarkArray and LigatureAttach: one class, two component anchors.
    // Mark anchor (10,20), component anchors (100,200),(300,400).
    let sub=[1,12,18,1,24,36,1,1,c,1,1,20,1,0,6,1,10,20,
             1,4,2,6,12,1,100,200,1,300,400];
    let bytes=fixture(5,0,&sub);
    let p=compile_gpos_program(&bytes,&[0],n,Some(&gd),Default::default()).unwrap();
    let w:Vec<_>=glyphs.iter().map(|&g|(i32::from(m.glyph(g).unwrap().advance_width),0)).collect();
    for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
        let out=execute_gpos_mixed_joint(&p,&glyphs,&[GposRunAdjustment::default();3],&w,
            &[(0,0);3],&sources,GposRunMode::DesignOnly,flow,Default::default(),Default::default()).unwrap();
        for (i,delta) in [(1,(90,180)),(2,(290,380))]{
            let edge=out.edges[i].unwrap();assert_eq!(edge.parent,0);assert_eq!(edge.design_delta,delta);
            let o=out.design_origins[i];let a=out.adjustments[i].design;
            assert_eq!((o.0+a.x_placement,o.1+a.y_placement),
                (out.design_origins[0].0+delta.0,out.design_origins[0].1+delta.1));
        }
    }
}

fn mark_ligature_bytes(c:u16)->Vec<u8>{
    fixture(5,0,&[1,12,18,1,24,36,1,1,c,1,1,20,1,0,6,1,10,20,
             1,4,2,6,12,1,100,200,1,300,400])
}
#[test]fn opt_in_joint_ligature_pipeline(){
    let (r,m,n)=setup("ACBC");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;let b=r.glyphs()[2].glyph_id;
    let gd=gdef(n,20,c);
    let sb=fixture(4,LOOKUP_IGNORE_MARK,&[1,8,1,14,1,1,a,1,4,20,2,b]);
    let pb=mark_ligature_bytes(c);
    let gs=compile_gsub_program(&sb,&[0],n,Some(&gd),Default::default()).unwrap();
    let gp=compile_gpos_program(&pb,&[0],n,Some(&gd),Default::default()).unwrap();
    for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
        assert!(matches!(execute_joint_shape(&gs,&gp,&r,&m,&[],flow,Default::default()),Err(Error::Unsupported(_))));
        let x=execute_joint_shape_with_ligatures(&gs,&gp,&r,&m,&gd,&[],flow,Default::default(),Default::default()).unwrap();
        assert_eq!(x.substitution.expansion.execution.glyphs,vec![20,c,c]);
        assert_eq!(x.positioning.edges[1].unwrap().design_delta,(90,180));
        assert_eq!(x.positioning.edges[2].unwrap().design_delta,(290,380));
        assert_eq!(x.provenance.glyphs()[0].ligature_lineage().unwrap().id(),1);
        assert_eq!(x.design_advances[0].0,i32::from(m.glyph(20).unwrap().advance_width));
    }
}
#[test]fn strict_gpos_rejects_unmapped_or_wrong_parent(){
    let (r,_,n)=setup("ACBC");let c=r.glyphs()[1].glyph_id;
    let gd=gdef(n,20,c);let pb=mark_ligature_bytes(c);
    let gp=compile_gpos_program(&pb,&[0],n,Some(&gd),Default::default()).unwrap();
    let ids=[20,20,c];let w=[(500,0),(500,0),(0,0)];
    let a=GposLigatureSource::Ligature{id:1,components:2};
    let b=GposLigatureSource::Ligature{id:2,components:2};
    for mark in [GposLigatureSource::Unmapped,GposLigatureSource::Mark{id:1,component:0}]{
        let sources=[a,b,mark];
        assert!(execute_gpos_mixed_joint(&gp,&ids,&[GposRunAdjustment::default();3],&w,&[(0,0);3],&sources,
            GposRunMode::DesignOnly,GposHorizontalFlow::LeftToRight,Default::default(),Default::default()).is_ok());
        assert!(matches!(execute_gpos_mixed_joint_strict_ligatures(&gp,&ids,&[GposRunAdjustment::default();3],&w,&[(0,0);3],&sources,
            GposRunMode::DesignOnly,GposHorizontalFlow::LeftToRight,Default::default(),Default::default()),Err(Error::Invalid(_))));
    }
}
#[test]fn strict_gpos_rejects_wrong_component_count(){
    let (r,_,n)=setup("AC");let c=r.glyphs()[1].glyph_id;
    let gd=gdef(n,20,c);let pb=mark_ligature_bytes(c);
    let gp=compile_gpos_program(&pb,&[0],n,Some(&gd),Default::default()).unwrap();
    let sources=[GposLigatureSource::Ligature{id:1,components:3},GposLigatureSource::Mark{id:1,component:0}];
    assert!(matches!(execute_gpos_mixed_joint_strict_ligatures(&gp,&[20,c],&[GposRunAdjustment::default();2],
        &[(500,0),(0,0)],&[(0,0);2],&sources,GposRunMode::DesignOnly,GposHorizontalFlow::LeftToRight,
        Default::default(),Default::default()),Err(Error::Invalid(_))));
}
#[test]fn opt_in_joint_mapping_limits(){
    let (r,m,n)=setup("ACB");let a=r.glyphs()[0].glyph_id;let c=r.glyphs()[1].glyph_id;let b=r.glyphs()[2].glyph_id;
    let gd=gdef(n,20,c);let sb=fixture(4,LOOKUP_IGNORE_MARK,&[1,8,1,14,1,1,a,1,4,20,2,b]);let pb=mark_ligature_bytes(c);
    let gs=compile_gsub_program(&sb,&[0],n,Some(&gd),Default::default()).unwrap();
    let gp=compile_gpos_program(&pb,&[0],n,Some(&gd),Default::default()).unwrap();
    let before=r.clone();
    for limits in [JointLigatureLimits{trace:LigatureTraceLimits{max_events:0,..Default::default()},..Default::default()},
                   JointLigatureLimits{mapping:LigatureMappingLimits{max_scalars:2,..Default::default()},..Default::default()}]{
        assert!(matches!(execute_joint_shape_with_ligatures(&gs,&gp,&r,&m,&gd,&[],GposHorizontalFlow::LeftToRight,
            Default::default(),limits),Err(Error::Limit(_))));
    }
    assert_eq!(r,before);
}
#[test]fn same_face_opt_in_ligature_entry_native_kern(){
    let f=parse_font(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),Default::default()).unwrap();
    let face=f.face(0).unwrap();
    let tags=[*b"kern"];
    let o=FaceShapeOptions{script:*b"latn",language:None,substitution_tags:&[],positioning_tags:&tags,
        alternates:&[],flow:GposHorizontalFlow::LeftToRight,allow_missing_glyphs:false,require_requested_features:true};
    let x=shape_font_face_with_ligatures(face,"AVATAR",o,Default::default(),Default::default()).unwrap();
    let y=shape_font_face(face,"AVATAR",o,Default::default()).unwrap();
    assert_eq!(x.shaped.positioning.adjustments,y.shaped.positioning.adjustments);
    assert_eq!(x.shaped.positioning.design_origins,y.shaped.positioning.design_origins);
    assert_eq!(x.shaped.provenance.ligature_trace_stats().unwrap().0,0);
}
