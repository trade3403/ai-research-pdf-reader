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
