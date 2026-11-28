use rwcore::*;
const SAMPLE:&str="# @missing: 0000..10FFFF; Unknown\n0600..0605 ; Arabic\n0041..005A ; Latin # uppercase\n0300..036F ; Inherited\n0020 ; Common\n0061..007A ; Latin\n0905 ; Devanagari\n4E00 ; Han\n20000 ; Han\n";
fn data()->UnicodeScripts{parse_unicode_scripts(SAMPLE,Default::default()).unwrap()}
#[test]fn script_unsorted_records_and_defaults(){
    let d=data();assert_eq!(d.range_count(),8);
    for (ch,s) in [('A',"Latin"),('Z',"Latin"),('[',"Unknown"),('a',"Latin"),('z',"Latin"),(' ',"Common"),('\u{0301}',"Inherited"),('\u{0600}',"Arabic"),('\u{0905}',"Devanagari"),('\u{4e00}',"Han"),('\u{20000}',"Han"),('\u{10ffff}',"Unknown")]{assert_eq!(d.script(ch),s);}
}
#[test]fn script_rejects_overlap(){
    for text in ["41..5A;Latin\n5A;Latin","41..5A;Latin\n42;Greek","41;Latin\n41;Latin"]{
        assert!(matches!(parse_unicode_scripts(text,Default::default()),Err(Error::Invalid(_))));
    }
}
#[test]fn script_rejects_malformed_fields(){
    for text in ["5A..41;Latin","110000;Latin","-1;Latin","0x41;Latin","41;","41;Latin;Greek","41;Latin Greek","D800;Latin","D7FF..E000;Latin","41"]{
        assert!(matches!(parse_unicode_scripts(text,Default::default()),Err(Error::Invalid(_))),"{text}");
    }
}
#[test]fn script_default_directive_is_not_silently_ignored(){
    for text in ["# @missing: 0000..10FFFF; Latin","# @missing: 0000..00FF; Unknown","# @missing: 0000..10FFFF; Unknown\n# @missing: 0000..10FFFF; Unknown"]{
        assert!(matches!(parse_unicode_scripts(text,Default::default()),Err(Error::Invalid(_))));
    }
    assert_eq!(parse_unicode_scripts("",Default::default()).unwrap().script('A'),"Unknown");
}
#[test]fn script_parse_limits(){
    for l in [ScriptDataLimits{max_bytes:SAMPLE.len()-1,..Default::default()},ScriptDataLimits{max_records:7,..Default::default()},ScriptDataLimits{max_name_bytes:3,..Default::default()}]{
        assert!(matches!(parse_unicode_scripts(SAMPLE,l),Err(Error::Limit(_))));
    }
    assert!(parse_unicode_scripts(SAMPLE,ScriptDataLimits{max_bytes:SAMPLE.len(),max_records:8,..Default::default()}).is_ok());
}
#[test]fn script_spans_preserve_utf8_and_scalar_offsets(){
    let d=data();let t="Az \u{0301}\u{0905}\u{4e00}\u{20000}";
    let r=raw_script_spans(t,&d,Default::default()).unwrap();
    assert_eq!(r.iter().map(|s|s.script).collect::<Vec<_>>(),vec!["Latin","Common","Inherited","Devanagari","Han"]);
    assert_eq!(r.iter().map(|s|(s.scalar_start,s.scalar_end,s.byte_start,s.byte_end)).collect::<Vec<_>>(),vec![(0,2,0,2),(2,3,2,3),(3,4,3,5),(4,5,5,8),(5,7,8,15)]);
    assert_eq!(r.iter().map(|s|&t[s.byte_start..s.byte_end]).collect::<String>(),t);
}
#[test]fn script_span_limits_and_empty(){
    let d=data();assert!(raw_script_spans("",&d,SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0}).unwrap().is_empty());
    for l in [SourceLimits{max_bytes:1,..Default::default()},SourceLimits{max_scalars:1,..Default::default()},SourceLimits{max_glyphs:1,..Default::default()}]{
        assert!(matches!(raw_script_spans("A ",&d,l),Err(Error::Limit(_))));
    }
    assert_eq!(raw_script_spans("AA",&d,SourceLimits{max_bytes:2,max_scalars:2,max_glyphs:1}).unwrap().len(),1);
}
#[test]fn script_lookup_matches_linear_reference(){
    let d=data();
    for n in 0..=0x10ffff{
        let Some(ch)=char::from_u32(n) else{continue;};
        let want=match n{0x20=>"Common",0x41..=0x5a|0x61..=0x7a=>"Latin",0x300..=0x36f=>"Inherited",0x600..=0x605=>"Arabic",0x905=>"Devanagari",0x4e00|0x20000=>"Han",_=>"Unknown"};
        assert_eq!(d.script(ch),want);
    }
}
