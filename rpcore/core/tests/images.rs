#[path="support/icc_fixture.rs"] mod icc_fixture;
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
    let p=fixture::pdf(b"/Half gs",b"null".to_vec(),b"null".to_vec(),"<< /BM /Unknown >>");let d=Document::open(&p).unwrap();assert!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000).is_err());
}

#[test]fn packed_and_indexed(){
    for (bits,data,expected) in [(1,vec![0x40,0x80],vec![0,255,255,0]),(2,vec![0x30,0xc0],vec![0,255,255,0]),(4,vec![0x0f,0xf0],vec![0,255,255,0]),(16,vec![0,0,255,255,255,255,0,0],vec![0,255,255,0]) ]{
        let im=fixture::stream(&format!("/Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent {bits}"),&data);
        let p=fixture::pdf(b"2 0 0 2 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
        for y in 0..2{for x in 0..2{assert_eq!(px(&b,x,18+y),[expected[y*2+x],expected[y*2+x],expected[y*2+x],255]);}}
    }
    let im=fixture::stream("/Subtype /Image /Width 3 /Height 1 /ColorSpace [/Indexed /DeviceRGB 2 <ff000000ff000000ff>] /BitsPerComponent 2",&[0x18]);
    let p=fixture::pdf(b"3 0 0 1 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
    assert_eq!(px(&b,0,19),[255,0,0,255]);assert_eq!(px(&b,1,19),[0,255,0,255]);assert_eq!(px(&b,2,19),[0,0,255,255]);
}
#[test]fn interpolate_and_masks(){
    let im=fixture::stream("/Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Interpolate true",&[0,255]);
    let p=fixture::pdf(b"4 0 0 1 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
    for (x,v) in [0,64,191,255].into_iter().enumerate(){close(px(&b,x,19)[0],v);}
    for hard in [false,true]{
        let im=fixture::stream(&format!("/Subtype /Image /Width 4 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /{} 6 0 R",if hard{"Mask"}else{"SMask"}),&[255,0,0].repeat(4));
        let m=if hard{fixture::stream("/Subtype /Image /Width 2 /Height 1 /ImageMask true",&[0x40])}else{fixture::stream("/Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",&[255,0])};
        let p=fixture::pdf(b"4 0 0 1 0 0 cm /Im Do",im,m,"<< >>");let b=render(&p);
        assert_eq!(px(&b,0,19),[255,0,0,255]);assert_eq!(px(&b,1,19),[255,0,0,255]);assert_eq!(px(&b,2,19),[255,255,255,255]);assert_eq!(px(&b,3,19),[255,255,255,255]);
    }
}
#[test]fn blend_modes_and_restore(){
    for (mode,expected) in [("Multiply",[0,0,0,255]),("Screen",[255,0,255,255]),("Difference",[255,0,255,255]),("Darken",[0,0,0,255]),("Lighten",[255,0,255,255])]{
        let p=fixture::pdf(b"1 0 0 rg 0 0 20 20 re f q /Half gs 0 0 1 rg 0 0 10 10 re f Q 0 0 1 rg 10 0 10 10 re f",b"null".to_vec(),b"null".to_vec(),&format!("<< /BM /{mode} >>"));let b=render(&p);
        assert_eq!(px(&b,5,15),expected);assert_eq!(px(&b,15,15),[0,0,255,255]);
    }
    let p=fixture::pdf(b"1 0 0 rg 0 0 20 20 re f /Half gs 0 0 1 rg 0 0 20 20 re f",b"null".to_vec(),b"null".to_vec(),"<< /BM [/Unknown /Multiply] /ca 0.5 >>");let b=render(&p);
    assert_eq!(px(&b,5,15),[128,0,0,255]);
}
#[test]fn malformed_packed_and_palette(){
    for (dict,data) in [("/ColorSpace /DeviceGray /BitsPerComponent 4",vec![0]),("/ColorSpace [/Indexed /DeviceRGB 2 <ff0000>] /BitsPerComponent 2",vec![0,0])]{
        let im=fixture::stream(&format!("/Subtype /Image /Width 2 /Height 2 {dict}"),&data);
        let p=fixture::pdf(b"2 0 0 2 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let d=Document::open(&p).unwrap();
        assert!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000).is_err());
    }
}

#[test]fn progressive_and_four_components(){
    let gray=fixture::progressive_gradient();assert_eq!(gray.len(),285);
    let a=jpeg::decode(&gray,1000,Transform::Auto).unwrap();assert_eq!((a.width,a.height,a.channels),(8,8,1));
    for row in a.data.chunks_exact(8){for (&v,t) in row.iter().zip([1,30,61,90,121,149,181,210]){close(v,t);}}
    let mut truncated=gray.clone();truncated.truncate(gray.len()-2);assert!(jpeg::decode(&truncated,1000,Transform::Auto).is_err());
    let c=fixture::cmyk();assert_eq!(c.len(),177);
    let a=jpeg::decode(&c,1000,Transform::Auto).unwrap();assert_eq!((a.width,a.height,a.channels),(8,8,4));
    for p in a.data.chunks_exact(4){for (&v,t) in p.iter().zip([255,0,0,255]){close(v,t);}}
    // Adobe APP14 transform 2 test: same component coefficients with YCCK semantics.
    let mut y=c.clone();y[17]=2;let a=jpeg::decode(&y,1000,Transform::Auto).unwrap();
    for p in a.data.chunks_exact(4){for (&v,t) in p.iter().zip([179,0,227,255]){close(v,t);}}
    let im=fixture::stream("/Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /DCTDecode /Decode [1 0 1 0 1 0 1 0]",&c);
    let p=fixture::pdf(b"8 0 0 8 0 0 cm /Im Do",im,b"null".to_vec(),"<< >>");let b=render(&p);
    for (&v,t) in px(&b,2,15).iter().zip([255,0,0,255]){close(v,t);}
}
#[test]fn nonseparable_and_softlight(){
    use rpcore::graphics::BlendMode;
    let b=[0.4,0.4,0.4];let s=[1.,0.,0.];
    let c=BlendMode::Color.rgb(b,s);
    assert!((0.3*c[0]+0.59*c[1]+0.11*c[2]-0.4).abs()<1e-10);
    assert!(c[0]>c[1]);for v in BlendMode::Hue.rgb(b,s){assert!((v-0.4).abs()<1e-10);}
    for v in BlendMode::Saturation.rgb([0.8,0.2,0.1],[0.5;3]){assert!((v-0.369).abs()<1e-10);}
    assert!((BlendMode::SoftLight.channel(0.25,1.)-0.5).abs()<1e-12);
    let p=fixture::pdf(b"0.4 g 0 0 20 20 re f /Half gs 1 0 0 rg 0 0 20 20 re f",b"null".to_vec(),b"null".to_vec(),"<< /BM /Color >>");
    let out=render(&p);for (i,&v) in px(&out,5,15)[..3].iter().enumerate(){close(v,(c[i]*255.).round()as u8);}
}

#[test]fn icc_image_alternate(){
    let im=fixture::stream("/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8",&[255,0,0]);
    let profile=fixture::stream("/N 3",b"uninterpreted ICC profile");
    let p=fixture::pdf(b"1 0 0 1 0 0 cm /Im Do",im,profile,"<< >>");
    let b=render(&p);assert_eq!(px(&b,0,19),[255,0,0,255]);
}

#[test]fn native_icc_rgb_16bit_decode_and_raw_mask(){
    let samples:[u16;9]=[0,0,0,49152,49152,49152,65535,65535,65535];
    let data:Vec<u8>=samples.into_iter().flat_map(u16::to_be_bytes).collect();
    let im=fixture::stream("/Subtype /Image /Width 3 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 16 /Mask [0 0 0 0 0 0]",&data);
    let p=fixture::pdf(b"3 0 0 1 0 0 cm /Im Do",im,fixture::stream("/N 3",&icc_fixture::rgb_icc()),"<< >>");
    let b=render(&p);
    assert_eq!(px(&b,0,19),[255;4]); // raw zero transparent
    // 49152/65535 through gamma=2 and sRGB -> 198, prequantizing ->197.
    assert_eq!(px(&b,1,19),[198,198,198,255]);
    assert_eq!(px(&b,2,19),[255;4]);
    let im=fixture::stream("/Subtype /Image /Width 2 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Decode [1 0 1 0 1 0] /Mask [0 0 0 0 0 0]",&[0,0,0,255,255,255]);
    let p=fixture::pdf(b"1 0 0 rg 0 0 20 20 re f 2 0 0 1 0 0 cm /Im Do",im,fixture::stream("/N 3",&icc_fixture::rgb_icc()),"<< >>");
    let b=render(&p);assert_eq!(px(&b,0,19),[255,0,0,255]);assert_eq!(px(&b,1,19),[0,0,0,255]);
}
#[test]fn native_icc_rgb_jpeg_and_soft_mask(){
    let im=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Filter /DCTDecode /SMask 8 0 R",&fixture::rgb());
    let mask=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace /DeviceGray /BitsPerComponent 8",&[128;153]);
    let p=fixture::pdf_extra(b"17 0 0 9 0 0 cm /Im Do",im,fixture::stream("/N 3",&icc_fixture::rgb_icc()),"<< >>",vec![mask]);
    let b=render(&p);
    // Independent expected neutral channel from fixture RGB squares, not
    // native ICC output. Mask alpha is linear and independent of the TRCs.
    let linear=[200_f64,39.,70.].into_iter().map(|v|(v/255.).powi(2)).sum::<f64>()/3.;
    let s=(255.*(1.055*linear.powf(1./2.4)-0.055)).round();
    let expected=(255.+(s-255.)*128./255.).round()as u8;
    for &v in &px(&b,3,14)[..3]{close(v,expected);}
}
#[test]fn native_icc_cmyk_packed_and_jpeg(){
    // Relative A2B1 produces neutral L=100*(1-K); check K changes.
    let im=fixture::stream("/Subtype /Image /Width 2 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 4",&[0,0,0,15]);
    let p=fixture::pdf(b"2 0 0 1 0 0 cm /Im Do",im,fixture::stream("/N 4",&icc_fixture::cmyk_icc()),"<< >>");
    let b=render(&p);assert_eq!(px(&b,0,19),[255;4]);assert_eq!(px(&b,1,19),[0,0,0,255]);
    let im=fixture::stream("/Subtype /Image /Width 8 /Height 8 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /Filter /DCTDecode /Decode [1 0 1 0 1 0 1 0]",&fixture::cmyk());
    let p=fixture::pdf(b"8 0 0 8 0 0 cm /Im Do",im,fixture::stream("/N 4",&icc_fixture::cmyk_icc()),"<< >>");
    let b=render(&p);assert_eq!(px(&b,2,15),[255;4]); // K decoded from 255 to 0
}
#[test]fn native_icc_image_rejects_component_mismatch_and_work_limit(){
    let im=fixture::stream("/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8",&[0,0,0]);
    let p=fixture::pdf(b"/Im Do",im,fixture::stream("/N 3",&icc_fixture::cmyk_icc()),"<< >>");
    let d=Document::open(&p).unwrap();
    assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(rpcore::Error::Invalid("ICCBased profile N mismatch"))));
    // Large image on a tiny page must still charge its decode/ICC cost.
    let im=fixture::stream("/Subtype /Image /Width 1500 /Height 1500 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 1",&vec![0;750*1500]);
    let p=fixture::pdf(b"/Im Do",im,fixture::stream("/N 4",&icc_fixture::cmyk_icc()),"<< >>");
    let d=Document::open(&p).unwrap();
    assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(rpcore::Error::Limit("raster work (image ICC)"))));
}

#[test]fn image_icc_diagnostics_native_and_alternate(){
    use rpcore::icc::{ProfileUse,ProfilePath};
    for (profile,n,path) in [
        (icc_fixture::rgb_icc(),3,ProfilePath::RgbMatrix),
        (icc_fixture::cmyk_icc(),4,ProfilePath::CmykLut{tag:*b"A2B1"}),
        (b"short invalid ICC".to_vec(),3,ProfilePath::Alternate{reason:"ICC profile header"}),
    ]{
        let im=fixture::stream(&format!("/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8"),&vec![0;n]);
        let p=fixture::pdf(b"q /Im Do Q /Im Do",im,fixture::stream(&format!("/N {n}"),&profile),"<< >>");
        let d=Document::open(&p).unwrap();
        let out=d.render_page_with_text_quality(&d.pages().unwrap()[0],Options::default(),10000,None,rpcore::raster::TextQuality::Standard).unwrap();
        assert_eq!(out.icc_profiles.len(),1);
        let x=out.icc_profiles[0];
        assert_eq!((x.object_id,x.source,x.components,x.path),((6,0),ProfileUse::Image,n,path));
    }
}

#[test]fn native_gray_icc_image_and_alpha_separation(){
    use rpcore::icc::{ProfilePath,ProfileUse};
    let p=icc_fixture::gray_icc();
    let t=rpcore::icc::Profile::parse(&p).unwrap().gray_trc().unwrap();
    assert!((t.to_xyz_d50(0.5).unwrap()[1]-0.25).abs()<1e-12);
    assert!(t.to_xyz_d50(f64::NAN).is_err());
    let im=fixture::stream("/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8 /SMask 8 0 R",&[128]);
    let mask=fixture::stream("/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8",&[128]);
    let pdf=fixture::pdf_extra(b"/Im Do",im,fixture::stream("/N 1",&p),"<< >>",vec![mask]);
    let d=Document::open(&pdf).unwrap();
    let out=d.render_page_with_text_quality(&d.pages().unwrap()[0],Options::default(),10000,None,rpcore::raster::TextQuality::Standard).unwrap();
    assert_eq!(out.icc_profiles.len(),1);
    assert_eq!((out.icc_profiles[0].source,out.icc_profiles[0].path),(ProfileUse::Image,ProfilePath::GrayTrc));
    // Gray gamma 2 -> sRGB ~138; mask sample stays 128/255, not TRC.
    for &x in &px(&out.bitmap,0,19)[..3]{close(x,196);}
}
