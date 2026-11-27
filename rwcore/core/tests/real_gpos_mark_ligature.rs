//! Independent raw-byte native MarkToLigature oracle; no production parsing helpers.
use rwcore::*;
fn u(b:&[u8],p:usize)->usize{u16::from_be_bytes([b[p],b[p+1]]) as usize}
fn coverage(b:&[u8],p:usize)->Vec<u16>{
    let n=u(b,p+2);
    match u(b,p){
        1=>(0..n).map(|i|u(b,p+4+2*i) as u16).collect(),
        2=>{
            let mut v=Vec::new();
            for i in 0..n{
                let q=p+4+6*i;assert_eq!(u(b,q+4),v.len());
                v.extend((u(b,q)..=u(b,q+2)).map(|x|x as u16));
            }
            v
        }
        _=>panic!("unexpected coverage"),
    }
}
fn device(b:&[u8],p:usize)->Option<GposDevice>{
    if p==0{return None;}
    let a=u(b,p);let z=u(b,p+2);let f=u(b,p+4);
    if f==0x8000{return Some(GposDevice::VariationIndex{outer:a as u16,inner:z as u16});}
    let bits=match f{1=>2,2=>4,3=>8,_=>panic!("unexpected device")};
    // Read one bit at a time, independently of production word extraction.
    let deltas=(0..=z-a).map(|i|{
        let mut v=0i16;
        for j in 0..bits{
            let bit=i*bits+j;
            v=v*2+i16::from((b[p+6+bit/8]>>(7-bit%8))&1);
        }
        if v>=1<<(bits-1){v-=1<<bits;}v as i8
    }).collect();
    Some(GposDevice::Pixels{start:a as u16,end:z as u16,deltas})
}
fn raw_pixel(d:&Option<GposDevice>,ppem:u16)->Option<i32>{
    match d{
        None=>Some(0),
        Some(GposDevice::VariationIndex{..})=>None,
        Some(GposDevice::Pixels{start,end,deltas})=>Some(if ppem<*start||ppem>*end{0}else{i32::from(deltas[(ppem-*start) as usize])}),
    }
}

fn anchor(b:&[u8],p:usize)->Option<GposAnchor>{
    if p==0{return None;}
    let detail=match u(b,p){
        1=>GposAnchorDetail::Coordinate,
        2=>GposAnchorDetail::ContourPoint(u(b,p+6) as u16),
        3=>{
            let read=|at|{let rel=u(b,at);device(b,if rel==0{0}else{p+rel})};
            GposAnchorDetail::Device{x:read(p+6),y:read(p+8)}
        }
        _=>panic!("anchor format"),
    };
    Some(GposAnchor{x:u(b,p+2) as i16,y:u(b,p+4) as i16,detail})
}
fn px(a:&GposAnchor,x:u16,y:u16)->Option<(i32,i32)>{
    match &a.detail{
        GposAnchorDetail::Coordinate=>Some((0,0)),
        GposAnchorDetail::ContourPoint(_)=>None,
        GposAnchorDetail::Device{x:dx,y:dy}=>Some((raw_pixel(dx,x)?,raw_pixel(dy,y)?)),
    }
}

fn check(font:&[u8])->[usize;7]{
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let Some(b)=sf.table(*b"GPOS") else{return [0;7];};
    let n=u(sf.table(*b"maxp").unwrap(),4);
    let list=u(b,8);let mut counts=[0;7];
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==9{assert_eq!(u(b,at),1);kind=u(b,at+2);at+=u(b,at+4)*65536+u(b,at+6);}
            if kind!=5{continue;}
            let q=check_subtable(&b[at..],n);for k in 0..7{counts[k]+=q[k];}
        }
    }
    counts
}
fn check_subtable(s:&[u8],n:usize)->[usize;7]{
    let mut counts=[0;7];
            counts[0]+=1;
            assert_eq!(u(s,0),1);
            let marks=coverage(s,u(s,2));let ligatures=coverage(s,u(s,4));
            let classes=u(s,6);let ma=u(s,8);let la=u(s,10);
            assert_eq!(marks.len(),u(s,ma));assert_eq!(ligatures.len(),u(s,la));
            let parsed=parse_mark_to_ligature(s,n as u32,Default::default()).unwrap();
            let mut mr=vec![None;n];
            for (k,&g) in marks.iter().enumerate(){
                let q=ma+2+4*k;let class=u(s,q);let off=u(s,q+2);
                assert!(class<classes);assert_ne!(off,0);
                mr[g as usize]=Some(GposMarkRecord{class:class as u16,anchor:anchor(s,ma+off).unwrap()});
                counts[1]+=1;
            }
            for (g,m) in mr.iter().enumerate(){assert_eq!(parsed.mark(g as u16),m.as_ref());}
            for (k,&g) in ligatures.iter().enumerate(){
                let rel=u(s,la+2+2*k);assert_ne!(rel,0);let at=la+rel;
                let components=u(s,at);assert!(components>0);
                assert_eq!(parsed.component_count(g),Some(components));
                counts[2]+=usize::from(components>1);
                for component in 0..components{
                    let row:Vec<_>=(0..classes).map(|cl|{
                        let off=u(s,at+2+2*(component*classes+cl));
                        let a=anchor(s,if off==0{0}else{at+off});
                        counts[3]+=usize::from(a.is_some());
                        assert_eq!(parsed.component_anchor(g,component,cl as u16),a.as_ref());
                        a
                    }).collect();
                    for &mark in &marks{
                        let m=mr[mark as usize].as_ref().unwrap();
                        let pair=row[m.class as usize].as_ref().map(|a|(a,&m.anchor));
                        assert_eq!(parsed.pair(g,component,mark),pair);
                        let origins=[(13,-7),(101,19)];
                        let got=parsed.attach_design(g,component,mark,origins).unwrap();
                        let Some((a,z))=pair else{
                            assert_eq!(got,None);
                            assert_eq!(parsed.attach_pixels(g,component,mark,origins,12,16).unwrap(),None);
                            continue;
                        };
                        counts[4]+=1;
                        let d=got.unwrap();
                        assert_eq!(origins[0].0+i32::from(a.x),origins[1].0+i32::from(z.x)+d.0);
                        assert_eq!(origins[0].1+i32::from(a.y),origins[1].1+i32::from(z.y)+d.1);
                        counts[5]+=usize::from((a.x,a.y)!=(z.x,z.y));
                        for (x,y) in [(9,12),(16,20),(32,48)]{
                            let got=parsed.attach_pixels(g,component,mark,origins,x,y);
                            if let Some((aa,zz))=px(a,x,y).zip(px(z,x,y)){
                                let d=got.unwrap().unwrap();
                                assert_eq!(origins[0].0+aa.0,origins[1].0+zz.0+d.0);
                                assert_eq!(origins[0].1+aa.1,origins[1].1+zz.1+d.1);
                                counts[6]+=1;
                            }else{assert!(matches!(got,Err(Error::Unsupported(_))));}
                        }
                    }
                    if classes<u16::MAX as usize {assert!(parsed.component_anchor(g,component,classes as u16).is_none());}
                }
                assert!(parsed.component_anchor(g,components,0).is_none());
            }
            if let Some(g)=(0..n).find(|&g|!ligatures.contains(&(g as u16))){
                assert_eq!(parsed.component_count(g as u16),None);
                if let Some(&m)=marks.first(){assert_eq!(parsed.attach_design(g as u16,0,m,[(0,0);2]).unwrap(),None);}
            }
            if let Some(g)=(0..n).find(|&g|mr[g].is_none()){
                if let Some(&a)=ligatures.first(){assert_eq!(parsed.attach_design(a,0,g as u16,[(0,0);2]).unwrap(),None);}
            }
    counts
}
fn dejavu_raw_subtable()->Vec<u8>{
    // Exact 370-byte GPOS slice from DejaVu Sans 2.37, lookup 5/subtable 0.
    // Provenance and license: fixtures/dejavu_gpos_excerpt.txt.
    let hex=include_str!("fixtures/dejavu_gpos_excerpt.hex").trim();
    assert_eq!(hex.len(),740);
    (0..hex.len()).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect()
}
#[test]fn independent_nonzero_device_attachment(){
    // Explicit synthetic differential: format 1 mark; format 3 component.
    let words:[u16;30]=[1,12,18,1,24,30,1,1,3,1,1,7,1,0,14,1,4,1,10,1,10,20,3,100,200,10,0,10,12,1];
    let mut b:Vec<u8>=words.iter().flat_map(|v|v.to_be_bytes()).collect();
    b.extend_from_slice(&0x6c00u16.to_be_bytes());
    let p=parse_mark_to_ligature(&b,10,Default::default()).unwrap();
    let a=anchor(&b,44).unwrap();let m=anchor(&b,38).unwrap();
    for (ppem,delta) in [(9,0),(10,1),(11,-2),(12,-1),(13,0)]{
        assert_eq!(px(&a,ppem,16),Some((delta,0)));
        assert_eq!(px(&m,ppem,16),Some((0,0)));
        assert_eq!(p.attach_pixels(7,0,3,[(2,1),(0,3)],ppem,16).unwrap(),Some((2+delta,-2)));
    }
}
#[test]fn native_mark_ligature_reference(){
    let mut total=[0;7];
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
        let n=check(font);for i in 0..7{total[i]+=n[i];}
    }
    assert!(total[..6].iter().all(|&n|n>0),"retained full-font native evidence: {total:?}");
    let extra=check_subtable(&dejavu_raw_subtable(),6253);
    assert!(extra.iter().all(|&n|n>0),"DejaVu raw native subtable evidence: {extra:?}");
    eprintln!("DejaVu raw subtable evidence: {extra:?}");
    for i in 0..7{total[i]+=extra[i];}
    assert!(total.iter().all(|&n|n>0),"native MarkToLigature positive evidence: {total:?}");
    eprintln!("native MarkToLigature [subtables, marks, multi-component ligatures, anchors, pairs, unequal pairs, pixel checks]: {total:?}");
}
