use rwcore::*;
const PIC:&str=r#"<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><pic:blipFill><a:srcRect l="10000" t="20000" r="30000" b="10000"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm rot="5400000" flipH="1" flipV="false"><a:off x="-10" y="20"/><a:ext cx="200" cy="100"/></a:xfrm></pic:spPr></pic:pic>"#;
fn parse(s:&str)->Result<PictureGeometry,Error>{parse_picture_geometry(s.as_bytes(),Limits::default())}
fn point(x:f64,y:f64)->GeometryPoint{GeometryPoint{x,y}}
fn base()->PictureGeometry{PictureGeometry{crop:PictureCrop::default(),transform:PictureTransform{x:10,y:20,width:200,height:100,rotation:0,flip_h:false,flip_v:false}}}
fn near(a:GeometryPoint,b:GeometryPoint){assert!((a.x-b.x).abs()<1e-9,"{:?} {:?}",a,b);assert!((a.y-b.y).abs()<1e-9);}
#[test]fn parses_geometry_and_crop(){
    let p=parse(PIC).unwrap();
    assert_eq!(p.crop,PictureCrop{left:10000,top:20000,right:30000,bottom:10000});
    assert_eq!(p.transform,PictureTransform{x:-10,y:20,width:200,height:100,rotation:5400000,flip_h:true,flip_v:false});
}
#[test]fn strict_and_defaults(){
    let s=PIC.replace("http://schemas.openxmlformats.org/drawingml/2006/","http://purl.oclc.org/ooxml/drawingml/")
        .replace("<a:srcRect l=\"10000\" t=\"20000\" r=\"30000\" b=\"10000\"/>","")
        .replace(" rot=\"5400000\" flipH=\"1\" flipV=\"false\"","");
    let p=parse(&s).unwrap();assert_eq!(p.crop,PictureCrop::default());assert_eq!(p.transform.rotation,0);
    assert!(!p.transform.flip_h&&!p.transform.flip_v);
    assert!(parse(&s.replace("http://purl.oclc.org/ooxml/drawingml/main","http://schemas.openxmlformats.org/drawingml/2006/main")).is_err());
}
#[test]fn crop_maps_edges_and_center(){
    let mut g=base();g.crop=PictureCrop{left:10000,top:20000,right:30000,bottom:10000};let m=g.mapping().unwrap();
    near(m.source_to_page(point(0.1,0.2)).unwrap(),point(10.0,20.0));
    near(m.source_to_page(point(0.7,0.9)).unwrap(),point(210.0,120.0));
    near(m.source_to_page(point(0.4,0.55)).unwrap(),point(110.0,70.0));
}
#[test]fn pixel_window_rounds_outward(){
    let c=PictureCrop{left:10000,top:20000,right:30000,bottom:10000};
    assert_eq!(c.pixel_window(13,11).unwrap(),(1,2,10,10));
    assert_eq!(PictureCrop::default().pixel_window(u32::MAX,u32::MAX).unwrap(),(0,0,u32::MAX,u32::MAX));
    assert!(c.pixel_window(0,10).is_err());
}
#[test]fn exact_quarter_turn_bounds(){
    let mut g=base();g.transform.rotation=5_400_000;let m=g.mapping().unwrap();
    assert_eq!(m.corners().unwrap(),[point(160.0,-30.0),point(160.0,170.0),point(60.0,170.0),point(60.0,-30.0)]);
    assert_eq!(m.bounds().unwrap(),GeometryBounds{min:point(60.0,-30.0),max:point(160.0,170.0)});
}
#[test]fn normalized_rotation(){
    let mut g=base();g.transform.rotation=-5_400_000;let a=g.mapping().unwrap().corners().unwrap();
    g.transform.rotation=16_200_000;assert_eq!(a,g.mapping().unwrap().corners().unwrap());
    g.transform.rotation=21_600_000;assert_eq!(g.mapping().unwrap().corners().unwrap(),base().mapping().unwrap().corners().unwrap());
}
#[test]fn flips_before_rotation(){
    let mut g=base();g.transform.flip_h=true;
    assert_eq!(g.mapping().unwrap().corners().unwrap()[0],point(210.0,20.0));
    g.transform.rotation=5_400_000;assert_eq!(g.mapping().unwrap().corners().unwrap()[0],point(160.0,170.0));
    g.transform.flip_v=true;assert_eq!(g.mapping().unwrap().corners().unwrap()[0],point(60.0,170.0));
}
#[test]fn inverse_round_trip_with_crop_and_flips(){
    let mut g=parse(PIC).unwrap();g.transform.rotation=2_123_456;
    for h in [false,true]{for v in [false,true]{
        g.transform.flip_h=h;g.transform.flip_v=v;let m=g.mapping().unwrap();
        for p in [point(0.1,0.2),point(0.7,0.9),point(0.4,0.55),point(-0.1,1.2)]{
            near(m.page_to_source(m.source_to_page(p).unwrap()).unwrap(),p);
        }
    }}
}
#[test]fn arbitrary_angle_bounds(){
    let mut g=base();g.transform.rotation=2_700_000;let b=g.mapping().unwrap().bounds().unwrap();
    let half=150.0_f64/2.0_f64.sqrt();
    near(b.min,point(110.0-half,70.0-half));near(b.max,point(110.0+half,70.0+half));
}
#[test]fn rejects_bad_crop_and_unsupported_fills(){
    for s in [PIC.replace("l=\"10000\"","l=\"-1\""),PIC.replace("r=\"30000\"","r=\"90000\""),
        PIC.replace("<a:fillRect/>","<a:fillRect l=\"1\"/>"),PIC.replace("<a:stretch>","<a:tile/><a:stretch>"),
        PIC.replace("<a:stretch>","<a:srcRect/><a:stretch>")]{assert!(parse(&s).is_err());}
}
#[test]fn rejects_bad_transform_and_public_values(){
    for s in [PIC.replace("cx=\"200\"","cx=\"0\""),PIC.replace("flipH=\"1\"","flipH=\"yes\""),
        PIC.replace("rot=\"5400000\"","rot=\"2147483648\""),PIC.replace("<a:off x=\"-10\" y=\"20\"/>","")]{
        assert!(parse(&s).is_err());
    }
    let mut g=base();g.transform.width=u64::MAX;assert!(g.mapping().is_err());
    g=base();g.crop.left=u32::MAX;assert!(g.mapping().is_err());
    let m=base().mapping().unwrap();assert!(m.source_to_page(point(f64::NAN,0.0)).is_err());
    assert!(m.page_to_source(point(0.0,f64::INFINITY)).is_err());
}
#[test]fn part_budget_and_namespace_root(){
    let mut l=Limits::default();l.max_part_bytes=PIC.len()-1;
    assert!(parse_picture_geometry(PIC.as_bytes(),l).is_err());
    assert!(parse("<pic xmlns=\"urn:other\"/>").is_err());
}
