//! Independent raw-byte reference for GPOS type 7, including type 9 targets.
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
fn sets(b:&[u8],p:usize,n:usize,lookups:usize)->Vec<Option<Vec<ContextRule>>>{
    (0..n).map(|i|{let at=u(b,p+2*i);if at==0{return None;}
        Some((0..u(b,at)).map(|j|{let q=at+u(b,at+2+2*j);let len=u(b,q);assert!(len>0);
            ContextRule{remaining:(0..len-1).map(|k|u(b,q+4+2*k) as u16).collect(),actions:actions(b,q+4+2*(len-1),u(b,q+2),len,lookups)}
        }).collect())
    }).collect()
}
fn compare(got:Option<ContextMatch<'_>>,want:Option<&ContextRule>){
    match(got,want){
        (None,None)=>{},
        (Some(m),Some(r))=>{assert_eq!(m.input_len,r.remaining.len()+1);assert_eq!(m.actions,r.actions.as_slice());},
        _=>panic!("match mismatch"),
    }
}
// [subtables, positive matches, matched actions] per format.
fn check(b:&[u8],n:usize,lookups:usize)->[usize;3]{
    let mut result=[1,0,0];let f=u(b,0);
    match f{
        1|2=>{
            let coverage=cov(b,u(b,2));
            let cls=if f==2{classes(b,u(b,4),n)}else{Vec::new()};
            let raw=if f==1{sets(b,6,u(b,4),lookups)}else{sets(b,8,u(b,6),lookups)};
            let glyph=if f==1{Some(parse_context_positioning(b,n as u32,lookups,Default::default()).unwrap())}else{None};
            let class=if f==2{Some(parse_class_context_positioning(b,n as u32,lookups,Default::default()).unwrap())}else{None};
            assert_eq!(if let Some(p)=&glyph{p.rule_sets()}else{class.as_ref().unwrap().rule_sets()},raw.as_slice());
            let representatives=if f==2{let mut reps=vec![None;cls.iter().copied().max().unwrap_or(0) as usize+1];for (g,&c) in cls.iter().enumerate(){if let Some(x)=reps.get_mut(c as usize){if x.is_none(){*x=Some(g as u16);}}}reps}else{Vec::new()};
            for g in 0..n{
                let g=g as u16;
                let expected=coverage.binary_search(&g).ok().and_then(|i|raw.get(if f==1{i}else{cls[g as usize] as usize})).and_then(|x|x.as_deref());
                let actual=if let Some(p)=&glyph{p.rules(g)}else{class.as_ref().unwrap().rules(g)};
                assert_eq!(actual,expected);
                let matcher=|input:&[u16]|if let Some(p)=&glyph{p.match_prefix(input)}else{class.as_ref().unwrap().match_prefix(input)};
                if let Some(rs)=expected{
                    for rule in rs{
                        let mut input=vec![g];
                        let tail:Option<Vec<u16>>=rule.remaining.iter().map(|&x|if f==1{Some(x)}else{representatives.get(x as usize).copied().flatten()}).collect();
                        let Some(tail)=tail else{continue;};input.extend(tail);
                        for k in 1..=input.len(){
                            let p=&input[..k];
                            let want=rs.iter().find(|r|p.len()>r.remaining.len()&&r.remaining.iter().zip(&p[1..]).all(|(&a,&z)|a==if f==1{z}else{cls[z as usize]}));
                            compare(matcher(p),want);
                            if let Some(r)=want{result[1]+=1;result[2]+=r.actions.len();}
                        }
                    }
                }else{assert!(matcher(&[g]).is_none());}
            }
        }
        3=>{
            let len=u(b,2);let cv:Vec<_>=(0..len).map(|i|cov(b,u(b,6+2*i))).collect();
            let refs=actions(b,6+2*len,u(b,4),len,lookups);
            let p=parse_coverage_context_positioning(b,n as u32,lookups,Default::default()).unwrap();
            assert_eq!(p.input_len(),len);assert_eq!(p.actions(),refs.as_slice());
            for (i,c) in cv.iter().enumerate(){for g in 0..n as u16{assert_eq!(p.coverages()[i].index(g).is_some(),c.binary_search(&g).is_ok());}}
            if let Some(mut input)=cv.iter().map(|c|c.first().copied()).collect::<Option<Vec<_>>>(){
                for (i,c) in cv.iter().enumerate(){let old=input[i];for g in 0..n as u16{input[i]=g;let got=p.match_prefix(&input);assert_eq!(got.is_some(),c.binary_search(&g).is_ok());if let Some(m)=got{assert_eq!(m.input_len,len);assert_eq!(m.actions,refs.as_slice());result[1]+=1;result[2]+=refs.len();}}input[i]=old;}
                assert!(p.match_prefix(&input[..len-1]).is_none());
            }
        }
        _=>panic!("unexpected context format"),
    }result
}
#[test]fn native_gpos_context_reference(){
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
            if kind!=7{continue;}
            let f=u(b,at);assert!((1..=3).contains(&f));let q=check(&b[at..],n,lookups);
            for k in 0..3{counts[f-1][k]+=q[k];}
        }}
    }
    eprintln!("retained full-font native GPOS context evidence: {counts:?}");
    for (format,hex,glyphs,lookups) in [
        (1,include_str!("fixtures/noto_gpos_context/gurmukhi.hex"),306,11),
        (2,include_str!("fixtures/noto_gpos_context/gujarati.hex"),798,11),
    ]{
        let hex=hex.trim();
        let b:Vec<u8>=(0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect();
        assert_eq!(u(&b,0),format);
        let q=check(&b,glyphs,lookups);
        assert!(q.iter().all(|&n|n>0),"native excerpt format {format}: {q:?}");
        for k in 0..3{counts[format-1][k]+=q[k];}
    }
    assert!(counts[..2].iter().all(|q|q.iter().all(|&n|n>0)));
    eprintln!("native GPOS context [subtables, matches, actions] by format: {counts:?}");
    assert!(counts.iter().any(|x|x.iter().all(|&n|n>0)),"no effective native Type 7 evidence: {counts:?}");
}
#[test]fn independent_all_three_synthetic_formats(){
    for w in [
        &[1,10,2,18,0,1,2,3,7,2,6,18,3,1,4,5,1,2,1,0][..],
        &[2,14,20,3,0,34,0,1,1,3,1,3,4,1,2,2,0,1,4,3,1,2,0,1,2][..],
        &[3,2,2,18,26,1,2,0,1,1,2,3,4,2,1,7,9,0][..],
    ]{
        let b:Vec<u8>=w.iter().flat_map(|v|(*v as u16).to_be_bytes()).collect();
        assert!(check(&b,20,4).iter().all(|&n|n>0));
    }
}
