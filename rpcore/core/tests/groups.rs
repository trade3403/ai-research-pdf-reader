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
    for g in ["<< /S /Transparency >>","<< /S /Transparency /I false >>","<< /S /Transparency /I true /K true >>","<< /S /Transparency /I true /CS /DeviceCMYK >>"]{
        let p=fixture::pdf(b"/G Do",g,fixture::OVERLAP,b"","<< >>","<< >>");let d=Document::open(&p).unwrap();
        assert!(matches!(d.page_display_list(&d.pages().unwrap()[0],10000),Err(Error::Unsupported(_))));
    }
    let p=fixture::pdf(b"/G Do",fixture::GROUP,b"/G Do",b"","<< >>","<< >>");let d=Document::open(&p).unwrap();
    assert!(d.page_display_list(&d.pages().unwrap()[0],10000).is_err());
    let p=fixture::pdf(b"/G Do",fixture::GROUP,b"/N Do",b"0 0 1 rg 0 0 10 10 re f","<< >>","<< >>");let d=Document::open(&p).unwrap();
    assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options{scale:100.,allow_missing_text:false},10000),Err(Error::Limit(_))));
}
