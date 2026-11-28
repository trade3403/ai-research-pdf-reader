use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
fn fixture()->Vec<u8>{
    bytes(&[1,20,2,28,34,1,40,2,11,12,1,2,10,11,1,1,9,1,1,8,1,1,12])
}
fn parse(b:&[u8])->Result<ReverseSubstitution,Error>{parse_reverse_substitution(b,32,Default::default())}
fn set(b:&mut [u8],i:usize,v:u16){b[2*i..2*i+2].copy_from_slice(&v.to_be_bytes());}
#[test]fn reverse_context_order_and_mapping(){
    let x=parse(&fixture()).unwrap();
    assert_eq!(x.replacements(),&[11,12]);
    assert_eq!(x.match_at(&[8,9,10,12],2),Some(11));
    assert_eq!(x.match_at(&[8,9,11,12],2),Some(12));
    assert_eq!(x.backtrack()[0].index(9),Some(0));assert_eq!(x.lookahead()[0].index(12),Some(0));
}
#[test]fn mismatch_and_boundaries(){
    let x=parse(&fixture()).unwrap();
    for (g,p) in [(&[9,8,10,12][..],2),(&[9,10,12],1),(&[8,9,10],2),(&[8,9,13,12],2),(&[8,9,10,13],2),(&[8,32,10,12],2)]{
        assert!(x.match_at(g,p).is_none());
    }
    assert!(x.match_at(&[],0).is_none());assert!(x.match_at(&[10],usize::MAX).is_none());
}
#[test]fn reverse_pass_uses_updated_lookahead(){
    let x=parse(&bytes(&[1,14,0,1,20,1,11,1,1,10,1,1,11])).unwrap();
    let mut g=[10,10,11];
    assert_eq!(x.replace_reverse(&mut g,3,6).unwrap(),2);
    assert_eq!(g,[11,11,11]);
}
#[test]fn reverse_pass_does_not_revisit_positions(){
    let x=parse(&bytes(&[1,14,0,0,2,11,12,1,2,10,11])).unwrap();
    let mut g=[10,11,12];
    assert_eq!(x.replace_reverse(&mut g,3,3).unwrap(),2);
    assert_eq!(g,[11,12,12]);
}
#[test]fn identity_and_empty_run(){
    let x=parse(&bytes(&[1,12,0,0,1,10,1,1,10])).unwrap();
    let mut g=[10];assert_eq!(x.replace_reverse(&mut g,1,1).unwrap(),1);assert_eq!(g,[10]);
    assert_eq!(x.replace_reverse(&mut [],0,0).unwrap(),0);
}
#[test]fn atomic_run_validation_and_limits(){
    let x=parse(&fixture()).unwrap();
    let original=[8,9,10,12];
    for (len,probes) in [(3,16),(4,15)]{
        let mut g=original;assert!(x.replace_reverse(&mut g,len,probes).is_err());assert_eq!(g,original);
    }
    let mut bad=[32,8,9,10,12];let old=bad;
    assert!(x.replace_reverse(&mut bad,5,20).is_err());assert_eq!(bad,old);
    let mut g=original;assert_eq!(x.replace_reverse(&mut g,4,16).unwrap(),1);assert_eq!(g,[8,9,11,12]);
}
#[test]fn every_prefix_truncated(){
    let b=fixture();for n in 0..b.len(){assert!(parse(&b[..n]).is_err(),"prefix {n}");}
}
#[test]fn invalid_format_offsets_counts_outputs(){
    for (i,v) in [(0,2),(1,18),(2,65535),(3,0),(4,46),(5,65535),(6,2),(7,1),(8,32),(9,32),(13,10),(16,32)]{
        let mut b=fixture();set(&mut b,i,v);assert!(parse(&b).is_err(),"word {i}");
    }
}
#[test]fn cumulative_limits(){
    let b=fixture();
    for l in [ReverseSubstLimits{max_bytes:b.len()-1,..Default::default()},ReverseSubstLimits{max_positions:3,..Default::default()},ReverseSubstLimits{max_records:4,..Default::default()},ReverseSubstLimits{max_covered:4,..Default::default()},ReverseSubstLimits{max_replacements:1,..Default::default()}]{
        assert!(parse_reverse_substitution(&b,32,l).is_err());
    }
    let l=ReverseSubstLimits{max_bytes:b.len(),max_positions:4,max_records:5,max_covered:5,max_replacements:2};
    assert!(parse_reverse_substitution(&b,32,l).is_ok());
}
#[test]fn aliases_charged_per_reference(){
    let mut b=fixture();set(&mut b,3,20);
    let l=ReverseSubstLimits{max_records:5,..Default::default()};
    assert!(parse_reverse_substitution(&b,32,l).is_err());
    let l=ReverseSubstLimits{max_covered:5,..Default::default()};
    assert!(parse_reverse_substitution(&b,32,l).is_err());
    let x=parse(&b).unwrap();assert_eq!(x.match_at(&[8,11,10,12],2),Some(11));
}
#[test]fn coverage_range_and_empty_input(){
    let x=parse(&bytes(&[1,14,0,0,2,20,21,2,1,10,11,0])).unwrap();
    assert_eq!(x.match_at(&[11],0),Some(21));
    let x=parse(&bytes(&[1,10,0,0,0,1,0])).unwrap();
    assert!(x.match_at(&[10],0).is_none());
}
#[test]fn extension_type8_dispatch_and_limits(){
    let mut b=bytes(&[1,8,0,8]);b.extend(fixture());
    let x=parse_extension_target_substitution(&b,32,5,Default::default()).unwrap();
    assert_eq!(x.kind(),8);
    assert_eq!(x,ExtensionTarget::Reverse(parse(&fixture()).unwrap()));
    let l=ExtensionLimits{reverse:ReverseSubstLimits{max_positions:1,..Default::default()},..Default::default()};
    assert!(parse_extension_target_substitution(&b,32,5,l).is_err());
}
