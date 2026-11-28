#[path="support/group_fixture.rs"] mod fixture;
use rpcore::{Document,Error,graphics::{BlendMode,Command},raster::{Bitmap,Options}};
fn render(p:&[u8])->Bitmap{let d=Document::open(p).unwrap();d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap()}
fn px(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
fn near(a:&[u8],b:[u8;4]){for (&x,y) in a.iter().zip(b){assert!((i32::from(x)-i32::from(y)).abs()<=1,"{a:?} != {b:?}");}}
#[test]fn group_opacity_applies_once(){
    let p=fixture::pdf(b"q /Outer gs /G Do Q 0 g 15 0 5 10 re f",fixture::GROUP,fixture::OVERLAP,b"", "<< /ca 0.5 >>","<< >>");
    let d=Document::open(&p).unwrap();let list=d.page_display_list(&d.pages().unwrap()[0],10000).unwrap();
    assert!(list.commands.iter().any(|c|matches!(c,Command::BeginGroup{opacity,blend:BlendMode::Normal} if *opacity==0.5)));
    let b=render(&p);
    near(px(&b,2,15),[255,128,128,255]);near(px(&b,7,15),[128,128,255,255]);near(px(&b,12,15),[128,128,255,255]);
    assert_eq!(px(&b,17,15),[0,0,0,255]);assert_eq!(px(&b,2,2),[255,255,255,255]);
}
#[test]fn partially_transparent_group_and_nested(){
    let p=fixture::pdf(b"/Outer gs /G Do",fixture::GROUP,b"/Inner gs /N Do",b"1 0 0 rg 0 0 10 10 re f","<< /ca 0.5 >>","<< /ca 0.5 >>");
    let b=render(&p);near(px(&b,5,15),[255,191,191,255]);
    let p=fixture::pdf(b"/Outer gs /G Do",fixture::GROUP,b"/Inner gs 1 0 0 rg 0 0 10 10 re f 0 0 1 rg 5 0 10 10 re f",b"","<< /ca 0.5 >>","<< /ca 0.5 >>");
    let b=render(&p);near(px(&b,7,15),[191,159,223,255]);near(px(&b,2,15),[255,191,191,255]);
}
#[test]fn isolated_blend_and_bbox(){
    let p=fixture::pdf(b"1 0 0 rg 0 0 20 20 re f /Outer gs /G Do",fixture::GROUP,b"0 0 1 rg 0 0 20 20 re f 0 0 1 1 re W n",b"","<< /BM /Multiply >>","<< >>");
    let b=render(&p);assert_eq!(px(&b,7,15),[0,0,0,255]);assert_eq!(px(&b,17,15),[255,0,0,255]);
    // An isolated group's first Multiply object has a transparent backdrop, not black.
    let p=fixture::pdf(b"/G Do",fixture::GROUP,b"/Inner gs 1 0 0 rg 0 0 10 10 re f",b"","<< >>","<< /BM /Multiply >>");
    let b=render(&p);assert_eq!(px(&b,5,15),[255,0,0,255]);
}
#[test]fn unsupported_groups_cycles_and_budget(){
    for g in ["<< /S /Transparency /I true /CS /DeviceCMYK >>"]{
        let p=fixture::pdf(b"/G Do",g,fixture::OVERLAP,b"","<< >>","<< >>");let d=Document::open(&p).unwrap();
        assert!(matches!(d.page_display_list(&d.pages().unwrap()[0],10000),Err(Error::Unsupported(_))));
    }
    let p=fixture::pdf(b"/G Do",fixture::GROUP,b"/G Do",b"","<< >>","<< >>");let d=Document::open(&p).unwrap();
    assert!(d.page_display_list(&d.pages().unwrap()[0],10000).is_err());
    let p=fixture::pdf(b"/G Do",NONISO,b"/N Do",b"0 0 1 rg 0 0 10 10 re f","<< >>","<< >>");let d=Document::open(&p).unwrap();
    assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options{scale:100.,allow_missing_text:false},10000),Err(Error::Limit("transparency group buffers"))));
}

const NONISO:&str="<< /S /Transparency /I false /CS /DeviceRGB >>";
#[test]fn nonisolated_backdrop_and_default_isolation(){
    for g in [NONISO,"<< /S /Transparency >>"]{
        let p=fixture::pdf(b"0 1 0 rg 0 0 20 20 re f /G Do",g,b"/Inner gs 1 0 0 rg 0 0 10 10 re f",b"","<< >>","<< /BM /Multiply >>");
        let d=Document::open(&p).unwrap();let list=d.page_display_list(&d.pages().unwrap()[0],10000).unwrap();
        assert!(list.commands.iter().any(|c|matches!(c,Command::BeginNonIsolatedGroup{..})));
        let b=render(&p);assert_eq!(px(&b,5,15),[0,0,0,255]);assert_eq!(px(&b,17,15),[0,255,0,255]);
    }
    let p=fixture::pdf(b"0 1 0 rg 0 0 20 20 re f /G Do",fixture::GROUP,b"/Inner gs 1 0 0 rg 0 0 10 10 re f",b"","<< >>","<< /BM /Multiply >>");
    assert_eq!(px(&render(&p),5,15),[255,0,0,255]);
}
#[test]fn nonisolated_opacity_and_outer_blend(){
    let p=fixture::pdf(b"0 1 0 rg 0 0 20 20 re f /Outer gs /G Do",NONISO,b"/Inner gs 1 0 0 rg 0 0 10 10 re f",b"","<< /ca 0.5 >>","<< /ca 0.5 /BM /Multiply >>");
    let b=render(&p);near(px(&b,5,15),[0,191,0,255]);assert_eq!(px(&b,17,15),[0,255,0,255]);
    let p=fixture::pdf(b"0 1 0 rg 0 0 20 20 re f /Outer gs /G Do",NONISO,b"/Inner gs 1 0 0 rg 0 0 10 10 re f",b"","<< /BM /Multiply /ca 0.5 >>","<< /ca 0.5 >>");
    let b=render(&p);near(px(&b,5,15),[0,191,0,255]);assert_eq!(px(&b,17,15),[0,255,0,255]);
}
#[test]fn nonisolated_nested_groups(){
    let p=fixture::pdf(b"0.5 g 0 0 20 20 re f /Outer gs /G Do",NONISO,b"/Inner gs /N Do",b"1 0 0 rg 0 0 10 10 re f","<< /ca 0.5 >>","<< /ca 0.5 >>");
    let b=render(&p);near(px(&b,5,15),[160,96,96,255]);near(px(&b,17,15),[128,128,128,255]);
    let p=fixture::pdf_nested(b"/Outer gs /G Do",fixture::GROUP,b"/Inner gs 1 0 0 rg 0 0 10 10 re f /N Do",b"0 0 1 rg 0 0 10 10 re f","<< /ca 0.5 >>","<< /ca 0.5 >>",NONISO);
    let b=render(&p);near(px(&b,5,15),[191,159,223,255]);
}

#[test]fn knockout_path_overlap_and_zero_opacity(){
    for isolated in ["true","false"]{
        let group=format!("<< /S /Transparency /I {isolated} /K true /CS /DeviceRGB >>");
        let p=fixture::pdf(b"/G Do",&group,b"/Inner gs 1 0 0 rg 0 0 10 10 re f 0 0 1 rg 5 0 10 10 re f",b"","<< >>","<< /ca 0.5 >>");
        let b=render(&p);near(px(&b,2,15),[255,128,128,255]);near(px(&b,7,15),[128,128,255,255]);near(px(&b,12,15),[128,128,255,255]);
        let p=fixture::pdf(b"/G Do",&group,b"1 0 0 rg 0 0 10 10 re f /Inner gs 0 0 1 rg 5 0 10 10 re f",b"","<< >>","<< /ca 0 >>");
        let b=render(&p);near(px(&b,2,15),[255,0,0,255]);near(px(&b,7,15),[255,255,255,255]);
    }
}
#[test]fn knockout_unsupported_object_boundaries(){
    let g="<< /S /Transparency /I true /K true >>";
    for body in [b"0 0 10 10 re B".as_slice(),b"/N Do",b"BT ET"]{
        let p=fixture::pdf(b"/G Do",g,body,b"","<< >>","<< >>");let d=Document::open(&p).unwrap();
        assert!(matches!(d.page_display_list(&d.pages().unwrap()[0],10000),Err(Error::Unsupported(_))));
    }
}
#[test]fn alpha_and_luminosity_soft_masks(){
    for (kind,body,expected) in [
        ("Alpha",b"/Inner gs 0 g 0 0 5 10 re f".as_slice(),[255,128,128,255]),
        ("Luminosity",b"0.5 g 0 0 5 10 re f".as_slice(),[255,127,127,255]),
    ]{
        let gs=format!("<< /SMask << /S /{kind} /G 5 0 R /TR /Identity >> >>");
        let p=fixture::pdf(b"q /Outer gs 1 0 0 rg 0 0 10 10 re f Q 0 g 15 0 5 10 re f",fixture::GROUP,body,b"",&gs,"<< /ca 0.5 >>");
        let b=render(&p);near(px(&b,2,15),expected);assert_eq!(px(&b,7,15),[255,255,255,255]);assert_eq!(px(&b,17,15),[0,0,0,255]);
    }
}
#[test]fn mask_group_opacity_and_clear(){
    let p=fixture::pdf(b"/Outer gs /N Do /Inner gs 0 g 15 0 5 10 re f",fixture::GROUP,b"0.5 g 0 0 10 10 re f",b"1 0 0 rg 0 0 10 10 re f 0 0 1 rg 5 0 10 10 re f","<< /SMask << /S /Luminosity /G 5 0 R >> >>","<< /SMask /None >>");
    let b=render(&p);near(px(&b,2,15),[255,127,127,255]);near(px(&b,7,15),[127,127,255,255]);assert_eq!(px(&b,17,15),[0,0,0,255]);
}
#[test]fn soft_mask_rejects_invalid_or_recursive_groups(){
    for (g,body,extra) in [(NONISO,b"0 g 0 0 10 10 re f".as_slice(),""),(fixture::GROUP,b"/Outer gs",""),(fixture::GROUP,b"0 g 0 0 10 10 re f","/TR 9 0 R")]{
        let gs=format!("<< /SMask << /S /Alpha /G 5 0 R {extra} >> >>");
        let p=fixture::pdf(b"/Outer gs",g,body,b"",&gs,"<< >>");let d=Document::open(&p).unwrap();assert!(d.page_display_list(&d.pages().unwrap()[0],10000).is_err());
    }
}

#[test]fn small_groups_repeated_on_large_page(){
    let mut page=Vec::new();
    for _ in 0..500{page.extend(b"q 0.1 0 0 0.1 5 5 cm /G Do Q\n");}
    let p=fixture::pdf(&page,fixture::GROUP,b"1 0 0 rg 0 0 15 15 re f",b"","<< >>","<< >>");
    let d=Document::open(&p).unwrap();
    let b=d.render_page(&d.pages().unwrap()[0],Options{scale:50.,allow_missing_text:false},100000).unwrap();
    assert_eq!((b.width,b.height),(1000,1000));
    assert_eq!(px(&b,260,730),[255,0,0,255]);assert_eq!(px(&b,249,730),[255,255,255,255]);
}
#[test]fn translated_nested_nonisolated_group_coordinates(){
    let p=fixture::pdf_nested(b"0 1 0 rg 0 0 20 20 re f q 1 0 0 1 3 4 cm /G Do Q",NONISO,b"/Inner gs 1 0 0 rg 0 0 10 10 re f q 1 0 0 1 5 2 cm /N Do Q",b"0 0 1 rg 0 0 10 10 re f","<< >>","<< /ca 0.5 >>",NONISO);
    let b=render(&p);
    near(px(&b,4,13),[128,128,0,255]);near(px(&b,9,12),[64,64,128,255]);near(px(&b,16,12),[0,128,128,255]);assert_eq!(px(&b,1,12),[0,255,0,255]);
}
#[test]fn empty_group_and_local_mask_coordinates(){
    let p=fixture::pdf(b"q 1 0 0 1 30 30 cm /G Do Q 0 g 0 0 1 1 re f",NONISO,b"1 0 0 rg 0 0 15 15 re f",b"","<< >>","<< >>");
    let b=render(&p);assert_eq!(px(&b,0,19),[0,0,0,255]);assert_eq!(px(&b,10,10),[255,255,255,255]);
    let p=fixture::pdf(b"q 1 0 0 1 3 4 cm /Outer gs /N Do Q",fixture::GROUP,b"0.5 g 0 0 5 10 re f",b"1 0 0 rg 0 0 10 10 re f","<< /SMask << /S /Luminosity /G 5 0 R >> >>","<< >>");
    let b=render(&p);near(px(&b,4,13),[255,127,127,255]);assert_eq!(px(&b,9,13),[255,255,255,255]);
}
