//! Real-font differential for selected native cursive/mark-base lookups.
//! Independent raw SFNT, coverage, class and anchor decoding; no production
//! positioner is used as the oracle. Explicit glyph IDs, default coordinates,
//! selected lookup roots, NOT a HarfBuzz or full-script shaping comparison.
use rwcore::*;
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->Option<&'a [u8]>{
    for i in 0..u(b,4){let p=12+16*i;if &b[p..p+4]==tag{
        let at=d(b,p+8);return Some(&b[at..at+d(b,p+12)]);
    }}None
}
fn coverage(b:&[u8],p:usize)->Vec<u16>{
    match u(b,p){
        1=>(0..u(b,p+2)).map(|i|u(b,p+4+2*i) as u16).collect(),
        2=>{let mut v=Vec::new();for i in 0..u(b,p+2){
            let q=p+4+6*i;assert_eq!(v.len(),u(b,q+4));
            v.extend((u(b,q)..=u(b,q+2)).map(|g|g as u16));
        }v},
        _=>panic!("coverage format"),
    }
}
fn classes(g:Option<&[u8]>,n:usize)->Vec<usize>{
    let mut c=vec![0;n];let Some(g)=g else{return c;};let p=u(g,4);
    if p==0{return c;}
    match u(g,p){
        1=>{let start=u(g,p+2);for i in 0..u(g,p+4){c[start+i]=u(g,p+6+2*i);}},
        2=>{for i in 0..u(g,p+2){let q=p+4+6*i;for k in u(g,q)..=u(g,q+2){c[k]=u(g,q+4);}}},
        _=>panic!("class format"),
    }c
}
fn eligible(g:u16,flags:usize,c:&[usize])->bool{
    !matches!((c[g as usize],flags), (1,f) if f&2!=0)
    && !matches!((c[g as usize],flags),(2,f) if f&4!=0)
    && !matches!((c[g as usize],flags),(3,f) if f&8!=0)
}
type Point=(i32,i32);
fn anchor(b:&[u8],p:usize)->Option<Point>{
    if p==0{return None;}
    assert!((1..=3).contains(&u(b,p)));
    Some((i32::from(u(b,p+2) as i16),i32::from(u(b,p+4) as i16)))
}
fn diff(a:Point,b:Point)->Point{(a.0-b.0,a.1-b.1)}
struct Lookup<'a>{id:u16,kind:usize,flags:usize,subs:Vec<&'a [u8]>}
fn lookups(b:&[u8])->Vec<Lookup<'_>>{
    let list=u(b,8);let mut out=Vec::new();
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);let flags=u(b,p+2);
        // Mark filtering sets/attachment classes need a separate oracle.
        if flags& !15!=0{continue;}
        let mut subs=Vec::new();let mut kind=u(b,p);
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);
            if u(b,p)==9{kind=u(b,at+2);at+=d(b,at+4);}
            subs.push(&b[at..]);
        }
        if matches!(kind,3|4){out.push(Lookup{id:i as u16,kind,flags,subs});}
    }out
}
fn cursive(s:&[u8],a:u16,b:u16)->Option<Point>{
    let cov=coverage(s,u(s,2));
    let i=cov.binary_search(&a).ok()?;let j=cov.binary_search(&b).ok()?;
    Some(diff(anchor(s,u(s,8+4*i))?,anchor(s,u(s,6+4*j))?))
}
fn mark(s:&[u8],a:u16,b:u16)->Option<Point>{
    let m=coverage(s,u(s,2)).binary_search(&b).ok()?;
    let base=coverage(s,u(s,4)).binary_search(&a).ok()?;
    let ma=u(s,8);let ba=u(s,10);let class=u(s,ma+2+4*m);
    let mo=u(s,ma+4+4*m);let bo=u(s,ba+2+2*(base*u(s,6)+class));
    if mo==0||bo==0{return None;}
    Some(diff(anchor(s,ba+bo)?,anchor(s,ma+mo)?))
}
fn pair(l:&Lookup<'_>,a:u16,b:u16)->Option<Point>{
    l.subs.iter().find_map(|s|if l.kind==3{cursive(s,a,b)}else{mark(s,a,b)})
}
fn nominal(font:&[u8],g:u16)->i32{
    let h=table(font,b"hhea").unwrap();let m=table(font,b"hmtx").unwrap();
    u(m,4*(g as usize).min(u(h,34)-1)) as i32
}
fn samples(l:&Lookup<'_>,classes:&[usize])->Vec<(u16,u16,Point)>{
    let mut out=Vec::new();
    for s in &l.subs{
        let right=coverage(s,u(s,2));
        let left=if l.kind==3{right.clone()}else{coverage(s,u(s,4))};
        // Bound candidate sampling while retaining native nonzero examples.
        for &a in left.iter().filter(|&&g|eligible(g,l.flags,classes)).take(16){
            if l.kind==4&&classes[a as usize]==3{continue;}
            for &b in right.iter().filter(|&&g|eligible(g,l.flags,classes)).take(16){
                if let Some(delta)=pair(l,a,b){
                    if !out.iter().any(|&(x,y,_)|x==a&&y==b){out.push((a,b,delta));}
                    if out.len()==24{return out;}
                }
            }
        }
    }out
}
fn check_pair(font:&[u8],p:&CompiledGposProgram<'_>,l:&Lookup<'_>,a:u16,b:u16,delta:Point,flow:GposHorizontalFlow){
    let rtl=flow==GposHorizontalFlow::RightToLeft;
    let n=[nominal(font,a),nominal(font,b)];
    let mut w=n;
    let child=if l.kind==3&&l.flags&1!=0{0}else{1};
    let parent=1-child;
    if l.kind==3{if rtl{w[1]=-delta.0;}else{w[0]=delta.0;}}
    let ox=if rtl{[-w[0],-w[0]-w[1]]}else{[0,w[0]]};
    let edge_delta=if child==0{(-delta.0,-delta.1)}else{delta};
    let mut expected=[GposRunAdjustment::default();2];
    for i in 0..2{expected[i].design.x_advance=w[i]-n[i];}
    expected[child].design.x_placement=ox[parent]-ox[child]+edge_delta.0;
    expected[child].design.y_placement=edge_delta.1;
    let adv=[(n[0],0),(n[1],0)];let prior=[GposRunAdjustment::default();2];
    let sources=[GposLigatureSource::Unmapped;2];
    let x=execute_gpos_mixed_joint(p,&[a,b],&prior,&adv,&[(0,0);2],&sources,GposRunMode::DesignOnly,flow,Default::default(),Default::default()).unwrap();
    assert_eq!(x.design_origins,vec![(ox[0],0),(ox[1],0)]);
    assert_eq!(x.adjustments,expected.to_vec(),"lookup {} glyphs {a},{b}",l.id);
    assert_eq!(x.edges[child],Some(GposAttachmentEdge{parent,design_delta:edge_delta,pixel_delta:(0,0)}));
    assert!(x.edges[parent].is_none());
    let immediate=execute_gpos_mixed_directional(p,&[a,b],&prior,&adv,&[(0,0);2],&sources,GposRunMode::DesignOnly,flow,Default::default()).unwrap();
    assert_eq!(x,immediate);
}
fn check(font:&[u8],kind:usize)->[usize;3]{
    let Some(b)=table(font,b"GPOS") else{return [0;3];};
    let n=u(table(font,b"maxp").unwrap(),4);
    let gd=table(font,b"GDEF");
    let cls=classes(gd,n);
    let gdef=gd.map(|x|parse_gdef(x,n as u32,Default::default()).unwrap());
    let mut counts=[0;3];
    for l in lookups(b).iter().filter(|l|l.kind==kind){
        let cases=samples(l,&cls);
        if cases.is_empty(){continue;}
        let p=compile_gpos_program(b,&[l.id],n as u32,gdef.as_ref(),Default::default()).unwrap();
        counts[0]+=1;
        for (a,z,delta) in cases{
            for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                check_pair(font,&p,l,a,z,delta,flow);counts[1]+=1;
                if delta!=(0,0){counts[2]+=1;}
            }
        }
    }
    counts
}
const FONTS:[(&str,&[u8]);5]=[
    ("Aref Ruqaa",include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf")),
    ("Noto Nastaliq Urdu",include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf")),
    ("Noto Sans Mongolian",include_bytes!("fixtures/notosansmongolian/NotoSansMongolian-Regular.ttf")),
    ("Padauk",include_bytes!("fixtures/padauk/Padauk-Regular.ttf")),
    ("ABeeZee",include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf")),
];
fn suite(kind:usize){
    let mut total=[0;3];
    for (name,font) in FONTS{
        let c=check(font,kind);eprintln!("native GPOS {name} kind {kind}: {c:?}");
        for i in 0..3{total[i]+=c[i];}
    }
    assert!(total.iter().all(|&x|x>0),"native lookup, case and nonzero coverage: {total:?}");
}
#[test]fn native_cursive_joint_coordinate_differential(){suite(3);}
#[test]fn native_mark_base_joint_coordinate_differential(){suite(4);}

// Independently read the native anchor offsets, including the first matching
// subtable rule. This intentionally does not reuse the production anchor API.
fn anchor_offsets(s:&[u8],kind:usize,a:u16,b:u16)->Option<(usize,usize)>{
    let (p,q)=if kind==3{
        let cov=coverage(s,u(s,2));
        let i=cov.binary_search(&a).ok()?;let j=cov.binary_search(&b).ok()?;
        (u(s,8+4*i),u(s,6+4*j))
    }else{
        let m=coverage(s,u(s,2)).binary_search(&b).ok()?;
        let base=coverage(s,u(s,4)).binary_search(&a).ok()?;
        let ma=u(s,8);let ba=u(s,10);let class=u(s,ma+2+4*m);
        let mo=u(s,ma+4+4*m);let bo=u(s,ba+2+2*(base*u(s,6)+class));
        if mo==0||bo==0{return None;}
        (ba+bo,ma+mo)
    };
    if p==0||q==0{None}else{Some((p,q))}
}
fn raw_device(s:&[u8],at:usize,ppem:u16)->Option<i32>{
    let lo=u(s,at);let hi=u(s,at+2);let f=u(s,at+4);
    if f==0x8000{return None;}
    let bits=match f{1=>2,2=>4,3=>8,_=>panic!("device format")};
    let ppem=usize::from(ppem);
    if ppem<lo||ppem>hi{return Some(0);}
    let start=(ppem-lo)*bits;
    let mut v=0i32;
    for k in start..start+bits{
        v=2*v+i32::from((s[at+6+k/8]>>(7-k%8))&1);
    }
    if v>=1<<(bits-1){v-=1<<bits;}
    Some(v)
}
fn anchor_pixels(s:&[u8],p:usize,x:u16,y:u16)->Option<Point>{
    match u(s,p){
        1=>Some((0,0)),
        2=>None,
        3=>{
            let read=|off,ppem|{let rel=u(s,off);if rel==0{Some(0)}else{raw_device(s,p+rel,ppem)}};
            Some((read(p+6,x)?,read(p+8,y)?))
        }
        _=>panic!("anchor format"),
    }
}
fn pixel_pair(l:&Lookup<'_>,a:u16,b:u16,x:u16,y:u16)->Option<Point>{
    for s in &l.subs{
        if let Some((p,q))=anchor_offsets(s,l.kind,a,b){
            return Some(diff(anchor_pixels(s,p,x,y)?,anchor_pixels(s,q,x,y)?));
        }
    }
    panic!("expected native pair")
}
fn reference_pair(n:[i32;2],delta:Point,kind:usize,flags:usize,flow:GposHorizontalFlow)->([Point;2],[GposAdjustment;2],usize,Point){
    let rtl=flow==GposHorizontalFlow::RightToLeft;
    let mut w=n;
    if kind==3{if rtl{w[1]=-delta.0;}else{w[0]=delta.0;}}
    let origins=if rtl{[(-w[0],0),(-w[0]-w[1],0)]}else{[(0,0),(w[0],0)]};
    let child=if kind==3&&flags&1!=0{0}else{1};
    let edge=if child==0{(-delta.0,-delta.1)}else{delta};
    let mut adj=[GposAdjustment::default();2];
    for i in 0..2{adj[i].x_advance=w[i]-n[i];}
    adj[child].x_placement=origins[1-child].0-origins[child].0+edge.0;
    adj[child].y_placement=edge.1;
    (origins,adj,child,edge)
}
#[test]fn native_mixed_device_coordinate_differential(){
    let mut total=[0usize;4]; // supported, unresolved, nonzero, native lookup count
    for (name,font) in FONTS{
        let Some(b)=table(font,b"GPOS")else{continue;};
        let n=u(table(font,b"maxp").unwrap(),4);
        let gd=table(font,b"GDEF");let cls=classes(gd,n);
        let gdef=gd.map(|x|parse_gdef(x,n as u32,Default::default()).unwrap());
        let units=u(table(font,b"head").unwrap(),18) as i32;
        let mut counts=[0usize;4];
        for l in lookups(b){
            let cases=samples(&l,&cls);if cases.is_empty(){continue;}
            let p=compile_gpos_program(b,&[l.id],n as u32,gdef.as_ref(),Default::default()).unwrap();
            counts[3]+=1;
            for (a,z,delta) in cases{
                let design=[nominal(font,a),nominal(font,z)];
                let adv=[(design[0],0),(design[1],0)];
                for (x,y) in [(9u16,12u16),(16,20),(32,48)]{
                    // Integer pixel widths are caller-supplied, independent of design.
                    let px=design.map(|w|(w*i32::from(x)+units/2)/units);
                    let pixels=[(px[0],0),(px[1],0)];
                    let raw=pixel_pair(&l,a,z,x,y);
                    for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                        let mode=GposRunMode::DevicePixels{x_ppem:x,y_ppem:y};
                        let got=execute_gpos_mixed_joint(&p,&[a,z],&[Default::default();2],&adv,&pixels,
                            &[GposLigatureSource::Unmapped;2],mode,flow,Default::default(),Default::default());
                        let Some(pd)=raw else{
                            assert!(matches!(got,Err(Error::Unsupported(_))),"{name} lookup {} unresolved point/variation",l.id);
                            counts[1]+=1;continue;
                        };
                        let got=got.unwrap();
                        let (orig,da,child,edge)=reference_pair(design,delta,l.kind,l.flags,flow);
                        let (porig,pa,_,pedge)=reference_pair(px,pd,l.kind,l.flags,flow);
                        assert_eq!(got.design_origins,orig.to_vec());
                        assert_eq!(got.pixel_origins,porig.to_vec());
                        for i in 0..2{assert_eq!(got.adjustments[i],GposRunAdjustment{design:da[i],pixels:pa[i]});}
                        assert_eq!(got.edges[child],Some(GposAttachmentEdge{parent:1-child,design_delta:edge,pixel_delta:pedge}));
                        assert!(got.edges[1-child].is_none());
                        counts[0]+=1;if pd!=(0,0){counts[2]+=1;}
                    }
                }
            }
        }
        eprintln!("native device {name}: {counts:?}");
        for i in 0..4{total[i]+=counts[i];}
    }
    // A zero device delta is a real supported pixel-coordinate case; do not
    // claim nonzero Device-table coverage if the vendored fonts lack it.
    assert!(total[0]>0&&total[3]>0,"native device evidence: {total:?}");
}

// Independent bounded three-glyph schedule: require precisely one cursive
// pair (0,1) and one mark pair (1,2), with no other matches. The moving
// cursive endpoint may be either glyph. Root order cannot change these
// disjoint child edges. Direct world-coordinate propagation is the oracle.
fn cursive_matches(l:&Lookup<'_>,glyphs:&[u16;3],cls:&[usize])->Vec<(usize,usize,Point)>{
    let ids:Vec<_>=(0..3).filter(|&i|eligible(glyphs[i],l.flags,cls)).collect();
    ids.windows(2).filter_map(|w|pair(l,glyphs[w[0]],glyphs[w[1]]).map(|d|(w[0],w[1],d))).collect()
}
fn mark_matches(l:&Lookup<'_>,glyphs:&[u16;3],cls:&[usize])->Vec<(usize,usize,Point)>{
    let ids:Vec<_>=(0..3).filter(|&i|eligible(glyphs[i],l.flags,cls)).collect();
    let mut out=Vec::new();
    for k in 0..ids.len(){
        let child=ids[k];
        if let Some(&parent)=ids[..k].iter().rev().find(|&&i|cls[glyphs[i] as usize]!=3){
            if let Some(d)=pair(l,glyphs[parent],glyphs[child]){out.push((parent,child,d));}
        }
    }
    out
}
fn check_chain(font:&[u8],p:&CompiledGposProgram<'_>,glyphs:[u16;3],cd:Point,md:Point,flags:usize,flow:GposHorizontalFlow){
    let n=glyphs.map(|g|nominal(font,g));let mut w=n;
    let rtl=flow==GposHorizontalFlow::RightToLeft;
    if rtl{w[1]=-cd.0;}else{w[0]=cd.0;}
    let ox=if rtl{[-w[0],-w[0]-w[1],-w[0]-w[1]-w[2]]}else{[0,w[0],w[0]+w[1]]};
    let child=if flags&1!=0{0}else{1};let parent=1-child;
    let edge=if child==0{(-cd.0,-cd.1)}else{cd};
    let mut da=[GposRunAdjustment::default();3];
    for i in 0..3{da[i].design.x_advance=w[i]-n[i];}
    da[child].design.x_placement=ox[parent]-ox[child]+edge.0;
    da[child].design.y_placement=edge.1;
    da[2].design.x_placement=ox[1]+da[1].design.x_placement+md.0-ox[2];
    da[2].design.y_placement=da[1].design.y_placement+md.1;
    let adv=n.map(|w|(w,0));
    let got=execute_gpos_mixed_joint(p,&glyphs,&[Default::default();3],&adv,&[(0,0);3],
        &[GposLigatureSource::Unmapped;3],GposRunMode::DesignOnly,flow,Default::default(),Default::default()).unwrap();
    assert_eq!(got.design_origins,ox.map(|x|(x,0)).to_vec());assert_eq!(got.adjustments,da.to_vec());
    assert_eq!(got.edges[child],Some(GposAttachmentEdge{parent,design_delta:edge,pixel_delta:(0,0)}));
    assert_eq!(got.edges[2],Some(GposAttachmentEdge{parent:1,design_delta:md,pixel_delta:(0,0)}));
    assert!(got.edges[parent].is_none());
}
#[test]fn native_two_lookup_cursive_mark_chain_differential(){
    let mut total=0usize;
    for (name,font) in FONTS{
        let Some(b)=table(font,b"GPOS")else{continue;};
        let n=u(table(font,b"maxp").unwrap(),4);let gd=table(font,b"GDEF");
        let cls=classes(gd,n);let gdef=gd.map(|x|parse_gdef(x,n as u32,Default::default()).unwrap());
        let ls=lookups(b);let mut count=0;
        // For each cursive/mark lookup pair, sample from actual native
        // base coverages, not a synthetic table or substituted glyph ID.
        for cu in ls.iter().filter(|l|l.kind==3){
            for ma in ls.iter().filter(|l|l.kind==4){
                let mut candidate=None;
                'search:for cs in &cu.subs{
                    let cov=coverage(cs,u(cs,2));
                    for ms in &ma.subs{
                        let bases=coverage(ms,u(ms,4));let marks=coverage(ms,u(ms,2));
                        for &z in cov.iter().filter(|g|bases.binary_search(g).is_ok()).take(32){
                            if cls[z as usize]==3{continue;}
                            for &a in cov.iter().take(32){
                                if !eligible(a,cu.flags,&cls)||!eligible(z,cu.flags,&cls){continue;}
                                let Some(cd)=pair(cu,a,z)else{continue;};
                                for &m in marks.iter().take(32){
                                    if !eligible(z,ma.flags,&cls)||!eligible(m,ma.flags,&cls){continue;}
                                    let Some(md)=pair(ma,z,m)else{continue;};
                                    let gs=[a,z,m];
                                    if cursive_matches(cu,&gs,&cls)==vec![(0,1,cd)]
                                        &&mark_matches(ma,&gs,&cls)==vec![(1,2,md)]{
                                        candidate=Some((gs,cd,md));break 'search;
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some((gs,cd,md))=candidate{
                    let mut roots=[cu.id,ma.id];roots.sort_unstable();
                    let p=compile_gpos_program(b,&roots,n as u32,gdef.as_ref(),Default::default()).unwrap();
                    for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                        check_chain(font,&p,gs,cd,md,cu.flags,flow);count+=1;
                    }
                }
            }
        }
        eprintln!("native two-lookup chain {name}: {count}");total+=count;
    }
    assert!(total>0,"no native two-lookup chain samples");
}

// Test-only plan envelope retaining the original native LookupList,
// indices and lookup bodies. The roots are deliberately isolated from
// native LangSys selection to make the independent geometry oracle exact.
fn staged_native_layout(native:&[u8],roots:[u16;2])->Vec<u8>{
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,2,0,1]{w(&mut b,v);}
    let at=b.len();set(&mut b,6,at);w(&mut b,2);
    b.extend(b"curs");w(&mut b,14);b.extend(b"mark");w(&mut b,20);
    for id in roots{for v in [0,1,id]{w(&mut b,v);}}
    let at=b.len();set(&mut b,8,at);b.extend_from_slice(&native[u(native,8)..]);b
}
fn native_ledger(font:&[u8],ids:&[u16])->ProvenanceLedger{
    let f=parse_font(font,Default::default()).unwrap();let face=f.face(0).unwrap();
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    let r=map_source_run(&"A".repeat(ids.len()),&cmap,true,Default::default()).unwrap();
    let mut p=ProvenanceLedger::from_run(&r,u32::from(cmap.glyph_count()),Default::default()).unwrap();
    for (i,&g) in ids.iter().enumerate(){p.expand(i,&[g]).unwrap();}
    p
}
fn stage_native(p:&CompiledGposProgram<'_>,ledger:&ProvenanceLedger,ids:&[u16],adv:&[Point],
    edits:&[FeatureMaskOverride],flow:GposHorizontalFlow)->GposCursiveRun{
    let n=ids.len();let masks=GposScalarFeatureMasks::build(p.validated_plan().unwrap(),n,edits,Default::default()).unwrap();
    let specs=[ShapeStageSpec{features:vec![0]},ShapeStageSpec{features:vec![1]}];
    let (schedule,out)=execute_staged_gpos_joint(p,ledger,&masks,&specs,Default::default(),100,
        ids,&vec![GposRunAdjustment::default();n],adv,&vec![(0,0);n],&vec![GposLigatureSource::Unmapped;n],
        GposRunMode::DesignOnly,flow,Default::default(),Default::default(),false).unwrap();
    assert_eq!(schedule.stages().len(),2);out
}
#[test]fn native_staged_cursive_and_mark_pair_oracles(){
    let mut counts=[0usize;3];
    for (_,font) in FONTS{
        let Some(b)=table(font,b"GPOS") else{continue;};
        let n=u(table(font,b"maxp").unwrap(),4);
        let gd=table(font,b"GDEF");let cls=classes(gd,n);
        let gdef=gd.map(|x|parse_gdef(x,n as u32,Default::default()).unwrap());
        let ls=lookups(b);
        for l in &ls{
            let cases=samples(l,&cls);if cases.is_empty(){continue;}
            // Same root in two independently masked features. Only one
            // stage may execute, so repeated attachment is not assumed.
            let wrapped=staged_native_layout(b,[l.id,l.id]);
            let layout=parse_gpos_layout(&wrapped,Default::default()).unwrap();
            let plan=build_gpos_base_plan(&layout,*b"DFLT",None,&[*b"curs",*b"mark"],Default::default()).unwrap().unwrap();
            let p=compile_gpos_plan_program(&wrapped,&plan,n as u32,gdef.as_ref(),Default::default()).unwrap();
            for (a,z,delta) in cases{
                let ids=[a,z];let ledger=native_ledger(font,&ids);
                let adv=[(nominal(font,a),0),(nominal(font,z),0)];
                for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                    let (orig,adj,child,edge)=reference_pair([adv[0].0,adv[1].0],delta,l.kind,l.flags,flow);
                    for disabled in [0u16,1]{
                        let out=stage_native(&p,&ledger,&ids,&adv,&[FeatureMaskOverride{feature:disabled,start:0,end:2,enabled:false}],flow);
                        assert_eq!(out.design_origins,orig.to_vec());
                        assert_eq!(out.adjustments.iter().map(|a|a.design).collect::<Vec<_>>(),adj.to_vec());
                        assert_eq!(out.edges[child],Some(GposAttachmentEdge{parent:1-child,design_delta:edge,pixel_delta:(0,0)}));
                        assert!(out.edges[1-child].is_none());
                        counts[usize::from(l.kind==4)]+=1;
                    }
                    let edits=[FeatureMaskOverride{feature:0,start:1,end:2,enabled:false},FeatureMaskOverride{feature:1,start:1,end:2,enabled:false}];
                    let out=stage_native(&p,&ledger,&ids,&adv,&edits,flow);
                    assert!(out.edges.iter().all(Option::is_none));
                    assert_eq!(out.adjustments,vec![GposRunAdjustment::default();2]);counts[2]+=1;
                }
            }
        }
    }
    assert!(counts.iter().all(|&x|x>0),"native staged positive and blocked evidence: {counts:?}");
    eprintln!("NATIVE_STAGED_PAIR_REFERENCE {counts:?}");
}
#[test]fn native_staged_cursive_then_mark_attachment_chain(){
    let mut checks=0;
    for (_,font) in FONTS{
        let Some(b)=table(font,b"GPOS") else{continue;};
        let n=u(table(font,b"maxp").unwrap(),4);
        let gd=table(font,b"GDEF");let cls=classes(gd,n);
        let gdef=gd.map(|x|parse_gdef(x,n as u32,Default::default()).unwrap());
        let ls=lookups(b);
        for c in ls.iter().filter(|l|l.kind==3){
            for m in ls.iter().filter(|l|l.kind==4){
                // Same bounded raw native candidate search as the admitted
                // legacy chain oracle. Require exactly two expected edges.
                let mut candidate=None;
                'search:for cs in &c.subs{
                    let cov=coverage(cs,u(cs,2));
                    for ms in &m.subs{
                        let bases=coverage(ms,u(ms,4));let marks=coverage(ms,u(ms,2));
                        for &z in cov.iter().filter(|g|bases.binary_search(g).is_ok()).take(32){
                            if cls[z as usize]==3{continue;}
                            for &a in cov.iter().take(32){
                                if !eligible(a,c.flags,&cls)||!eligible(z,c.flags,&cls){continue;}
                                let Some(cd)=pair(c,a,z)else{continue;};
                                for &mark in marks.iter().take(32){
                                    if !eligible(z,m.flags,&cls)||!eligible(mark,m.flags,&cls){continue;}
                                    let Some(md)=pair(m,z,mark)else{continue;};
                                    let ids=[a,z,mark];
                                    if cursive_matches(c,&ids,&cls)==vec![(0,1,cd)]&&mark_matches(m,&ids,&cls)==vec![(1,2,md)]{
                                        candidate=Some((ids,cd,md));break 'search;
                                    }
                                }
                            }
                        }
                    }
                }
                let Some((ids,delta,md))=candidate else{continue;};
                let ledger=native_ledger(font,&ids);let adv=ids.map(|g|(nominal(font,g),0));
                let wrapped=staged_native_layout(b,[c.id,m.id]);
                let layout=parse_gpos_layout(&wrapped,Default::default()).unwrap();
                let plan=build_gpos_base_plan(&layout,*b"DFLT",None,&[*b"curs",*b"mark"],Default::default()).unwrap().unwrap();
                let p=compile_gpos_plan_program(&wrapped,&plan,n as u32,gdef.as_ref(),Default::default()).unwrap();
                for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                    let out=stage_native(&p,&ledger,&ids,&adv,&[],flow);
                    let child=if c.flags&1!=0{0}else{1};
                    let edge=if child==0{(-delta.0,-delta.1)}else{delta};
                    assert_eq!(out.edges[child],Some(GposAttachmentEdge{parent:1-child,design_delta:edge,pixel_delta:(0,0)}));
                    assert_eq!(out.edges[2],Some(GposAttachmentEdge{parent:1,design_delta:md,pixel_delta:(0,0)}));
                    let world=|i:usize|{let o=out.design_origins[i];let v=out.adjustments[i].design;(o.0+v.x_placement,o.1+v.y_placement)};
                    assert_eq!(diff(world(1),world(0)),delta);
                    assert_eq!(diff(world(2),world(1)),md);
                    let blocked=stage_native(&p,&ledger,&ids,&adv,&[FeatureMaskOverride{feature:0,start:0,end:3,enabled:false}],flow);
                    assert!(blocked.edges[0].is_none()&&blocked.edges[1].is_none());
                    assert_eq!(blocked.edges[2],out.edges[2]);checks+=1;
                }
            }
        }
    }
    assert!(checks>0,"native staged chain positive evidence required");
    eprintln!("NATIVE_STAGED_CHAIN_REFERENCE checks={checks}");
}
