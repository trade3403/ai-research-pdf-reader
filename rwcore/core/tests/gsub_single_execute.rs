use rwcore::*;
fn words(w:&[u16])->Vec<u8>{w.iter().flat_map(|x|x.to_be_bytes()).collect()}
// Each lookup contains explicit format 2 single substitutions, optionally
// wrapped in Type 7. One input and output per subtable.
fn fixture(spec:&[(u16,u16,Option<u16>,&[(u16,u16)])])->Vec<u8>{
    let mut w=vec![1,0,10,12,14,0,0,spec.len() as u16];
    w.resize(8+spec.len(),0);
    for (i,&(kind,flags,mark,rules)) in spec.iter().enumerate(){
        let at=w.len();w[8+i]=((at-7)*2) as u16;
        w.extend([kind,flags,rules.len() as u16]);
        w.resize(at+3+rules.len(),0);
        if let Some(m)=mark{w.push(m);}
        for (j,&(input,output)) in rules.iter().enumerate(){
            w[at+3+j]=((w.len()-at)*2) as u16;
            if kind==7{w.extend([1,1,0,8]);}
            w.extend([2,8,1,output,1,1,input]);
        }
    }
    words(&w)
}
fn gdef()->Gdef{
    parse_gdef(&words(&[1,2,14,0,0,34,48,
        1,10,7,1,2,3,3,3,4,0,
        1,12,4,1,2,0,0,
        1,2,0,12,0,20,1,2,10,13,1,2,12,14]),32,Default::default()).unwrap()
}
fn run(b:&[u8],roots:&[u16],input:&[u16],g:Option<&Gdef>)->GsubSingleExecResult{
    let p=compile_gsub_program(b,roots,32,g,Default::default()).unwrap();
    execute_gsub_single(&p,input,Default::default()).unwrap()
}
#[test]fn first_match_wins_and_no_same_lookup_cascade(){
    let b=fixture(&[(1,0,None,&[(10,11),(10,12),(11,13)])]);
    let a=run(&b,&[0],&[10,11,9],None);
    assert_eq!(a.glyphs,vec![11,13,9]);
    assert_eq!((a.visits,a.probes,a.matches,a.changes),(3,7,2,2));
}
#[test]fn identity_match_blocks_later_subtables(){
    let b=fixture(&[(1,0,None,&[(10,10),(10,11)])]);
    let a=run(&b,&[0],&[10],None);
    assert_eq!(a.glyphs,vec![10]);assert_eq!((a.probes,a.matches,a.changes),(1,1,0));
}
#[test]fn ordered_roots_see_previous_edits(){
    let b=fixture(&[(1,0,None,&[(10,11)]),(1,0,None,&[(11,12)]),(1,0,None,&[(12,13)])]);
    assert_eq!(run(&b,&[0,1],&[10,11],None).glyphs,vec![12,12]);
    assert_eq!(run(&b,&[1],&[10,11],None).glyphs,vec![10,12]);
}
#[test]fn native_wrapper_dispatch_and_format_one(){
    let b=fixture(&[(7,0,None,&[(10,11)])]);
    assert_eq!(run(&b,&[0],&[10,9],None).glyphs,vec![11,9]);
    let b=words(&[1,0,10,12,14,0,0,1,4,1,0,1,8,1,6,65535,1,1,10]);
    assert_eq!(run(&b,&[0],&[10,9],None).glyphs,vec![9,9]);
}
#[test]fn ignore_classes_and_preserve_logical_order(){
    let g=gdef();
    let b=fixture(&[(1,LOOKUP_RIGHT_TO_LEFT|LOOKUP_IGNORE_BASE|LOOKUP_IGNORE_LIGATURE|LOOKUP_IGNORE_MARK,
        None,&[(10,20),(11,21),(12,22),(15,23),(16,24)])]);
    let a=run(&b,&[0],&[16,12,11,10,15],Some(&g));
    assert_eq!(a.glyphs,vec![24,12,11,10,23]);assert_eq!(a.visits,5);
    assert_eq!(run(&b,&[0],&[10,11,12],None).glyphs,vec![20,21,22]);
}
#[test]fn mark_filter_and_attachment_precedence(){
    let g=gdef();
    let rules=&[(12,20),(13,21),(14,22)];
    let b=fixture(&[(1,LOOKUP_USE_MARK_SET|0x0100,Some(0),rules),
        (1,0x0100,None,rules)]);
    assert_eq!(run(&b,&[0],&[12,13,14],Some(&g)).glyphs,vec![12,21,14]);
    assert_eq!(run(&b,&[1],&[12,13,14],Some(&g)).glyphs,vec![20,13,14]);
}
#[test]fn eligibility_recomputed_after_replacement(){
    let g=gdef();
    let b=fixture(&[(1,0,None,&[(10,12)]),(1,LOOKUP_IGNORE_MARK,None,&[(12,20)])]);
    let a=run(&b,&[0,1],&[10],Some(&g));
    assert_eq!(a.glyphs,vec![12]);assert_eq!((a.visits,a.probes,a.matches),(2,1,1));
}
#[test]fn exact_cumulative_budgets_and_immutable_input(){
    let b=fixture(&[(1,0,None,&[(10,11)]),(1,0,None,&[(11,12)])]);
    let p=compile_gsub_program(&b,&[0,1],32,None,Default::default()).unwrap();
    let input=[10,9];
    let l=GsubSingleExecLimits{max_glyphs:2,max_roots:2,max_visits:4,max_probes:4,max_matches:2};
    assert_eq!(execute_gsub_single(&p,&input,l).unwrap().glyphs,vec![12,9]);
    for bad in [GsubSingleExecLimits{max_glyphs:1,..l},GsubSingleExecLimits{max_roots:1,..l},
        GsubSingleExecLimits{max_visits:3,..l},GsubSingleExecLimits{max_probes:3,..l},
        GsubSingleExecLimits{max_matches:1,..l}]{
        assert!(matches!(execute_gsub_single(&p,&input,bad),Err(Error::Limit(_))));
        assert_eq!(input,[10,9]);
    }
}
#[test]fn empty_program_still_validates_glyph_domain(){
    let b=fixture(&[(1,0,None,&[(10,11)])]);
    let p=compile_gsub_program(&b,&[],32,None,Default::default()).unwrap();
    assert_eq!(p.glyph_count(),32);
    assert_eq!(execute_gsub_single(&p,&[31],Default::default()).unwrap().glyphs,vec![31]);
    assert!(execute_gsub_single(&p,&[32],Default::default()).is_err());
    assert!(run(&b,&[0],&[],None).glyphs.is_empty());
}
#[test]fn unsupported_root_rejected_even_for_empty_input(){
    // One contextual root referencing a single substitution lookup.
    let b=words(&[1,0,10,12,14,0,0,2,6,26,
        1,0,1,8,1,6,1,1,1,10,
        5,0,1,8,3,1,1,12,0,0,1,1,10]);
    let p=compile_gsub_program(&b,&[1],32,None,Default::default()).unwrap();
    assert_eq!(p.lookups().len(),2);
    for input in [&[][..],&[10][..]]{
        assert!(matches!(execute_gsub_single(&p,input,Default::default()),Err(Error::Unsupported(_))));
    }
}
