use rwcore::*;
use JoiningType as J;
use ArabicJoiningForm as F;
fn run(t:&str)->Vec<JoiningScalar>{analyze_arabic_joining(t,&bundled_joining_types().unwrap(),Default::default()).unwrap()}
fn forms(t:&str)->Vec<F>{run(t).iter().map(|s|s.form).collect()}
#[test]fn joining_full_data_reference(){
    let d=bundled_joining_types().unwrap();assert_eq!(d.range_count(),542);
    let mut dense=vec![J::NonJoining;0x110000];let mut count=[0usize;5];
    for raw in BUNDLED_JOINING_TYPES.lines(){
        let s=raw.split('#').next().unwrap().trim();if s.is_empty(){continue;}
        let (r,v)=s.split_once(';').unwrap();let mut b=r.trim().split("..");
        let first=usize::from_str_radix(b.next().unwrap(),16).unwrap();
        let last=b.next().map(|s|usize::from_str_radix(s,16).unwrap()).unwrap_or(first);
        let (k,j)=match v.trim(){"C"=>(J::JoinCausing,0),"D"=>(J::Dual,1),"R"=>(J::Right,2),"L"=>(J::Left,3),"T"=>(J::Transparent,4),_=>panic!("type")};
        for slot in &mut dense[first..=last]{assert_eq!(*slot,J::NonJoining);*slot=k;count[j]+=1;}
    }
    assert_eq!(count,[7,615,153,5,2224]);let mut checked=0;
    for (n,k) in dense.into_iter().enumerate(){if let Some(ch)=char::from_u32(n as u32){assert_eq!(d.get(ch),k,"U+{n:04X}");checked+=1;}}
    assert_eq!(checked,1112064);
}
#[test]fn joining_property_examples(){
    let d=bundled_joining_types().unwrap();
    for (ch,k) in [('ب',J::Dual),('ا',J::Right),('\u{64e}',J::Transparent),('\u{200d}',J::JoinCausing),('\u{200c}',J::NonJoining),('A',J::NonJoining),('\u{10ffff}',J::NonJoining)]{assert_eq!(d.get(ch),k);}
}
#[test]fn joining_four_forms(){
    assert_eq!(forms("ب"),vec![F::Isolated]);assert_eq!(forms("ببب"),vec![F::Initial,F::Medial,F::Final]);
    assert_eq!(forms("باب"),vec![F::Initial,F::Final,F::Isolated]);
    assert_eq!(forms("اب"),vec![F::Isolated,F::Isolated]);
    assert_eq!(F::Initial.feature(),Some(*b"init"));assert_eq!(F::None.feature(),None);
}
#[test]fn joining_transparent_marks_keep_offsets(){
    let t="ب\u{64e}\u{651}ب";let r=run(t);
    assert_eq!(r.iter().map(|s|s.form).collect::<Vec<_>>(),vec![F::Initial,F::None,F::None,F::Final]);
    assert!(r[0].joins_next&&r[3].joins_previous);
    assert!(r[1..3].iter().all(|s|!s.joins_next&&!s.joins_previous));
    for (s,(at,ch)) in r.iter().zip(t.char_indices()){assert_eq!((s.character,s.byte_start,s.byte_end),(ch,at,at+ch.len_utf8()));}
}
#[test]fn joining_controls_and_barriers(){
    assert_eq!(forms("ب\u{200c}ب"),vec![F::Isolated,F::None,F::Isolated]);
    assert_eq!(forms("ب\u{200d}ب"),vec![F::Initial,F::None,F::Final]);
    assert_eq!(forms("\u{200d}ب"),vec![F::None,F::Final]);
    assert_eq!(forms("ب\u{200d}"),vec![F::Initial,F::None]);
    assert_eq!(forms("ب ب"),vec![F::Isolated,F::None,F::Isolated]);
    assert_eq!(forms("بـب"),vec![F::Initial,F::None,F::Final]);
}
#[test]fn joining_all_type_pairs(){
    let d=parse_joining_types("41;U\n42;C\n43;D\n44;R\n45;L\n46;T",Default::default()).unwrap();
    for a in ['A','B','C','D','E','F']{for b in ['A','B','C','D','E','F']{
        let r=analyze_arabic_joining(&format!("{a}{b}"),&d,Default::default()).unwrap();
        let yes=matches!(a,'B'|'C'|'E')&&matches!(b,'B'|'C'|'D');
        assert_eq!((r[0].joins_next,r[1].joins_previous),(yes,yes),"{a}{b}");
    }}
}
#[test]fn joining_invalid_data_and_limits(){
    for s in ["41;X","42..41;D","110000;R","D800;T","41;D\n41;R","# @missing: 0000..10FFFF; D","41;D;R"]{
        assert!(matches!(parse_joining_types(s,Default::default()),Err(Error::Invalid(_))),"{s}");
    }
    assert!(matches!(parse_joining_types("41;D",JoiningDataLimits{max_bytes:3,..Default::default()}),Err(Error::Limit(_))));
    assert!(matches!(parse_joining_types("41;D",JoiningDataLimits{max_ranges:0,..Default::default()}),Err(Error::Limit(_))));
}
#[test]fn joining_analysis_limits(){
    let d=bundled_joining_types().unwrap();let t="بب";
    for l in [JoiningAnalysisLimits{source:SourceLimits{max_bytes:3,..Default::default()},..Default::default()},
        JoiningAnalysisLimits{source:SourceLimits{max_scalars:1,..Default::default()},..Default::default()},
        JoiningAnalysisLimits{source:SourceLimits{max_glyphs:1,..Default::default()},..Default::default()},
        JoiningAnalysisLimits{max_work:3,..Default::default()}]{
        assert!(matches!(analyze_arabic_joining(t,&d,l),Err(Error::Limit(_))));
    }
    assert!(analyze_arabic_joining(t,&d,JoiningAnalysisLimits{max_work:4,..Default::default()}).is_ok());
    assert!(analyze_arabic_joining("",&d,JoiningAnalysisLimits{source:SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0},max_work:0}).unwrap().is_empty());
}
