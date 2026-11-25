//! Independent raw GSUB contextual action inventory, all six formats.
//! Static references only; no claim of runtime reachability or execution.
use rwcore::*;
use std::collections::{BTreeMap,BTreeSet};

fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->&'a [u8]{
    for i in 0..u(b,4){
        let p=12+16*i;
        if &b[p..p+4]==tag{
            let at=d(b,p+8);return &b[at..at+d(b,p+12)];
        }
    }
    panic!("missing table")
}
#[derive(Default)]
struct RawInventory{
    edges:Vec<GsubDependency>,
    scan:usize,
    contextual:usize,
    wrapped:usize,
    slots:usize,
    rules:usize,
    empty:usize,
    histogram:BTreeMap<(u16,u16),usize>,
}
impl RawInventory{
    fn read_actions(&mut self,b:&[u8],site:GsubDependencySite,p:usize,count:usize,input:usize,n:usize){
        self.rules+=1;self.scan+=1;
        if count==0{self.empty+=1;}
        for i in 0..count{
            let seq=u(b,p+4*i);
            let target=u(b,p+4*i+2);
            assert!(seq<input);assert!(target<n);
            self.edges.push(GsubDependency{site,action_index:i,
                action:ContextAction{sequence_index:seq as u16,lookup_index:target as u16}});
            *self.histogram.entry((site.effective_kind,site.format)).or_insert(0)+=1;
        }
    }
    fn read_rule(&mut self,b:&[u8],site:GsubDependencySite,mut p:usize,n:usize){
        if site.effective_kind==5{
            let input=u(b,p);let count=u(b,p+2);
            assert!(input>0);
            let stored=if site.format==3{input}else{input-1};
            self.read_actions(b,site,p+4+2*stored,count,input,n);
        }else{
            let back=u(b,p);p+=2+2*back;
            let input=u(b,p);assert!(input>0);
            let stored=if site.format==3{input}else{input-1};
            p+=2+2*stored;
            let ahead=u(b,p);p+=2+2*ahead;
            self.read_actions(b,site,p+2,u(b,p),input,n);
        }
    }
}
fn raw_inventory(b:&[u8])->(usize,RawInventory){
    let list=u(b,8);
    let n=u(b,list);
    let mut raw=RawInventory{scan:n,..Default::default()};
    for i in 0..n{
        let p=list+u(b,list+2+2*i);
        let declared=u(b,p) as u16;
        let count=u(b,p+4);
        raw.scan+=count;
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let (kind,target)=if declared==7{
                assert_eq!(u(b,source),1);
                (u(b,source+2) as u16,source+d(b,source+4))
            }else{(declared,source)};
            if !matches!(kind,5|6){continue;}
            raw.contextual+=1;
            if declared==7{raw.wrapped+=1;}
            let format=u(b,target) as u16;
            assert!((1..=3).contains(&format));
            let site=GsubDependencySite{lookup_index:i as u16,subtable_index:j,
                source_offset:source,target_offset:target,effective_kind:kind,format,
                set_index:None,rule_index:0};
            if format==3{
                raw.read_rule(b,site,target+2,n);
                continue;
            }
            let base=target+match (kind,format){(_,1)=>4,(5,2)=>6,(6,2)=>10,_=>unreachable!()};
            let sets=u(b,base);
            raw.slots+=sets;raw.scan+=sets;
            for s in 0..sets{
                let rel=u(b,base+2+2*s);
                if rel==0{continue;}
                let set=target+rel;
                for r in 0..u(b,set){
                    let rule=set+u(b,set+2+2*r);
                    raw.read_rule(b,GsubDependencySite{set_index:Some(s),rule_index:r,..site},rule,n);
                }
            }
        }
    }
    (n,raw)
}
fn verify(font:&[u8],glyphs:u32,counts:[usize;9],formats:&[((u16,u16),usize)]){
    let b=table(font,b"GSUB");
    let gd=table(font,b"GDEF");
    assert_eq!(u(table(font,b"maxp"),4),glyphs as usize);
    let (n,raw)=raw_inventory(b);
    let pairs:BTreeSet<_>=raw.edges.iter().map(|e|(e.site.lookup_index,e.action.lookup_index)).collect();
    let self_refs=raw.edges.iter().filter(|e|e.site.lookup_index==e.action.lookup_index).count();
    assert_eq!([n,raw.contextual,raw.wrapped,raw.slots,raw.rules,raw.empty,raw.edges.len(),pairs.len(),raw.scan],counts);
    assert_eq!(self_refs,0);
    assert_eq!(raw.histogram.iter().map(|(&k,&v)|(k,v)).collect::<Vec<_>>(),formats);
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let ids:Vec<u16>=(0..n as u16).collect();
    let all=compile_gsub_lookups(b,&ids,glyphs,Some(&g),Default::default()).unwrap();
    let actual=discover_gsub_dependencies(&all,n,Default::default()).unwrap();
    assert_eq!(actual.len(),raw.edges.len());
    for (i,(a,e)) in actual.iter().zip(&raw.edges).enumerate(){
        assert_eq!(a,e,"action {i}");
    }
    let l=GsubDependencyLimits{max_scan_records:raw.scan,max_actions:raw.edges.len()};
    assert_eq!(discover_gsub_dependencies(&all,n,l).unwrap(),raw.edges);
    assert!(matches!(discover_gsub_dependencies(&all,n,GsubDependencyLimits{max_scan_records:raw.scan-1,..l}),Err(Error::Limit(_))));
    assert!(matches!(discover_gsub_dependencies(&all,n,GsubDependencyLimits{max_actions:raw.edges.len()-1,..l}),Err(Error::Limit(_))));
    // A subset preserves original LookupList indices, including references
    // to unselected lookups. No transitive closure should be implied.
    let subset:Vec<_>=all.into_iter().filter(|x|x.index()%3==0).collect();
    let want:Vec<_>=raw.edges.iter().filter(|e|e.site.lookup_index%3==0).copied().collect();
    assert_eq!(discover_gsub_dependencies(&subset,n,Default::default()).unwrap(),want);
}
#[test]fn abeezee_italic_dependencies(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,
        [15,9,0,2,10,0,10,7,47],&[((6,1),2),((6,3),8)]);
}
#[test]fn abeezee_regular_dependencies(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,
        [15,9,0,2,10,0,10,7,47],&[((6,1),2),((6,3),8)]);
}
#[test]fn alegreya_dependencies(){
    verify(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,
        [57,35,0,0,35,0,35,14,174],&[((6,3),35)]);
}
#[test]fn aref_dependencies(){
    verify(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,
        [86,52,0,30,81,2,79,33,305],&[((6,1),9),((6,2),24),((6,3),46)]);
}
#[test]fn padauk_dependencies(){
    verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,
        [117,109,0,29,198,40,347,64,541],
        &[((5,2),259),((5,3),6),((6,1),4),((6,2),7),((6,3),71)]);
}
#[test]fn devanagari_dependencies(){
    verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,
        [177,303,0,265,2118,67,3581,126,3005],
        &[((5,1),134),((5,2),2730),((5,3),158),((6,1),21),((6,2),378),((6,3),160)]);
}
#[test]fn nastaliq_dependencies(){
    verify(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,
        [202,2364,1406,305,11087,312,17562,290,14104],
        &[((5,1),425),((5,2),9536),((5,3),1775),((6,2),4914),((6,3),912)]);
}
