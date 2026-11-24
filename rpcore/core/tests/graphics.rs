use rpcore::{Document,Error,graphics::{Color,Command,FillRule,LineCap,LineJoin,Segment,Warning}};
fn stream(dict:&str,content:&[u8])->Vec<u8>{
    let mut b=format!("<< {dict} /Length {} >>\nstream\n",content.len()).into_bytes();
    b.extend_from_slice(content);b.extend_from_slice(b"\nendstream");b
}
fn fixture(content:&[u8],extra:Vec<Vec<u8>>)->Vec<u8>{
    let mut objs=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /XObject << /Fm 5 0 R /Im 6 0 R >> /ColorSpace << /C /DeviceRGB /ICC [/ICCBased 5 0 R] /Cal [/CalRGB << /WhitePoint [1 1 1] >>] >> /ExtGState << /GS << /LW 2 /LC 1 /LJ 2 /ML 4 /D [[3 2] 1] /OP false /op false /OPM 1 /TR2 /Default >> /Bad << /OP true >> >> >> >>".to_vec(),
        stream("",content)];
    objs.extend(extra);
    let mut p=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for (i,o) in objs.iter().enumerate(){offsets.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend_from_slice(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objs.len()+1).as_bytes());
    for off in offsets{p.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objs.len()+1).as_bytes());p
}
fn list(content:&[u8],extra:Vec<Vec<u8>>)->rpcore::graphics::DisplayList{
    let b=fixture(content,extra);let d=Document::open(&b).unwrap();d.page_display_list(&d.pages().unwrap()[0],10000).unwrap()
}
#[test]
fn transformed_path_and_stroke(){
    let l=list(b"q 2 0 0 3 10 20 cm 1 0 0 rg 0 1 0 0 K 2 w 1 J 2 j 5 M [3 2] 1 d 1 2 m 3 4 l 5 6 7 8 9 10 c b* Q",vec![]);
    assert_eq!(l.commands.len(),3);assert_eq!(l.commands[0],Command::Save);
    let Command::Paint{path,fill,stroke}= &l.commands[1] else{panic!("paint")};
    assert_eq!(path,&vec![Segment::Move([12.,26.]),Segment::Line([16.,32.]),Segment::Cubic([20.,38.],[24.,44.],[28.,50.]),Segment::Close]);
    assert_eq!(*fill,Some((Color::Rgb([1.,0.,0.]),FillRule::EvenOdd)));
    let s=stroke.as_ref().unwrap();assert_eq!(s.color,Color::Cmyk([0.,1.,0.,0.]));assert_eq!(s.width,2.);assert_eq!(s.cap,LineCap::Round);assert_eq!(s.join,LineJoin::Bevel);assert_eq!(s.dash,vec![3.,2.]);assert_eq!(s.ctm.0,[2.,0.,0.,3.,10.,20.]);
    assert_eq!(l.commands[2],Command::Restore);
}
#[test]
fn clipping_order_and_curve_shorthands(){
    let l=list(b"q 0 0 20 10 re W* B 0 0 m 2 3 4 5 v 6 7 8 9 y S Q",vec![]);
    assert!(matches!(&l.commands[1],Command::Paint{..}));
    assert!(matches!(&l.commands[2],Command::Clip{rule:FillRule::EvenOdd,..}));
    let Command::Paint{path,..}=&l.commands[3] else{panic!("paint")};
    assert_eq!(path,&vec![Segment::Move([0.,0.]),Segment::Cubic([0.,0.],[2.,3.],[4.,5.]),Segment::Cubic([6.,7.],[8.,9.],[8.,9.])]);
}
#[test]
fn nested_form_and_image_order(){
    let f=stream("/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Matrix [1 0 0 1 3 4]",b"0.5 g 0 0 10 10 re f /Im Do");
    let im=stream("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",&[0]);
    let l=list(b"2 0 0 2 10 20 cm /Fm Do 0 0 1 1 re f",vec![f,im]);
    assert_eq!(l.commands.len(),6);
    let Command::Clip{path,..}=&l.commands[1] else{panic!("clip")};assert_eq!(path[0],Segment::Move([16.,28.]));
    assert!(matches!(&l.commands[3],Command::Image{object_id:(6,0),..}));
    assert_eq!(l.commands[4],Command::Restore);
    let Command::Paint{fill,path,..}=&l.commands[5] else{panic!("paint")};assert_eq!(*fill,Some((Color::Gray(0.),FillRule::NonZero)));assert_eq!(path[0],Segment::Move([10.,20.]));
    assert_eq!(l.warnings,vec![Warning::ImageRenderingDeferred]);
}
#[test]
fn color_state_and_deferred_text(){
    let l=list(b"/DeviceRGB cs 0.1 0.2 0.3 sc q 0.7 g 0 0 2 2 re f Q BT /F1 12 Tf (Hi) Tj ET 0 0 1 1 re f",vec![]);
    let Command::Paint{fill,..}=l.commands.last().unwrap() else{panic!("paint")};assert_eq!(*fill,Some((Color::Rgb([0.1,0.2,0.3]),FillRule::NonZero)));
    assert!(l.warnings.contains(&Warning::TextRenderingDeferred));assert!(l.commands.iter().any(|c|matches!(c,Command::Text{operation,..} if operation.operator==b"Tj")));
}
#[test]
fn invalid_and_cycle(){
    for data in [b"Q".as_slice(),b"1 2 l S",b"q",b"[0 0] 0 d",b"/GS1 gs",b"BT 4 Tr ET",b"0 0 5 5 re W"]{
        let b=fixture(data,vec![]);let d=Document::open(&b).unwrap();assert!(d.page_display_list(&d.pages().unwrap()[0],1000).is_err());
    }
    let f=stream("/Subtype /Form /BBox [0 0 1 1]",b"/Fm Do");
    let b=fixture(b"/Fm Do",vec![f]);let d=Document::open(&b).unwrap();assert!(matches!(d.page_display_list(&d.pages().unwrap()[0],1000),Err(Error::Invalid(_))));
}
#[test]
fn cumulative_form_budget(){
    let f=stream("/Subtype /Form /BBox [0 0 1 1]",b"0 0 1 1 re f");
    let b=fixture(b"/Fm Do /Fm Do",vec![f]);let d=Document::open(&b).unwrap();let p=d.pages().unwrap();
    assert!(matches!(d.page_display_list(&p[0],30),Err(Error::Limit(_))));
    assert!(d.page_display_list(&p[0],100).is_ok());
}

#[test]fn named_device_space_and_extgstate_stroke(){
    let l=list(b"/C cs 1 0 0 sc /GS gs 0 0 10 10 re B",vec![]);
    let Command::Paint{fill,stroke,..}=l.commands.last().unwrap() else{panic!("paint")};
    assert_eq!(*fill,Some((Color::Rgb([1.,0.,0.]),FillRule::NonZero)));
    let s=stroke.as_ref().unwrap();assert_eq!(s.width,2.);assert_eq!(s.cap,LineCap::Round);assert_eq!(s.join,LineJoin::Bevel);assert_eq!(s.miter_limit,4.);assert_eq!(s.dash,vec![3.,2.]);
    let b=fixture(b"/Bad gs",vec![]);let d=Document::open(&b).unwrap();
    assert!(d.page_display_list(&d.pages().unwrap()[0],10000).is_err());
}

#[test]fn icc_device_alternate_and_diagnostics(){
    let profile=stream("/N 3 /Alternate /DeviceRGB /Range [0 1 0 1 0 1]",b"uninterpreted profile fixture");
    let l=list(b"/ICC cs 0.1 0.2 0.3 sc 0 0 10 10 re f",vec![profile]);
    assert!(l.warnings.contains(&Warning::IccAlternateUsed));
    let Command::Paint{fill,..}=l.commands.last().unwrap() else{panic!("paint")};
    assert_eq!(*fill,Some((Color::Rgb([0.1,0.2,0.3]),FillRule::NonZero)));
    let p=fixture(b"/Cal cs",vec![]);let d=Document::open(&p).unwrap();
    assert!(matches!(d.page_display_list(&d.pages().unwrap()[0],10000),Err(Error::Unsupported("graphics CalRGB color space"))));
    let profile=stream("/N 3 /Alternate /DeviceGray",b"bad");
    let p=fixture(b"/ICC cs",vec![profile]);let d=Document::open(&p).unwrap();
    assert!(d.page_display_list(&d.pages().unwrap()[0],10000).is_err());
}
