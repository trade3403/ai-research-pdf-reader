use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn wrap(kind:u16,offset:u32,target:&[u16])->Vec<u8>{
    let mut b=words(&[1,kind]);b.extend(offset.to_be_bytes());b.resize(offset as usize,0);
    b.extend(words(target));b
}
fn parse(kind:u16,target:&[u16])->ExtensionTarget{
    parse_extension_target_substitution(&wrap(kind,8,target),32,5,ExtensionLimits::default()).unwrap()
}
const SINGLE:&[u16]=&[1,6,1,1,1,10];
const MULTI:&[u16]=&[1,8,1,14,1,1,10,2,11,12];
const LIG:&[u16]=&[1,8,1,14,1,1,10,1,4,12,2,11];
#[test]fn extended_offset_beyond_u16(){
    let b=wrap(1,70000,SINGLE);
    let x=parse_extension_substitution(&b,b.len()).unwrap();
    assert_eq!(x.offset(),70000);assert_eq!(x.kind(),1);
    assert_eq!(x.target(),&b[70000..]);assert_eq!(x.target().as_ptr(),b[70000..].as_ptr());
    let ExtensionTarget::Single(s)=parse_extension_target(x,32,5,Default::default()).unwrap()else{panic!()};
    assert_eq!(s.substitute(10),Some(11));
}
#[test]fn dispatch_single_multiple_alternate_ligature(){
    let ExtensionTarget::Single(s)=parse(1,SINGLE)else{panic!()};
    assert_eq!(s.substitute(10),Some(11));
    let ExtensionTarget::Multiple(m)=parse(2,MULTI)else{panic!()};
    assert_eq!(m.substitute(10),Some(&[11,12][..]));
    let ExtensionTarget::Alternate(a)=parse(3,MULTI)else{panic!()};
    assert_eq!(a.select(10,1).unwrap(),Some(12));
    let ExtensionTarget::Ligature(l)=parse(4,LIG)else{panic!()};
    assert_eq!(l.match_prefix(&[10,11]).unwrap().output,12);
}
#[test]fn dispatch_context_formats(){
    let ExtensionTarget::Context(x)=parse(5,&[1,8,1,0,1,1,10])else{panic!()};
    assert!(x.match_prefix(&[10]).is_none());
    let ExtensionTarget::ClassContext(x)=parse(5,&[2,12,18,2,0,0,1,1,10,1,10,1,1])else{panic!()};
    assert!(x.match_prefix(&[10]).is_none());
    let ExtensionTarget::CoverageContext(x)=parse(5,&[3,1,0,8,1,1,10])else{panic!()};
    assert_eq!(x.match_prefix(&[10]).unwrap().input_len,1);
}
#[test]fn dispatch_chain_formats(){
    let ExtensionTarget::Chain(x)=parse(6,&[1,8,1,0,1,1,10])else{panic!()};
    assert!(x.match_at(&[10],0).is_none());
    let ExtensionTarget::ClassChain(x)=parse(6,&[2,18,26,42,58,3,0,66,0,
        1,2,10,20,2,2,8,8,2,9,9,1,2,2,10,11,1,20,21,2,1,12,1,7,
        2,6,30,2,1,2,2,2,1,7,2,1,4,0,3,0,1,0,0])else{panic!()};
    assert_eq!(x.match_at(&[8,9,10,21,12],2).unwrap().input_len,2);
    let ExtensionTarget::CoverageChain(x)=parse(6,&[3,0,1,12,0,0,1,1,10])else{panic!()};
    assert_eq!(x.match_at(&[10],0).unwrap().input_len,1);
}
#[test]fn wrapper_truncation(){
    let b=wrap(1,8,SINGLE);
    for n in 0..10{assert!(parse_extension_substitution(&b[..n],100).is_err());}
    for n in 0..b.len(){assert!(parse_extension_target_substitution(&b[..n],32,5,Default::default()).is_err());}
}
#[test]fn invalid_kind_and_format(){
    for kind in [0,7,9,65535]{assert!(parse_extension_substitution(&wrap(kind,8,SINGLE),100).is_err());}
    let mut b=wrap(1,8,SINGLE);b[1]=2;
    assert!(parse_extension_substitution(&b,100).is_err());
}
#[test]fn invalid_offsets(){
    let mut b=wrap(1,8,SINGLE);
    for offset in [0u32,1,7,19,20,u32::MAX]{
        b[4..8].copy_from_slice(&offset.to_be_bytes());
        assert!(parse_extension_substitution(&b,100).is_err());
    }
}
#[test]fn type8_is_valid_metadata_but_truncated_target_rejected(){
    let b=wrap(8,8,&[1,0]);let e=parse_extension_substitution(&b,100).unwrap();
    assert_eq!(e.kind(),8);
    assert!(matches!(parse_extension_target(e,32,5,Default::default()),Err(Error::Invalid(_))));
}
#[test]fn independent_limits(){
    let b=wrap(1,8,SINGLE);
    assert!(parse_extension_substitution(&b,b.len()-1).is_err());
    let e=parse_extension_substitution(&b,b.len()).unwrap();
    let l=ExtensionLimits{single:SingleSubstLimits{max_records:0,..Default::default()},..Default::default()};
    assert!(parse_extension_target(e,32,5,l).is_err());
    let l=ExtensionLimits{max_bytes:1,..Default::default()};
    assert!(parse_extension_target(e,32,5,l).is_err());
}
#[test]fn target_validation_propagates(){
    let b=wrap(1,8,&[1,6,30,1,1,10]);
    assert!(parse_extension_target_substitution(&b,32,5,Default::default()).is_err());
    for kind in [5,6]{assert!(parse_extension_target_substitution(&wrap(kind,8,&[4,0]),32,5,Default::default()).is_err());}
    let b=wrap(5,8,&[3,1,1,12,0,5,1,1,10]);
    assert!(parse_extension_target_substitution(&b,32,5,Default::default()).is_err());
}
fn lookup(kind:u16,subtables:Vec<usize>)->GsubLookup{GsubLookup{kind,flags:0,subtables,mark_filtering_set:None}}
#[test]fn lookup_same_type_order_and_shared_target(){
    let mut b=words(&[1,1,0,16,1,1,0,8]);b.extend(words(SINGLE));
    let r=resolve_extension_lookup(&b,&lookup(7,vec![8,0,8]),b.len(),3).unwrap();
    assert_eq!(r.iter().map(|x|x.offset()).collect::<Vec<_>>(),vec![8,16,8]);
    assert!(r.iter().all(|x|x.target().as_ptr()==b[16..].as_ptr()));
}
#[test]fn lookup_validation_and_budgets(){
    let mut b=wrap(1,8,SINGLE);let at=b.len();b.extend(wrap(2,8,MULTI));
    for x in [lookup(6,vec![0]),lookup(7,vec![]),lookup(7,vec![0,at]),lookup(7,vec![usize::MAX])]{
        assert!(resolve_extension_lookup(&b,&x,b.len(),2).is_err());
    }
    assert!(resolve_extension_lookup(&b,&lookup(7,vec![0]),b.len()-1,1).is_err());
    assert!(resolve_extension_lookup(&b,&lookup(7,vec![0,0]),b.len(),1).is_err());
}
