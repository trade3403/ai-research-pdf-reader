use rwcore::*;
fn fixture()->Vec<u8>{
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0,2,1,2]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,3);
    for (tag,rel) in [(b"ccmp",20),(b"liga",28),(b"salt",36)]{b.extend(tag);w(&mut b,rel);}
    for ids in [[0,2],[1,2],[0,1]]{for v in [0,2,ids[0],ids[1]]{w(&mut b,v);}}
    let at=b.len();set(&mut b,8,at);w(&mut b,3);for _ in 0..3{w(&mut b,0);}
    for i in 0..3{let rel=b.len()-at;set(&mut b,at+2+2*i,rel);
        for v in [1,0,1,8,2,8,1,11+i as u16,1,1,10]{w(&mut b,v);}
    }
    b
}
fn spec(ids:&[u16])->ShapeStageSpec{ShapeStageSpec{features:ids.to_vec()}}
fn program(b:&[u8])->CompiledGsubProgram<'_>{
    let l=parse_gsub_layout(b,Default::default()).unwrap();
    let plan=build_gsub_base_plan(&l,*b"DFLT",None,&[*b"liga",*b"salt"],Default::default()).unwrap().unwrap();
    compile_gsub_plan_program(b,&plan,32,None,Default::default()).unwrap()
}
#[test]fn stage_order_and_shared_lookup(){
    let b=fixture();let p=program(&b);
    let s=build_shape_stage_schedule(&p,&[spec(&[1]),spec(&[0,2])],Default::default()).unwrap();
    assert_eq!(s.stages()[0].lookups().iter().map(|r|r.index).collect::<Vec<_>>(),vec![1,2]);
    assert_eq!(s.stages()[1].lookups().iter().map(|r|r.index).collect::<Vec<_>>(),vec![0,1,2]);
    assert_eq!(s.stages()[1].lookups()[0].features,vec![0,2]);
    assert_eq!((s.root_count(),s.reference_count()),(5,6));
}
#[test]fn stage_partition_validation(){
    let b=fixture();let p=program(&b);
    for specs in [vec![],vec![spec(&[1,2])],vec![spec(&[0,1]),spec(&[1,2])],vec![spec(&[2,1,0])],vec![spec(&[0,1,3])],vec![spec(&[]),spec(&[0,1,2])]]{
        assert!(build_shape_stage_schedule(&p,&specs,Default::default()).is_err());
    }
}
#[test]fn stage_exact_budget(){
    let b=fixture();let p=program(&b);let specs=[spec(&[1]),spec(&[0,2])];
    let s=build_shape_stage_schedule(&p,&specs,Default::default()).unwrap();
    let l=ShapeStageLimits{max_stages:2,max_features:3,max_roots:5,max_refs:6,max_work:s.work()};
    assert_eq!(s,build_shape_stage_schedule(&p,&specs,l).unwrap());
    for bad in [ShapeStageLimits{max_stages:1,..l},ShapeStageLimits{max_features:2,..l},ShapeStageLimits{max_roots:4,..l},ShapeStageLimits{max_refs:5,..l},ShapeStageLimits{max_work:s.work()-1,..l}]{
        assert!(matches!(build_shape_stage_schedule(&p,&specs,bad),Err(Error::Limit(_))));
    }
}
#[test]fn stage_reject_unvalidated_program(){
    let b=fixture();let p=compile_gsub_program(&b,&[0],32,None,Default::default()).unwrap();
    assert!(build_shape_stage_schedule(&p,&[spec(&[0])],Default::default()).is_err());
}
#[test]fn stage_independent_partition_reference(){
    let b=fixture();let p=program(&b);
    for mask in 1u16..7{
        let a:Vec<_>=(0..3).filter(|i|mask&(1<<i)!=0).collect();
        let z:Vec<_>=(0..3).filter(|i|mask&(1<<i)==0).collect();
        let s=build_shape_stage_schedule(&p,&[spec(&a),spec(&z)],Default::default()).unwrap();
        let original=[[0u16,2],[1,2],[0,1]];
        for (stage,ids) in s.stages().iter().zip([a,z]){
            for root in 0..3u16{
                let expected:Vec<_>=ids.iter().copied().filter(|&id|original[id as usize].contains(&root)).collect();
                let found=stage.lookups().iter().find(|r|r.index==root);
                assert_eq!(found.map(|r|r.features.as_slice()).unwrap_or(&[]),expected.as_slice());
            }
        }
    }
}
