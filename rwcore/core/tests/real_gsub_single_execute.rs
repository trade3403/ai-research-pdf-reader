//! Independent raw SFNT oracle for Type 1 execution. No production
//! coverage, ClassDef, GDEF, lookup parser or substitution in the oracle.
use rwcore::*;
use std::collections::BTreeMap;
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(font:&'a [u8],tag:&[u8;4])->&'a [u8]{
    for i in 0..u(font,4){
        let p=12+i*16;
        if &font[p..p+4]==tag{let at=d(font,p+8);return &font[at..at+d(font,p+12)];}
    }
    panic!("missing table")
}
// Expand the raw Coverage to glyph/index pairs, independently of the
// production binary-search representation.
fn coverage(b:&[u8],p:usize)->Vec<(u16,usize)>{
    match u(b,p){
        1=>(0..u(b,p+2)).map(|i|(u(b,p+4+2*i) as u16,i)).collect(),
        2=>{
            let mut out=Vec::new();
            for i in 0..u(b,p+2){
                let r=p+4+6*i;let a=u(b,r);let z=u(b,r+2);let k=u(b,r+4);
                for g in a..=z{out.push((g as u16,k+g-a));}
            }
            out
        }
        _=>panic!("coverage format")
    }
}
fn classes(b:&[u8],p:usize,n:usize)->Vec<usize>{
    let mut out=vec![0;n];
    if p==0{return out;}
    match u(b,p){
        1=>{let a=u(b,p+2);for i in 0..u(b,p+4){out[a+i]=u(b,p+6+2*i);}}
        2=>{for i in 0..u(b,p+2){
            let r=p+4+6*i;out[u(b,r)..=u(b,r+2)].fill(u(b,r+4));
        }}
        _=>panic!("class format")
    }
    out
}
struct RawGdef{class:Vec<usize>,attachment:Vec<usize>,sets:Vec<Vec<bool>>}
impl RawGdef{
    fn read(b:&[u8],n:usize)->Self{
        let class=classes(b,u(b,4),n);
        let attachment=classes(b,u(b,10),n);
        let mut sets=Vec::new();
        let at=if u(b,2)>=2{u(b,12)}else{0};
        if at!=0{
            assert_eq!(u(b,at),1);
            for i in 0..u(b,at+2){
                let mut set=vec![false;n];
                for (g,_) in coverage(b,at+d(b,at+4+4*i)){set[g as usize]=true;}
                sets.push(set);
            }
        }
        Self{class,attachment,sets}
    }
    fn eligible(&self,g:u16,flags:usize,mark:Option<usize>)->bool{
        let i=g as usize;
        match self.class[i]{
            1=>flags&2==0,
            2=>flags&4==0,
            3=>flags&8==0 && match mark{
                Some(s)=>self.sets[s][i],
                None=>flags>>8==0||self.attachment[i]==flags>>8,
            },
            _=>true,
        }
    }
}
struct RawSingle{index:u16,flags:usize,mark:Option<usize>,wrapped:bool,rules:Vec<BTreeMap<u16,u16>>}
fn single_roots(b:&[u8],n:usize)->Vec<RawSingle>{
    let list=u(b,8);
    let mut out=Vec::new();
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);let kind=u(b,p);let flags=u(b,p+2);
        let count=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*count));
        let mut rules=Vec::new();
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let (k,t)=if kind==7{
                assert_eq!(u(b,source),1);
                (u(b,source+2),source+d(b,source+4))
            }else{(kind,source)};
            if k!=1{continue;}
            let mut map=BTreeMap::new();
            for (g,idx) in coverage(b,t+u(b,t+2)){
                let replacement=match u(b,t){
                    1=>g.wrapping_add(u(b,t+4) as u16),
                    2=>{assert!(idx<u(b,t+4));u(b,t+6+2*idx) as u16},
                    _=>panic!("single format"),
                };
                assert!((g as usize)<n && (replacement as usize)<n);
                assert!(map.insert(g,replacement).is_none());
            }
            rules.push(map);
        }
        if !rules.is_empty(){
            assert_eq!(rules.len(),count);
            out.push(RawSingle{index:i as u16,flags,mark,wrapped:kind==7,rules});
        }
    }
    out
}
fn reference(input:&[u16],roots:&[RawSingle],gd:&RawGdef)->GsubSingleExecResult{
    let mut a=GsubSingleExecResult{glyphs:input.to_vec(),visits:0,probes:0,matches:0,changes:0};
    for root in roots{
        // Independent out-of-place pass; each glyph is read once from the
        // preceding pass, never revisited after replacement within a root.
        let previous=std::mem::take(&mut a.glyphs);
        a.glyphs=previous.iter().map(|&g|{
            a.visits+=1;
            if !gd.eligible(g,root.flags,root.mark){return g;}
            for rule in &root.rules{
                a.probes+=1;
                if let Some(&v)=rule.get(&g){
                    a.matches+=1;a.changes+=usize::from(v!=g);return v;
                }
            }
            g
        }).collect();
    }
    a
}
fn verify(font:&[u8],glyphs:u32,expected_subtables:usize,expect_wrapped:bool){
    let b=table(font,b"GSUB");let gd=table(font,b"GDEF");
    let n=glyphs as usize;
    assert_eq!(u(table(font,b"maxp"),4),n);
    let rawgd=RawGdef::read(gd,n);
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let raw=single_roots(b,n);
    assert_eq!(raw.iter().map(|r|r.rules.len()).sum::<usize>(),expected_subtables);
    if !expect_wrapped{assert!(!raw.iter().any(|r|r.wrapped));}
    // Native wrapper inventory is determined from bytes; synthetic wrapper
    // coverage remains in gsub_single_execute.rs even if absent here.
    let input:Vec<u16>=(0..n).map(|i|i as u16).collect();
    let roots:Vec<_>=raw.iter().map(|r|r.index).collect();
    let p=compile_gsub_program(b,&roots,glyphs,Some(&g),Default::default()).unwrap();
    assert_eq!(p.roots(),roots.as_slice());
    assert_eq!(p.lookups().len(),roots.len());
    assert!(p.dependencies().is_empty());
    let mut matched=0;
    let mut changed=0;
    for r in &raw{
        let p=compile_gsub_program(b,&[r.index],glyphs,Some(&g),Default::default()).unwrap();
        let want=reference(&input,std::slice::from_ref(r),&rawgd);
        let got=execute_gsub_single(&p,&input,Default::default()).unwrap();
        assert_eq!(got,want,"lookup {}",r.index);
        matched+=want.matches;changed+=want.changes;
    }
    assert!(matched>0 && changed>0);
    // Full domain in both orders plus repeated glyphs, so sequence order
    // and multiplicity are preserved across successive root passes.
    let mixed:Vec<_>=input.iter().copied().chain(input.iter().rev().copied())
        .chain(input.iter().step_by(3).copied()).collect();
    let want=reference(&mixed,&raw,&rawgd);
    let limits=GsubSingleExecLimits{max_glyphs:mixed.len(),max_roots:roots.len(),
        max_visits:want.visits,max_probes:want.probes,max_matches:want.matches};
    assert_eq!(execute_gsub_single(&p,&mixed,limits).unwrap(),want);
    assert_eq!(want.glyphs.len(),mixed.len());
    for l in [
        GsubSingleExecLimits{max_glyphs:mixed.len()-1,..limits},
        GsubSingleExecLimits{max_roots:roots.len()-1,..limits},
        GsubSingleExecLimits{max_visits:want.visits-1,..limits},
        GsubSingleExecLimits{max_probes:want.probes-1,..limits},
        GsubSingleExecLimits{max_matches:want.matches.saturating_sub(1),..limits},
    ].into_iter().take(if want.matches>0{5}else{4}){
        assert!(matches!(execute_gsub_single(&p,&mixed,l),Err(Error::Limit(_))));
    }
}
#[test]fn abeezee_regular_single_execution(){verify(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,8,false);}
#[test]fn abeezee_italic_single_execution(){verify(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,8,false);}
#[test]fn alegreya_single_execution(){verify(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,35,false);}
#[test]fn aref_single_execution(){verify(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,49,false);}
#[test]fn padauk_single_execution(){verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,46,false);}
#[test]fn devanagari_single_execution(){verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,104,false);}
#[test]fn nastaliq_single_execution(){verify(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,82,true);}


// C4.4d2b: independent raw Type 2 decoding, integrated with the
// previously verified Type 1 and GDEF reference. Build a dense per-glyph
// decision table for each root, then apply it to the actual input stream.
struct RawMixed{
    index:u16,
    kind:u16,
    flags:usize,
    mark:Option<usize>,
    rules:Vec<BTreeMap<u16,Vec<u16>>>,
}
fn mixed_roots(b:&[u8],n:usize)->Vec<RawMixed>{
    let mut out:Vec<_>=single_roots(b,n).into_iter().map(|r|RawMixed{
        index:r.index,kind:1,flags:r.flags,mark:r.mark,
        rules:r.rules.into_iter().map(|m|m.into_iter().map(|(g,v)|(g,vec![v])).collect()).collect(),
    }).collect();
    let list=u(b,8);
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);
        let kind=u(b,p);let flags=u(b,p+2);let count=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*count));
        let mut rules=Vec::new();
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let (k,t)=if kind==7{
                assert_eq!(u(b,source),1);
                (u(b,source+2),source+d(b,source+4))
            }else{(kind,source)};
            if k!=2{continue;}
            assert_eq!(u(b,t),1);
            let covered=coverage(b,t+u(b,t+2));
            assert_eq!(covered.len(),u(b,t+4));
            let mut map=BTreeMap::new();
            for (g,idx) in covered{
                assert!(idx<u(b,t+4));
                let at=t+u(b,t+6+2*idx);
                let len=u(b,at);
                assert!(len>0);
                let seq:Vec<u16>=(0..len).map(|j|u(b,at+2+2*j) as u16).collect();
                assert!((g as usize)<n && seq.iter().all(|&v|(v as usize)<n));
                assert!(map.insert(g,seq).is_none());
            }
            rules.push(map);
        }
        if !rules.is_empty(){
            assert_eq!(rules.len(),count);
            out.push(RawMixed{index:i as u16,kind:2,flags,mark,rules});
        }
    }
    out.sort_by_key(|r|r.index);
    assert!(out.windows(2).all(|w|w[0].index<w[1].index));
    out
}
struct Decision<'a>{probes:usize,replacement:Option<&'a [u16]>}
fn decisions<'a>(r:&'a RawMixed,gd:&RawGdef)->Vec<Decision<'a>>{
    (0..gd.class.len()).map(|i|{
        let g=i as u16;
        if !gd.eligible(g,r.flags,r.mark){
            return Decision{probes:0,replacement:None};
        }
        let first=r.rules.iter().enumerate().find_map(|(j,m)|m.get(&g).map(|v|(j,v)));
        match first{
            Some((j,v))=>Decision{probes:j+1,replacement:Some(v.as_slice())},
            None=>Decision{probes:r.rules.len(),replacement:None},
        }
    }).collect()
}
fn mixed_reference(input:&[u16],roots:&[&RawMixed],gd:&RawGdef)->GsubMixedExecResult{
    let mut a=GsubMixedExecResult{glyphs:input.to_vec(),visits:0,probes:0,matches:0,changes:0,emitted:0};
    for root in roots{
        let table=decisions(root,gd);
        // Compute counts and final length separately from constructing the
        // output, using the dense lookup table instead of probing subtables
        // for each input occurrence.
        let mut len=0;
        for &g in &a.glyphs{
            let d=&table[g as usize];
            a.visits+=1;
            a.probes+=d.probes;
            if let Some(seq)=d.replacement{
                a.matches+=1;
                a.changes+=usize::from(seq!=[g]);
                len+=seq.len();
            }else{len+=1;}
        }
        a.emitted+=len;
        let mut next=Vec::with_capacity(len);
        for &g in &a.glyphs{
            match table[g as usize].replacement{
                Some(seq)=>next.extend_from_slice(seq),
                None=>next.push(g),
            }
        }
        assert_eq!(next.len(),len);
        a.glyphs=next;
    }
    a
}
fn verify_mixed(font:&[u8],glyphs:u32,counts:[usize;2]){
    let b=table(font,b"GSUB");let gd=table(font,b"GDEF");
    let n=glyphs as usize;
    assert_eq!(u(table(font,b"maxp"),4),n);
    let rawgd=RawGdef::read(gd,n);
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let raw=mixed_roots(b,n);
    for kind in [1u16,2]{
        assert_eq!(raw.iter().filter(|r|r.kind==kind).map(|r|r.rules.len()).sum::<usize>(),counts[kind as usize-1]);
    }
    let input:Vec<u16>=(0..n).map(|i|i as u16).collect();
    let mut type2_matches=0;
    for root in &raw{
        let p=compile_gsub_program(b,&[root.index],glyphs,Some(&g),Default::default()).unwrap();
        let want=mixed_reference(&input,&[root],&rawgd);
        let got=execute_gsub_mixed(&p,&input,Default::default()).unwrap();
        assert_eq!(got,want,"lookup {}",root.index);
        if root.kind==1{
            let legacy=execute_gsub_single(&p,&input,Default::default()).unwrap();
            assert_eq!(legacy.glyphs,got.glyphs);
            assert_eq!((legacy.visits,legacy.probes,legacy.matches,legacy.changes),
                (got.visits,got.probes,got.matches,got.changes));
        }else{type2_matches+=want.matches;}
    }
    // A stored Type 2 table is not necessarily eligible for every input,
    // but the selected real Type 2 fixtures must exercise actual matches.
    assert_eq!(type2_matches>0,counts[1]>0);
    let mixed:Vec<_>=input.iter().copied().chain(input.iter().rev().copied())
        .chain(input.iter().step_by(3).copied()).collect();
    let all:Vec<_>=raw.iter().collect();
    let multiple:Vec<_>=raw.iter().filter(|r|r.kind==2).collect();
    for selected in [&all[..],&multiple[..]]{
        let roots:Vec<_>=selected.iter().map(|r|r.index).collect();
        let p=compile_gsub_program(b,&roots,glyphs,Some(&g),Default::default()).unwrap();
        assert_eq!(p.roots(),roots.as_slice());
        assert_eq!(p.lookups().len(),roots.len());
        assert!(p.dependencies().is_empty());
        let want=mixed_reference(&mixed,selected,&rawgd);
        // Nonempty Type 2 sequences cannot shrink the stream.
        assert!(want.glyphs.len()>=mixed.len());
        let l=GsubMixedExecLimits{max_glyphs:want.glyphs.len(),max_roots:roots.len(),
            max_visits:want.visits,max_probes:want.probes,max_matches:want.matches,max_emitted:want.emitted};
        assert_eq!(execute_gsub_mixed(&p,&mixed,l).unwrap(),want);
        let mut bad=vec![GsubMixedExecLimits{max_glyphs:l.max_glyphs-1,..l}];
        if l.max_roots>0{bad.push(GsubMixedExecLimits{max_roots:l.max_roots-1,..l});}
        if l.max_visits>0{bad.push(GsubMixedExecLimits{max_visits:l.max_visits-1,..l});}
        if l.max_probes>0{bad.push(GsubMixedExecLimits{max_probes:l.max_probes-1,..l});}
        if l.max_matches>0{bad.push(GsubMixedExecLimits{max_matches:l.max_matches-1,..l});}
        if l.max_emitted>0{bad.push(GsubMixedExecLimits{max_emitted:l.max_emitted-1,..l});}
        for limit in bad{
            assert!(matches!(execute_gsub_mixed(&p,&mixed,limit),Err(Error::Limit(_))));
        }
    }
}
#[test]fn abeezee_regular_mixed_execution(){verify_mixed(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,[8,0]);}
#[test]fn abeezee_italic_mixed_execution(){verify_mixed(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,[8,0]);}
#[test]fn alegreya_mixed_execution(){verify_mixed(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,[35,0]);}
#[test]fn aref_mixed_execution(){verify_mixed(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,[49,6]);}
#[test]fn padauk_mixed_execution(){verify_mixed(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,[46,19]);}
#[test]fn devanagari_mixed_execution(){verify_mixed(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,[104,3]);}
#[test]fn nastaliq_mixed_execution(){verify_mixed(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,[82,59]);}


// C4.4d3b: raw AlternateSet decoding and independent candidate selection.
// Unlike the Type 2 decoder, a set is a list of choices, not an output run.
fn alternate_roots(b:&[u8],n:usize)->Vec<RawMixed>{
    let list=u(b,8);
    let mut out=Vec::new();
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);
        let declared=u(b,p);let flags=u(b,p+2);let count=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*count));
        let mut rules=Vec::new();
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let (kind,t)=if declared==7{
                assert_eq!(u(b,source),1);
                (u(b,source+2),source+d(b,source+4))
            }else{(declared,source)};
            if kind!=3{continue;}
            assert_eq!(u(b,t),1);
            let covered=coverage(b,t+u(b,t+2));
            assert_eq!(covered.len(),u(b,t+4));
            let mut map=BTreeMap::new();
            for (g,k) in covered{
                assert!(k<u(b,t+4));
                let at=t+u(b,t+6+2*k);
                let count=u(b,at);assert!(count>0);
                let candidates:Vec<u16>=(0..count).map(|j|u(b,at+2+2*j) as u16).collect();
                assert!((g as usize)<n && candidates.iter().all(|&v|(v as usize)<n));
                assert!(map.insert(g,candidates).is_none());
            }
            rules.push(map);
        }
        if !rules.is_empty(){
            assert_eq!(rules.len(),count);
            out.push(RawMixed{index:i as u16,kind:3,flags,mark,rules});
        }
    }
    out
}
fn alternate_reference(input:&[u16],roots:&[&RawMixed],gd:&RawGdef,
    choices:&[GsubAlternateSelection])->Option<GsubMixedExecResult>{
    let mut a=GsubMixedExecResult{glyphs:input.to_vec(),visits:0,probes:0,matches:0,changes:0,emitted:0};
    for root in roots{
        let table=decisions(root,gd);
        let choice=choices.iter().find(|c|c.lookup_index==root.index);
        let mut next=Vec::new();
        for &g in &a.glyphs{
            let d=&table[g as usize];
            a.visits+=1;a.probes+=d.probes;
            match d.replacement{
                None=>next.push(g),
                Some(seq)=>{
                    a.matches+=1;
                    if root.kind==3{
                        let v=*seq.get(choice?.candidate_index)?;
                        a.changes+=usize::from(v!=g);
                        next.push(v);
                    }else{
                        a.changes+=usize::from(seq!=[g]);
                        next.extend_from_slice(seq);
                    }
                }
            }
        }
        a.emitted+=next.len();a.glyphs=next;
    }
    Some(a)
}
fn alternate_choice(lookup_index:u16,candidate_index:usize)->GsubAlternateSelection{
    GsubAlternateSelection{lookup_index,candidate_index}
}
fn verify_alternate(font:&[u8],glyphs:u32,expected:usize){
    let b=table(font,b"GSUB");let gd=table(font,b"GDEF");
    let n=glyphs as usize;
    assert_eq!(u(table(font,b"maxp"),4),n);
    let rawgd=RawGdef::read(gd,n);
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let mut raw=mixed_roots(b,n);
    let alternates=alternate_roots(b,n);
    assert_eq!(alternates.iter().map(|r|r.rules.len()).sum::<usize>(),expected);
    let input:Vec<u16>=(0..n).map(|i|i as u16).collect();
    let mut matched=0;
    for root in &alternates{
        let p=compile_gsub_program(b,&[root.index],glyphs,Some(&g),Default::default()).unwrap();
        let table=decisions(root,&rawgd);
        let max=table.iter().filter_map(|d|d.replacement.map(|s|s.len())).max().unwrap_or(0);
        for index in 0..max{
            // All eligible first-match sets supporting this candidate;
            // also include every miss and filtered glyph.
            let valid:Vec<_>=input.iter().copied().filter(|&id|
                table[id as usize].replacement.map_or(true,|s|index<s.len())).collect();
            let choices=[alternate_choice(root.index,index)];
            let want=alternate_reference(&valid,&[root],&rawgd,&choices).unwrap();
            let got=execute_gsub_with_alternates(&p,&valid,&choices,Default::default()).unwrap();
            assert_eq!(got,want,"lookup {} candidate {}",root.index,index);
            assert_eq!(got.glyphs.len(),valid.len());
            matched+=want.matches;
            if let Some(&id)=input.iter().find(|&&id|table[id as usize].replacement.is_some_and(|s|index>=s.len())){
                assert!(alternate_reference(&[id],&[root],&rawgd,&choices).is_none());
                assert!(matches!(execute_gsub_with_alternates(&p,&[id],&choices,Default::default()),Err(Error::Invalid(_))));
            }
        }
        let invalid=[alternate_choice(root.index,usize::MAX)];
        let misses:Vec<_>=input.iter().copied().filter(|&id|table[id as usize].replacement.is_none()).collect();
        assert_eq!(execute_gsub_with_alternates(&p,&misses,&invalid,Default::default()).unwrap().glyphs,misses);
        // Check one-past-end for every eligible covered glyph, without
        // treating a valid later subtable as a fallback.
        for &id in &input{
            if let Some(seq)=table[id as usize].replacement{
                let bad=[alternate_choice(root.index,seq.len())];
                assert!(matches!(execute_gsub_with_alternates(&p,&[id],&bad,Default::default()),Err(Error::Invalid(_))));
            }
        }
        assert!(matches!(execute_gsub_with_alternates(&p,&[],&[],Default::default()),Err(Error::Invalid(_))));
        assert!(matches!(execute_gsub_mixed(&p,&[],Default::default()),Err(Error::Unsupported(_))));
    }
    assert_eq!(matched>0,expected>0);
    raw.extend(alternates);raw.sort_by_key(|r|r.index);
    assert!(raw.windows(2).all(|w|w[0].index<w[1].index));
    let mixed:Vec<_>=input.iter().copied().chain(input.iter().rev().copied())
        .chain(input.iter().step_by(3).copied()).collect();
    let all:Vec<_>=raw.iter().collect();
    let only_alt:Vec<_>=raw.iter().filter(|r|r.kind==3).collect();
    for selected in [&all[..],&only_alt[..]]{
        let ids:Vec<_>=selected.iter().map(|r|r.index).collect();
        let p=compile_gsub_program(b,&ids,glyphs,Some(&g),Default::default()).unwrap();
        assert_eq!(p.roots(),ids.as_slice());
        assert!(p.dependencies().is_empty());
        // Candidate 0, plus the highest candidate that is valid for every
        // eligible covered glyph in each selected root.
        for highest in [false,true]{
            let choices:Vec<_>=selected.iter().filter(|r|r.kind==3).map(|r|{
                let index=if highest{
                    decisions(r,&rawgd).iter().filter_map(|d|d.replacement.map(|s|s.len()))
                        .min().unwrap_or(1)-1
                }else{0};
                alternate_choice(r.index,index)
            }).collect();
            let want=alternate_reference(&mixed,selected,&rawgd,&choices).unwrap();
            let l=GsubMixedExecLimits{max_glyphs:want.glyphs.len(),max_roots:ids.len(),
                max_visits:want.visits,max_probes:want.probes,max_matches:want.matches,max_emitted:want.emitted};
            assert_eq!(execute_gsub_with_alternates(&p,&mixed,&choices,l).unwrap(),want);
            if choices.is_empty(){
                assert_eq!(execute_gsub_mixed(&p,&mixed,l).unwrap(),want);
            }
            let mut bad=vec![GsubMixedExecLimits{max_glyphs:l.max_glyphs-1,..l}];
            if l.max_roots>0{bad.push(GsubMixedExecLimits{max_roots:l.max_roots-1,..l});}
            if l.max_visits>0{bad.push(GsubMixedExecLimits{max_visits:l.max_visits-1,..l});}
            if l.max_probes>0{bad.push(GsubMixedExecLimits{max_probes:l.max_probes-1,..l});}
            if l.max_matches>0{bad.push(GsubMixedExecLimits{max_matches:l.max_matches-1,..l});}
            if l.max_emitted>0{bad.push(GsubMixedExecLimits{max_emitted:l.max_emitted-1,..l});}
            for limit in bad{
                assert!(matches!(execute_gsub_with_alternates(&p,&mixed,&choices,limit),Err(Error::Limit(_))));
            }
        }
    }
}
#[test]fn abeezee_regular_alternate_execution(){verify_alternate(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,0);}
#[test]fn abeezee_italic_alternate_execution(){verify_alternate(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,0);}
#[test]fn alegreya_alternate_execution(){verify_alternate(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,1);}
#[test]fn aref_alternate_execution(){verify_alternate(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,0);}
#[test]fn padauk_alternate_execution(){verify_alternate(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,1);}
#[test]fn devanagari_alternate_execution(){verify_alternate(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,0);}
#[test]fn nastaliq_alternate_execution(){verify_alternate(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,0);}


// C4.4d4b: raw LigatureSet oracle. Unlike the production eligible-index
// pass, consume a queue of physical glyphs and retain ignored gaps.
struct RawLigRule{output:u16,tail:Vec<u16>}
struct RawLigRoot{index:u16,flags:usize,mark:Option<usize>,rules:Vec<BTreeMap<u16,Vec<RawLigRule>>>}
fn ligature_roots(b:&[u8],n:usize)->Vec<RawLigRoot>{
    let list=u(b,8);
    let mut out=Vec::new();
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);
        let declared=u(b,p);let flags=u(b,p+2);let count=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*count));
        let mut rules=Vec::new();
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let (kind,t)=if declared==7{assert_eq!(u(b,source),1);(u(b,source+2),source+d(b,source+4))}
                else{(declared,source)};
            if kind!=4{continue;}
            assert_eq!(u(b,t),1);
            let covered=coverage(b,t+u(b,t+2));
            assert_eq!(covered.len(),u(b,t+4));
            let mut map=BTreeMap::new();
            for (g,k) in covered{
                assert!(k<u(b,t+4)&&(g as usize)<n);
                let set=t+u(b,t+6+2*k);
                let mut entries=Vec::new();
                for j in 0..u(b,set){
                    let at=set+u(b,set+2+2*j);
                    let output=u(b,at) as u16;let count=u(b,at+2);
                    assert!(count>=2&&(output as usize)<n);
                    let tail:Vec<u16>=(1..count).map(|k|u(b,at+2+2*k) as u16).collect();
                    assert!(tail.iter().all(|&v|(v as usize)<n));
                    entries.push(RawLigRule{output,tail});
                }
                assert!(!entries.is_empty());
                assert!(map.insert(g,entries).is_none());
            }
            rules.push(map);
        }
        if !rules.is_empty(){
            assert_eq!(rules.len(),count);
            out.push(RawLigRoot{index:i as u16,flags,mark,rules});
        }
    }
    out
}
fn lig_reference_pass(a:&mut GsubLigatureExecResult,root:&RawLigRoot,gd:&RawGdef){
    use std::collections::VecDeque;
    let e=&mut a.execution;
    let mut queue:VecDeque<_>=e.glyphs.iter().copied()
        .map(|g|(g,gd.eligible(g,root.flags,root.mark))).collect();
    let mut left=queue.iter().filter(|p|p.1).count();
    e.visits+=queue.len();
    let mut next=Vec::new();
    while let Some((first,eligible))=queue.pop_front(){
        if !eligible{next.push(first);continue;}
        left-=1;
        let mut found=None;
        'sub: for sub in &root.rules{
            e.probes+=1;
            if let Some(rules)=sub.get(&first){
                for rule in rules{
                    a.rule_attempts+=1;
                    if rule.tail.len()>left{continue;}
                    let mut good=true;
                    for ((g,_),want) in queue.iter().filter(|p|p.1).zip(&rule.tail){
                        a.component_checks+=1;
                        if g!=want{good=false;break;}
                    }
                    if good{found=Some(rule);break 'sub;}
                }
            }
        }
        if let Some(rule)=found{
            e.matches+=1;e.changes+=1;a.ligatures+=1;
            a.removed_glyphs+=rule.tail.len();
            next.push(rule.output);
            let mut consumed=0;
            while consumed<rule.tail.len(){
                let (g,eligible)=queue.pop_front().unwrap();
                if eligible{consumed+=1;left-=1;}else{next.push(g);}
            }
        }else{next.push(first);}
    }
    assert_eq!(left,0);
    e.emitted+=next.len();e.glyphs=next;
}
fn lig_reference(input:&[u16],mixed:&[RawMixed],ligatures:&[RawLigRoot],
    ids:&[u16],gd:&RawGdef,choices:&[GsubAlternateSelection])->(GsubLigatureExecResult,usize){
    let mut a=GsubLigatureExecResult{execution:GsubMixedExecResult{glyphs:input.to_vec(),
        visits:0,probes:0,matches:0,changes:0,emitted:0},ligatures:0,removed_glyphs:0,
        rule_attempts:0,component_checks:0};
    let mut peak=input.len();
    for &id in ids{
        if let Some(root)=ligatures.iter().find(|r|r.index==id){
            lig_reference_pass(&mut a,root,gd);
        }else{
            let root=mixed.iter().find(|r|r.index==id).unwrap();
            let step=alternate_reference(&a.execution.glyphs,&[root],gd,choices).unwrap();
            a.execution.visits+=step.visits;a.execution.probes+=step.probes;
            a.execution.matches+=step.matches;a.execution.changes+=step.changes;
            a.execution.emitted+=step.emitted;a.execution.glyphs=step.glyphs;
        }
        peak=peak.max(a.execution.glyphs.len());
    }
    (a,peak)
}
fn verify_ligature(font:&[u8],glyphs:u32,expected:usize){
    let b=table(font,b"GSUB");let gd=table(font,b"GDEF");
    let n=glyphs as usize;
    assert_eq!(u(table(font,b"maxp"),4),n);
    let rawgd=RawGdef::read(gd,n);
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let mut mixed=mixed_roots(b,n);mixed.extend(alternate_roots(b,n));mixed.sort_by_key(|r|r.index);
    let raw=ligature_roots(b,n);
    assert_eq!(raw.iter().map(|r|r.rules.len()).sum::<usize>(),expected);
    let mut matched=0;
    let mut sample=Vec::new();
    for root in &raw{
        let p=compile_gsub_program(b,&[root.index],glyphs,Some(&g),Default::default()).unwrap();
        assert!(matches!(execute_gsub_with_alternates(&p,&[],&[],Default::default()),Err(Error::Unsupported(_))));
        let ignored=(0..n as u16).find(|&id|!rawgd.eligible(id,root.flags,root.mark));
        let mut samples=0;
        for sub in &root.rules{
            for (&first,rules) in sub{
                for rule in rules{
                    let input:Vec<_>=std::iter::once(first).chain(rule.tail.iter().copied()).collect();
                    let (want,_)=lig_reference(&input,&mixed,&raw,&[root.index],&rawgd,&[]);
                    assert_eq!(execute_gsub_with_ligatures(&p,&input,&[],Default::default()).unwrap(),want,
                        "lookup {} input {:?}",root.index,input);
                    matched+=want.ligatures;
                    if let Some(mark)=ignored{
                        let mut decorated=vec![mark,first];
                        for &v in &rule.tail{decorated.extend([mark,v]);}
                        decorated.push(mark);
                        let (want,_)=lig_reference(&decorated,&mixed,&raw,&[root.index],&rawgd,&[]);
                        assert_eq!(execute_gsub_with_ligatures(&p,&decorated,&[],Default::default()).unwrap(),want);
                    }
                    if samples<8{sample.extend_from_slice(&input);samples+=1;}
                }
            }
        }
    }
    assert!(matched>0);
    // Include every glyph in both directions and genuine rule sequences.
    // For mixed execution record the peak, not only the final length:
    // Type 2 may expand a stream before a later ligature shrinks it.
    let input:Vec<u16>=(0..n as u16).chain((0..n as u16).rev())
        .chain(sample.iter().copied()).collect();
    let lig_ids:Vec<_>=raw.iter().map(|r|r.index).collect();
    let mut all=lig_ids.clone();all.extend(mixed.iter().map(|r|r.index));all.sort_unstable();
    assert!(all.windows(2).all(|w|w[0]<w[1]));
    for ids in [&lig_ids[..],&all[..]]{
        let choices:Vec<_>=mixed.iter().filter(|r|r.kind==3&&ids.binary_search(&r.index).is_ok())
            .map(|r|alternate_choice(r.index,0)).collect();
        let p=compile_gsub_program(b,ids,glyphs,Some(&g),Default::default()).unwrap();
        assert_eq!(p.roots(),ids);assert!(p.dependencies().is_empty());
        let (want,peak)=lig_reference(&input,&mixed,&raw,ids,&rawgd,&choices);
        let e=&want.execution;
        let l=GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_glyphs:peak,max_roots:ids.len(),
            max_visits:e.visits,max_probes:e.probes,max_matches:e.matches,max_emitted:e.emitted},
            max_rule_attempts:want.rule_attempts,max_component_checks:want.component_checks};
        assert_eq!(execute_gsub_with_ligatures(&p,&input,&choices,l).unwrap(),want);
        let mut bad=vec![GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_glyphs:peak-1,..l.mixed},..l},
            GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_roots:ids.len()-1,..l.mixed},..l}];
        if e.visits>0{bad.push(GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_visits:e.visits-1,..l.mixed},..l});}
        if e.probes>0{bad.push(GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_probes:e.probes-1,..l.mixed},..l});}
        if e.matches>0{bad.push(GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_matches:e.matches-1,..l.mixed},..l});}
        if e.emitted>0{bad.push(GsubLigatureExecLimits{mixed:GsubMixedExecLimits{max_emitted:e.emitted-1,..l.mixed},..l});}
        if want.rule_attempts>0{bad.push(GsubLigatureExecLimits{max_rule_attempts:want.rule_attempts-1,..l});}
        if want.component_checks>0{bad.push(GsubLigatureExecLimits{max_component_checks:want.component_checks-1,..l});}
        for limit in bad{
            assert!(matches!(execute_gsub_with_ligatures(&p,&input,&choices,limit),Err(Error::Limit(_))));
        }
    }
}
#[test]fn abeezee_regular_ligature_execution(){verify_ligature(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,3);}
#[test]fn abeezee_italic_ligature_execution(){verify_ligature(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,3);}
#[test]fn alegreya_ligature_execution(){verify_ligature(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,11);}
#[test]fn aref_ligature_execution(){verify_ligature(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,1);}
#[test]fn padauk_ligature_execution(){verify_ligature(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,22);}
#[test]fn devanagari_ligature_execution(){verify_ligature(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,35);}
#[test]fn nastaliq_ligature_execution(){verify_ligature(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,5);}


// C4.4d5b: execute native Type 3 subtable bytes through a constructed
// Type 5 Format 3 parent. The parent is synthetic, not a native font
// context. Raw candidate decoding and selection remain independent.
fn native_alternate_parent(target:&[u8],first:u16)->Vec<u8>{
    // LookupList at word 7; two roots, first is dependency-only.
    let mut b:Vec<u8>=[1u16,0,10,12,14,0,0,2,6,0,3,0,1,8]
        .iter().flat_map(|v|v.to_be_bytes()).collect();
    b.extend_from_slice(target);
    if b.len()%2!=0{b.push(0);}
    let offset=u16::try_from(b.len()-14).unwrap();
    b[18..20].copy_from_slice(&offset.to_be_bytes());
    // Context format 3: one covered input, one action (0, lookup 0).
    b.extend([5u16,0,1,8,3,1,1,12,0,0,1,1,first].iter().flat_map(|v|v.to_be_bytes()));
    b
}
fn verify_native_alternate_context(font:&[u8],n:u32){
    let b=table(font,b"GSUB");
    let roots=alternate_roots(b,n as usize);
    assert!(!roots.is_empty());
    let list=u(b,8);
    let mut checked=0;
    for root in &roots{
        let p=list+u(b,list+2+2*root.index as usize);
        for (j,map) in root.rules.iter().enumerate(){
            let source=p+u(b,p+6+2*j);
            let t=if u(b,p)==7{source+d(b,source+4)}else{source};
            // Copy only the actual subtable extent: header, coverage and
            // alternate sets. Relative offsets are preserved unchanged.
            let c=t+u(b,t+2);
            let cov_end=c+4+u(b,c+2)*if u(b,c)==1{2}else{6};
            let mut end=cov_end.max(t+6+2*u(b,t+4));
            for i in 0..u(b,t+4){
                let s=t+u(b,t+6+2*i);
                end=end.max(s+2+2*u(b,s));
            }
            for (&glyph,candidates) in map{
                let bytes=native_alternate_parent(&b[t..end],glyph);
                let program=compile_gsub_program(&bytes,&[1],n,None,Default::default()).unwrap();
                for (index,&want) in candidates.iter().enumerate(){
                    let input=[glyph,glyph];
                    let a=execute_gsub_context_with_alternates(&program,&input,&[alternate_choice(0,index)],Default::default()).unwrap();
                    assert_eq!(a.glyphs,vec![want,want]);
                    assert_eq!((a.actions,a.matches,a.peak_depth),(2,4,2));
                    checked+=1;
                }
                assert!(matches!(execute_gsub_context_with_alternates(&program,&[glyph],
                    &[alternate_choice(0,candidates.len())],Default::default()),Err(Error::Invalid(_))));
            }
        }
    }
    assert!(checked>0);
}
#[test]fn alegreya_native_alternate_nested_reference(){verify_native_alternate_context(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109);}
#[test]fn padauk_native_alternate_nested_reference(){verify_native_alternate_context(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782);}


// C4.4d5c: native Type 5 Format 1/2/3, independent raw rule decoding.
// The oracle uses raw coverage/class arrays and physical iterators, not
// production matchers, lookup filters, compiled actions or closure.
#[derive(Clone)]
struct NativeContextRule{tail:Vec<u16>,actions:Vec<(usize,u16)>}
enum NativeContextBody{
    Glyph{sets:Vec<Option<Vec<NativeContextRule>>>},
    Class{classes:Vec<usize>,sets:Vec<Option<Vec<NativeContextRule>>>},
    Coverage{sets:Vec<Vec<bool>>,actions:Vec<(usize,u16)>},
}
struct NativeContextSub{first:Vec<Option<usize>>,body:NativeContextBody}
struct NativeContextRoot{index:u16,flags:usize,mark:Option<usize>,subs:Vec<NativeContextSub>}
fn native_actions(b:&[u8],p:usize,count:usize,input:usize,n:usize)->Vec<(usize,u16)>{
    (0..count).map(|i|{
        let pos=u(b,p+4*i);let id=u(b,p+4*i+2);
        assert!(pos<input&&id<n);
        (pos,id as u16)
    }).collect()
}
fn native_context_sets(b:&[u8],t:usize,base:usize,n:usize)->Vec<Option<Vec<NativeContextRule>>>{
    (0..u(b,base)).map(|i|{
        let off=u(b,base+2+2*i);
        if off==0{return None;}
        let set=t+off;
        Some((0..u(b,set)).map(|j|{
            let at=set+u(b,set+2+2*j);
            let input=u(b,at);assert!(input>0);
            let tail=(1..input).map(|k|u(b,at+2+2*k) as u16).collect();
            let actions=native_actions(b,at+4+2*(input-1),u(b,at+2),input,n);
            NativeContextRule{tail,actions}
        }).collect())
    }).collect()
}
fn native_context_roots(b:&[u8],n:usize)->Vec<NativeContextRoot>{
    let list=u(b,8);let count=u(b,list);
    let mut out=Vec::new();
    for i in 0..count{
        let p=list+u(b,list+2+2*i);
        let declared=u(b,p);let flags=u(b,p+2);let subcount=u(b,p+4);
        let mark=(flags&16!=0).then(||u(b,p+6+2*subcount));
        let mut subs=Vec::new();
        for j in 0..subcount{
            let s=p+u(b,p+6+2*j);
            let (kind,t)=if declared==7{(u(b,s+2),s+d(b,s+4))}else{(declared,s)};
            if kind!=5{continue;}
            let mut first=vec![None;n];
            let body=match u(b,t){
                1=>{
                    for (g,k) in coverage(b,t+u(b,t+2)){first[g as usize]=Some(k);}
                    NativeContextBody::Glyph{sets:native_context_sets(b,t,t+4,count)}
                }
                2=>{
                    for (g,k) in coverage(b,t+u(b,t+2)){first[g as usize]=Some(k);}
                    NativeContextBody::Class{classes:classes(b,t+u(b,t+4),n),
                        sets:native_context_sets(b,t,t+6,count)}
                }
                3=>{
                    let len=u(b,t+2);assert!(len>0);
                    let sets:Vec<Vec<bool>>=(0..len).map(|k|{
                        let mut set=vec![false;n];
                        for (g,_) in coverage(b,t+u(b,t+6+2*k)){set[g as usize]=true;}
                        set
                    }).collect();
                    NativeContextBody::Coverage{sets,actions:native_actions(b,t+6+2*len,u(b,t+4),len,count)}
                }
                _=>panic!("context format"),
            };
            subs.push(NativeContextSub{first,body});
        }
        if !subs.is_empty(){assert_eq!(subs.len(),subcount);out.push(NativeContextRoot{index:i as u16,flags,mark,subs});}
    }
    out
}
fn native_edges(root:&NativeContextRoot)->Vec<u16>{
    let mut out=Vec::new();
    for sub in &root.subs{
        match &sub.body{
            NativeContextBody::Glyph{sets}|NativeContextBody::Class{sets,..}=>{
                for rules in sets.iter().flatten(){for rule in rules{out.extend(rule.actions.iter().map(|a|a.1));}}
            }
            NativeContextBody::Coverage{actions,..}=>out.extend(actions.iter().map(|a|a.1)),
        }
    }
    out
}
fn native_kind(b:&[u8],id:u16)->usize{
    let list=u(b,8);let p=list+u(b,list+2+2*id as usize);
    if u(b,p)==7{u(b,p+u(b,p+6)+2)}else{u(b,p)}
}
fn native_supported_closure(b:&[u8],root:u16,contexts:&[NativeContextRoot])->Option<Vec<u16>>{
    let mut seen=std::collections::BTreeSet::new();
    let mut pending=vec![root];
    while let Some(id)=pending.pop(){
        if !seen.insert(id){continue;}
        match native_kind(b,id){
            1|3=>{},
            5=>pending.extend(native_edges(contexts.iter().find(|r|r.index==id).unwrap())),
            _=>return None,
        }
    }
    Some(seen.into_iter().collect())
}
fn native_tail<F:FnMut(usize,u16)->bool>(a:&mut GsubContextExecResult,gd:&RawGdef,
    root:&NativeContextRoot,p:usize,len:usize,mut eq:F)->Option<Vec<usize>>{
    let mut positions=vec![p];
    // Scan physical positions once for this attempted rule. Stop on the
    // first mismatch or after collecting all requested symbols.
    if len==0{return Some(positions);}
    for i in p+1..a.glyphs.len(){
        a.filter_checks+=1;
        let g=a.glyphs[i];
        if !gd.eligible(g,root.flags,root.mark){continue;}
        a.symbols+=1;
        if !eq(positions.len()-1,g){return None;}
        positions.push(i);
        if positions.len()==len+1{return Some(positions);}
    }
    None
}
fn native_match<'a>(a:&mut GsubContextExecResult,gd:&RawGdef,root:&NativeContextRoot,
    sub:&'a NativeContextSub,p:usize)->Option<(Vec<usize>,&'a [(usize,u16)])>{
    let g=a.glyphs[p] as usize;
    match &sub.body{
        NativeContextBody::Glyph{sets}=>{
            let rules=sets.get(sub.first[g]?)?.as_ref()?;
            for rule in rules{
                a.rules+=1;
                if let Some(pos)=native_tail(a,gd,root,p,rule.tail.len(),|i,v|v==rule.tail[i]){
                    return Some((pos,&rule.actions));
                }
            }
        }
        NativeContextBody::Class{classes,sets}=>{
            sub.first[g]?;
            let rules=sets.get(classes[g])?.as_ref()?;
            for rule in rules{
                a.rules+=1;
                if let Some(pos)=native_tail(a,gd,root,p,rule.tail.len(),|i,v|classes[v as usize]==rule.tail[i] as usize){
                    return Some((pos,&rule.actions));
                }
            }
        }
        NativeContextBody::Coverage{sets,actions}=>{
            a.rules+=1;a.symbols+=1;
            if sets[0][g]{
                if let Some(pos)=native_tail(a,gd,root,p,sets.len()-1,|i,v|sets[i+1][v as usize]){
                    return Some((pos,actions));
                }
            }
        }
    }
    None
}
fn native_apply(a:&mut GsubContextExecResult,gd:&RawGdef,mixed:&[RawMixed],
    contexts:&[NativeContextRoot],id:u16,p:usize,depth:usize)->Option<usize>{
    // All existing native fixtures have acyclic static dependencies.
    assert!(depth<=32);
    a.calls+=1;a.peak_depth=a.peak_depth.max(depth);a.filter_checks+=1;
    if let Some(root)=mixed.iter().find(|r|r.index==id){
        let g=a.glyphs[p];
        if !gd.eligible(g,root.flags,root.mark){return None;}
        for sub in &root.rules{
            a.probes+=1;
            if let Some(v)=sub.get(&g){
                assert!(root.kind==1||root.kind==3);
                a.matches+=1;a.changes+=usize::from(v[0]!=g);a.glyphs[p]=v[0];
                return Some(p+1);
            }
        }
        return None;
    }
    let root=contexts.iter().find(|r|r.index==id).unwrap();
    if !gd.eligible(a.glyphs[p],root.flags,root.mark){return None;}
    for sub in &root.subs{
        a.probes+=1;
        if let Some((pos,actions))=native_match(a,gd,root,sub,p){
            a.matches+=1;
            for &(sequence,target) in actions{
                a.actions+=1;
                native_apply(a,gd,mixed,contexts,target,pos[sequence],depth+1);
            }
            return Some(pos.last().unwrap()+1);
        }
    }
    None
}
fn native_reference(input:&[u16],gd:&RawGdef,mixed:&[RawMixed],contexts:&[NativeContextRoot],root:u16)
    ->GsubContextExecResult{
    let mut a=GsubContextExecResult{glyphs:input.to_vec(),..Default::default()};
    let mut p=0;
    while p<input.len(){p=native_apply(&mut a,gd,mixed,contexts,root,p,1).unwrap_or(p+1);}
    a
}
// Build deterministic rule witnesses without using production coverage or
// class matchers. Limit the number per subtable to keep CI bounded.
fn native_witnesses(root:&NativeContextRoot,gd:&RawGdef)->Vec<Vec<u16>>{
    let eligible:Vec<_>=(0..gd.class.len()).map(|g|g as u16)
        .filter(|&g|gd.eligible(g,root.flags,root.mark)).collect();
    let ignored=(0..gd.class.len()).map(|g|g as u16).find(|&g|!gd.eligible(g,root.flags,root.mark));
    let mut all=Vec::new();
    for sub in &root.subs{
        let mut samples=Vec::new();
        match &sub.body{
            NativeContextBody::Glyph{sets}|NativeContextBody::Class{sets,..}=>{
                let class=if let NativeContextBody::Class{classes,..}=&sub.body{Some(classes)}else{None};
                'first: for &g in &eligible{
                    let Some(index)=sub.first[g as usize] else{continue};
                    let set=class.map_or(index,|c|c[g as usize]);
                    if let Some(Some(rules))=sets.get(set){
                        for r in rules{
                            let tail:Option<Vec<u16>>=r.tail.iter().map(|&symbol|{
                                if let Some(c)=class{eligible.iter().copied().find(|&v|c[v as usize]==symbol as usize)}
                                else{eligible.binary_search(&symbol).ok().map(|_|symbol)}
                            }).collect();
                            if let Some(tail)=tail{
                                let mut v=vec![g];v.extend(tail);samples.push(v);
                                if samples.len()>=8{break 'first;}
                            }
                        }
                    }
                }
            }
            NativeContextBody::Coverage{sets,..}=>{
                let v:Option<Vec<u16>>=sets.iter().map(|s|eligible.iter().copied().find(|&g|s[g as usize])).collect();
                if let Some(v)=v{samples.push(v);}
            }
        }
        for v in samples{
            if let Some(mark)=ignored{
                let mut decorated=vec![mark];
                for &g in &v{decorated.extend([g,mark]);}
                all.push(decorated);
            }
            all.push(v);
        }
    }
    all
}
fn verify_native_context(font:&[u8],n:u32,expected:usize)->(usize,usize){
    let b=table(font,b"GSUB");let gd=table(font,b"GDEF");
    assert_eq!(u(table(font,b"maxp"),4),n as usize);
    let rawgd=RawGdef::read(gd,n as usize);
    let g=parse_gdef(gd,n,Default::default()).unwrap();
    let mut mixed=mixed_roots(b,n as usize);mixed.extend(alternate_roots(b,n as usize));
    mixed.retain(|r|r.kind==1||r.kind==3);
    let contexts=native_context_roots(b,n as usize);
    assert_eq!(contexts.iter().map(|r|r.subs.len()).sum::<usize>(),expected);
    let mut supported=0;let mut actions=0;
    for root in &contexts{
        let p=compile_gsub_program(b,&[root.index],n,Some(&g),Default::default()).unwrap();
        let Some(ids)=native_supported_closure(b,root.index,&contexts) else{
            // Supply all reachable alternate choices to isolate unsupported
            // kind rejection from the policy's missing-choice validation.
            let choices:Vec<_>=p.lookups().iter().filter(|l|l.effective_kind()==3)
                .map(|l|alternate_choice(l.index(),0)).collect();
            assert!(matches!(execute_gsub_context_with_alternates(&p,&[],&choices,Default::default()),Err(Error::Unsupported(_))));
            continue;
        };
        supported+=1;
        assert_eq!(p.closure().reachable,ids);
        assert!(p.closure().is_proven_acyclic());
        let choices:Vec<_>=ids.iter().copied().filter(|&id|native_kind(b,id)==3).map(|id|alternate_choice(id,0)).collect();
        let witnesses=native_witnesses(root,&rawgd);
        // Verify each bounded witness independently, so neighbouring
        // samples cannot accidentally suppress its context.
        for input in &witnesses{
            let want=native_reference(input,&rawgd,&mixed,&contexts,root.index);
            actions+=want.actions;
            assert_eq!(execute_gsub_context_with_alternates(&p,input,&choices,Default::default()).unwrap(),want);
        }
        let input:Vec<_>=(0..n as u16).chain((0..n as u16).rev())
            .chain(witnesses.iter().take(16).flatten().copied()).collect();
        let want=native_reference(&input,&rawgd,&mixed,&contexts,root.index);
        actions+=want.actions;
        let l=GsubContextExecLimits{max_glyphs:input.len(),max_roots:1,max_lookups:ids.len(),
            max_depth:want.peak_depth,max_calls:want.calls,max_probes:want.probes,
            max_filter_checks:want.filter_checks,max_rules:want.rules,max_symbols:want.symbols,
            max_actions:want.actions,max_matches:want.matches};
        assert_eq!(execute_gsub_context_with_alternates(&p,&input,&choices,l).unwrap(),want);
        if choices.is_empty(){assert_eq!(execute_gsub_context(&p,&input,l).unwrap(),want);}
        let mut bad=vec![GsubContextExecLimits{max_glyphs:l.max_glyphs-1,..l},
            GsubContextExecLimits{max_roots:0,..l},GsubContextExecLimits{max_lookups:l.max_lookups-1,..l}];
        for (count,limit) in [
            (l.max_depth,GsubContextExecLimits{max_depth:l.max_depth.saturating_sub(1),..l}),
            (l.max_calls,GsubContextExecLimits{max_calls:l.max_calls.saturating_sub(1),..l}),
            (l.max_probes,GsubContextExecLimits{max_probes:l.max_probes.saturating_sub(1),..l}),
            (l.max_filter_checks,GsubContextExecLimits{max_filter_checks:l.max_filter_checks.saturating_sub(1),..l}),
            (l.max_rules,GsubContextExecLimits{max_rules:l.max_rules.saturating_sub(1),..l}),
            (l.max_symbols,GsubContextExecLimits{max_symbols:l.max_symbols.saturating_sub(1),..l}),
            (l.max_actions,GsubContextExecLimits{max_actions:l.max_actions.saturating_sub(1),..l}),
            (l.max_matches,GsubContextExecLimits{max_matches:l.max_matches.saturating_sub(1),..l}),
        ]{if count>0{bad.push(limit);}}
        for limit in bad{assert!(matches!(execute_gsub_context_with_alternates(&p,&input,&choices,limit),Err(Error::Limit(_))));}
    }
    (supported,actions)
}
#[test]fn abeezee_regular_native_context(){assert_eq!(verify_native_context(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,0),(0,0));}
#[test]fn abeezee_italic_native_context(){assert_eq!(verify_native_context(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,0),(0,0));}
#[test]fn alegreya_native_context(){assert_eq!(verify_native_context(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,0),(0,0));}
#[test]fn aref_native_context(){assert_eq!(verify_native_context(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,0),(0,0));}
#[test]fn padauk_native_context(){let _=verify_native_context(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,6);}
#[test]fn devanagari_native_context(){let _=verify_native_context(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,85);}
#[test]fn nastaliq_native_context(){let _=verify_native_context(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,1341);}
