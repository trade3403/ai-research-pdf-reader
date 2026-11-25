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
