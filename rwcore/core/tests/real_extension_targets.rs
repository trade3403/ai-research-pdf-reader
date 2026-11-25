//! Real direct target data in synthetic extension wrappers.
//! This is NOT native Type 7 font acceptance: the original fonts contain none.
use rwcore::*;
const AB:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf");
const ABI:&[u8]=include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf");
const ABEL:&[u8]=include_bytes!("fixtures/abel/Abel-Regular.ttf");
const ALEG:&[u8]=include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf");
const AREF:&[u8]=include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf");
const PAD:&[u8]=include_bytes!("fixtures/padauk/Padauk-Regular.ttf");
const NOTO:&[u8]=include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf");
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->Option<&'a [u8]>{
    for i in 0..u(b,4){
        let p=12+16*i;
        if &b[p..p+4]==tag{let s=d(b,p+8);return Some(&b[s..s+d(b,p+12)]);}
    }
    None
}
fn lookup_records(b:&[u8])->Vec<(u16,usize,usize)>{
    let list=u(b,8);
    let mut out=Vec::new();
    for i in 0..u(b,list){
        let at=list+u(b,list+2+2*i);
        let kind=u(b,at) as u16;
        for j in 0..u(b,at+4){out.push((kind,i,at+u(b,at+6+2*j)));}
    }
    out
}
fn direct(kind:u16,b:&[u8],n:u32,k:usize)->ExtensionTarget{
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
        _=>panic!("unexpected target kind or format"),
    }
}
fn wrap(kind:u16,offset:u32,target:&[u8])->Vec<u8>{
    let mut v=vec![0;offset as usize];
    v[..2].copy_from_slice(&1u16.to_be_bytes());
    v[2..4].copy_from_slice(&kind.to_be_bytes());
    v[4..8].copy_from_slice(&offset.to_be_bytes());
    v.extend_from_slice(target);v
}
fn compare(font:&[u8],kind:u16,format:Option<usize>,select:impl Fn(usize,usize)->bool)->usize{
    let b=table(font,b"GSUB").unwrap();
    let n=u(table(font,b"maxp").unwrap(),4) as u32;
    let k=u(b,u(b,8));
    let mut count=0;
    for (t,i,at) in lookup_records(b){
        if t!=kind||format.is_some_and(|f|u(b,at)!=f)||!select(i,at){continue;}
        let target=&b[at..];
        let expected=direct(t,target,n,k);
        // Keep child relative offsets unchanged. Both short and long wrapper
        // offsets are tested, without modifying the source font on disk.
        for offset in [8,70000]{
            let wrapped=wrap(t,offset,target);
            let ext=parse_extension_substitution(&wrapped,wrapped.len()).unwrap();
            assert_eq!(ext.offset(),offset as usize);
            assert_eq!(ext.kind(),t);
            assert_eq!(ext.target(),target);
            let actual=parse_extension_target(ext,n,k,ExtensionLimits::default()).unwrap();
            assert_eq!(actual.kind(),t);assert_eq!(actual,expected);
        }
        count+=1;
    }
    assert!(count>0,"no selected target of kind {kind}, format {format:?}");
    count
}
#[test]fn existing_fixture_native_type7_census(){
    for (font,k) in [(AB,15),(ABI,15),(ALEG,57),(AREF,86),(PAD,117),(NOTO,177)]{
        let b=table(font,b"GSUB").unwrap();
        assert_eq!(u(b,u(b,8)),k);
        let records=lookup_records(b);
        assert!(!records.is_empty());
        assert!(!records.iter().any(|r|r.0==7));
    }
    assert!(table(ABEL,b"GSUB").is_none());
}
#[test]fn wrap_real_direct_types_one_to_four(){
    compare(AB,1,None,|_,_|true);
    compare(AREF,2,None,|_,_|true);
    compare(ALEG,3,None,|_,_|true);
    compare(AB,4,None,|_,_|true);
}
#[test]fn wrap_real_context_and_chained_all_formats(){
    // Previously independently verified direct Type 5 fixtures.
    assert_eq!(compare(NOTO,5,Some(1),|i,o|i==135&&o==5302),1);
    assert_eq!(compare(PAD,5,Some(2),|_,_|true),3);
    assert_eq!(compare(PAD,5,Some(3),|_,_|true),3);
    // All 321 direct Type 6 subtables passed the preceding real-font gate.
    for (font,counts) in [(PAD,[3,3,97]),(NOTO,[5,1,212])]{
        for f in 1..=3{
            assert_eq!(compare(font,6,Some(f),|_,_|true),counts[f-1]);
        }
    }
}

const NASTALIQ:&[u8]=include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf");
#[test]fn native_nastaliq_extension_inventory_and_offsets(){
    let b=table(NASTALIQ,b"GSUB").unwrap();
    assert_eq!(b.len(),267884);
    assert_eq!(u(table(NASTALIQ,b"maxp").unwrap(),4),1723);
    let font=parse_font(NASTALIQ,FontLimits::default()).unwrap();
    assert_eq!(font.face(0).unwrap().table(*b"GSUB").unwrap(),b);
    let layout=parse_gsub_layout(b,GsubLimits::default()).unwrap();
    assert_eq!(layout.lookups.len(),202);
    let records=lookup_records(b);
    let mut counts=std::collections::BTreeMap::new();
    let mut max_offset=0;
    for (i,l) in layout.lookups.iter().enumerate(){
        if l.kind!=7{continue;}
        let expected:Vec<_>=records.iter().filter(|&&(t,j,_)|t==7&&j==i).map(|&(_,_,o)|o).collect();
        assert_eq!(l.subtables,expected);
        let wrappers=resolve_extension_lookup(b,l,b.len(),expected.len()).unwrap();
        assert_eq!(wrappers.len(),expected.len());
        for (ext,at) in wrappers.iter().zip(expected){
            assert_eq!(u(b,at),1);
            let kind=u(b,at+2) as u16;
            let offset=d(b,at+4);
            assert_eq!(ext.kind(),kind);
            assert_eq!(ext.offset(),offset);
            assert_eq!(ext.target().as_ptr(),b[at+offset..].as_ptr());
            assert_eq!(ext.target(),&b[at+offset..]);
            *counts.entry((kind,u(b,at+offset))).or_insert(0usize)+=1;
            max_offset=max_offset.max(offset);
        }
    }
    assert_eq!(counts,std::collections::BTreeMap::from([
        ((1,2),41),((2,1),51),((4,1),5),((5,1),2),
        ((5,2),7),((5,3),1325),((6,2),17),((6,3),55)
    ]));
    assert_eq!(counts.values().sum::<usize>(),1503);
    assert_eq!(max_offset,207128);
}
#[test]fn native_nastaliq_extension_dispatch_representatives(){
    let b=table(NASTALIQ,b"GSUB").unwrap();
    let layout=parse_gsub_layout(b,GsubLimits::default()).unwrap();
    // First independently extracted wrapper in each observed kind/format.
    let cases=[
        (0,7442,121868,129310,1,2),(2,7450,121676,129126,4,1),
        (3,8566,120586,129152,5,3),(6,7474,114486,121960,2,1),
        (8,7490,35446,42936,6,2),(34,8670,116692,125362,6,3),
        (53,7682,35166,42848,5,2),(106,7922,115566,123488,5,1)
    ];
    for (i,at,relative,target,kind,format) in cases{
        assert_eq!(layout.lookups[i].subtables[0],at);
        assert_eq!(at+relative,target);
        assert_eq!(u(b,target),format);
        let ext=parse_extension_substitution(&b[at..],b.len()).unwrap();
        assert_eq!(ext.kind(),kind);
        assert_eq!(ext.offset(),relative);
        let actual=parse_extension_target(ext,1723,202,Default::default()).unwrap();
        assert_eq!(actual.kind(),kind);
        assert_eq!(actual,direct(kind,&b[target..],1723,202));
    }
}
