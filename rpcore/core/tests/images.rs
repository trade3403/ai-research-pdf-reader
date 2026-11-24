#[path="support/image_fixture.rs"] mod fixture;
use rpcore::{Document,filters::jpeg::{self,Transform},raster::{Bitmap,Options}};
fn close(a:u8,b:u8){assert!((i32::from(a)-i32::from(b)).abs()<=2,"{a} != {b}");}
fn render(p:&[u8])->Bitmap{let d=Document::open(p).unwrap();d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap()}
fn px(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
#[test]fn baseline_jpeg_and_ac(){
    assert_eq!(fixture::gray().len(),161);assert_eq!(fixture::rgb().len(),291);assert_eq!(fixture::gradient().len(),165);
    let a=jpeg::decode(&fixture::gray(),1000,Transform::Auto).unwrap();assert_eq!((a.width,a.height,a.channels),(9,9,1));assert!(a.data.iter().all(|&v|v==85));
    let a=jpeg::decode(&fixture::rgb(),1000,Transform::Auto).unwrap();assert_eq!((a.width,a.height,a.channels),(17,9,3));
    for p in a.data.chunks_exact(3){for (&v,t) in p.iter().zip([200,39,70]){close(v,t);}}
    let a=jpeg::decode(&fixture::gradient(),1000,Transform::Auto).unwrap();for (&v,t) in a.data[..8].iter().zip([1,30,61,90,121,149,181,210]){close(v,t);}
}
#[test]fn jpeg_invalid_and_limit(){
    let mut zero=fixture::gray();let q=zero.windows(2).position(|v|v==[255,219]).unwrap();zero[q+5]=0;
    assert!(matches!(jpeg::decode(&zero,1000,Transform::Auto),Err(rpcore::Error::Invalid("JPEG zero quantizer"))));
    let mut d=fixture::gray();d.truncate(d.len()-2);assert!(jpeg::decode(&d,1000,Transform::Auto).is_err());
    let d=fixture::rgb();assert!(jpeg::decode(&d,100,Transform::Auto).is_err());
    let mut d=fixture::gray();let i=d.windows(2).position(|a|a==[255,192]).unwrap();d[i+1]=194;assert!(jpeg::decode(&d,1000,Transform::Auto).is_err());
}
#[test]fn dct_pdf_and_soft_mask(){
    let im=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /SMask 6 0 R",&fixture::rgb());
    let mut mask=vec![128;17*9];mask[0]=0;
    let m=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace /DeviceGray /BitsPerComponent 8",&mask);
    let p=fixture::pdf(b"17 0 0 9 0 0 cm /Im Do",im,m,"<< /ca 0.5 >>");let b=render(&p);
    assert_eq!(px(&b,0,11),[255,255,255,255]);for (&v,t) in px(&b,1,11).iter().zip([227,147,162,255]){close(v,t);}
}
#[test]fn stencil_polarity_and_color_key(){
    for (decode,left,right) in [("",[255,0,0,255],[255,255,255,255]),("/Decode [1 0]",[255,255,255,255],[255,0,0,255])]{
        let im=fixture::stream(&format!("/Subtype /Image /Width 2 /Height 1 /ImageMask true {decode}"),&[0x40]);
        let p=fixture::pdf(b"1 0 0 rg 2 0 0 1 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
        assert_eq!(px(&b,0,19),left);assert_eq!(px(&b,1,19),right);
    }
    let im=fixture::stream("/Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Mask [0 0] /Decode [1 0]",&[0,255]);
    let p=fixture::pdf(b"2 0 0 1 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
    assert_eq!(px(&b,0,19),[255,255,255,255]);assert_eq!(px(&b,1,19),[0,0,0,255]);
}
#[test]fn normal_opacity_restore(){
    let p=fixture::pdf(b"q /Half gs 1 0 0 rg 0 0 10 10 re f 0 0 1 rg 5 0 10 10 re f Q 0 g 15 0 5 10 re f",b"null".to_vec(),b"null".to_vec(),"<< /ca 0.5 /CA 0.25 /BM /Normal >>");let b=render(&p);
    assert_eq!(px(&b,2,15),[255,128,128,255]);assert_eq!(px(&b,7,15),[128,64,192,255]);assert_eq!(px(&b,12,15),[128,128,255,255]);assert_eq!(px(&b,17,15),[0,0,0,255]);
    let p=fixture::pdf(b"/Half gs",b"null".to_vec(),b"null".to_vec(),"<< /BM /Multiply >>");let d=Document::open(&p).unwrap();assert!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000).is_err());
}
