//! Independent raw-byte native MarkToBase oracle; no production parsing helpers.
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

fn check(font:&[u8])->[usize;6]{
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let Some(b)=sf.table(*b"GPOS") else{return [0;6];};
    let n=u(sf.table(*b"maxp").unwrap(),4);
    let list=u(b,8);let mut counts=[0;6];
    for i in 0..u(b,list){
        let p=list+u(b,list+2+2*i);
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==9{assert_eq!(u(b,at),1);kind=u(b,at+2);at+=u(b,at+4)*65536+u(b,at+6);}
            if kind!=4{continue;}
            counts[0]+=1;
            let s=&b[at..];assert_eq!(u(s,0),1);
            let marks=coverage(s,u(s,2));let bases=coverage(s,u(s,4));
            let classes=u(s,6);let ma=u(s,8);let ba=u(s,10);
            assert_eq!(marks.len(),u(s,ma));assert_eq!(bases.len(),u(s,ba));
            let parsed=parse_mark_to_base(s,n as u32,Default::default()).unwrap();
            assert_eq!(parsed.dimensions(),(bases.len(),classes));
            let mut mr=vec![None;n];let mut br=vec![None;n];
            for (k,&g) in marks.iter().enumerate(){
                let q=ma+2+4*k;let class=u(s,q);
                let off=u(s,q+2);assert_ne!(off,0);
                let rec=GposMarkRecord{class:class as u16,anchor:anchor(s,ma+off).unwrap()};
                assert!(class<classes);mr[g as usize]=Some(rec);counts[1]+=1;
            }
            for (k,&g) in bases.iter().enumerate(){
                let row:Vec<_>=(0..classes).map(|cl|{
                    let off=u(s,ba+2+2*(k*classes+cl));
                    let a=anchor(s,if off==0{0}else{ba+off});
                    counts[2]+=usize::from(a.is_some());a
                }).collect();
                br[g as usize]=Some(row);
            }
            for g in 0..n{
                assert_eq!(parsed.mark(g as u16),mr[g].as_ref());
                for cl in 0..classes{
                    assert_eq!(parsed.base_anchor(g as u16,cl as u16),br[g].as_ref().and_then(|r|r[cl].as_ref()));
                }
            }
            // Compare all covered pairs, including null class cells.
            for &base in &bases{
                for &mark in &marks{
                    let m=mr[mark as usize].as_ref().unwrap();
                    let pair=br[base as usize].as_ref().unwrap()[m.class as usize].as_ref().map(|a|(a,&m.anchor));
                    assert_eq!(parsed.pair(base,mark),pair);
                    let origins=[(13,-7),(101,19)];
                    let got=parsed.attach_design(base,mark,origins).unwrap();
                    let Some((a,z))=pair else{
                        assert_eq!(got,None);
                        assert_eq!(parsed.attach_pixels(base,mark,origins,12,16).unwrap(),None);
                        continue;
                    };
                    counts[3]+=1;
                    // Independent world-coordinate equality after translation.
                    let delta=got.expect("covered attachment");
                    assert_eq!(origins[0].0+i32::from(a.x),origins[1].0+i32::from(z.x)+delta.0);
                    assert_eq!(origins[0].1+i32::from(a.y),origins[1].1+i32::from(z.y)+delta.1);
                    if (a.x,a.y)!=(z.x,z.y){counts[4]+=1;}
                    for (x,y) in [(9,12),(16,20),(32,48)]{
                        let got=parsed.attach_pixels(base,mark,origins,x,y);
                        if let Some((aa,zz))=px(a,x,y).zip(px(z,x,y)){
                            let d=got.unwrap().unwrap();
                            assert_eq!(origins[0].0+aa.0,origins[1].0+zz.0+d.0);
                            assert_eq!(origins[0].1+aa.1,origins[1].1+zz.1+d.1);
                            counts[5]+=1;
                        }else{assert!(matches!(got,Err(Error::Unsupported(_))));}
                    }
                }
            }
            if let Some(g)=(0..n).find(|&g|br[g].is_none()){
                if let Some(&m)=marks.first(){assert_eq!(parsed.attach_design(g as u16,m,[(0,0);2]).unwrap(),None);}
            }
            if let Some(g)=(0..n).find(|&g|mr[g].is_none()){
                if let Some(&a)=bases.first(){assert_eq!(parsed.attach_design(a,g as u16,[(0,0);2]).unwrap(),None);}
            }
        }
    }
    counts
}
#[test]fn native_mark_base_reference(){
    let mut total=[0;6];
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
        let n=check(font);for i in 0..6{total[i]+=n[i];}
    }
    assert!(total.iter().all(|&n|n>0),"native MarkToBase positive evidence: {total:?}");
    eprintln!("native MarkToBase [subtables, marks, anchors, pairs, unequal pairs, pixel checks]: {total:?}");
}
