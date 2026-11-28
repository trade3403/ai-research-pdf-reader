use rwcore::*;
use std::collections::BTreeMap;
#[test]fn bundled_script_version_and_inventory(){
    assert_eq!(UNICODE_SCRIPT_VERSION,"17.0.0");
    assert!(BUNDLED_UNICODE_SCRIPTS.starts_with("# Scripts-17.0.0.txt"));
    assert!(BUNDLED_UNICODE_SCRIPTS.contains("# @missing: 0000..10FFFF; Unknown"));
    assert!(BUNDLED_UNICODE_SCRIPTS.trim_end().ends_with("# EOF"));
    assert_eq!(bundled_unicode_scripts().unwrap().range_count(),2287);
}
#[test]fn bundled_script_exhaustive_dense_reference(){
    // Independent reference: expand each raw record into a dense scalar
    // table, not the production interval parser or binary search.
    let mut dense=vec!["Unknown";0x110000];let mut counts=BTreeMap::new();
    for line in BUNDLED_UNICODE_SCRIPTS.lines(){
        let line=line.split('#').next().unwrap().trim();
        if line.is_empty(){continue;}
        let (r,name)=line.split_once(';').unwrap();
        let name=name.trim();let mut ends=r.trim().split("..");
        let a=usize::from_str_radix(ends.next().unwrap(),16).unwrap();
        let b=ends.next().map(|x|usize::from_str_radix(x,16).unwrap()).unwrap_or(a);
        assert!(ends.next().is_none());
        for slot in &mut dense[a..=b]{assert_eq!(*slot,"Unknown");*slot=name;}
        *counts.entry(name).or_insert(0usize)+=b-a+1;
    }
    assert_eq!(counts.len(),174);
    assert_eq!(counts.values().sum::<usize>(),159866);
    assert_eq!(counts["Latin"],1492);assert_eq!(counts["Arabic"],1413);
    assert_eq!(counts["Han"],103351);assert_eq!(counts["Common"],9123);
    assert_eq!(counts["Inherited"],684);
    let d=bundled_unicode_scripts().unwrap();let mut scalars=0;
    for (i,want) in dense.iter().enumerate(){
        if let Some(ch)=char::from_u32(i as u32){assert_eq!(d.script(ch),*want,"U+{i:04X}");scalars+=1;}
    }
    assert_eq!(scalars,1112064);
}
#[test]fn bundled_script_mixed_text_and_offsets(){
    let d=bundled_unicode_scripts().unwrap();
    let t="Aé 汉字あア한عَक्😀\u{20000}";
    let spans=raw_script_spans(t,&d,Default::default()).unwrap();
    assert_eq!(spans.iter().map(|s|s.script).collect::<Vec<_>>(),
        vec!["Latin","Common","Han","Hiragana","Katakana","Hangul","Arabic","Inherited","Devanagari","Common","Han"]);
    assert_eq!(spans.iter().map(|s|&t[s.byte_start..s.byte_end]).collect::<String>(),t);
    assert_eq!(spans.last().unwrap().scalar_end,t.chars().count());
}
#[test]fn bundled_script_new_and_unassigned(){
    let d=bundled_unicode_scripts().unwrap();
    for (n,name) in [(0x16ea0,"Beria_Erfe"),(0x16ed3,"Beria_Erfe"),(0x10ffff,"Unknown"),(0xe000,"Unknown"),(0x0378,"Unknown"),(0x200d,"Inherited"),(0xfe0f,"Inherited")]{
        assert_eq!(d.script(char::from_u32(n).unwrap()),name,"U+{n:04X}");
    }
}
