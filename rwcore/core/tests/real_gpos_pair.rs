//! Independent native PairPos reference: dense class maps, raw pair
//! records and per-bit Device decoding. No production parsers as oracle.
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
fn raw_value(b:&[u8],mut p:usize,f:usize,base:usize)->GposValueRecord{
    let mut a=[0i16;4];
    for (i,v) in a.iter_mut().enumerate(){if f&(1<<i)!=0{*v=u(b,p) as i16;p+=2;}}
    let mut d=[None,None,None,None];
    for (i,v) in d.iter_mut().enumerate(){if f&(16<<i)!=0{let rel=u(b,p);*v=device(b,if rel==0{0}else{base+rel});p+=2;}}
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

fn classes(b:&[u8],p:usize,n:usize)->Vec<u16>{
    let mut out=vec![0;n];
    match u(b,p){
        1=>{let start=u(b,p+2);for i in 0..u(b,p+4){out[start+i]=u(b,p+6+2*i) as u16;}},
        2=>{for i in 0..u(b,p+2){let q=p+4+6*i;for g in u(b,q)..=u(b,q+2){out[g]=u(b,q+4) as u16;}}},
        _=>panic!("class format"),
    }
    out
}
fn design(v:&GposValueRecord,p:GposAdjustment)->GposAdjustment{
    GposAdjustment{x_placement:p.x_placement+i32::from(v.x_placement),
        y_placement:p.y_placement+i32::from(v.y_placement),
        x_advance:p.x_advance+i32::from(v.x_advance),y_advance:p.y_advance+i32::from(v.y_advance)}
}
fn pixels(v:&GposValueRecord,x:u16,y:u16)->Option<GposAdjustment>{
    Some(GposAdjustment{x_placement:raw_pixel(&v.x_placement_device,x)?,
        y_placement:raw_pixel(&v.y_placement_device,y)?,x_advance:raw_pixel(&v.x_advance_device,x)?,
        y_advance:raw_pixel(&v.y_advance_device,y)?})
}
const PRIOR:[GposAdjustment;2]=[GposAdjustment{x_placement:13,y_placement:-7,x_advance:100,y_advance:-50},
    GposAdjustment{x_placement:-17,y_placement:11,x_advance:80,y_advance:20}];
fn check_apply(v:&[GposValueRecord;2],actual:Option<[GposAdjustment;2]>,get:impl Fn(u16,u16)->Result<Option<([GposAdjustment;2],[GposAdjustment;2])>,Error>)->bool{
    let want=[design(&v[0],PRIOR[0]),design(&v[1],PRIOR[1])];
    assert_eq!(actual,Some(want));
    for (x,y) in [(9,12),(16,20),(32,48)]{
        let expected=pixels(&v[0],x,y).zip(pixels(&v[1],x,y));
        let got=get(x,y);
        if let Some((a,b))=expected{assert_eq!(got.unwrap(),Some((want,[a,b])));}
        else{assert!(matches!(got,Err(Error::Unsupported(_))));}
    }
    want!=PRIOR
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
            if kind!=2{continue;}
            let s=&b[at..];let fmt=u(s,0);let f1=u(s,4);let f2=u(s,6);
            let a_size=(0..8).filter(|i|f1&(1<<i)!=0).count()*2;
            let b_size=(0..8).filter(|i|f2&(1<<i)!=0).count()*2;
            let cov=coverage(s,u(s,2));
            if fmt==1{
                counts[0]+=1;
                let parsed=parse_pair_positioning_format1(s,n as u32,Default::default()).unwrap();
                assert_eq!(parsed.len(),cov.len());assert_eq!(parsed.second_format(),f2 as u16);
                assert_eq!(u(s,8),cov.len());
                for (k,&a) in cov.iter().enumerate(){
                    let set=u(s,10+2*k);let count=u(s,set);
                    let mut expected=vec![None;n];
                    for q in 0..count{
                        let r=set+2+q*(2+a_size+b_size);let g=u(s,r);
                        expected[g]=Some([raw_value(s,r+2,f1,set),raw_value(s,r+2+a_size,f2,set)]);
                    }
                    for (g,v) in expected.iter().enumerate(){
                        let z=g as u16;
                        let got=parsed.pair(a,z);
                        assert_eq!(got.map(|x|[&x.first_value,&x.second_value]),v.as_ref().map(|x|[&x[0],&x[1]]));
                        if let Some(v)=v{
                            if check_apply(v,parsed.apply_design(a,z,PRIOR).unwrap(),|x,y|parsed.apply(a,z,PRIOR,[Default::default();2],x,y)){counts[2]+=1;}
                        }else{assert_eq!(parsed.apply_design(a,z,PRIOR).unwrap(),None);}
                    }
                }
                if let Some(a)=(0..n).map(|g|g as u16).find(|g|!cov.contains(g)){assert!(parsed.pair(a,0).is_none());}
            }else{
                assert_eq!(fmt,2);counts[1]+=1;
                let rows=u(s,12);let cols=u(s,14);
                let first=classes(s,u(s,8),n);let second=classes(s,u(s,10),n);
                let parsed=parse_pair_positioning_format2(s,n as u32,Default::default()).unwrap();
                assert_eq!(parsed.dimensions(),(rows,cols));assert_eq!(parsed.second_format(),f2 as u16);
                let mut representatives=vec![None;cols];
                for (g,&class) in second.iter().enumerate(){representatives[class as usize]=Some(g as u16);}
                // Check every covered first glyph against every second
                // glyph's class, then check all reachable class cells
                // using a representative of each second class.
                for &a in &cov{
                    let row=first[a as usize] as usize;
                    for (g,&class) in second.iter().enumerate(){
                        assert_eq!(parsed.classes(a,g as u16),Some((row as u16,class)));
                    }
                    for (col,z) in representatives.iter().enumerate(){
                        let Some(z)=*z else{continue;};
                        let r=16+(row*cols+col)*(a_size+b_size);
                        let v=[raw_value(s,r,f1,0),raw_value(s,r+a_size,f2,0)];
                        let got=parsed.pair(a,z).unwrap();
                        assert_eq!([&got.first_value,&got.second_value],[&v[0],&v[1]]);
                        if check_apply(&v,parsed.apply_design(a,z,PRIOR).unwrap(),|x,y|parsed.apply(a,z,PRIOR,[Default::default();2],x,y)){counts[3]+=1;}
                    }
                }
                if let Some(a)=(0..n).map(|g|g as u16).find(|g|!cov.contains(g)){assert!(parsed.pair(a,0).is_none());}
            }
        }
    }
    counts
}
#[test]fn native_pair_positioning_reference(){
    let mut total=[0;4];
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
        let counts=check(font);
        for i in 0..4{total[i]+=counts[i];}
    }
    assert!(total.iter().all(|&n|n>0),"native PairPos format and positive adjustment coverage: {total:?}");
}
