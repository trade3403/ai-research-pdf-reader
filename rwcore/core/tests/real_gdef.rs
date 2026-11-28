//! Independent dense reference for GDEF classes and mark-set memberships.
//! Original OFL fixture bytes; no external shaper or production dependency.
use rwcore::*;

fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn d(b:&[u8],p:usize)->usize{u32::from_be_bytes(b[p..p+4].try_into().unwrap()) as usize}
fn table<'a>(b:&'a [u8],tag:&[u8;4])->Option<&'a [u8]>{
    for i in 0..u(b,4){
        let p=12+16*i;
        if &b[p..p+4]==tag{
            let start=d(b,p+8);
            return Some(&b[start..start+d(b,p+12)]);
        }
    }
    None
}
fn dense_classes(b:&[u8],at:usize,n:usize)->Vec<u16>{
    let mut out=vec![0;n];
    if at==0{return out;}
    match u(b,at){
        1=>{
            let first=u(b,at+2);
            for i in 0..u(b,at+4){out[first+i]=u(b,at+6+2*i) as u16;}
        }
        2=>for i in 0..u(b,at+2){
            let p=at+4+6*i;
            out[u(b,p)..=u(b,p+2)].fill(u(b,p+4) as u16);
        },
        f=>panic!("unexpected class format {f}"),
    }
    out
}
fn ref_coverage(b:&[u8],at:usize)->Vec<u16>{
    match u(b,at){
        1=>(0..u(b,at+2)).map(|i|u(b,at+4+2*i) as u16).collect(),
        2=>(0..u(b,at+2)).flat_map(|i|{
            let p=at+4+6*i;
            u(b,p) as u16..=u(b,p+2) as u16
        }).collect(),
        f=>panic!("unexpected coverage format {f}"),
    }
}
fn nullable(n:usize)->Option<usize>{if n==0{None}else{Some(n)}}
fn verify(font:&[u8],n:usize,len:usize,minor:usize,offsets:[usize;6],classes:[usize;5],
          mark_hist:&[(u16,usize)],sets:&[(usize,usize)]){
    let b=table(font,b"GDEF").unwrap();
    assert_eq!(u(table(font,b"maxp").unwrap(),4),n);
    assert_eq!(b.len(),len);
    assert_eq!(u(b,0),1);assert_eq!(u(b,2),minor);
    let raw=[u(b,4),u(b,6),u(b,8),u(b,10),
        if minor>=2{u(b,12)}else{0},if minor==3{d(b,14)}else{0}];
    assert_eq!(raw,offsets);
    let parsed=parse_font(font,FontLimits::default()).unwrap();
    let face=parsed.face(0).unwrap();
    assert_eq!(face.table(*b"GDEF").unwrap(),b);
    let g=parse_gdef(b,n as u32,GdefLimits::default()).unwrap();
    let dir=g.directory();
    assert_eq!(dir.minor_version as usize,minor);
    assert_eq!([dir.glyph_class_def,dir.attach_list,dir.lig_caret_list,
        dir.mark_attach_class_def,dir.mark_glyph_sets_def,dir.item_var_store],raw.map(nullable));
    assert_eq!(&parse_gdef_directory(b,GdefLimits::default()).unwrap(),dir);
    assert_eq!(g.glyph_classes().is_some(),raw[0]!=0);
    assert_eq!(g.mark_attach_classes().is_some(),raw[3]!=0);
    let glyph=dense_classes(b,raw[0],n);
    let mark=dense_classes(b,raw[3],n);
    let mut glyph_counts=[0;5];
    let mut mark_counts=std::collections::BTreeMap::new();
    use GdefGlyphClass::*;
    let kinds=[Unclassified,Base,Ligature,Mark,Component];
    for i in 0..n{
        let id=i as u16;
        glyph_counts[glyph[i] as usize]+=1;
        *mark_counts.entry(mark[i]).or_insert(0usize)+=1;
        assert_eq!(g.glyph_class(id),Some(kinds[glyph[i] as usize]),"glyph {i}");
        assert_eq!(g.mark_attachment_class(id),Some(mark[i]),"glyph {i}");
        if let Some(c)=g.glyph_classes(){assert_eq!(c.class(id),Some(glyph[i]));}
        if let Some(c)=g.mark_attach_classes(){assert_eq!(c.class(id),Some(mark[i]));}
    }
    assert_eq!(glyph_counts,classes);
    assert_eq!(mark_counts.into_iter().collect::<Vec<_>>(),mark_hist);
    assert_eq!(g.glyph_class(n as u16),None);
    assert_eq!(g.mark_attachment_class(n as u16),None);
    if raw[4]==0{
        assert!(g.mark_sets().is_none());
        assert!(sets.is_empty());
    }else{
        let p=raw[4];
        assert_eq!(u(b,p),1);
        assert_eq!(u(b,p+2),sets.len());
        let m=g.mark_sets().unwrap();
        assert_eq!(m.len(),sets.len());
        for (i,&(format,count)) in sets.iter().enumerate(){
            let at=p+d(b,p+4+4*i);
            assert_eq!(u(b,at),format);
            let expected=ref_coverage(b,at);
            assert_eq!(expected.len(),count);
            let mut dense=vec![None;n];
            for (index,&id) in expected.iter().enumerate(){dense[id as usize]=Some(index as u32);}
            let c=m.coverage(i).unwrap();
            assert_eq!(c.len() as usize,count);
            for (id,index) in dense.into_iter().enumerate(){
                assert_eq!(c.index(id as u16),index,"set {i}, glyph {id}");
                assert_eq!(m.contains(i,id as u16),Some(index.is_some()));
            }
            assert_eq!(m.contains(i,n as u16),None);
        }
        assert!(m.coverage(sets.len()).is_none());
        assert_eq!(m.contains(sets.len(),0),None);
        let standalone=parse_mark_glyph_sets(&b[p..],n as u32,Default::default()).unwrap();
        assert_eq!(&standalone,m);
    }
}
#[test]fn abeezee_regular_gdef_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,96,2,
        [14,0,0,0,60,0],[115,134,0,19,0],&[(0,268)],&[(1,2),(2,17)]);
}
#[test]fn abeezee_italic_gdef_reference(){
    verify(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,102,2,
        [14,0,0,0,66,0],[116,133,0,19,0],&[(0,268)],&[(1,2),(2,17)]);
}
#[test]fn alegreya_gdef_reference(){
    verify(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,974,0,
        [12,0,0,910,0,0],[582,1452,0,75,0],&[(0,2037),(1,7),(2,63),(3,2)],&[]);
}
#[test]fn aref_gdef_reference(){
    verify(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,330,2,
        [14,0,0,0,234,0],[161,540,0,38,0],&[(0,739)],&[(1,18),(1,8),(1,1),(2,7)]);
}
#[test]fn padauk_gdef_reference(){
    verify(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,1860,2,
        [14,0,0,954,1282,0],[274,368,8,132,0],&[(0,704),(1,78)],&[(2,48),(1,7),(1,6),(1,108),(1,107)]);
}
#[test]fn devanagari_gdef_reference(){
    verify(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,7209,3,
        [18,0,0,0,370,588],[367,518,0,232,0],&[(0,1117)],
        &[(1,6),(2,6),(2,11),(1,7),(1,3),(1,25),(2,47),(1,3),(2,12)]);
}
#[test]fn nastaliq_gdef_reference(){
    verify(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,2317,3,
        [18,0,0,496,980,1740],[384,998,18,323,0],&[(0,1438),(1,52),(2,65),(3,25),(4,133),(5,10)],
        &[(1,21),(2,22),(2,136),(1,62),(1,34),(1,12),(1,94),(1,12)]);
}
#[test]fn abel_absent_gdef_reference(){
    let b=include_bytes!("fixtures/abel/Abel-Regular.ttf");
    assert_eq!(u(table(b,b"maxp").unwrap(),4),259);
    assert!(table(b,b"GDEF").is_none());
    let f=parse_font(b,FontLimits::default()).unwrap();
    assert!(f.face(0).unwrap().table(*b"GDEF").is_none());
}


// Independently read actual GSUB lookup flags and dense GDEF assignments.
// These checks certify eligibility/index projection, not substitution execution.
fn reference_eligibility(flags:usize,class:u16,attachment:u16,set_member:Option<bool>)->GlyphEligibility{
    use GlyphEligibility::*;
    if class==1 && flags&2!=0{return IgnoreBase;}
    if class==2 && flags&4!=0{return IgnoreLigature;}
    if class!=3{return Eligible;}
    if flags&8!=0{return IgnoreMark;}
    if flags&16!=0{return if set_member.expect("reference mark set"){Eligible}else{OutsideMarkSet};}
    let expected=flags/256;
    if expected!=0 && usize::from(attachment)!=expected{return WrongMarkAttachmentClass;}
    Eligible
}
fn verify_lookup_flags(font:&[u8],n:usize,lookups:usize,flag_hist:&[(usize,usize)],set_hist:&[(usize,usize)]){
    let gdef=table(font,b"GDEF").unwrap();
    let gsub=table(font,b"GSUB").unwrap();
    assert_eq!(u(table(font,b"maxp").unwrap(),4),n);
    let glyph=dense_classes(gdef,u(gdef,4),n);
    let attachment=dense_classes(gdef,u(gdef,10),n);
    let minor=u(gdef,2);
    let mark_at=if minor>=2{u(gdef,12)}else{0};
    let sets:Vec<Vec<bool>>=if mark_at==0{Vec::new()}else{
        assert_eq!(u(gdef,mark_at),1);
        (0..u(gdef,mark_at+2)).map(|i|{
            let p=mark_at+d(gdef,mark_at+4+4*i);
            let mut members=vec![false;n];
            for id in ref_coverage(gdef,p){members[id as usize]=true;}
            members
        }).collect()
    };
    let parsed_gdef=parse_gdef(gdef,n as u32,Default::default()).unwrap();
    let parsed_gsub=parse_gsub_layout(gsub,Default::default()).unwrap();
    let list=u(gsub,8);
    assert_eq!(u(gsub,list),lookups);
    assert_eq!(parsed_gsub.lookups.len(),lookups);
    let mut flags_seen=std::collections::BTreeMap::new();
    let mut sets_seen=std::collections::BTreeMap::new();
    let mut checked=0;
    let asc:Vec<u16>=(0..n as u16).collect();
    // Reversed domain plus duplicates exposes accidental reordering or dedup.
    let mixed:Vec<u16>=asc.iter().rev().copied().chain(asc.iter().step_by(17).copied()).collect();
    for i in 0..lookups{
        let at=list+u(gsub,list+2+2*i);
        let kind=u(gsub,at);
        let flags=u(gsub,at+2);
        let sub_count=u(gsub,at+4);
        let index=if flags&16!=0{Some(u(gsub,at+6+2*sub_count))}else{None};
        *flags_seen.entry(flags).or_insert(0usize)+=1;
        if let Some(s)=index{
            assert!(s<sets.len());
            *sets_seen.entry(s).or_insert(0usize)+=1;
        }
        let lookup=&parsed_gsub.lookups[i];
        assert_eq!(usize::from(lookup.kind),kind);
        assert_eq!(usize::from(lookup.flags),flags);
        assert_eq!(lookup.mark_filtering_set.map(usize::from),index);
        assert_eq!(lookup.subtables.len(),sub_count);
        for (j,&sub) in lookup.subtables.iter().enumerate(){
            assert_eq!(sub,at+u(gsub,at+6+2*j));
        }
        let f=LookupFilter::from_gsub_lookup(lookup,n as u32,Some(&parsed_gdef)).unwrap();
        assert_eq!(usize::from(f.flags()),flags);
        assert_eq!(f.right_to_left(),flags&1!=0);
        assert_eq!(usize::from(f.mark_attachment_type()),flags/256);
        assert_eq!(f.mark_filtering_set(),index);
        let expected:Vec<_>=(0..n).map(|id|{
            reference_eligibility(flags,glyph[id],attachment[id],index.map(|s|sets[s][id]))
        }).collect();
        for (id,&e) in expected.iter().enumerate(){
            assert_eq!(f.eligibility(id as u16).unwrap(),e,"lookup {i}, glyph {id}, flag {flags:#06x}");
            checked+=1;
        }
        assert!(f.eligibility(n as u16).is_err());
        for run in [&asc,&mixed]{
            let want:Vec<usize>=run.iter().enumerate().filter_map(|(j,&id)|{
                (expected[id as usize]==GlyphEligibility::Eligible).then_some(j)
            }).collect();
            let actual=f.eligible_indices(run,LookupFilterLimits{max_run_glyphs:run.len(),max_eligible:want.len()}).unwrap();
            assert_eq!(actual,want,"lookup {i}, projected indices");
            if !want.is_empty(){
                assert!(f.eligible_indices(run,LookupFilterLimits{max_run_glyphs:run.len(),max_eligible:want.len()-1}).is_err());
            }
        }
    }
    assert_eq!(checked,n*lookups);
    assert_eq!(flags_seen.into_iter().collect::<Vec<_>>(),flag_hist);
    assert_eq!(sets_seen.into_iter().collect::<Vec<_>>(),set_hist);
}
#[test]fn abeezee_lookup_flags_reference(){
    for font in [include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf").as_slice(),
                 include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf").as_slice()]{
        verify_lookup_flags(font,268,15,&[(0,15)],&[]);
    }
}
#[test]fn alegreya_lookup_flags_reference(){
    verify_lookup_flags(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,57,&[(0,57)],&[]);
}
#[test]fn aref_lookup_flags_reference(){
    verify_lookup_flags(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,86,&[(0,21),(8,65)],&[]);
}
#[test]fn padauk_lookup_flags_reference(){
    verify_lookup_flags(include_bytes!("fixtures/padauk/Padauk-Regular.ttf"),782,117,
        &[(0,109),(8,4),(16,1),(18,1),(22,1),(258,1)],&[(0,1),(1,1),(2,1)]);
}
#[test]fn devanagari_lookup_flags_reference(){
    verify_lookup_flags(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),1117,177,
        &[(0,160),(8,4),(16,13)],&[(0,10),(2,3)]);
}
#[test]fn nastaliq_lookup_flags_reference(){
    verify_lookup_flags(include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf"),1723,202,
        &[(0,161),(8,11),(12,5),(16,3),(256,1),(512,8),(768,3),(1024,9),(1280,1)],&[(0,1),(7,2)]);
}
