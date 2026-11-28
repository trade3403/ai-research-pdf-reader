use rwcore::*;
const P:ScriptConflictPolicy=ScriptConflictPolicy::Preserve;
const L:ScriptConflictPolicy=ScriptConflictPolicy::PreferLeft;
fn resolve<'a>(t:&str,p:&'a UnicodeScriptProperties,policy:ScriptConflictPolicy)->ResolvedScriptRun<'a>{
    resolve_script_runs(t,p,policy,Default::default()).unwrap()
}
fn labels<'a>(r:&ResolvedScriptRun<'a>)->Vec<&'a str>{r.scalars.iter().map(|s|s.script).collect()}
#[test]fn context_same_script_and_edges(){
    let p=bundled_script_properties().unwrap();let r=resolve(" A-é! ",&p,P);
    assert!(r.scalars.iter().all(|s|s.script=="Latin"));assert_eq!(r.spans.len(),1);
    assert_eq!(r.scalars[0].kind,ScriptResolutionKind::Context);
    assert_eq!(r.scalars[1].kind,ScriptResolutionKind::Fixed);
}
#[test]fn context_conflict_is_explicit(){
    let p=bundled_script_properties().unwrap();
    let r=resolve("A 汉",&p,P);assert_eq!(labels(&r),vec!["Latin","Common","Han"]);
    assert_eq!(r.scalars[1].kind,ScriptResolutionKind::Unresolved);
    let r=resolve("A 汉",&p,L);assert_eq!(labels(&r),vec!["Latin","Latin","Han"]);
    assert_eq!(r.scalars[1].kind,ScriptResolutionKind::Context);
}
#[test]fn context_shared_kana_and_conflicting_anchors(){
    let p=bundled_script_properties().unwrap();
    assert_eq!(labels(&resolve("あーあ",&p,P)),vec!["Hiragana";3]);
    assert_eq!(labels(&resolve("アーア",&p,P)),vec!["Katakana";3]);
    assert_eq!(labels(&resolve("あーア",&p,P)),vec!["Hiragana","Common","Katakana"]);
    assert_eq!(labels(&resolve("あーア",&p,L)),vec!["Hiragana","Hiragana","Katakana"]);
}
#[test]fn context_incompatible_stops_propagation(){
    let p=bundled_script_properties().unwrap();let r=resolve("Aー ",&p,L);
    assert_eq!(labels(&r),vec!["Latin","Common","Common"]);
    assert_eq!(r.scalars[2].kind,ScriptResolutionKind::Unresolved);
    assert_eq!(labels(&resolve("Aーア",&p,P)),vec!["Latin","Katakana","Katakana"]);
}
#[test]fn context_unknown_is_barrier(){
    let p=bundled_script_properties().unwrap();let r=resolve("A\u{0378} ",&p,L);
    assert_eq!(labels(&r),vec!["Latin","Unknown","Common"]);
    assert_eq!(r.scalars[1].kind,ScriptResolutionKind::Unknown);
}
#[test]fn context_combining_marks_preserve_source(){
    let p=bundled_script_properties().unwrap();let t="A\u{301} ع\u{64e}汉\u{20000}";
    let r=resolve(t,&p,P);
    assert_eq!(labels(&r),vec!["Latin","Latin","Common","Arabic","Arabic","Han","Han"]);
    assert_eq!(r.scalars[1].primary,"Inherited");assert_eq!(r.scalars[1].kind,ScriptResolutionKind::Context);
    for (s,(at,ch)) in r.scalars.iter().zip(t.char_indices()){
        assert_eq!((s.character,s.byte_start,s.byte_end),(ch,at,at+ch.len_utf8()));
    }
    assert_eq!(r.spans.iter().map(|s|&t[s.byte_start..s.byte_end]).collect::<String>(),t);
    assert_eq!(r.spans.last().unwrap().scalar_end,t.chars().count());
}
#[test]fn context_no_anchor_and_singleton(){
    let p=bundled_script_properties().unwrap();let r=resolve("ー\u{301} ",&p,P);
    assert!(r.scalars.iter().all(|s|s.kind==ScriptResolutionKind::Unresolved));
    let r=resolve("\u{1f250}",&p,P);assert_eq!(labels(&r),vec!["Han"]);
    assert_eq!(r.scalars[0].kind,ScriptResolutionKind::Fixed);
}
#[test]fn context_resource_limits_and_empty(){
    let p=bundled_script_properties().unwrap();let t="A汉";
    for l in [
        ScriptResolveLimits{source:SourceLimits{max_bytes:3,..Default::default()},..Default::default()},
        ScriptResolveLimits{source:SourceLimits{max_scalars:1,..Default::default()},..Default::default()},
        ScriptResolveLimits{source:SourceLimits{max_glyphs:1,..Default::default()},..Default::default()},
        ScriptResolveLimits{max_work:7,..Default::default()},
    ]{assert!(matches!(resolve_script_runs(t,&p,P,l),Err(Error::Limit(_))));}
    assert!(resolve_script_runs(t,&p,P,ScriptResolveLimits{max_work:8,..Default::default()}).is_ok());
    let r=resolve_script_runs("",&p,P,ScriptResolveLimits{source:SourceLimits{max_bytes:0,max_scalars:0,max_glyphs:0},max_work:0}).unwrap();
    assert!(r.scalars.is_empty()&&r.spans.is_empty());
}
