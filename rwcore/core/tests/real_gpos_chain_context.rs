//! Independent raw-byte reference for GPOS type 8, including type 9 targets.
use rwcore::*;
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn cov(b:&[u8],p:usize)->Vec<u16>{
    match u(b,p){
        1=>(0..u(b,p+2)).map(|i|u(b,p+4+2*i) as u16).collect(),
        2=>{let mut v=Vec::new();for i in 0..u(b,p+2){let q=p+4+6*i;assert_eq!(u(b,q+4),v.len());v.extend((u(b,q)..=u(b,q+2)).map(|x|x as u16));}v},
        _=>panic!("coverage format"),
    }
}
fn classes(b:&[u8],p:usize,n:usize)->Vec<u16>{
    let mut v=vec![0;n];
    match u(b,p){
        1=>{let a=u(b,p+2);for i in 0..u(b,p+4){v[a+i]=u(b,p+6+2*i) as u16;}},
        2=>{for i in 0..u(b,p+2){let q=p+4+6*i;for g in u(b,q)..=u(b,q+2){v[g]=u(b,q+4) as u16;}}},
        _=>panic!("class format"),
    }v
}
fn actions(b:&[u8],p:usize,n:usize,input:usize,lookups:usize)->Vec<ContextAction>{
    (0..n).map(|i|{let q=p+4*i;let a=u(b,q);let z=u(b,q+2);assert!(a<input&&z<lookups);ContextAction{sequence_index:a as u16,lookup_index:z as u16}}).collect()
}

fn words(b:&[u8],p:&mut usize,n:usize)->Vec<u16>{
    let v=(0..n).map(|i|u(b,*p+2*i) as u16).collect();*p+=2*n;v
}
fn count(b:&[u8],p:&mut usize)->usize{let n=u(b,*p);*p+=2;n}
fn sets(b:&[u8],p:usize,n:usize,lookups:usize)->Vec<Option<Vec<ChainContextRule>>>{
    (0..n).map(|i|{let at=u(b,p+2*i);if at==0{return None;}
        Some((0..u(b,at)).map(|j|{
            let mut q=at+u(b,at+2+2*j);
            let k=count(b,&mut q);let backtrack=words(b,&mut q,k);
            let len=count(b,&mut q);assert!(len>0);
            let remaining=words(b,&mut q,len-1);
            let k=count(b,&mut q);let lookahead=words(b,&mut q,k);
            let k=count(b,&mut q);
            ChainContextRule{backtrack,remaining,lookahead,actions:actions(b,q,k,len,lookups)}
        }).collect())
    }).collect()
}
fn check(b:&[u8],n:usize,lookups:usize)->[usize;3]{
    let f=u(b,0);let mut totals=[1,0,0];
    if f==3{
        let mut q=2;
        let cv:[Vec<Vec<u16>>;3]=std::array::from_fn(|_|{
            let k=count(b,&mut q);words(b,&mut q,k).iter().map(|&o|cov(b,o as usize)).collect()
        });
        let k=count(b,&mut q);let refs=actions(b,q,k,cv[1].len(),lookups);
        let p=parse_coverage_chain_context_positioning(b,n as u32,lookups,Default::default()).unwrap();
        assert_eq!(p.actions(),refs.as_slice());
        for (actual,expected) in [p.backtrack(),p.input(),p.lookahead()].iter().zip(&cv){
            assert_eq!(actual.len(),expected.len());
            for (a,e) in actual.iter().zip(expected){for g in 0..n{assert_eq!(a.index(g as u16).is_some(),e.contains(&(g as u16)));}}
        }
        let ordered:Vec<_>=cv[0].iter().rev().chain(&cv[1]).chain(&cv[2]).collect();
        if let Some(mut run)=ordered.iter().map(|c|c.first().copied()).collect::<Option<Vec<_>>>(){
            for (i,c) in ordered.iter().enumerate(){let old=run[i];for g in 0..n{
                run[i]=g as u16;let m=p.match_at(&run,cv[0].len());
                assert_eq!(m.is_some(),c.contains(&(g as u16)));
                if let Some(m)=m{assert_eq!(m.input_len,cv[1].len());assert_eq!(m.actions,refs.as_slice());totals[1]+=1;totals[2]+=refs.len();}
            }run[i]=old;}
        }
        return totals;
    }
    assert!(f==1||f==2);
    let covered=cov(b,u(b,2));
    let cls:[Vec<u16>;3]=std::array::from_fn(|i|if f==2{classes(b,u(b,4+2*i),n)}else{(0..n).map(|g|g as u16).collect()});
    let raw=if f==1{sets(b,6,u(b,4),lookups)}else{sets(b,12,u(b,10),lookups)};
    let glyph=if f==1{Some(parse_chain_context_positioning(b,n as u32,lookups,Default::default()).unwrap())}else{None};
    let class=if f==2{Some(parse_class_chain_context_positioning(b,n as u32,lookups,Default::default()).unwrap())}else{None};
    assert_eq!(if let Some(p)=&glyph{p.rule_sets()}else{class.as_ref().unwrap().rule_sets()},raw.as_slice());
    let matcher=|run:&[u16],pos:usize|if let Some(p)=&glyph{p.match_at(run,pos)}else{class.as_ref().unwrap().match_at(run,pos)};
    for g in 0..n{
        let g=g as u16;
        let rs=covered.binary_search(&g).ok().and_then(|i|raw.get(if f==1{i}else{cls[1][g as usize] as usize})).and_then(|x|x.as_deref());
        assert_eq!(if let Some(p)=&glyph{p.rules(g)}else{class.as_ref().unwrap().rules(g)},rs);
        let Some(rs)=rs else{assert!(matcher(&[g],0).is_none());continue;};
        for rule in rs{
            let rep=|c:u16,k:usize|cls[k].iter().position(|&v|v==c).map(|x|x as u16);
            let Some(back)=rule.backtrack.iter().rev().map(|&c|rep(c,0)).collect::<Option<Vec<_>>>() else{continue;};
            let Some(input)=rule.remaining.iter().map(|&c|rep(c,1)).collect::<Option<Vec<_>>>() else{continue;};
            let Some(ahead)=rule.lookahead.iter().map(|&c|rep(c,2)).collect::<Option<Vec<_>>>() else{continue;};
            let pos=back.len();let mut run=back;run.push(g);run.extend(input);run.extend(ahead);
            // Every prefix/suffix truncation of this representative sequence.
            for start in 0..=pos{for end in pos+1..=run.len(){
                let stream=&run[start..end];let i=pos-start;
                let want=rs.iter().find(|r|{
                    let tail=&stream[i+1..];
                    i>=r.backtrack.len()&&tail.len()>=r.remaining.len()+r.lookahead.len()&&
                    r.backtrack.iter().enumerate().all(|(j,&c)|cls[0][stream[i-1-j] as usize]==c)&&
                    r.remaining.iter().enumerate().all(|(j,&c)|cls[1][tail[j] as usize]==c)&&
                    r.lookahead.iter().enumerate().all(|(j,&c)|cls[2][tail[r.remaining.len()+j] as usize]==c)
                });
                match (matcher(stream,i),want){
                    (None,None)=>{},
                    (Some(m),Some(r))=>{assert_eq!(m.input_len,r.remaining.len()+1);assert_eq!(m.actions,r.actions.as_slice());totals[1]+=1;totals[2]+=r.actions.len();},
                    _=>panic!("chain match mismatch"),
                }
            }}
        }
    }
    totals
}
#[test]fn native_gpos_chain_reference(){
    let mut counts=[[0usize;3];3];
    for font in [
        include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf").as_slice(),
        include_bytes!("fixtures/abel/Abel-Regular.ttf").as_slice(),
        include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf").as_slice(),
        include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf").as_slice(),
        include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf").as_slice(),
        include_bytes!("fixtures/notosansmongolian/NotoSansMongolian-Regular.ttf").as_slice(),
        include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/padauk/Padauk-Regular.ttf").as_slice(),
    ]{
        let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
        let Some(b)=sf.table(*b"GPOS") else{continue;};let n=u(sf.table(*b"maxp").unwrap(),4);
        let list=u(b,8);let lookups=u(b,list);
        for i in 0..lookups{let p=list+u(b,list+2+2*i);for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==9{assert_eq!(u(b,at),1);kind=u(b,at+2);at+=u(b,at+4)*65536+u(b,at+6);}
            if kind!=8{continue;}
            let f=u(b,at);assert!((1..=3).contains(&f));let q=check(&b[at..],n,lookups);
            for k in 0..3{counts[f-1][k]+=q[k];}
        }}
    }
    eprintln!("native GPOS chain [subtables, matches, actions] by format: {counts:?}");
    assert!(counts.iter().any(|x|x.iter().all(|&n|n>0)),"no effective native Type 8 evidence: {counts:?}");
}
#[test]fn independent_all_three_synthetic_chain_formats(){
    for w in [
        &[1,8,1,14,1,1,10,2,6,30,2,9,8,2,11,1,12,2,1,4,0,3,0,1,0,0][..],
        &[2,18,26,42,58,3,0,66,0,1,2,10,20,2,2,8,8,2,9,9,1,2,2,10,11,1,20,21,2,1,12,1,7,2,6,30,2,1,2,2,2,1,7,2,1,4,0,3,0,1,0,0][..],
        &[3,2,28,36,2,42,50,1,60,2,1,4,0,3,1,2,9,19,1,1,8,1,2,10,20,2,1,11,12,0,1,1,13][..],
    ]{
        let b:Vec<u8>=w.iter().flat_map(|v|(*v as u16).to_be_bytes()).collect();
        assert!(check(&b,32,5).iter().all(|&n|n>0));
    }
}
