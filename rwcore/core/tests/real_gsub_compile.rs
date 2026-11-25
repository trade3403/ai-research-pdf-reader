//! Independent raw offset, type, count and work-charge reference for
//! real lookup compilation. Typed payload equality checks integration
//! against standalone decoders, not an independent shaping implementation.
use rwcore::*;

fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->&'a [u8]{
    for i in 0..u(b,4){
        let p=12+16*i;
        if &b[p..p+4]==tag{
            let at=d(b,p+8);return &b[at..at+d(b,p+12)];
        }
    }
    panic!("missing table")
}
fn standalone(kind:u16,b:&[u8],n:u32,k:usize)->ExtensionTarget{
    let l=ExtensionLimits::default();
    use ExtensionTarget::*;
    match (kind,u(b,0)){
        (1,_)=>Single(parse_single_substitution(b,n,l.single).unwrap()),
        (2,_)=>Multiple(parse_multiple_substitution(b,n,l.multiple).unwrap()),
        (3,_)=>Alternate(parse_alternate_substitution(b,n,l.alternate).unwrap()),
        (4,_)=>Ligature(parse_ligature_substitution(b,n,l.ligature).unwrap()),
        (5,1)=>Context(parse_context_substitution(b,n,k,l.context).unwrap()),
        (5,2)=>ClassContext(parse_class_context_substitution(b,n,k,l.context).unwrap()),
        (5,3)=>CoverageContext(parse_coverage_context_substitution(b,n,k,l.context).unwrap()),
        (6,1)=>Chain(parse_chain_context_substitution(b,n,k,l.context).unwrap()),
        (6,2)=>ClassChain(parse_class_chain_context_substitution(b,n,k,l.context).unwrap()),
        (6,3)=>CoverageChain(parse_coverage_chain_context_substitution(b,n,k,l.context).unwrap()),
        (8,1)=>Reverse(parse_reverse_substitution(b,n,l.reverse).unwrap()),
        _=>panic!("unexpected target kind or format"),
    }
}
#[derive(Debug)]
struct RawLookup{
    kind:u16,flags:u16,mark:Option<usize>,subtables:Vec<(usize,usize,u16)>,
}
fn raw_inventory(b:&[u8])->Vec<RawLookup>{
    let list=u(b,8);
    (0..u(b,list)).map(|i|{
        let p=list+u(b,list+2+2*i);
        let kind=u(b,p) as u16;
        let flags=u(b,p+2) as u16;
        let count=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*count));
        let subtables=(0..count).map(|j|{
            let at=p+u(b,p+6+2*j);
            if kind==7{
                assert_eq!(u(b,at),1);
                (at,at+d(b,at+4),u(b,at+2) as u16)
            }else{(at,at,kind)}
        }).collect();
        RawLookup{kind,flags,mark,subtables}
    }).collect()
}
fn verify(font:&[u8],n:u32,counts:[usize;5],formats:&[((u16,usize),usize)]){
    let b=table(font,b"GSUB");
    let gd=table(font,b"GDEF");
    assert_eq!(u(table(font,b"maxp"),4),n as usize);
    let f=parse_font(font,FontLimits::default()).unwrap();
    let face=f.face(0).unwrap();
    assert_eq!(face.table(*b"GSUB").unwrap(),b);
    assert_eq!(face.table(*b"GDEF").unwrap(),gd);
    let g=parse_gdef(gd,n,Default::default()).unwrap();
    let raw=raw_inventory(b);
    let mut count=0;
    let mut extension=0;
    let mut bytes=0usize;
    let mut max_subs=0;
    let mut histogram=std::collections::BTreeMap::new();
    for r in &raw{
        max_subs=max_subs.max(r.subtables.len());
        assert!(!r.subtables.is_empty());
        for &(source,target,kind) in &r.subtables{
            count+=1;
            if r.kind==7{extension+=1;bytes+=8;assert!(target>=source+8);}
            else{assert_eq!(target,source);}
            bytes+=b.len()-target;
            *histogram.entry((kind,u(b,target))).or_insert(0usize)+=1;
        }
    }
    assert_eq!([raw.len(),count,extension,bytes,max_subs],counts);
    assert_eq!(histogram.into_iter().collect::<Vec<_>>(),formats);
    let ids:Vec<u16>=(0..raw.len() as u16).collect();
    // First compile under defaults, then compare the exact independent
    // budget in a second pass. Drop owned targets before budget failure tests.
    let all=compile_gsub_lookups(b,&ids,n,Some(&g),Default::default()).unwrap();
    assert_eq!(all.len(),raw.len());
    for (i,(a,r)) in all.iter().zip(&raw).enumerate(){
        assert_eq!(a.index() as usize,i);
        assert_eq!(a.declared_kind(),r.kind);
        assert_eq!(a.filter().flags(),r.flags);
        assert_eq!(a.filter().mark_filtering_set(),r.mark);
        assert_eq!(a.subtables().len(),r.subtables.len());
        for (s,&(at,target,kind)) in a.subtables().iter().zip(&r.subtables){
            assert_eq!(a.effective_kind(),kind);
            assert_eq!(s.source_offset(),at);
            assert_eq!(s.target_offset(),target);
            assert_eq!(s.target().kind(),kind);
            assert_eq!(s.target(),&standalone(kind,&b[target..],n,raw.len()),
                "lookup {i}, source {at}, target {target}");
        }
    }
    // All-feature plan for every script's default LangSys: compare with
    // all-lookup compilation by original indices, not output position.
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    for s in &layout.scripts{
        let Some(lang)=&s.default_language else{continue;};
        let tags:Vec<_>=lang.features.iter().map(|&i|layout.features[i as usize].tag).collect();
        let plan=build_gsub_base_plan(&layout,s.tag,None,&tags,Default::default()).unwrap().unwrap();
        let selected=compile_gsub_plan(b,&plan,n,Some(&g),Default::default()).unwrap();
        assert_eq!(selected.len(),plan.lookups.len());
        for (actual,p) in selected.iter().zip(&plan.lookups){
            let reference=&all[p.index as usize];
            assert_eq!(actual.index(),reference.index());
            assert_eq!(actual.declared_kind(),reference.declared_kind());
            assert_eq!(actual.effective_kind(),reference.effective_kind());
            assert_eq!(actual.filter().flags(),reference.filter().flags());
            assert_eq!(actual.filter().mark_filtering_set(),reference.filter().mark_filtering_set());
            assert_eq!(actual.subtables(),reference.subtables());
        }
    }
    drop(all);
    let limits=GsubCompileLimits{max_lookups:raw.len(),max_subtables_per_lookup:max_subs,
        max_total_subtables:count,max_total_target_bytes:bytes,..Default::default()};
    assert!(compile_gsub_lookups(b,&ids,n,Some(&g),limits).is_ok());
    for l in [
        GsubCompileLimits{max_lookups:raw.len()-1,..limits},
        GsubCompileLimits{max_subtables_per_lookup:max_subs-1,..limits},
        GsubCompileLimits{max_total_subtables:count-1,..limits},
        GsubCompileLimits{max_total_target_bytes:bytes-1,..limits},
    ]{
        assert!(matches!(compile_gsub_lookups(b,&ids,n,Some(&g),l),Err(Error::Limit(_))));
    }
}
#[test]fn abeezee_regular_compile_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,[15,20,0,6890,4],
        &[((1,1),1),((1,2),7),((4,1),3),((6,1),1),((6,3),8)]);
}
#[test]fn abeezee_italic_compile_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,[15,20,0,6890,4],
        &[((1,1),1),((1,2),7),((4,1),3),((6,1),1),((6,3),8)]);
}
#[test]fn alegreya_compile_reference(){
    verify(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,[57,82,0,679650,15],
        &[((1,1),15),((1,2),20),((3,1),1),((4,1),11),((6,3),35)]);
}
#[test]fn aref_compile_reference(){
    verify(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,[86,108,0,213916,10],
        &[((1,1),25),((1,2),24),((2,1),6),((4,1),1),((6,1),3),((6,2),1),((6,3),48)]);
}
#[test]fn padauk_compile_reference(){
    verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,[117,197,0,8642440,17],
        &[((1,1),25),((1,2),21),((2,1),19),((3,1),1),((4,1),22),
          ((5,2),3),((5,3),3),((6,1),3),((6,2),3),((6,3),97)]);
}
#[test]fn devanagari_compile_reference(){
    verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,[177,445,0,25132994,114],
        &[((1,1),76),((1,2),28),((2,1),3),((4,1),35),((5,1),1),((5,2),5),
          ((5,3),79),((6,1),5),((6,2),1),((6,3),212)]);
}
#[test]fn nastaliq_compile_reference(){
    verify(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,[202,2510,1503,514916388,1320],
        &[((1,1),26),((1,2),56),((2,1),59),((4,1),5),((5,1),2),((5,2),7),
          ((5,3),1332),((6,2),17),((6,3),1006)]);
}
