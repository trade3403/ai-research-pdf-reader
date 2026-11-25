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
