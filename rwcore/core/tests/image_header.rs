use rwcore::{parse_image_dimensions,ImageFormat as F,ImageLimits,ImageDimensions,MediaResource,Error};
fn dim(f:F,b:&[u8])->Result<ImageDimensions,Error>{parse_image_dimensions(f,b,ImageLimits::default())}
fn expect(f:F,b:&[u8],width:u32,height:u32){assert_eq!(dim(f,b).unwrap(),ImageDimensions{width,height});}
fn png(w:u32,h:u32)->Vec<u8>{
    let mut b=b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    b.extend(w.to_be_bytes());b.extend(h.to_be_bytes());b.extend([8,2,0,0,0,0,0,0,0]);b
}
#[test]
fn png_ihdr_dimensions_and_truncations(){
    let b=png(640,480);expect(F::Png,&b,640,480);
    for n in 0..b.len(){assert!(dim(F::Png,&b[..n]).is_err());}
    let mut bad=b;bad[12]=b'X';assert!(dim(F::Png,&bad).is_err());
}
#[test]
fn jpeg_baseline_progressive_and_bounded_segments(){
    let b=[0xff,0xd8,0xff,0xe0,0,4,1,2,0xff,0xc0,0,11,8,0,20,0,30,1,1,0x11,0];
    expect(F::Jpeg,&b,30,20);
    let mut progressive=b;progressive[9]=0xc2;expect(F::Jpeg,&progressive,30,20);
    for n in 0..b.len(){assert!(dim(F::Jpeg,&b[..n]).is_err());}
    let mut bad=b;bad[5]=1;assert!(dim(F::Jpeg,&bad).is_err());
    assert_eq!(parse_image_dimensions(F::Jpeg,&b,ImageLimits{max_header_bytes:10,..Default::default()}),Err(Error::Limit("image header bytes")));
}
#[test]
fn gif_logical_screen(){
    let b=b"GIF89a\x40\x01\xf0\x00\x00\x00\x00";
    expect(F::Gif,b,320,240);
    assert!(dim(F::Gif,&b[..12]).is_err());
}
#[test]
fn bmp_core_and_signed_top_down_dimensions(){
    let mut b=vec![0;54];b[..2].copy_from_slice(b"BM");
    b[14..18].copy_from_slice(&40u32.to_le_bytes());
    b[18..22].copy_from_slice(&120i32.to_le_bytes());b[22..26].copy_from_slice(&(-80i32).to_le_bytes());
    b[26..28].copy_from_slice(&1u16.to_le_bytes());expect(F::Bmp,&b,120,80);
    b[22..26].copy_from_slice(&i32::MIN.to_le_bytes());assert!(dim(F::Bmp,&b).is_err());
    let mut core=vec![0;26];core[..2].copy_from_slice(b"BM");core[14..18].copy_from_slice(&12u32.to_le_bytes());
    core[18..20].copy_from_slice(&40u16.to_le_bytes());core[20..22].copy_from_slice(&30u16.to_le_bytes());
    core[22..24].copy_from_slice(&1u16.to_le_bytes());expect(F::Bmp,&core,40,30);
}
fn tiff(le:bool)->Vec<u8>{
    fn put16(b:&mut Vec<u8>,v:u16,le:bool){b.extend(if le{v.to_le_bytes()}else{v.to_be_bytes()});}
    fn put32(b:&mut Vec<u8>,v:u32,le:bool){b.extend(if le{v.to_le_bytes()}else{v.to_be_bytes()});}
    let mut b=if le{b"II".to_vec()}else{b"MM".to_vec()};
    put16(&mut b,42,le);put32(&mut b,8,le);put16(&mut b,2,le);
    put16(&mut b,256,le);put16(&mut b,3,le);put32(&mut b,1,le);put16(&mut b,123,le);put16(&mut b,0,le);
    put16(&mut b,257,le);put16(&mut b,4,le);put32(&mut b,1,le);put32(&mut b,456,le);put32(&mut b,0,le);b
}
#[test]
fn tiff_endianness_and_dimension_entries(){
    for le in [true,false]{let b=tiff(le);expect(F::Tiff,&b,123,456);assert!(dim(F::Tiff,&b[..b.len()-1]).is_err());}
    let mut b=tiff(true);b[22..24].copy_from_slice(&256u16.to_le_bytes());
    assert_eq!(dim(F::Tiff,&b),Err(Error::Invalid("duplicate TIFF dimension")));
    let mut b=tiff(true);b[4..8].copy_from_slice(&u32::MAX.to_le_bytes());assert!(dim(F::Tiff,&b).is_err());
}
fn webp(tag:&[u8;4],payload:&[u8])->Vec<u8>{
    let mut b=b"RIFF".to_vec();b.extend((12+payload.len() as u32+(payload.len() as u32%2)).to_le_bytes());
    b.extend(b"WEBP");b.extend(tag);b.extend((payload.len() as u32).to_le_bytes());b.extend(payload);
    if payload.len()%2!=0{b.push(0);}b
}
#[test]
fn webp_three_header_variants_and_bounds(){
    let x=webp(b"VP8X",&[0,0,0,0,99,0,0,199,0,0]);expect(F::WebP,&x,100,200);
    let mut lossless=vec![0x2f];lossless.extend((99u32|(199u32<<14)).to_le_bytes());
    let l=webp(b"VP8L",&lossless);expect(F::WebP,&l,100,200);
    let lossy=webp(b"VP8 ",&[0,0,0,0x9d,1,0x2a,100,0,200,0]);expect(F::WebP,&lossy,100,200);
    assert!(dim(F::WebP,&l[..l.len()-1]).is_err());
    let mut bad=x;bad[16..20].copy_from_slice(&u32::MAX.to_le_bytes());assert!(dim(F::WebP,&bad).is_err());
}
#[test]
fn pixel_and_dimension_limits_and_zero(){
    let b=png(100,200);
    assert_eq!(parse_image_dimensions(F::Png,&b,ImageLimits{max_pixels:19999,..Default::default()}),Err(Error::Limit("image pixels")));
    assert_eq!(parse_image_dimensions(F::Png,&b,ImageLimits{max_dimension:199,..Default::default()}),Err(Error::Limit("image dimension")));
    assert_eq!(dim(F::Png,&png(0,100)),Err(Error::Invalid("zero image dimension")));
    assert_eq!(ImageDimensions{width:u32::MAX,height:u32::MAX}.pixels(),u64::from(u32::MAX)*u64::from(u32::MAX));
}
#[test]
fn resource_api_and_format_mismatch(){
    let r=MediaResource{format:F::Png,bytes:png(12,34)};
    assert_eq!(r.dimensions(ImageLimits::default()).unwrap().pixels(),408);
    assert_eq!(dim(F::Jpeg,&r.bytes),Err(Error::Invalid("image signature mismatch")));
}
