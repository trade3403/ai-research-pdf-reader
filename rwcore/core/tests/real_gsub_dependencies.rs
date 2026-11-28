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


// Independent Boolean matrix oracle for positive-length paths. This uses
// Warshall closure, rather than the production iterative DFS. Real-font
// action coordinates come from the raw-byte inventory above.
struct MatrixOracle{
    direct:Vec<Vec<bool>>,
    reach:Vec<Vec<bool>>,
    covered:Vec<bool>,
}
impl MatrixOracle{
    fn new(n:usize,sources:&[u16],raw:&[GsubDependency])->Self{
        let mut covered=vec![false;n];
        for &i in sources{covered[i as usize]=true;}
        let mut direct=vec![vec![false;n];n];
        for e in raw{
            let i=e.site.lookup_index as usize;
            if covered[i]{direct[i][e.action.lookup_index as usize]=true;}
        }
        let mut reach=direct.clone();
        for k in 0..n{
            let row=reach[k].clone();
            for i in 0..n{
                if reach[i][k]{
                    for j in 0..n{reach[i][j]|=row[j];}
                }
            }
        }
        Self{direct,reach,covered}
    }
    fn compare(&self,actual:&GsubDependencyClosure,roots:&[u16]){
        let n=self.covered.len();
        let reached:Vec<u16>=(0..n).filter(|&i|{
            roots.contains(&(i as u16))||roots.iter().any(|&r|self.reach[r as usize][i])
        }).map(|i|i as u16).collect();
        let missing:Vec<u16>=reached.iter().copied().filter(|&i|!self.covered[i as usize]).collect();
        let edges:usize=reached.iter().map(|&i|self.direct[i as usize].iter().filter(|&&v|v).count()).sum();
        let cyclic=reached.iter().any(|&i|self.reach[i as usize][i as usize]);
        assert_eq!(actual.reachable,reached);
        assert_eq!(actual.missing_sources,missing);
        assert_eq!(actual.reachable_edges,edges);
        assert_eq!(actual.is_complete(),missing.is_empty());
        assert_eq!(actual.is_proven_acyclic(),missing.is_empty()&&!cyclic);
        assert_eq!(actual.cycle_witness.is_some(),cyclic);
        if let Some(w)=&actual.cycle_witness{
            assert!(w.len()>=2);assert_eq!(w.first(),w.last());
            assert!(w.iter().all(|i|reached.contains(i)));
            assert!(w.windows(2).all(|p|self.direct[p[0] as usize][p[1] as usize]));
        }
    }
}
fn verify_closure(font:&[u8],glyphs:u32,counts:[usize;5]){
    let b=table(font,b"GSUB");
    let gd=table(font,b"GDEF");
    let (n,raw)=raw_inventory(b);
    let ids:Vec<u16>=(0..n as u16).collect();
    let g=parse_gdef(gd,glyphs,Default::default()).unwrap();
    let compiled=compile_gsub_lookups(b,&ids,glyphs,Some(&g),Default::default()).unwrap();
    let edges=discover_gsub_dependencies(&compiled,n,Default::default()).unwrap();
    assert_eq!(edges,raw.edges);
    let oracle=MatrixOracle::new(n,&ids,&raw.edges);
    let mut total_reached=0usize;
    let mut max_reached=0usize;
    let mut transitive_only=0usize;
    let mut cyclic_vertices=0usize;
    for i in 0..n{
        let roots=[i as u16];
        let a=analyze_gsub_dependency_closure(n,&ids,&edges,&roots,Default::default()).unwrap();
        oracle.compare(&a,&roots);
        total_reached+=a.reachable.len();
        max_reached=max_reached.max(a.reachable.len());
        cyclic_vertices+=usize::from(oracle.reach[i][i]);
        transitive_only+=(0..n).filter(|&j|oracle.reach[i][j]&&!oracle.direct[i][j]).count();
    }
    let distinct:usize=oracle.direct.iter().map(|r|r.iter().filter(|&&v|v).count()).sum();
    assert_eq!([distinct,total_reached,max_reached,cyclic_vertices,transitive_only],counts);
    let a=analyze_gsub_dependency_closure(n,&ids,&edges,&ids,Default::default()).unwrap();
    oracle.compare(&a,&ids);
    assert!(a.is_proven_acyclic());
    assert_eq!(a.reachable_edges,distinct);
    // Exercise roots from every real language system's base plan.
    // Planning itself was separately reference-tested in C4.4c2.
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    for script in &layout.scripts{
        let languages=script.default_language.iter().map(|l|(None,l))
            .chain(script.languages.iter().map(|(t,l)|(Some(*t),l)));
        for (language,lang) in languages{
            let enabled:Vec<_>=lang.features.iter().map(|&i|layout.features[i as usize].tag).collect();
            let plan=build_gsub_base_plan(&layout,script.tag,language,&enabled,Default::default()).unwrap().unwrap();
            let roots:Vec<_>=plan.lookups.iter().map(|r|r.index).collect();
            let a=analyze_gsub_dependency_closure(n,&ids,&edges,&roots,Default::default()).unwrap();
            oracle.compare(&a,&roots);
        }
    }
    // Partial source coverage: use actual compiled subset, preserving
    // edges to missing sources, and independently mask raw outgoing rows.
    let subset:Vec<_>=compiled.into_iter().filter(|l|l.index()%3==0).collect();
    let sources:Vec<_>=subset.iter().map(|l|l.index()).collect();
    let partial=discover_gsub_dependencies(&subset,n,Default::default()).unwrap();
    let partial_oracle=MatrixOracle::new(n,&sources,&raw.edges);
    for &i in &ids{
        let a=analyze_gsub_dependency_closure(n,&sources,&partial,&[i],Default::default()).unwrap();
        partial_oracle.compare(&a,&[i]);
    }
    let a=analyze_gsub_dependency_closure(n,&sources,&partial,&ids,Default::default()).unwrap();
    partial_oracle.compare(&a,&ids);
    assert!(!a.is_proven_acyclic());
    assert_eq!(a.missing_sources.len(),n-sources.len());
}
#[test]fn abeezee_regular_closure_reference(){
    verify_closure(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,[7,22,3,0,0]);
}
#[test]fn abeezee_italic_closure_reference(){
    verify_closure(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,[7,22,3,0,0]);
}
#[test]fn alegreya_closure_reference(){
    verify_closure(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,[14,71,4,0,0]);
}
#[test]fn aref_closure_reference(){
    verify_closure(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,[33,119,3,0,0]);
}
#[test]fn padauk_closure_reference(){
    verify_closure(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,[64,181,14,0,0]);
}
#[test]fn devanagari_closure_reference(){
    verify_closure(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,[126,303,38,0,0]);
}
#[test]fn nastaliq_closure_reference(){
    verify_closure(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,[290,729,44,0,237]);
}


// C4.4c5b: integrate the independent raw action inventory and matrix
// reachability oracle with compiled programs. Typed target equality uses
// the existing standalone compiler, not an independent shaper.
fn check_program(
    program:&CompiledGsubProgram<'_>, roots:&[u16], oracle:&MatrixOracle,
    raw:&RawInventory, reference:&[CompiledGsubLookup<'_>],
){
    assert_eq!(program.roots(),roots);
    oracle.compare(program.closure(),roots);
    assert!(program.closure().is_complete());
    let reached=&program.closure().reachable;
    assert_eq!(program.lookups().iter().map(|l|l.index()).collect::<Vec<_>>(),*reached);
    let expected:Vec<_>=raw.edges.iter().filter(|e|reached.binary_search(&e.site.lookup_index).is_ok()).copied().collect();
    assert_eq!(program.dependencies(),expected.as_slice());
    for (i,want) in reference.iter().enumerate(){
        let got=program.lookup(i as u16);
        if reached.binary_search(&(i as u16)).is_err(){assert!(got.is_none());continue;}
        let got=got.unwrap();
        assert_eq!(got.index(),want.index());
        assert_eq!(got.declared_kind(),want.declared_kind());
        assert_eq!(got.effective_kind(),want.effective_kind());
        assert_eq!(got.filter().flags(),want.filter().flags());
        assert_eq!(got.filter().mark_filtering_set(),want.filter().mark_filtering_set());
        assert_eq!(got.subtables(),want.subtables());
    }
    assert!(program.lookup(reference.len() as u16).is_none());
}
fn verify_program(font:&[u8],glyphs:u32,expect_transitive:bool){
    let b=table(font,b"GSUB");
    assert_eq!(u(table(font,b"maxp"),4),glyphs as usize);
    let g=parse_gdef(table(font,b"GDEF"),glyphs,Default::default()).unwrap();
    let (n,raw)=raw_inventory(b);
    let ids:Vec<_>=(0..n as u16).collect();
    let oracle=MatrixOracle::new(n,&ids,&raw.edges);
    let reference=compile_gsub_lookups(b,&ids,glyphs,Some(&g),Default::default()).unwrap();
    // Independently calculate compile budgets from raw LookupList offsets,
    // including each native extension wrapper and each aliased reference.
    let list=u(b,8);
    let (mut subtables,mut bytes,mut max_subtables)=(0usize,0usize,0usize);
    for i in 0..n{
        let p=list+u(b,list+2+2*i);
        let kind=u(b,p);
        let count=u(b,p+4);
        subtables+=count;max_subtables=max_subtables.max(count);
        for j in 0..count{
            let source=p+u(b,p+6+2*j);
            let target=if kind==7{bytes+=8;source+d(b,source+4)}else{source};
            bytes+=b.len()-target;
        }
    }
    let limits=GsubProgramLimits{
        compile:GsubCompileLimits{max_lookups:n,max_total_subtables:subtables,
            max_subtables_per_lookup:max_subtables,max_total_target_bytes:bytes,..Default::default()},
        dependencies:GsubDependencyLimits{max_scan_records:raw.scan,max_actions:raw.edges.len()},
        ..Default::default()
    };
    let all=compile_gsub_program(b,&ids,glyphs,Some(&g),limits).unwrap();
    check_program(&all,&ids,&oracle,&raw,&reference);
    assert!(all.closure().is_proven_acyclic());
    drop(all);
    // Exercise the widest single-root closure and an independent root
    // set. In the Nastaliq fixture require a genuinely indirect edge.
    let root=(0..n).max_by_key(|&i|oracle.reach[i].iter().filter(|&&v|v).count()).unwrap();
    let single=[root as u16];
    let p=compile_gsub_program(b,&single,glyphs,Some(&g),Default::default()).unwrap();
    check_program(&p,&single,&oracle,&raw,&reference);
    assert!(p.lookups().len()>1);
    drop(p);
    let roots:Vec<_>=ids.iter().copied().filter(|i|i%3==0).collect();
    let p=compile_gsub_program(b,&roots,glyphs,Some(&g),Default::default()).unwrap();
    check_program(&p,&roots,&oracle,&raw,&reference);
    drop(p);
    let indirect=(0..n).find(|&i|(0..n).any(|j|oracle.reach[i][j]&&!oracle.direct[i][j]));
    assert_eq!(indirect.is_some(),expect_transitive);
    if let Some(i)=indirect{
        let roots=[i as u16];
        let p=compile_gsub_program(b,&roots,glyphs,Some(&g),Default::default()).unwrap();
        check_program(&p,&roots,&oracle,&raw,&reference);
        assert!(p.closure().reachable.iter().any(|&j|j!=roots[0]&&!oracle.direct[i][j as usize]));
    }
    // One real default and one explicit language plan, when present.
    let layout=parse_gsub_layout(b,Default::default()).unwrap();
    for selected in [
        layout.scripts.iter().find_map(|s|s.default_language.as_ref().map(|l|(s.tag,None,l))),
        layout.scripts.iter().find_map(|s|s.languages.first().map(|(t,l)|(s.tag,Some(*t),l))),
    ].into_iter().flatten(){
        let (script,language,lang)=selected;
        let tags:Vec<_>=lang.features.iter().map(|&i|layout.features[i as usize].tag).collect();
        let plan=build_gsub_base_plan(&layout,script,language,&tags,Default::default()).unwrap().unwrap();
        let roots:Vec<_>=plan.lookups.iter().map(|r|r.index).collect();
        let p=compile_gsub_plan_program(b,&plan,glyphs,Some(&g),Default::default()).unwrap();
        check_program(&p,&roots,&oracle,&raw,&reference);
    }
    // Exact raw budgets succeed above; one-less aggregate budgets must
    // fail. Exercise the full set on a small fixture to bound CI work.
    if n==15{
        for l in [
            GsubProgramLimits{compile:GsubCompileLimits{max_lookups:n-1,..limits.compile},..limits},
            GsubProgramLimits{compile:GsubCompileLimits{max_total_subtables:subtables-1,..limits.compile},..limits},
            GsubProgramLimits{compile:GsubCompileLimits{max_total_target_bytes:bytes-1,..limits.compile},..limits},
            GsubProgramLimits{dependencies:GsubDependencyLimits{max_scan_records:raw.scan-1,..limits.dependencies},..limits},
            GsubProgramLimits{dependencies:GsubDependencyLimits{max_actions:raw.edges.len()-1,..limits.dependencies},..limits},
        ]{
            assert!(matches!(compile_gsub_program(b,&ids,glyphs,Some(&g),l),Err(Error::Limit(_))));
        }
    }
}
#[test]fn abeezee_regular_program_reference(){
    verify_program(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,false);
}
#[test]fn abeezee_italic_program_reference(){
    verify_program(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,false);
}
#[test]fn alegreya_program_reference(){
    verify_program(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,false);
}
#[test]fn aref_program_reference(){
    verify_program(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,false);
}
#[test]fn padauk_program_reference(){
    verify_program(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,false);
}
#[test]fn devanagari_program_reference(){
    verify_program(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,false);
}
#[test]fn nastaliq_program_reference(){
    verify_program(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,true);
}
