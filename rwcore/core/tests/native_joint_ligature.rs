//! Independent raw GSUB4/GPOS5 intersection diagnostic on retained fonts.
//! Explicit lookup roots and direct cmap input, not full script shaping.
use rwcore::*;
use std::collections::{BTreeMap,BTreeSet};
type Point=(i32,i32);
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
struct Lookup<'a>{id:u16,flags:usize,subs:Vec<&'a [u8]>}
fn lookups(b:&[u8],target:usize,extension:usize)->Vec<Lookup<'_>>{
    let list=u(b,8);let mut out=Vec::new();
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);let flags=u(b,p+2);
        if flags & !15 !=0{continue;}
        let mut subs=Vec::new();
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==extension{kind=u(b,at+2);at+=d(b,at+4);}
            if kind==target{subs.push(&b[at..]);}
        }
        if !subs.is_empty(){out.push(Lookup{id:i as u16,flags,subs});}
    }out
}
fn classes(g:&[u8],n:usize)->Vec<usize>{
    let mut c=vec![0;n];let p=u(g,4);if p==0{return c;}
    match u(g,p){
        1=>{let start=u(g,p+2);for i in 0..u(g,p+4){c[start+i]=u(g,p+6+2*i);}},
        2=>{for i in 0..u(g,p+2){let q=p+4+6*i;for k in u(g,q)..=u(g,q+2){c[k]=u(g,q+4);}}},
        _=>panic!("class format"),
    }c
}
fn eligible(g:u16,flags:usize,c:&[usize])->bool{
    match c[g as usize]{1=>flags&2==0,2=>flags&4==0,3=>flags&8==0,_=>true}
}
fn rules(l:&Lookup<'_>)->Vec<(Vec<u16>,u16)>{
    let mut out=Vec::new();
    for s in &l.subs{
        assert_eq!(u(s,0),1);let cov=coverage(s,u(s,2));assert_eq!(cov.len(),u(s,4));
        for (i,&first) in cov.iter().enumerate(){
            let set=u(s,6+2*i);
            for j in 0..u(s,set){
                let p=set+u(s,set+2+2*j);
                let mut input=vec![first];
                input.extend((1..u(s,p+2)).map(|k|u(s,p+2+2*k) as u16));
                out.push((input,u(s,p) as u16));
            }
        }
    }out
}
fn first_match(rules:&[(Vec<u16>,u16)],input:&[u16])->Option<(usize,u16)>{
    rules.iter().find(|(seq,_)|input.starts_with(seq)).map(|(s,g)|(s.len(),*g))
}
fn anchor(s:&[u8],at:usize)->Point{
    assert!((1..=3).contains(&u(s,at)));
    (i32::from(u(s,at+2) as i16),i32::from(u(s,at+4) as i16))
}
// First matching subtable, same lookup order. None for absent anchor.
fn mark_pair(l:&Lookup<'_>,parent:u16,mark:u16,component:usize)->Option<(usize,Point)>{
    for s in &l.subs{
        let Ok(i)=coverage(s,u(s,2)).binary_search(&mark) else{continue;};
        let Ok(j)=coverage(s,u(s,4)).binary_search(&parent) else{continue;};
        let ma=u(s,8);let la=u(s,10);let a=la+u(s,la+2+2*j);let n=u(s,a);
        if component>=n{continue;}
        let cl=u(s,ma+2+4*i);let mo=u(s,ma+4+4*i);let po=u(s,a+2+2*(component*u(s,6)+cl));
        if mo==0||po==0{continue;}
        let (x,y)=anchor(s,a+po);let (mx,my)=anchor(s,ma+mo);
        return Some((n,(x-mx,y-my)));
    }None
}
fn nominal(font:&[u8],g:u16)->i32{
    let h=table(font,b"hhea").unwrap();let m=table(font,b"hmtx").unwrap();
    u(m,4*(g as usize).min(u(h,34)-1)) as i32
}
const FONTS:[(&str,&[u8]);9]=[
    ("ABeeZee",include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf")),
    ("ABeeZee italic",include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf")),
    ("Alegreya Sans",include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf")),
    ("Archivo",include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf")),
    ("Aref Ruqaa",include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf")),
    ("Noto Nastaliq Urdu",include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf")),
    ("Noto Sans Mongolian",include_bytes!("fixtures/notosansmongolian/NotoSansMongolian-Regular.ttf")),
    ("Noto Sans Devanagari",include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf")),
    ("Padauk",include_bytes!("fixtures/padauk/Padauk-Regular.ttf")),
];

// Test-only isolate an explicit native root under a synthetic DFLT feature.
// Preserve the entire native LookupList suffix byte-for-byte, including
// original indices and all relative offsets; no lookup body is rewritten.
// This deliberately tests explicit lookup execution, not native LangSys selection.
fn isolated_layout(native:&[u8],tag:[u8;4],root:Option<u16>)->Vec<u8>{
    fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
    fn set(b:&mut [u8],at:usize,v:usize){b[at..at+2].copy_from_slice(&(v as u16).to_be_bytes());}
    let mut b=vec![0,1,0,0,0,0,0,0,0,0];
    let at=b.len();set(&mut b,4,at);w(&mut b,1);b.extend(b"DFLT");w(&mut b,8);
    for v in [4,0,0,0xffff,u16::from(root.is_some())]{w(&mut b,v);}
    if root.is_some(){w(&mut b,0);}
    let at=b.len();set(&mut b,6,at);w(&mut b,u16::from(root.is_some()));
    if let Some(id)=root{b.extend(tag);for v in [8,0,1,id]{w(&mut b,v);}}
    let at=b.len();set(&mut b,8,at);b.extend_from_slice(&native[u(native,8)..]);b
}
fn isolated_dual(sb:&[u8],pb:&[u8],sroot:u16,proot:Option<u16>,r:&SourceRun,m:&FontMetrics<'_>,
    n:u32,gd:&Gdef,flow:GposHorizontalFlow,trace:bool,edits:&[FeatureMaskOverride])->DualStageShapeResult{
    let s=isolated_layout(sb,*b"liga",Some(sroot));
    let p=isolated_layout(pb,*b"mark",proot);
    let sl=parse_gsub_layout(&s,Default::default()).unwrap();
    let pl=parse_gpos_layout(&p,Default::default()).unwrap();
    let sp=build_gsub_base_plan(&sl,*b"DFLT",None,&[*b"liga"],Default::default()).unwrap().unwrap();
    let pp=build_gpos_base_plan(&pl,*b"DFLT",None,&[*b"mark"],Default::default()).unwrap().unwrap();
    let gs=compile_gsub_plan_program(&s,&sp,n,Some(gd),Default::default()).unwrap();
    let gp=compile_gpos_plan_program(&p,&pp,n,Some(gd),Default::default()).unwrap();
    assert_eq!(gs.roots(),&[sroot]);assert!(gp.roots().iter().copied().eq(proot));
    let ss=[ShapeStageSpec{features:vec![0]}];
    let ps=if proot.is_some(){&ss[..]}else{&[][..]};
    let sm=ScalarFeatureMasks::build(&sp,r.scalars().len(),&[],Default::default()).unwrap();
    let pm=GposScalarFeatureMasks::build(&pp,r.scalars().len(),edits,Default::default()).unwrap();
    execute_dual_stage_joint_shape(&gs,&gp,r,m,&[],&sm,&ss,flow,Default::default(),Default::default(),
        if trace{Some((gd,Default::default()))}else{None},
        JointPositionStages{masks:&pm,specs:ps,limits:Default::default()}).unwrap()
}

// counts: direct rules, eligible Unicode contraction candidates, tested
// contractions, raw output/Type5 parent intersections, admitted attachment
// rows, exact attachment rows, nonzero attachment rows.
fn check(font:&[u8])->[usize;7]{
    let mut counts=[0;7];
    let (Some(sb),Some(pb),Some(gb))=(table(font,b"GSUB"),table(font,b"GPOS"),table(font,b"GDEF")) else{return counts;};
    let f=parse_font(font,Default::default()).unwrap();let face=f.face(0).unwrap();
    let m=parse_font_metrics(face,Default::default()).unwrap();let n=u32::from(m.maxp().glyph_count);
    let gd=parse_gdef(gb,n,Default::default()).unwrap();let cls=classes(gb,n as usize);
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    let mut reverse=BTreeMap::new();
    for code in 0..=0xffff{
        if let Some(ch)=char::from_u32(code){
            if let Some(g)=cmap.glyph_for_char(ch).unwrap(){reverse.entry(g).or_insert(ch);}
        }
    }
    let sl=lookups(sb,4,7);let pl=lookups(pb,5,9);
    let mut mark_inventory=Vec::new();
    for p in &pl{
        let mut parents=BTreeSet::new();let mut marks=BTreeSet::new();
        for s in &p.subs{
            parents.extend(coverage(s,u(s,4)));
            marks.extend(coverage(s,u(s,2)).into_iter().filter(|&g|
                cls[g as usize]==3&&eligible(g,p.flags,&cls)&&reverse.contains_key(&g)));
        }
        mark_inventory.push((parents,marks));
    }
    let empty=compile_gpos_program(pb,&[],n,Some(&gd),Default::default()).unwrap();
    for s in &sl{
        let all=rules(s);counts[0]+=all.len();
        let mut seen=BTreeSet::new();
        for (seq,parent) in &all{
            if !seen.insert(seq.clone()){continue;}
            if first_match(&all,seq)!=Some((seq.len(),*parent)){continue;}
            if seq.len()<2||seq.len()>8||seq.iter().any(|&g|!eligible(g,s.flags,&cls)){continue;}
            let Some(text)=seq.iter().map(|g|reverse.get(g).copied()).collect::<Option<String>>() else{continue;};
            counts[1]+=1;
            let r=map_source_run(&text,&cmap,false,Default::default()).unwrap();
            let gs=compile_gsub_program(sb,&[s.id],n,Some(&gd),Default::default()).unwrap();
            if counts[2]<32{
                let out=execute_joint_shape(&gs,&empty,&r,&m,&[],GposHorizontalFlow::LeftToRight,Default::default()).unwrap();
                assert_eq!(out.substitution.expansion.execution.glyphs,vec![*parent]);
                assert_eq!(out.provenance.glyphs()[0].sources(),(0..seq.len()).collect::<Vec<_>>().as_slice());
                assert_eq!(out.design_advances,vec![(nominal(font,*parent),0)]);
                let dual=isolated_dual(sb,pb,s.id,None,&r,&m,n,&gd,GposHorizontalFlow::LeftToRight,false,&[]);
                assert_eq!(dual.staged.shaped.substitution,out.substitution);
                assert_eq!(dual.staged.shaped.provenance,out.provenance);
                assert_eq!(dual.staged.shaped.design_advances,vec![(nominal(font,*parent),0)]);
                assert_eq!(dual.staged.shaped.positioning,out.positioning);
                counts[2]+=1;
            }
            for (p,(parents,marks)) in pl.iter().zip(&mark_inventory){
                if !parents.contains(parent){continue;}
                counts[3]+=1;
                if cls[*parent as usize]!=2||!eligible(*parent,p.flags,&cls)||seq.iter().any(|g|cls[*g as usize]==3){continue;}
                // Deterministic bounded candidate inventory: at most two
                // native trailing marks per direct GSUB/GPOS lookup pair.
                for &mark in marks.iter().filter(|&&mark|{
                    let mut input=seq.clone();
                    if eligible(mark,s.flags,&cls){input.push(mark);}
                    first_match(&all,&input)==Some((seq.len(),*parent))
                }).filter(|&&mark|mark_pair(p,*parent,mark,seq.len()-1).is_some_and(|(k,_)|k==seq.len())).take(2){
                    if counts[4]>=32{break;}
                    let (_,delta)=mark_pair(p,*parent,mark,seq.len()-1).unwrap();
                    let mut text=text.clone();text.push(reverse[&mark]);
                    let r=map_source_run(&text,&cmap,false,Default::default()).unwrap();
                    let gp=compile_gpos_program(pb,&[p.id],n,Some(&gd),Default::default()).unwrap();
                    counts[4]+=1;
                    for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                        let out=execute_joint_shape_with_ligatures(&gs,&gp,&r,&m,&gd,&[],flow,
                            Default::default(),Default::default()).unwrap();
                        assert_eq!(out.substitution.expansion.execution.glyphs,vec![*parent,mark]);
                        assert_eq!(out.design_advances,vec![(nominal(font,*parent),0),(nominal(font,mark),0)]);
                        assert_eq!(out.positioning.edges[1],Some(GposAttachmentEdge{parent:0,design_delta:delta,pixel_delta:(0,0)}));
                        let a=out.positioning.adjustments[1].design;let o=out.positioning.design_origins;
                        assert_eq!((o[1].0+a.x_placement,o[1].1+a.y_placement),(o[0].0+delta.0,o[0].1+delta.1));
                    }
                    counts[5]+=1;counts[6]+=usize::from(delta!=(0,0));
                }
            }
        }
    }
    counts
}
// Supplemental Linux fixture from the distribution's DejaVu Sans package.
// Explicit Arabic presentation forms isolate GSUB4 from joining analysis.
// Expected glyphs and anchor deltas are decoded from original font bytes,
// independently of the production lookup parser and positioner.
fn dejavu_positive(font:&[u8])->[usize;3]{
    let sb=table(font,b"GSUB").unwrap();let pb=table(font,b"GPOS").unwrap();
    let gb=table(font,b"GDEF").unwrap();
    let f=parse_font(font,Default::default()).unwrap();let face=f.face(0).unwrap();
    let m=parse_font_metrics(face,Default::default()).unwrap();let n=u32::from(m.maxp().glyph_count);
    let gd=parse_gdef(gb,n,Default::default()).unwrap();let cls=classes(gb,n as usize);
    let cmap=parse_unicode_cmap(face,Default::default()).unwrap();
    let gid=|ch|cmap.glyph_for_char(ch).unwrap().unwrap();
    let a=gid('\u{fedf}');let b=gid('\u{fe8e}');let parent=gid('\u{fefb}');
    assert_eq!(cls[parent as usize],2);
    let sl=lookups(sb,4,7);let pl=lookups(pb,5,9);
    let s=sl.iter().find(|l|l.flags&8!=0&&first_match(&rules(l),&[a,b])==Some((2,parent)))
        .expect("DejaVu native lam-alef contraction");
    let gs=compile_gsub_program(sb,&[s.id],n,Some(&gd),Default::default()).unwrap();
    let mut counts=[0;3];
    for ch in ['\u{64b}','\u{64e}']{
        let mark=gid(ch);assert_eq!(cls[mark as usize],3);
        let p=pl.iter().find(|l|eligible(parent,l.flags,&cls)&&eligible(mark,l.flags,&cls)&&
            (0..2).all(|i|mark_pair(l,parent,mark,i).is_some_and(|(k,_)|k==2)))
            .expect("DejaVu native mark-to-ligature anchors");
        let gp=compile_gpos_program(pb,&[p.id],n,Some(&gd),Default::default()).unwrap();
        let deltas=[mark_pair(p,parent,mark,0).unwrap().1,mark_pair(p,parent,mark,1).unwrap().1];
        assert_ne!(deltas[0],deltas[1],"component anchors must distinguish ownership");
        for (text,component) in [
            (format!("{}{}{}",'\u{fedf}',ch,'\u{fe8e}'),0usize),
            (format!("{}{}{}",'\u{fedf}','\u{fe8e}',ch),1usize)]{
            let r=map_source_run(&text,&cmap,false,Default::default()).unwrap();
            let delta=deltas[component];assert_ne!(delta,(0,0));
            counts[0]+=1;
            for flow in [GposHorizontalFlow::LeftToRight,GposHorizontalFlow::RightToLeft]{
                let out=execute_joint_shape_with_ligatures(&gs,&gp,&r,&m,&gd,&[],flow,
                    Default::default(),Default::default()).unwrap();
                assert_eq!(out.substitution.expansion.execution.glyphs,vec![parent,mark]);
                let history=out.provenance.glyphs()[0].ligature_lineage().unwrap();
                let want=if component==0{vec![vec![0],vec![2]]}else{vec![vec![0],vec![1]]};
                assert_eq!(history.components(),want.as_slice());
                assert_eq!(out.design_advances,vec![(nominal(font,parent),0),(nominal(font,mark),0)]);
                assert_eq!(out.positioning.edges[1],Some(GposAttachmentEdge{parent:0,design_delta:delta,pixel_delta:(0,0)}));
                let o=out.positioning.design_origins;let adjust=out.positioning.adjustments[1].design;
                assert_eq!((o[1].0+adjust.x_placement,o[1].1+adjust.y_placement),(o[0].0+delta.0,o[0].1+delta.1));
                let dual=isolated_dual(sb,pb,s.id,Some(p.id),&r,&m,n,&gd,flow,true,&[]).staged.shaped;
                assert_eq!(dual.substitution,out.substitution);
                assert_eq!(dual.design_advances,out.design_advances);
                assert_eq!(dual.provenance.glyphs()[0].ligature_lineage().unwrap().components(),want.as_slice());
                assert_eq!(dual.positioning.edges[1],Some(GposAttachmentEdge{parent:0,design_delta:delta,pixel_delta:(0,0)}));
                assert_eq!(dual.positioning,out.positioning);
                // Disable only the second contributing scalar of the
                // ligature in GPOS. GSUB still contracts; parent intersection
                // must block the otherwise valid native component attachment.
                let index=if component==0{2}else{1};
                let edit=[FeatureMaskOverride{feature:0,start:index,end:index+1,enabled:false}];
                let masked=isolated_dual(sb,pb,s.id,Some(p.id),&r,&m,n,&gd,flow,true,&edit).staged.shaped;
                assert_eq!(masked.substitution,out.substitution);
                assert_eq!(masked.positioning.edges,vec![None,None]);
                counts[1]+=1;counts[2]+=1;
            }
        }
    }
    assert_eq!(counts,[4,8,8]);counts
}
#[test]fn native_joint_ligature_inventory_and_geometry(){
    let mut total=[0;7];let mut report=String::from("scope=direct BMP cmap, explicit GSUB4/GPOS5 roots, trailing mark, simple lineage; both physical flows\nfields=rules,unicode_candidates,contractions,unicode_parent_intersections,admitted,exact,nonzero\n");
    for (name,font) in FONTS{
        let c=check(font);for i in 0..7{total[i]+=c[i];}
        report+=&format!("{name}: {c:?}\n");
    }
    report+=&format!("TOTAL {total:?}\n");
    report+=&format!("DUAL_STAGE_NATIVE_CONTRACTIONS exact={}\n",total[2]);
    report+=&format!("combined_positive_evidence={}\n",total[6]>0);
    if let Some(path)=std::env::var_os("RWCORE_DEJAVU_REFERENCE"){
        let bytes=std::fs::read(path).expect("read supplemental DejaVu font");
        let c=dejavu_positive(&bytes);
        report+=&format!("DUAL_STAGE_DEJAVU_REFERENCE exact={} masked_parent_blocked={} max_absolute_error=0\n",c[1],c[1]);
        report+=&format!("DEJAVU_JOINT_REFERENCE cases={} direction_checks={} nonzero_direction_checks={} max_absolute_error=0\n",c[0],c[1],c[2]);
    }else{
        report+="DEJAVU_JOINT_REFERENCE not_requested (supplemental Linux gate)\n";
    }
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/native-joint-ligature.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path,&report).unwrap();eprintln!("{report}");
    assert!(total[0]>0&&total[1]>0&&total[2]>0,"native contraction evidence missing");
    assert_eq!(total[4],total[5]);
    // No fabricated positive Type5 claim when direct Unicode intersection
    // is empty. The report explicitly distinguishes this from parity.
}

#[test]fn isolated_native_layout_preserves_lookup_bytes_and_root_indices(){
    for (_,font) in FONTS{
        for (tag,feature) in [(b"GSUB",*b"liga"),(b"GPOS",*b"mark")]{
            let Some(native)=table(font,tag) else{continue;};
            let list=u(native,8);let count=u(native,list);
            for root in [None,if count>0{Some((count-1) as u16)}else{None}]{
                let b=isolated_layout(native,feature,root);
                assert_eq!(&b[u(&b,8)..],&native[list..]);
                if tag==b"GSUB"{
                    let l=parse_gsub_layout(&b,Default::default()).unwrap();
                    let p=build_gsub_base_plan(&l,*b"DFLT",None,&[feature],Default::default()).unwrap().unwrap();
                    assert!(p.lookups.iter().map(|x|x.index).eq(root));
                }else{
                    let l=parse_gpos_layout(&b,Default::default()).unwrap();
                    let p=build_gpos_base_plan(&l,*b"DFLT",None,&[feature],Default::default()).unwrap().unwrap();
                    assert!(p.lookups.iter().map(|x|x.index).eq(root));
                }
            }
        }
    }
}
