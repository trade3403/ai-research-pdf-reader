use rwcore::*;
use std::collections::BTreeMap;
const ALIASES:&str="sc ; Latn ; Latin\nsc ; Grek ; Greek\nsc ; Zinh ; Inherited ; Qaai\nsc ; Zyyy ; Common\nsc ; Zzzz ; Unknown\n";
fn aliases()->ScriptAliases{parse_script_aliases(ALIASES,Default::default()).unwrap()}
#[test]fn script_alias_loose_matching(){
    let a=aliases();for name in ["LATN"," l-a_t n ","latin"]{assert_eq!(a.resolve(name).unwrap().unwrap().long,"Latin");}
    assert_eq!(a.resolve("Qaai").unwrap().unwrap().short,"Zinh");
    assert!(a.resolve("missing").unwrap().is_none());
    assert!(a.resolve("!").is_err());
}
#[test]fn script_alias_collision_and_limits(){
    assert!(parse_script_aliases("sc;Latn;Latin\nsc;Grek;Latin",Default::default()).is_err());
    assert!(parse_script_aliases("sc;Lat;Latin",Default::default()).is_err());
    assert!(parse_script_aliases(ALIASES,ScriptExtensionLimits{max_aliases:1,..Default::default()}).is_err());
}
#[test]fn script_extensions_override_and_default(){
    let a=aliases();let d=parse_script_extensions("# @missing: 0000..10FFFF; <script>\n0300; Latn Grek", &a,Default::default()).unwrap();
    let p=parse_unicode_scripts("0041;Latin\n0300;Inherited",Default::default()).unwrap();
    let s=d.scripts('\u{300}',&p);assert_eq!(s.len(),2);assert!(s.contains("Greek"));assert!(!s.contains("Inherited"));
    assert_eq!(s.get(0),Some("Greek"));assert_eq!(s.get(1),Some("Latin"));assert_eq!(s.get(2),None);
    assert_eq!(d.scripts('A',&p),ScriptSet::Primary("Latin"));assert_eq!(d.scripts('B',&p),ScriptSet::Primary("Unknown"));
}
#[test]fn script_extensions_invalid_records(){
    let a=aliases();for s in ["41;Latn Latin","41;Missing","41;","41;Zyyy","D800;Latn","42..41;Latn","41..42;Latn\n42;Grek","# @missing: 0000..10FFFF; Unknown"]{
        assert!(parse_script_extensions(s,&a,Default::default()).is_err(),"{s}");
    }
}
#[test]fn script_extensions_limits(){
    let a=aliases();for l in [ScriptExtensionLimits{max_bytes:1,..Default::default()},ScriptExtensionLimits{max_ranges:0,..Default::default()},ScriptExtensionLimits{max_set_members:1,..Default::default()},ScriptExtensionLimits{max_total_members:1,..Default::default()}]{
        assert!(matches!(parse_script_extensions("41;Latn Grek",&a,l),Err(Error::Limit(_))));
    }
}
#[test]fn bundled_script_alias_inventory(){
    let d=bundled_script_properties().unwrap();assert_eq!(d.aliases.values().len(),176);assert_eq!(d.extensions.range_count(),206);
    for v in d.aliases.values(){assert_eq!(d.aliases.resolve(&v.short).unwrap(),Some(v));assert_eq!(d.aliases.resolve(&v.long).unwrap(),Some(v));}
    assert_eq!(d.aliases.resolve("Qaac").unwrap().unwrap().short,"Copt");
    assert_eq!(d.aliases.resolve("Qaai").unwrap().unwrap().short,"Zinh");
}
#[test]fn bundled_script_extensions_examples(){
    let d=bundled_script_properties().unwrap();let s=d.extensions.scripts('\u{30fc}',&d.primary);
    assert_eq!(s.len(),2);assert!(s.contains("Hiragana")&&s.contains("Katakana"));assert!(!s.contains("Common"));
    let s=d.extensions.scripts('\u{301}',&d.primary);assert!(s.contains("Latin")&&s.contains("Greek"));assert!(!s.contains("Inherited"));
    assert_eq!(d.extensions.scripts('A',&d.primary),ScriptSet::Primary("Latin"));
    assert_eq!(d.extensions.scripts('\u{10ffff}',&d.primary),ScriptSet::Primary("Unknown"));
}
#[test]fn bundled_script_extensions_exhaustive_reference(){
    // Separate raw-data expansion, with direct short-to-long map.
    let mut alias=BTreeMap::new();for raw in BUNDLED_SCRIPT_ALIASES.lines(){
        let mut f=raw.split('#').next().unwrap().split(';').map(str::trim);
        if f.next()==Some("sc"){alias.insert(f.next().unwrap(),f.next().unwrap());}
    }
    let mut dense=vec![None;0x110000];let mut sets=Vec::new();let mut count=0;
    for raw in BUNDLED_SCRIPT_EXTENSIONS.lines(){
        let line=raw.split('#').next().unwrap().trim();if line.is_empty(){continue;}
        let (r,v)=line.split_once(';').unwrap();let mut b=r.trim().split("..");
        let first=usize::from_str_radix(b.next().unwrap(),16).unwrap();
        let last=b.next().map(|s|usize::from_str_radix(s,16).unwrap()).unwrap_or(first);
        let mut names:Vec<_>=v.split_ascii_whitespace().map(|s|alias[s]).collect();names.sort_unstable();
        for slot in &mut dense[first..=last]{assert!(slot.is_none());*slot=Some(sets.len());count+=1;}
        sets.push(names);
    }
    assert_eq!(count,669);let d=bundled_script_properties().unwrap();let mut checked=0;
    for (n,index) in dense.into_iter().enumerate(){
        let Some(ch)=char::from_u32(n as u32) else{continue;};
        let got=d.extensions.scripts(ch,&d.primary);
        if let Some(i)=index{assert_eq!(got.len(),sets[i].len());for (j,&name) in sets[i].iter().enumerate(){assert_eq!(got.get(j),Some(name));}}
        else{assert_eq!(got,ScriptSet::Primary(d.primary.script(ch)));}
        checked+=1;
    }assert_eq!(checked,1112064);
}
