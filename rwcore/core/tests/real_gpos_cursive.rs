//! Independent native CursivePos anchor and connection oracle.
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
fn expected(a:(i32,i32),b:(i32,i32),origins:[(i32,i32);2],fixed:CursiveFixedEnd)->CursiveConnection{
    let delta=(origins[0].0-origins[1].0+a.0-b.0,origins[0].1-origins[1].1+a.1-b.1);
    match fixed{
        CursiveFixedEnd::Exit=>CursiveConnection{exit_shift:(0,0),entry_shift:delta},
        CursiveFixedEnd::Entry=>CursiveConnection{exit_shift:(-delta.0,-delta.1),entry_shift:(0,0)},
    }
}
fn check(font:&[u8])->[usize;4]{
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let Some(b)=sf.table(*b"GPOS") else{return [0;4];};
    let n=u(sf.table(*b"maxp").unwrap(),4);
    let base=u(b,8);let mut counts=[0;4];
    for i in 0..u(b,base){
        let p=base+u(b,base+2+2*i);
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==9{assert_eq!(u(b,at),1);kind=u(b,at+2);at+=u(b,at+4)*65536+u(b,at+6);}
            if kind!=3{continue;}
            counts[0]+=1;
            let s=&b[at..];assert_eq!(u(s,0),1);
            let cov=coverage(s,u(s,2));assert_eq!(cov.len(),u(s,4));
            let parsed=parse_cursive_positioning(s,n as u32,Default::default()).unwrap();
            assert_eq!(parsed.len(),cov.len());
            let mut records=vec![None;n];
            for (k,&g) in cov.iter().enumerate(){
                let entry=anchor(s,u(s,6+4*k));let exit=anchor(s,u(s,8+4*k));
                counts[1]+=usize::from(entry.is_some())+usize::from(exit.is_some());
                records[g as usize]=Some(CursiveAnchors{entry,exit});
            }
            for (g,r) in records.iter().enumerate(){assert_eq!(parsed.anchors(g as u16),r.as_ref());}
            // Cross-product of covered glyphs, including absent anchors.
            for &a in &cov{
                for &z in &cov{
                    let pair=records[a as usize].as_ref().unwrap().exit.as_ref()
                        .zip(records[z as usize].as_ref().unwrap().entry.as_ref());
                    assert_eq!(parsed.pair(a,z),pair);
                    let origins=[(13,-7),(101,19)];
                    for fixed in [CursiveFixedEnd::Exit,CursiveFixedEnd::Entry]{
                        let got=parsed.connect_design(a,z,origins,fixed).unwrap();
                        let Some((ex,en))=pair else{assert_eq!(got,None);continue;};
                        let e=expected((ex.x as i32,ex.y as i32),(en.x as i32,en.y as i32),origins,fixed);
                        assert_eq!(got,Some(e));
                        if (ex.x,ex.y)!=(en.x,en.y){counts[2]+=1;}
                        for (x,y) in [(9,12),(16,20),(32,48)]{
                            let got=parsed.connect_pixels(a,z,[(2,-1),(0,3)],fixed,x,y);
                            if let Some((aa,bb))=px(ex,x,y).zip(px(en,x,y)){
                                assert_eq!(got.unwrap(),Some(expected(aa,bb,[(2,-1),(0,3)],fixed)));
                                counts[3]+=1;
                            }else{assert!(matches!(got,Err(Error::Unsupported(_))));}
                        }
                    }
                }
            }
            if let Some(g)=(0..n).find(|&g|records[g].is_none()){
                assert!(parsed.pair(g as u16,0).is_none());
                assert!(parsed.pair(0,g as u16).is_none());
            }
        }
    }
    counts
}
#[test]fn native_cursive_positioning_reference(){
    let mut total=[0;4];
    for font in [
        include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf").as_slice(),
        include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf").as_slice(),
        include_bytes!("fixtures/notosansmongolian/NotoSansMongolian-Regular.ttf").as_slice(),
        include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/padauk/Padauk-Regular.ttf").as_slice(),
    ]{
        let n=check(font);for i in 0..4{total[i]+=n[i];}
    }
    assert!(total[..3].iter().all(|&n|n>0),"native cursive and distinct anchor evidence: {total:?}");
}
