use rwcore::*;
fn w(b:&mut Vec<u8>,v:u16){b.extend(v.to_be_bytes());}
fn f(b:&mut Vec<u8>,v:i32){b.extend(v.to_be_bytes());}
fn sw(b:&mut [u8],p:usize,v:u16){b[p..p+2].copy_from_slice(&v.to_be_bytes());}
fn sf(b:&mut [u8],p:usize,v:i32){b[p..p+4].copy_from_slice(&v.to_be_bytes());}
fn axis(tag:[u8;4],min:i32,default:i32,max:i32)->FvarAxis{FvarAxis{tag,min,default,max,flags:0,name_id:256}}
fn fixture(axes:&[FvarAxis],instances:&[(u16,Vec<i32>,u16)],ps:bool,pad:u16)->Vec<u8>{
    let mut b=Vec::new();
    for v in [1,0,16+pad,2,axes.len() as u16,20,instances.len() as u16,
        4+4*axes.len() as u16+if ps{2}else{0}]{w(&mut b,v);}
    b.resize(16+pad as usize,0);
    for a in axes{b.extend(a.tag);for v in [a.min,a.default,a.max]{f(&mut b,v);}w(&mut b,a.flags);w(&mut b,a.name_id);}
    for (id,coords,name) in instances{w(&mut b,*id);w(&mut b,0);for &v in coords{f(&mut b,v);}if ps{w(&mut b,*name);}}
    b
}
fn parse(b:&[u8])->Result<Fvar,Error>{parse_fvar(b,Default::default())}
fn axes()->[FvarAxis;2]{[axis(*b"wght",100*65536,400*65536,900*65536),
    FvarAxis{flags:1,name_id:257,..axis(*b"wdth",50*65536,100*65536,200*65536)}]}
#[test]fn fvar_axis_order_and_named_instances(){
    let a=axes();let b=fixture(&a,&[(2,vec![400*65536,100*65536],6),
        (258,vec![900*65536,50*65536],65535)],true,4);
    let p=parse(&b).unwrap();assert_eq!(p.axes(),&a);assert!(p.axis(*b"wdth").unwrap().is_hidden());
    assert_eq!(p.instances()[0].postscript_name_id,Some(6));assert_eq!(p.instances()[1].postscript_name_id,None);
    assert_eq!(p.instances()[1].coordinates,vec![900*65536,50*65536]);
    let p=parse(&fixture(&a,&[(258,vec![100*65536,200*65536],0)],false,0)).unwrap();
    assert_eq!(p.instances()[0].postscript_name_id,None);
}
#[test]fn fvar_asymmetric_ranges_clamping_and_default(){
    let p=parse(&fixture(&axes(),&[],false,0)).unwrap();
    for (v,want) in [([100,50],[-16384,-16384]),([400,100],[0,0]),([900,200],[16384,16384]),
        ([250,75],[-8192,-8192]),([650,150],[8192,8192]),([0,0],[-16384,-16384]),([1000,300],[16384,16384])]{
        assert_eq!(p.normalize_f2dot14(&v.map(|x|x*65536)).unwrap(),want);
    }
}
#[test]fn fvar_tagged_defaults_and_order(){
    let p=parse(&fixture(&axes(),&[],false,0)).unwrap();
    assert_eq!(p.normalize_tagged_fixed(&[]).unwrap(),vec![0,0]);
    assert_eq!(p.normalize_tagged_fixed(&[(*b"wdth",150*65536)]).unwrap(),vec![0,32768]);
    assert_eq!(p.normalize_tagged_fixed(&[(*b"wdth",50*65536),(*b"wght",900*65536)]).unwrap(),vec![65536,-65536]);
    for bad in [vec![(*b"zzzz",0)],vec![(*b"wght",0),(*b"wght",1)]]{
        assert!(matches!(p.normalize_tagged_fixed(&bad),Err(Error::Invalid(_))));
    }
    assert!(p.normalize_fixed(&[0]).is_err());
}
#[test]fn fvar_collapsed_halves_and_full_signed_domain(){
    for (a,v,want) in [(axis(*b"test",0,0,10),-1,0),(axis(*b"test",0,10,10),11,0),
        (axis(*b"test",1,1,1),i32::MAX,0),
        (axis(*b"test",i32::MIN,0,i32::MAX),i32::MIN,-65536),
        (axis(*b"test",i32::MIN,i32::MIN,i32::MAX),i32::MAX,65536),
        (axis(*b"test",i32::MIN,i32::MAX,i32::MAX),i32::MIN,-65536)]{
        assert_eq!(a.normalize_fixed(v).unwrap(),want);
    }
    assert!(axis(*b"test",1,0,2).normalize_fixed(1).is_err());
}
#[test]fn fvar_two_stage_quantization_signed_ties(){
    let a=axis(*b"test",-131072,0,131072);
    assert_eq!(a.normalize_fixed(1).unwrap(),1);
    assert_eq!(a.normalize_fixed(-1).unwrap(),-1);
    for (v,want) in [(-65536,-16384),(-6,-1),(-2,0),(-3,-1),(-1,0),(0,0),(1,0),(2,1),(6,2),(65536,16384)]{
        assert_eq!(normalized_fixed_to_f2dot14(v).unwrap(),want);
    }
    assert!(normalized_fixed_to_f2dot14(65537).is_err());
    assert!(normalized_fixed_to_f2dot14(i32::MIN).is_err());
}
#[test]fn fvar_exhaustive_integer_normalization_reference(){
    let a=axis(*b"test",-97,13,211);
    let mut prev=-65536;
    for v in -120..=240{
        let x=v.clamp(-97,211);
        let real=if x<13{(x-13) as f64/110.0}else{(x-13) as f64/198.0};
        let expected=(real*65536.0).round() as i32;
        let got=a.normalize_fixed(v).unwrap();assert_eq!(got,expected);assert!(got>=prev);prev=got;
        assert_eq!(normalized_fixed_to_f2dot14(got).unwrap(),((expected as f64/4.0+0.5).floor()) as i16);
    }
}
#[test]fn fvar_every_truncated_prefix(){
    let b=fixture(&axes(),&[(258,vec![100*65536,200*65536],259)],true,0);
    for i in 0..b.len(){assert!(parse(&b[..i]).is_err(),"prefix {i}");}
    assert!(parse(&b).is_ok());
}
#[test]fn fvar_header_and_size_validation(){
    let b=fixture(&axes(),&[],false,0);
    for (p,v) in [(4,14),(6,3),(10,19),(10,21),(14,11),(14,16)]{
        let mut x=b.clone();sw(&mut x,p,v);assert!(parse(&x).is_err());
    }
    let mut x=b.clone();sw(&mut x,2,1);assert!(matches!(parse(&x),Err(Error::Unsupported(_))));
    assert!(matches!(parse(&fixture(&[],&[],false,0)),Err(Error::Unsupported(_))));
}
#[test]fn fvar_invalid_axes(){
    let b=fixture(&axes(),&[],false,0);
    let mut x=b.clone();x[36..40].copy_from_slice(b"wght");assert!(parse(&x).is_err());
    let mut x=b.clone();x[16]=0;assert!(parse(&x).is_err());
    let mut x=b.clone();sf(&mut x,20,500*65536);assert!(parse(&x).is_err());
    let mut x=b.clone();sw(&mut x,32,2);assert!(parse(&x).is_err());
    let mut x=b.clone();sw(&mut x,34,255);assert!(parse(&x).is_err());
}
#[test]fn fvar_invalid_named_instances(){
    let b=fixture(&axes(),&[(258,vec![100*65536,200*65536],259)],true,0);
    let mut x=b.clone();sw(&mut x,56,1);assert!(parse(&x).is_err());
    let mut x=b.clone();sw(&mut x,58,1);assert!(parse(&x).is_err());
    let mut x=b.clone();sf(&mut x,60,99*65536);assert!(parse(&x).is_err());
    let mut x=b.clone();sw(&mut x,68,5);assert!(parse(&x).is_err());
}
#[test]fn fvar_exact_limits(){
    let b=fixture(&axes(),&[(258,vec![100*65536,200*65536],259)],true,0);
    let l=FvarLimits{max_bytes:b.len(),max_axes:2,max_instances:1,max_instance_coordinates:2};
    assert!(parse_fvar(&b,l).is_ok());
    for bad in [FvarLimits{max_bytes:b.len()-1,..l},FvarLimits{max_axes:1,..l},
        FvarLimits{max_instances:0,..l},FvarLimits{max_instance_coordinates:1,..l}]{
        assert!(matches!(parse_fvar(&b,bad),Err(Error::Limit(_))));
    }
}
#[test]fn fvar_native_archivo_axis_reference(){
    let font=include_bytes!("fixtures/archivo/Archivo[wdth,wght].ttf");
    let u=|p:usize|u16::from_be_bytes(font[p..p+2].try_into().unwrap()) as usize;
    let d=|p:usize|u32::from_be_bytes(font[p..p+4].try_into().unwrap()) as usize;
    let rec=(0..u(4)).map(|i|12+16*i).find(|&p|&font[p..p+4]==b"fvar").unwrap();
    let b=&font[d(rec+8)..d(rec+8)+d(rec+12)];
    let p=parse(b).unwrap();assert_eq!(p.axes().len(),2);
    assert_eq!(p.axes()[0].tag,*b"wght");assert_eq!(p.axes()[1].tag,*b"wdth");
    let at=u16::from_be_bytes(b[4..6].try_into().unwrap()) as usize;
    for (i,a) in p.axes().iter().enumerate(){
        let s=at+20*i;
        let fixed=|p:usize|i32::from_be_bytes(b[p..p+4].try_into().unwrap());
        assert_eq!((a.min,a.default,a.max),(fixed(s+4),fixed(s+8),fixed(s+12)));
        assert_eq!(a.normalize_fixed(a.default).unwrap(),0);
        if a.min<a.default{assert_eq!(a.normalize_fixed(a.min).unwrap(),-65536);}
        if a.default<a.max{assert_eq!(a.normalize_fixed(a.max).unwrap(),65536);}
    }
    assert_eq!(p.instances().len(),u16::from_be_bytes(b[12..14].try_into().unwrap()) as usize);
    for i in p.instances(){assert_eq!(p.normalize_fixed(&i.coordinates).unwrap().len(),2);}
}
