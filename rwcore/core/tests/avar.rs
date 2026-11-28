use rwcore::*;
const ID:[(i16,i16);3]=[(-16384,-16384),(0,0),(16384,16384)];
const CURVE:[(i16,i16);5]=[(-16384,-16384),(-8192,-4096),(0,0),(8192,12288),(16384,16384)];
fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
fn set(b:&mut [u8],p:usize,v:u16){b[p..p+2].copy_from_slice(&v.to_be_bytes());}
fn fixture(maps:&[&[(i16,i16)]])->Vec<u8>{
    let mut b=Vec::new();for v in [1,0,0,maps.len() as u16]{w(&mut b,v);}
    for m in maps{w(&mut b,m.len() as u16);for &(x,y) in *m{w(&mut b,x as u16);w(&mut b,y as u16);}}
    b
}
fn parse(b:&[u8],n:usize)->Result<Avar,Error>{parse_avar(b,n,Default::default())}
#[test]fn avar_endpoints_segments_and_axis_order(){
    let a=parse(&fixture(&[&CURVE,&ID]),2).unwrap();
    assert_eq!(a.maps().len(),2);
    assert_eq!(a.maps()[0].pairs().len(),5);
    for (v,want) in [(-65536,-65536),(-49152,-40960),(-32768,-16384),(-16384,-8192),
        (0,0),(16384,24576),(32768,49152),(49152,57344),(65536,65536)]{
        assert_eq!(a.map_fixed(&[v,v]).unwrap(),vec![want,v]);
    }
}
#[test]fn avar_empty_and_missing_essential_anchors_are_identity(){
    for m in [&[][..],&[(-16384,-16384),(16384,16384)][..],&[(-16384,-16384),(0,1),(16384,16384)][..]]{
        let a=parse(&fixture(&[m]),1).unwrap();
        assert!(!a.maps()[0].is_active());
        for v in [-65536,-12345,0,12345,65536]{assert_eq!(a.map_fixed(&[v]).unwrap(),vec![v]);}
    }
}
#[test]fn avar_flat_segment_is_legal(){
    let pairs=[(-16384,-16384),(0,0),(8192,0),(16384,16384)];
    let a=parse(&fixture(&[&pairs]),1).unwrap();
    assert_eq!(a.map_fixed(&[16384]).unwrap(),vec![0]);
    assert_eq!(a.map_fixed(&[49152]).unwrap(),vec![32768]);
}
#[test]fn avar_reject_invalid_order_and_range(){
    for m in [
        vec![(-16384,-16384),(0,0),(0,0),(16384,16384)],
        vec![(-16384,-16384),(8192,8192),(0,0),(16384,16384)],
        vec![(-16384,-16384),(0,0),(8192,-1),(16384,16384)],
        vec![(-16385,-16384),(0,0),(16384,16384)],
        vec![(-16384,-16384),(0,0),(16384,16385)]
    ]{assert!(matches!(parse(&fixture(&[&m]),1),Err(Error::Invalid(_))));}
}
#[test]fn avar_invalid_header_versions_and_domain(){
    let b=fixture(&[&ID]);
    for (p,v) in [(0,2),(2,1)]{let mut x=b.clone();set(&mut x,p,v);assert!(matches!(parse(&x,1),Err(Error::Unsupported(_))));}
    for (p,v) in [(4,1),(6,2)]{let mut x=b.clone();set(&mut x,p,v);assert!(matches!(parse(&x,1),Err(Error::Invalid(_))));}
    assert!(parse(&b,2).is_err());
    assert!(parse(&fixture(&[]),0).is_err());
    let a=parse(&b,1).unwrap();
    for input in [vec![],vec![0,0],vec![-65537],vec![65537]]{assert!(a.map_fixed(&input).is_err());}
}
#[test]fn avar_all_truncated_prefixes_and_trailing_bytes(){
    let b=fixture(&[&CURVE,&ID]);
    for i in 0..b.len(){assert!(parse(&b[..i],2).is_err(),"prefix {i}");}
    let mut x=b.clone();x.push(0);assert!(parse(&x,2).is_err());
    let mut x=b;set(&mut x,8,65535);assert!(parse(&x,2).is_err());
}
#[test]fn avar_exact_cumulative_limits(){
    let b=fixture(&[&CURVE,&ID]);let l=AvarLimits{max_bytes:b.len(),max_axes:2,max_pairs:8};
    assert!(parse_avar(&b,2,l).is_ok());
    for bad in [AvarLimits{max_bytes:b.len()-1,..l},AvarLimits{max_axes:1,..l},AvarLimits{max_pairs:7,..l}]{
        assert!(matches!(parse_avar(&b,2,bad),Err(Error::Limit(_))));
    }
}
fn table<'a>(font:&'a [u8],tag:&[u8;4])->Option<&'a [u8]>{
    let u=|p|u16::from_be_bytes(font[p..p+2].try_into().unwrap()) as usize;
    let d=|p|u32::from_be_bytes(font[p..p+4].try_into().unwrap()) as usize;
    let r=(0..u(4)).map(|i|12+16*i).find(|&p|&font[p..p+4]==tag)?;
    Some(&font[d(r+8)..d(r+8)+d(r+12)])
}
// Independent linear scan and integer rational reference, preserving
// the specified two rounding stages. No production parser/search.
fn raw_reference(pairs:&[(i16,i16)],x:i32)->i32{
    for p in pairs.windows(2){
        let (lx,ly)=(i64::from(p[0].0)*4,i64::from(p[0].1)*4);
        let (hx,hy)=(i64::from(p[1].0)*4,i64::from(p[1].1)*4);
        let x=i64::from(x);
        if lx<=x&&x<=hx{
            let ratio=(2*(x-lx)*65536+(hx-lx))/(2*(hx-lx));
            return (ly+(2*ratio*(hy-ly)+65536)/131072) as i32;
        }
    }
    panic!("reference outside domain");
}
fn sweep(m:&AvarSegmentMap,pairs:&[(i16,i16)]){
    let mut previous=-65536;
    for x in -65536..=65536{
        let y=m.map_fixed(x).unwrap();
        assert_eq!(y,raw_reference(pairs,x),"coordinate {x}");
        assert!(y>=previous);previous=y;
        // Real-valued piecewise interpolation differs only by the
        // bounded fixed arithmetic rounding, at most two Fixed units.
        let p=pairs.windows(2).find(|p|i32::from(p[0].0)*4<=x&&x<=i32::from(p[1].0)*4).unwrap();
        let x0=f64::from(p[0].0)*4.0;let x1=f64::from(p[1].0)*4.0;
        let exact=f64::from(p[0].1)*4.0+(f64::from(x)-x0)/(x1-x0)*(f64::from(p[1].1)-f64::from(p[0].1))*4.0;
        assert!((f64::from(y)-exact).abs()<=2.0);
    }
}
#[test]fn avar_exhaustive_fixed_synthetic_reference(){
    let a=parse(&fixture(&[&CURVE]),1).unwrap();sweep(&a.maps()[0],&CURVE);
}
fn native(font:&[u8],expected:&[usize]){
    let f=parse_fvar(table(font,b"fvar").unwrap(),Default::default()).unwrap();
    let b=table(font,b"avar").unwrap();
    let a=parse(b,f.axes().len()).unwrap();
    let u=|p|u16::from_be_bytes(b[p..p+2].try_into().unwrap());
    assert_eq!(u(6) as usize,expected.len());
    let mut at=8;
    for (m,&n) in a.maps().iter().zip(expected){
        assert_eq!(u(at) as usize,n);at+=2;
        let pairs:Vec<_>=(0..n).map(|i|(u(at+4*i) as i16,u(at+4*i+2) as i16)).collect();
        at+=4*n;
        assert_eq!(m.pairs().iter().map(|p|(p.from,p.to)).collect::<Vec<_>>(),pairs);
        sweep(m,&pairs);
    }
    assert_eq!(at,b.len());
    let defaults:Vec<_>=f.axes().iter().map(|a|a.default).collect();
    assert_eq!(normalize_variation_coordinates(&f,Some(&a),&defaults).unwrap(),vec![0;expected.len()]);
    assert_eq!(normalize_tagged_variation_coordinates(&f,Some(&a),&[]).unwrap(),vec![0;expected.len()]);
    for instance in f.instances(){
        let fixed=f.normalize_fixed(&instance.coordinates).unwrap();
        let mapped=a.map_fixed(&fixed).unwrap();
        let expected:Vec<_>=mapped.into_iter().map(|v|normalized_fixed_to_f2dot14(v).unwrap()).collect();
        assert_eq!(normalize_variation_coordinates(&f,Some(&a),&instance.coordinates).unwrap(),expected);
    }
}
#[test]fn avar_native_archivo_full_fixed_domain(){
    native(include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf"),&[9,3]);
}
#[test]fn avar_native_devanagari_full_fixed_domain(){
    native(include_bytes!("fixtures/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf"),&[9,5]);
}
#[test]fn avar_absent_and_mismatched_axis_count(){
    let font=include_bytes!("fixtures/notonastaliqurdu/NotoNastaliqUrdu[wght].ttf");
    assert!(table(font,b"avar").is_none());
    let f=parse_fvar(table(font,b"fvar").unwrap(),Default::default()).unwrap();
    assert_eq!(f.axes().len(),1);
    for user in [f.axes()[0].min,f.axes()[0].default,f.axes()[0].max]{
        assert_eq!(normalize_variation_coordinates(&f,None,&[user]).unwrap(),f.normalize_f2dot14(&[user]).unwrap());
    }
    let wrong=parse(&fixture(&[&ID,&ID]),2).unwrap();
    assert!(normalize_variation_coordinates(&f,Some(&wrong),&[f.axes()[0].default]).is_err());
}
#[test]fn avar_archivo_native_feature_boundary_anchor(){
    let font=include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf");
    let f=parse_fvar(table(font,b"fvar").unwrap(),Default::default()).unwrap();
    let a=parse(table(font,b"avar").unwrap(),2).unwrap();
    let weight=f.axis(*b"wght").unwrap();
    // Native avar has an exact knot -3277 -> -3985 in F2DOT14.
    assert_eq!(a.maps()[0].map_fixed(-3277*4).unwrap(),-3985*4);
    assert_eq!(normalized_fixed_to_f2dot14(a.maps()[0].map_fixed(-3277*4).unwrap()).unwrap(),-3985);
    let user=[weight.default,f.axes()[1].default];
    assert_eq!(normalize_variation_coordinates(&f,Some(&a),&user).unwrap(),vec![0,0]);
    // Do not quantize default normalization before the nonlinear map.
    let x=10923*4+1;
    let y=a.maps()[0].map_fixed(x).unwrap();
    assert_eq!(y,raw_reference(&a.maps()[0].pairs().iter().map(|p|(p.from,p.to)).collect::<Vec<_>>(),x));
}
