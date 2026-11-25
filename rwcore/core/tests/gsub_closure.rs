use rwcore::*;
fn edge(from:u16,to:u16)->GsubDependency{
    GsubDependency{site:GsubDependencySite{lookup_index:from,subtable_index:0,source_offset:0,
        target_offset:0,effective_kind:5,format:3,set_index:None,rule_index:0},
        action_index:0,action:ContextAction{sequence_index:0,lookup_index:to}}
}
fn run(n:usize,sources:&[u16],edges:&[GsubDependency],roots:&[u16])->GsubDependencyClosure{
    analyze_gsub_dependency_closure(n,sources,edges,roots,Default::default()).unwrap()
}
#[test]fn transitive_diamond_and_duplicate_actions(){
    let e=[edge(0,2),edge(0,1),edge(0,1),edge(1,3),edge(2,3),edge(3,4)];
    let x=run(6,&[0,1,2,3,4,5],&e,&[0,0]);
    assert_eq!(x.reachable,vec![0,1,2,3,4]);
    assert_eq!(x.reachable_edges,5);
    assert!(x.is_proven_acyclic());assert!(x.is_complete());
}
#[test]fn self_cycle_witness(){
    let x=run(2,&[0,1],&[edge(0,0),edge(0,1)],&[0]);
    assert_eq!(x.cycle_witness,Some(vec![0,0]));
    assert_eq!(x.reachable,vec![0,1]);
    assert!(x.is_complete());assert!(!x.is_proven_acyclic());
}
#[test]fn indirect_cycle_witness_and_following_branches(){
    let e=[edge(0,1),edge(1,2),edge(2,1),edge(2,3),edge(0,4)];
    let x=run(5,&[0,1,2,3,4],&e,&[0]);
    assert_eq!(x.cycle_witness,Some(vec![1,2,1]));
    assert_eq!(x.reachable,vec![0,1,2,3,4]);assert_eq!(x.reachable_edges,5);
}
#[test]fn unreachable_cycle_not_reported(){
    let x=run(4,&[0,1,2,3],&[edge(0,1),edge(2,3),edge(3,2)],&[0]);
    assert!(x.is_proven_acyclic());assert_eq!(x.reachable,vec![0,1]);
}
#[test]fn missing_sources_not_certified_acyclic(){
    let x=run(5,&[0,1,4],&[edge(0,1),edge(1,2),edge(4,3)],&[0,3]);
    assert_eq!(x.reachable,vec![0,1,2,3]);assert_eq!(x.missing_sources,vec![2,3]);
    assert_eq!(x.cycle_witness,None);assert!(!x.is_complete());assert!(!x.is_proven_acyclic());
}
#[test]fn cycle_and_missing_sources_coexist(){
    let x=run(4,&[0,1],&[edge(0,1),edge(1,0),edge(1,3)],&[0]);
    assert_eq!(x.cycle_witness,Some(vec![0,1,0]));assert_eq!(x.missing_sources,vec![3]);
}
#[test]fn explicit_covered_leaf_differs_from_missing_leaf(){
    assert!(run(2,&[1],&[],&[1]).is_proven_acyclic());
    let x=run(2,&[],&[],&[1]);assert_eq!(x.missing_sources,vec![1]);
    assert!(!x.is_proven_acyclic());
}
#[test]fn deterministic_witness_under_reordered_roots_and_edges(){
    let e=[edge(0,2),edge(2,0),edge(0,1),edge(1,0)];
    let a=run(3,&[0,1,2],&e,&[2,0,2]);
    let b=run(3,&[0,1,2],&e.into_iter().rev().collect::<Vec<_>>(),&[0,2]);
    assert_eq!(a,b);assert_eq!(a.cycle_witness,Some(vec![0,1,0]));
}
#[test]fn invalid_domains_and_source_coverage(){
    let f=|n,s:&[u16],e:&[GsubDependency],r:&[u16]|analyze_gsub_dependency_closure(n,s,e,r,Default::default());
    assert!(f(65536,&[],&[],&[]).is_err());
    assert!(f(2,&[0,0],&[],&[]).is_err());
    assert!(f(2,&[1,0],&[],&[]).is_err());
    assert!(f(2,&[2],&[],&[]).is_err());
    assert!(f(2,&[0],&[edge(1,0)],&[]).is_err());
    assert!(f(2,&[0],&[edge(0,2)],&[]).is_err());
    assert!(f(2,&[0],&[edge(2,0)],&[]).is_err());
    assert!(f(2,&[0],&[],&[2]).is_err());
}
#[test]fn exact_limits_and_raw_duplicate_charging(){
    let e=[edge(0,1),edge(0,1),edge(1,2)];
    let s=[0,1,2];let r=[0,0];
    let l=GsubClosureLimits{max_lookup_count:3,max_roots:2,max_actions:3,max_reachable:3,max_stack_depth:3};
    assert!(analyze_gsub_dependency_closure(3,&s,&e,&r,l).is_ok());
    for l in [GsubClosureLimits{max_lookup_count:2,..l},GsubClosureLimits{max_roots:1,..l},
        GsubClosureLimits{max_actions:2,..l},GsubClosureLimits{max_reachable:2,..l},
        GsubClosureLimits{max_stack_depth:2,..l}]{
        assert!(matches!(analyze_gsub_dependency_closure(3,&s,&e,&r,l),Err(Error::Limit(_))));
    }
}
#[test]fn empty_graph_and_empty_roots(){
    let l=GsubClosureLimits{max_lookup_count:0,max_roots:0,max_actions:0,max_reachable:0,max_stack_depth:0};
    let x=analyze_gsub_dependency_closure(0,&[],&[],&[],l).unwrap();
    assert!(x.reachable.is_empty());assert!(x.is_proven_acyclic());
    let x=run(2,&[0,1],&[edge(0,1),edge(1,0)],&[]);
    assert!(x.is_proven_acyclic());assert_eq!(x.reachable_edges,0);
}
#[test]fn deep_chain_uses_iterative_stack(){
    let n=20000;
    let s:Vec<u16>=(0..n).collect();
    let e:Vec<_>=(0..n-1).map(|i|edge(i,i+1)).collect();
    let x=run(n as usize,&s,&e,&[0]);
    assert_eq!(x.reachable,s);assert_eq!(x.reachable_edges,n as usize-1);
    assert!(x.is_proven_acyclic());
    let mut e=e;e.push(edge(n-1,n-2));
    let x=run(n as usize,&s,&e,&[0]);
    assert_eq!(x.cycle_witness,Some(vec![n-2,n-1,n-2]));
}
#[test]fn independent_small_graph_exhaustive_reachability_and_cycles(){
    // All 512 directed graphs on three vertices. Boolean transitive closure
    // independently detects paths of positive length including self cycles.
    for mask in 0u16..512{
        let mut e=Vec::new();let mut reach=[[false;3];3];
        for i in 0..3{for j in 0..3{
            if mask&(1<<(3*i+j))!=0{reach[i][j]=true;e.push(edge(i as u16,j as u16));}
        }}
        for k in 0..3{for i in 0..3{for j in 0..3{
            reach[i][j]=reach[i][j]||(reach[i][k]&&reach[k][j]);
        }}}
        for r in 0..3{
            let x=run(3,&[0,1,2],&e,&[r as u16]);
            let expected:Vec<_>=(0..3).filter(|&i|i==r||reach[r][i]).map(|i|i as u16).collect();
            assert_eq!(x.reachable,expected);
            let cyclic=(0..3).any(|i|(i==r||reach[r][i])&&reach[i][i]);
            assert_eq!(x.cycle_witness.is_some(),cyclic);
            if let Some(w)=x.cycle_witness{
                assert_eq!(w.first(),w.last());
                assert!(w.windows(2).all(|p|e.iter().any(|a|a.site.lookup_index==p[0]&&a.action.lookup_index==p[1])));
            }
        }
    }
}
