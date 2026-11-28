//! Independent raw native GPOS SinglePos oracle. No production coverage,
 //! value or device parser is used to construct the expected results.
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
fn raw_value(b:&[u8],mut p:usize,f:usize)->GposValueRecord{
    let mut a=[0i16;4];
    for (i,v) in a.iter_mut().enumerate(){if f&(1<<i)!=0{*v=u(b,p) as i16;p+=2;}}
    let mut d=[None,None,None,None];
    for (i,v) in d.iter_mut().enumerate(){if f&(16<<i)!=0{*v=device(b,u(b,p));p+=2;}}
    let [x,y,ax,ay]=d;
    GposValueRecord{x_placement:a[0],y_placement:a[1],x_advance:a[2],y_advance:a[3],
        x_placement_device:x,y_placement_device:y,x_advance_device:ax,y_advance_device:ay}
}
fn raw_pixel(d:&Option<GposDevice>,ppem:u16)->Option<i32>{
    match d{
        None=>Some(0),
        Some(GposDevice::VariationIndex{..})=>None,
        Some(GposDevice::Pixels{start,end,deltas})=>Some(if ppem<*start||ppem>*end{0}else{i32::from(deltas[(ppem-*start) as usize])}),
    }
}
fn check(font:&[u8])->(usize,usize,usize){
    let c=parse_font(font,Default::default()).unwrap();let sf=c.face(0).unwrap();
    let Some(b)=sf.table(*b"GPOS") else{return (0,0,0);};
    let glyph_count=u(sf.table(*b"maxp").unwrap(),4) as u32;
    let base=u(b,8);let mut counts=(0,0,0);
    for i in 0..u(b,base){
        let p=base+u(b,base+2+2*i);
        for j in 0..u(b,p+4){
            let mut at=p+u(b,p+6+2*j);let mut kind=u(b,p);
            if kind==9{
                assert_eq!(u(b,at),1);kind=u(b,at+2);
                at+=u(b,at+4)*65536+u(b,at+6);
            }
            if kind!=1{continue;}
            let s=&b[at..];let fmt=u(s,0);let f=u(s,4);
            let cov=coverage(s,u(s,2));let stride=(0..8).filter(|i|f&(1<<i)!=0).count()*2;
            let parsed=parse_single_positioning(s,glyph_count,Default::default()).unwrap();
            assert_eq!(parsed.len() as usize,cov.len());
            assert_eq!(parsed.is_shared(),fmt==1);
            match fmt{1=>counts.0+=1,2=>{counts.1+=1;assert_eq!(u(s,6),cov.len());},_=>panic!("bad SinglePos format")}
            for g in 0..glyph_count{
                let expected=cov.iter().position(|&id|u32::from(id)==g)
                    .map(|k|raw_value(s,if fmt==1{6}else{8+k*stride},f));
                assert_eq!(parsed.value(g as u16),expected.as_ref());
                let Some(v)=expected else{
                    assert_eq!(parsed.apply_design(g as u16,Default::default()).unwrap(),None);continue;
                };
                let a=GposAdjustment{x_placement:i32::from(v.x_placement)+13,y_placement:i32::from(v.y_placement)-7,
                    x_advance:i32::from(v.x_advance)+100,y_advance:i32::from(v.y_advance)-50};
                let prior=GposAdjustment{x_placement:13,y_placement:-7,x_advance:100,y_advance:-50};
                assert_eq!(parsed.apply_design(g as u16,prior).unwrap(),Some(a));
                if v.x_placement!=0||v.y_placement!=0||v.x_advance!=0||v.y_advance!=0{counts.2+=1;}
                for (x,y) in [(9,12),(16,20),(32,48)]{
                    let pixels=(||Some(GposAdjustment{x_placement:raw_pixel(&v.x_placement_device,x)?,
                        y_placement:raw_pixel(&v.y_placement_device,y)?,x_advance:raw_pixel(&v.x_advance_device,x)?,
                        y_advance:raw_pixel(&v.y_advance_device,y)?}))();
                    let got=parsed.apply(g as u16,prior,Default::default(),x,y);
                    if let Some(px)=pixels{assert_eq!(got.unwrap(),Some((a,px)));}else{assert!(matches!(got,Err(Error::Unsupported(_))));}
                }
            }
        }
    }
    counts
}
#[test]fn native_single_positioning_reference(){
    let mut total=(0,0,0);
    for font in [
        include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf").as_slice(),
        include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf").as_slice(),
        include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf").as_slice(),
        include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf").as_slice(),
        include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf").as_slice(),
        include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf").as_slice(),
        include_bytes!("fixtures/padauk/Padauk-Regular.ttf").as_slice(),
    ]{
        let n=check(font);total.0+=n.0;total.1+=n.1;total.2+=n.2;
    }
    assert!(total.0>0&&total.1>0&&total.2>0,"native format and positive adjustment coverage: {total:?}");
}
