use rwcore::*;
fn bytes(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
const SINGLE:&[u16]=&[1,10,5,65516,30,1,2,3,7];
const CTX:&[u16]=&[3,1,0,8,1,1,10];
const CHAIN:&[u16]=&[3,0,1,12,0,0,1,1,10];
fn wrap(kind:u16,target:&[u16])->Vec<u16>{
    let mut w=vec![1,kind,0,8];w.extend(target);w
}
// A GPOS with empty Script/Feature lists and an actual LookupList.
// Each lookup's subtables are emitted in order. Offsets are byte relative.
fn fixture(lookups:&[(u16,u16,Option<u16>,Vec<Vec<u16>>)]) -> Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,lookups.len() as u16];
    let list=7;
    w.resize(8+lookups.len(),0);
    for (i,(kind,flags,set,subs)) in lookups.iter().enumerate(){
        let at=w.len();w[list+1+i]=((at-list)*2) as u16;
        w.extend([*kind,*flags,subs.len() as u16]);
        w.resize(w.len()+subs.len(),0);
        if flags&16!=0{w.push(set.unwrap());}
        for (j,s) in subs.iter().enumerate(){
            w[at+3+j]=((w.len()-at)*2) as u16;w.extend(s);
        }
    }
    bytes(&w)
}
fn one(kind:u16,sub:&[u16])->Vec<u8>{fixture(&[(kind,0,None,vec![sub.to_vec()])])}
fn compile(b:&[u8],indices:&[u16])->Result<Vec<CompiledGposLookup<'static>>,Error>{
    compile_gpos_lookups(b,indices,32,None,Default::default())
}

#[test]fn direct_and_extension_order(){
    let b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()]),(7,0,None,vec![CTX.to_vec()]),(9,0,None,vec![wrap(8,CHAIN)])]);
    let x=compile(&b,&[0,1,2]).unwrap();
    assert_eq!(x.iter().map(|l|l.index()).collect::<Vec<_>>(),vec![0,1,2]);
    assert_eq!(x[2].declared_kind(),9);assert_eq!(x[2].effective_kind(),8);
    assert_eq!(x[2].subtables()[0].target_offset(),x[2].subtables()[0].source_offset()+8);
    assert_eq!(x[2].subtables()[0].target(),&parse_direct_gpos_target(8,&bytes(CHAIN),32,3,Default::default()).unwrap());
    assert!(x[0].subtables()[0].source_offset()<x[0].subtables()[1].source_offset());
}
#[test]fn selection_order_and_domain(){
    let b=one(1,SINGLE);for ids in [&[0,0][..],&[1],&[1,0]]{assert!(compile(&b,ids).is_err());}
    assert!(compile(&b,&[]).unwrap().is_empty());
    for n in [0,65537]{assert!(compile_gpos_lookups(&b,&[],n,None,Default::default()).is_err());}
}
#[test]fn mixed_and_empty_lookup(){
    assert!(compile(&fixture(&[(9,0,None,vec![wrap(1,SINGLE),wrap(7,CTX)])]),&[0]).is_err());
    assert!(compile(&fixture(&[(1,0,None,vec![])]),&[0]).is_err());
}
#[test]fn nested_indices_preserved(){
    let c=vec![3,1,1,12,0,1,1,1,10];
    assert!(compile(&one(7,&c),&[0]).is_err());
    let b=fixture(&[(7,0,None,vec![c]),(1,0,None,vec![SINGLE.to_vec()])]);
    let x=compile(&b,&[0]).unwrap();
    let GposExtensionTarget::CoverageContext(c)=x[0].subtables()[0].target()else{panic!()};
    assert_eq!(c.actions()[0].lookup_index,1);assert_eq!(x.len(),1);
}
#[test]fn cumulative_suffix_and_subtable_budgets(){
    let b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()]),(9,0,None,vec![wrap(1,SINGLE)])]);
    let x=compile(&b,&[0,1]).unwrap();
    let total=x.iter().flat_map(|l|l.subtables()).map(|s|b.len()-s.target_offset()+s.target_offset()-s.source_offset()).sum::<usize>();
    let l=GposCompileLimits{max_lookups:2,max_subtables_per_lookup:2,max_total_subtables:3,max_total_target_bytes:total,..Default::default()};
    assert!(compile_gpos_lookups(&b,&[0,1],32,None,l).is_ok());
    for l in [GposCompileLimits{max_lookups:1,..l},GposCompileLimits{max_subtables_per_lookup:1,..l},
        GposCompileLimits{max_total_subtables:2,..l},GposCompileLimits{max_total_target_bytes:total-1,..l}]{
        assert!(compile_gpos_lookups(&b,&[0,1],32,None,l).is_err());
    }
}
#[test]fn alias_charged_twice(){
    let mut b=fixture(&[(1,0,None,vec![SINGLE.to_vec(),SINGLE.to_vec()])]);
    let layout=parse_gpos_layout(&b,Default::default()).unwrap();
    let at=layout.directory.lookups[0];
    let first=[b[at+6],b[at+7]];
    b[at+8..at+10].copy_from_slice(&first);
    let x=compile(&b,&[0]).unwrap();let subs=x[0].subtables();
    assert_eq!(subs[0],subs[1]);
    let cost=2*(b.len()-subs[0].target_offset());
    assert!(compile_gpos_lookups(&b,&[0],32,None,GposCompileLimits{max_total_target_bytes:cost,..Default::default()}).is_ok());
    assert!(compile_gpos_lookups(&b,&[0],32,None,GposCompileLimits{max_total_target_bytes:cost-1,..Default::default()}).is_err());
}
#[test]fn invalid_glyph_domain_even_empty_selection(){
    let b=one(1,SINGLE);
    for n in [0,65537]{assert!(compile_gpos_lookups(&b,&[],n,None,Default::default()).is_err());}
    let g=parse_gdef(&bytes(&[1,0,0,0,0,0]),31,Default::default()).unwrap();
    assert!(compile_gpos_lookups(&b,&[],32,Some(&g),Default::default()).is_err());
}
#[test]fn lookup_flags_validated_before_compilation(){
    let b=fixture(&[(1,16,Some(0),vec![SINGLE.to_vec()])]);
    assert!(compile(&b,&[0]).is_err());
    let g=parse_gdef(&bytes(&[1,2,0,0,0,0,14,1,1,0,8,1,1,10]),32,Default::default()).unwrap();
    let x=compile_gpos_lookups(&b,&[0],32,Some(&g),Default::default()).unwrap();
    assert_eq!(x[0].filter().mark_filtering_set(),Some(0));
}

#[test]fn positioning_filter_kind_domain(){
    let mut l=GposLookup{kind:9,flags:0,subtables:vec![0],mark_filtering_set:None};
    assert!(LookupFilter::from_gpos_lookup(&l,32,None).is_ok());
    assert!(LookupFilter::from_gsub_lookup(&l,32,None).is_err());
    for k in [0,10,65535]{l.kind=k;assert!(LookupFilter::from_gpos_lookup(&l,32,None).is_err());}
    l.kind=9;l.flags=0x20;
    assert!(LookupFilter::from_gpos_lookup(&l,32,None).is_err());
    l.flags=16;l.mark_filtering_set=Some(0);
    assert!(LookupFilter::from_gpos_lookup(&l,32,None).is_err());
}
