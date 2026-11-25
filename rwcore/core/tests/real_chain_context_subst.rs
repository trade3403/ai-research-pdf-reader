//! Independent reference decoder for direct Type 6 subtables in original OFL fonts.
//! Test-only dense class arrays and expanded coverage lists, not the core decoders.
use rwcore::*;

fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->&'a [u8]{
    for i in 0..u(b,4){
        let p=12+16*i;
        if &b[p..p+4]==tag{let start=d(b,p+8);return &b[start..start+d(b,p+12)];}
    }
    panic!("missing table")
}
fn words(b:&[u8],p:&mut usize,n:usize)->Vec<u16>{
    (0..n).map(|_|{let v=u(b,*p) as u16;*p+=2;v}).collect()
}
fn counted(b:&[u8],p:&mut usize)->Vec<u16>{let n=u(b,*p);*p+=2;words(b,p,n)}
fn coverage(b:&[u8],p:usize)->Vec<u16>{
    match u(b,p){
        1=>words(b,&mut (p+4),u(b,p+2)),
        2=>(0..u(b,p+2)).flat_map(|i|{let q=p+4+6*i;u(b,q) as u16..=u(b,q+2) as u16}).collect(),
        f=>panic!("coverage format {f}"),
    }
}
fn classes(b:&[u8],p:usize,n:usize)->Vec<u16>{
    let mut r=vec![0;n];
    match u(b,p){
        1=>{let start=u(b,p+2);for i in 0..u(b,p+4){r[start+i]=u(b,p+6+2*i) as u16;}}
        2=>{for i in 0..u(b,p+2){let q=p+4+6*i;for g in u(b,q)..=u(b,q+2){r[g]=u(b,q+4) as u16;}}}
        f=>panic!("class format {f}"),
    }
    r
}
#[derive(Debug)]
struct Rule{back:Vec<u16>,tail:Vec<u16>,ahead:Vec<u16>,actions:Vec<(u16,u16)>}
fn actions(b:&[u8],p:&mut usize)->Vec<(u16,u16)>{
    let n=u(b,*p);*p+=2;
    (0..n).map(|_|{let r=(u(b,*p) as u16,u(b,*p+2) as u16);*p+=4;r}).collect()
}
fn rules(b:&[u8],at:usize)->Option<Vec<Rule>>{
    if at==0{return None;}
    Some((0..u(b,at)).map(|i|{
        let mut p=at+u(b,at+2+2*i);
        let back=counted(b,&mut p);
        let input=u(b,p);p+=2;
        let tail=words(b,&mut p,input-1);
        let ahead=counted(b,&mut p);
        let actions=actions(b,&mut p);
        Rule{back,tail,ahead,actions}
    }).collect())
}
fn action_pairs(a:&[ContextAction])->Vec<(u16,u16)>{a.iter().map(|x|(x.sequence_index,x.lookup_index)).collect()}
fn compare_match(actual:Option<ContextMatch<'_>>,expected:Option<(usize,&[(u16,u16)])>){
    assert_eq!(actual.map(|m|(m.input_len,action_pairs(m.actions))),expected.map(|(n,a)|(n,a.to_vec())));
}
fn check_rules(actual:Option<&[ChainContextRule]>,expected:Option<&[Rule]>){
    match (actual,expected){
        (None,None)=>{},
        (Some(a),Some(e))=>{
            assert_eq!(a.len(),e.len());
            for (x,y) in a.iter().zip(e){
                assert_eq!(x.backtrack,y.back);
                assert_eq!(x.remaining,y.tail);
                assert_eq!(x.lookahead,y.ahead);
                assert_eq!(action_pairs(&x.actions),y.actions);
            }
        }
        _=>panic!("rule-set mismatch"),
    }
}
fn matches(r:&Rule,g:&[u16],p:usize,maps:&[Vec<u16>;3])->bool{
    if p>=g.len()||p<r.back.len()||g.len()-p-1<r.tail.len()+r.ahead.len(){return false;}
    let symbol=|k:usize,id:u16|maps[k].get(id as usize).copied();
    r.back.iter().enumerate().all(|(i,&x)|symbol(0,g[p-1-i])==Some(x))&&
    r.tail.iter().enumerate().all(|(i,&x)|symbol(1,g[p+1+i])==Some(x))&&
    r.ahead.iter().enumerate().all(|(i,&x)|symbol(2,g[p+1+r.tail.len()+i])==Some(x))
}
fn representative(seq:&[u16],map:&[u16])->Option<Vec<u16>>{
    seq.iter().map(|c|map.iter().position(|x|x==c).map(|g|g as u16)).collect()
}
fn check_sets(b:&[u8],n:usize,k:usize,format:usize)->usize{
    let cov=coverage(b,u(b,2));
    let (base,count,maps)=if format==1{
        (6,u(b,4),std::array::from_fn(|_|(0..n).map(|x|x as u16).collect()))
    }else{
        (12,u(b,10),[classes(b,u(b,4),n),classes(b,u(b,6),n),classes(b,u(b,8),n)])
    };
    let sets:Vec<_>=(0..count).map(|i|rules(b,u(b,base+2*i))).collect();
    let a=if format==1{Some(parse_chain_context_substitution(b,n as u32,k,ContextLimits::default()).unwrap())}else{None};
    let c=if format==2{Some(parse_class_chain_context_substitution(b,n as u32,k,ContextLimits::default()).unwrap())}else{None};
    let mut cases=0;
    for id in 0..n as u16{
        let idx=cov.iter().position(|&g|g==id).map(|i|if format==1{i}else{maps[1][id as usize] as usize});
        let expected=idx.and_then(|i|sets.get(i)).and_then(|x|x.as_deref());
        let actual=if let Some(x)=&a{x.rules(id)}else{c.as_ref().unwrap().rules(id)};
        check_rules(actual,expected);
        let check=|g:&[u16],p:usize|{
            let e=expected.and_then(|rs|rs.iter().find(|r|matches(r,g,p,&maps))).map(|r|(r.tail.len()+1,r.actions.as_slice()));
            let m=if let Some(x)=&a{x.match_at(g,p)}else{c.as_ref().unwrap().match_at(g,p)};
            compare_match(m,e);
        };
        check(&[id],0);
        if let Some(rs)=expected{
            for r in rs{
                let (Some(mut back),Some(tail),Some(ahead))=(representative(&r.back,&maps[0]),representative(&r.tail,&maps[1]),representative(&r.ahead,&maps[2]))else{continue;};
                back.reverse();let p=back.len();back.push(id);back.extend(tail);back.extend(ahead);
                check(&back,p);cases+=1;
                if p>0{check(&back[1..],p-1);}
                if back.len()>p+1{check(&back[..back.len()-1],p);}
                // Every context position is invalidated once, preserving the first input.
                for i in 0..back.len(){
                    if i==p{continue;}
                    let old=back[i];back[i]=n as u16;check(&back,p);back[i]=old;
                }
            }
        }
    }
    cases
}
fn check_coverage(b:&[u8],n:usize,k:usize)->usize{
    let mut p=2;
    let back:Vec<_>=counted(b,&mut p).iter().map(|&o|coverage(b,o as usize)).collect();
    let input:Vec<_>=counted(b,&mut p).iter().map(|&o|coverage(b,o as usize)).collect();
    let ahead:Vec<_>=counted(b,&mut p).iter().map(|&o|coverage(b,o as usize)).collect();
    let refs=actions(b,&mut p);
    let a=parse_coverage_chain_context_substitution(b,n as u32,k,ContextLimits::default()).unwrap();
    assert_eq!(a.input_len(),input.len());assert_eq!(action_pairs(a.actions()),refs);
    for (ac,ex) in [(a.backtrack(),&back),(a.input(),&input),(a.lookahead(),&ahead)]{
        assert_eq!(ac.len(),ex.len());
        for (c,e) in ac.iter().zip(ex){
            assert_eq!(c.len() as usize,e.len());
            for g in 0..n as u16{assert_eq!(c.index(g).map(|x|x as usize),e.iter().position(|&x|x==g));}
        }
    }
    let all:Vec<_>=back.iter().rev().chain(input.iter()).chain(ahead.iter()).collect();
    if all.iter().any(|x|x.is_empty()){assert!(a.match_at(&vec![0;all.len()],back.len()).is_none());return 0;}
    let mut g:Vec<u16>=all.iter().map(|x|x[0]).collect();
    let pos=back.len();
    compare_match(a.match_at(&g,pos),Some((input.len(),&refs)));
    for (i,c) in all.iter().enumerate(){
        let old=g[i];
        for id in 0..n as u16{
            g[i]=id;
            compare_match(a.match_at(&g,pos),if c.contains(&id){Some((input.len(),&refs))}else{None});
        }
        g[i]=old;
    }
    if pos>0{assert!(a.match_at(&g[1..],pos-1).is_none());}
    assert!(a.match_at(&g[..g.len()-1],pos).is_none());
    1
}
fn verify(font:&[u8],n:usize,k:usize,expected:[usize;3]){
    let b=table(font,b"GSUB");
    assert_eq!(u(table(font,b"maxp"),4),n);
    let list=u(b,8);assert_eq!(u(b,list),k);
    let parsed=parse_font(font,FontLimits::default()).unwrap();
    assert_eq!(parsed.face(0).unwrap().table(*b"GSUB").unwrap(),b);
    let layout=parse_gsub_layout(b,GsubLimits::default()).unwrap();
    let mut counts=[0;3];let mut cases=[0;3];
    for i in 0..k{
        let at=list+u(b,list+2+2*i);
        if u(b,at)!=6{continue;}
        assert_eq!(layout.lookups[i].kind,6);
        let offsets:Vec<_>=(0..u(b,at+4)).map(|j|at+u(b,at+6+2*j)).collect();
        assert_eq!(layout.lookups[i].subtables,offsets);
        for o in offsets{
            let f=u(b,o);
            assert!((1..=3).contains(&f));
            counts[f-1]+=1;
            cases[f-1]+=if f==3{check_coverage(&b[o..],n,k)}else{check_sets(&b[o..],n,k,f)};
        }
    }
    assert_eq!(counts,expected);
    assert!(cases.iter().all(|&x|x>0),"no reachable match in format: {cases:?}");
}
#[test]fn real_padauk_chained_context(){
    verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,117,[3,3,97]);
}
#[test]fn real_noto_chained_context(){
    verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,177,[5,1,212]);
}
