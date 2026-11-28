use rwcore::*;
use IndicSyllabicCategory as S;
use IndicPositionalCategory as P;
fn dense(text:&str,default:&'static str,expected:usize,query:impl Fn(char)->&'static str){
    let mut refdata=vec![default;0x110000];let mut count=0;
    for raw in text.lines(){
        let s=raw.split('#').next().unwrap().trim();if s.is_empty(){continue;}
        let (r,v)=s.split_once(';').unwrap();let mut b=r.trim().split("..");
        let first=usize::from_str_radix(b.next().unwrap(),16).unwrap();
        let last=b.next().map(|s|usize::from_str_radix(s,16).unwrap()).unwrap_or(first);
        for slot in &mut refdata[first..=last]{assert_eq!(*slot,default);*slot=v.trim();count+=1;}
    }
    assert_eq!(count,expected);let mut checked=0;
    for (i,want) in refdata.into_iter().enumerate(){
        if let Some(ch)=char::from_u32(i as u32){assert_eq!(query(ch),want,"U+{i:04X}");checked+=1;}
    }assert_eq!(checked,1112064);
}
#[test]fn indic_syllabic_exhaustive_reference(){
    let d=bundled_indic_properties().unwrap();assert_eq!(d.syllabic.range_count(),967);
    dense(BUNDLED_INDIC_SYLLABIC,"Other",4856,|ch|d.syllabic.get(ch).name());
}
#[test]fn indic_positional_exhaustive_reference(){
    let d=bundled_indic_properties().unwrap();assert_eq!(d.positional.range_count(),653);
    dense(BUNDLED_INDIC_POSITIONAL,"Not_Applicable",1299,|ch|d.positional.get(ch).name());
}
#[test]fn indic_character_examples(){
    let d=bundled_indic_properties().unwrap();
    for (ch,s,p) in [('क',S::Consonant,P::NotApplicable),('ि',S::VowelDependent,P::Left),('्',S::Virama,P::Bottom),
        ('़',S::Nukta,P::Bottom),('अ',S::VowelIndependent,P::NotApplicable),('A',S::Other,P::NotApplicable),
        ('\u{200c}',S::NonJoiner,P::NotApplicable),('\u{200d}',S::Joiner,P::NotApplicable)]{
        assert_eq!((d.syllabic.get(ch),d.positional.get(ch)),(s,p),"{ch}");
    }
}
#[test]fn indic_data_invalid_and_defaults(){
    for s in ["41;Bogus","42..41;Consonant","110000;Consonant","D800;Consonant","41;Consonant\n41;Virama","# @missing: 0000..10FFFF; Virama"]{
        assert!(matches!(parse_indic_syllabic(s,Default::default()),Err(Error::Invalid(_))));
    }
    assert!(parse_indic_positional("41;Consonant",Default::default()).is_err());
    assert!(parse_indic_syllabic("41;Top",Default::default()).is_err());
    assert_eq!(parse_indic_syllabic("",Default::default()).unwrap().get('क'),S::Other);
    assert_eq!(parse_indic_positional("",Default::default()).unwrap().get('ि'),P::NotApplicable);
}
#[test]fn indic_data_limits_and_unsorted(){
    let s="43;Virama\n41..42;Consonant";
    let d=parse_indic_syllabic(s,IndicDataLimits{max_bytes:s.len(),max_ranges:2}).unwrap();
    assert_eq!(d.get('B'),S::Consonant);assert_eq!(d.get('C'),S::Virama);assert_eq!(d.get('D'),S::Other);
    for l in [IndicDataLimits{max_bytes:s.len()-1,max_ranges:2},IndicDataLimits{max_bytes:s.len(),max_ranges:1}]{
        assert!(matches!(parse_indic_syllabic(s,l),Err(Error::Limit(_))));
    }
}
#[test]fn indic_annotation_preserves_offsets_and_limits(){
    let p=bundled_indic_properties().unwrap();let t="Aक्‍षि😀";
    let r=annotate_indic_scalars(t,&p,Default::default()).unwrap();
    assert_eq!(r.iter().map(|x|x.character).collect::<String>(),t);
    for (x,(at,ch)) in r.iter().zip(t.char_indices()){
        assert_eq!((x.byte_start,x.byte_end),(at,at+ch.len_utf8()));
    }
    assert_eq!(r[2].syllabic,S::Virama);assert_eq!(r[3].syllabic,S::Joiner);assert_eq!(r[5].positional,P::Left);
    for l in [SourceLimits{max_bytes:t.len()-1,..Default::default()},SourceLimits{max_scalars:1,..Default::default()},SourceLimits{max_glyphs:1,..Default::default()}]{
        assert!(matches!(annotate_indic_scalars(t,&p,l),Err(Error::Limit(_))));
    }
    assert!(annotate_indic_scalars("",&p,SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0}).unwrap().is_empty());
}
