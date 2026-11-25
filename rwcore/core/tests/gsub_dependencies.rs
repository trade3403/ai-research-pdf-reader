use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
const SINGLE:&[u16]=&[1,6,1,1,1,10];
const C1:&[u16]=&[1,8,1,14,1,1,10,1,4,1,1,0,1];
const C2:&[u16]=&[2,12,18,2,0,26,1,1,10,1,10,1,1,1,4,1,1,0,1];
const C3:&[u16]=&[3,1,2,16,0,1,0,1,1,1,10];
const H1:&[u16]=&[1,8,1,14,1,1,10,1,4,0,1,0,1,0,1];
const H2:&[u16]=&[2,16,22,22,22,2,0,30,1,1,10,1,10,1,1,1,4,0,1,0,1,0,1];
const H3:&[u16]=&[3,0,1,20,0,2,0,1,0,1,1,1,10];
fn wrap(kind:u16,w:&[u16])->Vec<u16>{let mut v=vec![1,kind,0,8];v.extend(w);v}
fn fixture(lookups:&[(u16,Vec<Vec<u16>>)]) -> Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,lookups.len() as u16];
    w.resize(8+lookups.len(),0);
    for (i,(kind,subs)) in lookups.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        w.extend([*kind,0,subs.len() as u16]);w.resize(w.len()+subs.len(),0);
        for (j,s) in subs.iter().enumerate(){w[at+3+j]=((w.len()-at)*2) as u16;w.extend(s);}
    }
    bytes(&w)
}
fn compiled(kind:u16,subs:Vec<Vec<u16>>)->Vec<CompiledGsubLookup<'static>>{
    let b=fixture(&[(kind,subs),(1,vec![SINGLE.to_vec()])]);
    compile_gsub_lookups(&b,&[0],32,None,Default::default()).unwrap()
}
fn edges(x:&[CompiledGsubLookup<'_>])->Vec<GsubDependency>{
    discover_gsub_dependencies(x,2,Default::default()).unwrap()
}
#[test]fn all_six_contextual_formats(){
    for (kind,format,w,set,count) in [
        (5,1,C1,Some(0),1),(5,2,C2,Some(1),1),(5,3,C3,None,2),
        (6,1,H1,Some(0),1),(6,2,H2,Some(1),1),(6,3,H3,None,2)]{
        let x=compiled(kind,vec![w.to_vec()]);
        let a=edges(&x);assert_eq!(a.len(),count);
        for (i,e) in a.iter().enumerate(){
            assert_eq!(e.site.lookup_index,0);assert_eq!(e.site.subtable_index,0);
            assert_eq!(e.site.effective_kind,kind);assert_eq!(e.site.format,format);
            assert_eq!(e.site.set_index,set);assert_eq!(e.site.rule_index,0);
            assert_eq!(e.action_index,i);assert_eq!(e.action.sequence_index,0);
            assert_eq!(e.action.lookup_index,1);
        }
    }
}
#[test]fn extension_preserves_source_and_target(){
    let x=compiled(7,vec![wrap(5,C1),wrap(5,C3)]);
    let a=edges(&x);assert_eq!(a.len(),3);
    assert_eq!(a.iter().map(|e|e.site.subtable_index).collect::<Vec<_>>(),vec![0,1,1]);
    for e in a{
        let sub=&x[0].subtables()[e.site.subtable_index];
        assert_eq!(e.site.source_offset,sub.source_offset());
        assert_eq!(e.site.target_offset,sub.target_offset());
        assert_eq!(e.site.target_offset,e.site.source_offset+8);
    }
}
#[test]fn unreachable_class_sets_and_aliases_preserved(){
    let mut w=C2.to_vec();w[4]=26; // class zero has no covered first glyph
    let x=compiled(5,vec![w]);
    let a=edges(&x);
    assert_eq!(a.iter().map(|e|e.site.set_index).collect::<Vec<_>>(),vec![Some(0),Some(1)]);
    assert_eq!(a[0].action,a[1].action);
}
#[test]fn rule_order_and_duplicate_actions(){
    // Format 1 with two rules in the same set, second with duplicate actions.
    let w=vec![1,8,1,14,1,1,10,2,6,14,1,1,0,0,1,2,0,1,0,1];
    let x=compiled(5,vec![w]);
    let a=edges(&x);
    assert_eq!(a.iter().map(|e|(e.site.rule_index,e.action_index,e.action.lookup_index)).collect::<Vec<_>>(),
        vec![(0,0,0),(1,0,1),(1,1,1)]);
}
#[test]fn null_sets_and_empty_actions_no_edges(){
    let x=compiled(5,vec![vec![1,8,1,0,1,1,10],vec![3,1,0,8,1,1,10]]);
    assert!(edges(&x).is_empty());
    // lookup + two subtables + one null set slot + one format 3 rule
    assert!(discover_gsub_dependencies(&x,2,GsubDependencyLimits{max_scan_records:5,max_actions:0}).is_ok());
    assert!(discover_gsub_dependencies(&x,2,GsubDependencyLimits{max_scan_records:4,max_actions:0}).is_err());
}
#[test]fn self_and_mutual_references_retained(){
    let mut c=C1.to_vec();*c.last_mut().unwrap()=0;
    let b=fixture(&[(5,vec![C1.to_vec(),c.clone()]),(5,vec![c])]);
    let x=compile_gsub_lookups(&b,&[0,1],32,None,Default::default()).unwrap();
    assert_eq!(edges(&x).iter().map(|e|(e.site.lookup_index,e.action.lookup_index)).collect::<Vec<_>>(),
        vec![(0,1),(0,0),(1,0)]);
}
#[test]fn exact_scan_and_action_limits(){
    let x=compiled(5,vec![C3.to_vec()]);
    let l=GsubDependencyLimits{max_scan_records:3,max_actions:2};
    assert_eq!(discover_gsub_dependencies(&x,2,l).unwrap().len(),2);
    assert!(discover_gsub_dependencies(&x,2,GsubDependencyLimits{max_scan_records:2,..l}).is_err());
    assert!(discover_gsub_dependencies(&x,2,GsubDependencyLimits{max_actions:1,..l}).is_err());
}
#[test]fn partial_selection_and_invalid_domain(){
    let x=compiled(5,vec![C1.to_vec()]);
    assert_eq!(edges(&x)[0].action.lookup_index,1);
    assert!(discover_gsub_dependencies(&x,1,Default::default()).is_err());
    assert!(discover_gsub_dependencies(&x,0,Default::default()).is_err());
    assert!(discover_gsub_dependencies(&x,65536,Default::default()).is_err());
    assert!(discover_gsub_dependencies(&[],0,GsubDependencyLimits{max_scan_records:0,max_actions:0}).unwrap().is_empty());
}
#[test]fn invalid_source_order(){
    let b=fixture(&[(5,vec![C1.to_vec()]),(1,vec![SINGLE.to_vec()])]);
    let mut x=compile_gsub_lookups(&b,&[0,1],32,None,Default::default()).unwrap();
    x.swap(0,1);assert!(discover_gsub_dependencies(&x,2,Default::default()).is_err());
    x[0]=x[1].clone();assert!(discover_gsub_dependencies(&x,2,Default::default()).is_err());
}
#[test]fn noncontextual_targets_no_edges(){
    let x=compiled(1,vec![SINGLE.to_vec()]);
    assert!(discover_gsub_dependencies(&x,2,GsubDependencyLimits{max_scan_records:2,max_actions:0}).unwrap().is_empty());
}
