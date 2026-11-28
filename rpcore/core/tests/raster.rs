use rpcore::{Document,Error,raster::{Options,Bitmap}};
fn pdf(data:&[u8],rotate:i32,image:bool)->Vec<u8>{
    let mut stream=format!("<< /Length {} >>\nstream\n",data.len()).into_bytes();stream.extend_from_slice(data);stream.extend_from_slice(b"\nendstream");
    let mut objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 20 20] >>".to_vec(),format!("<< /Type /Page /Parent 2 0 R /Contents 4 0 R /CropBox [2 3 12 11] /Rotate {rotate} /Resources << /XObject << /Im 5 0 R >> >> >>").into_bytes(),stream];
    if image{let mut im=b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 12 >>\nstream\n".to_vec();im.extend_from_slice(&[255,0,0,0,255,0,0,0,255,255,255,0]);im.extend_from_slice(b"\nendstream");objects.push(im);}
    let mut p=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for(i,o)in objects.iter().enumerate(){offsets.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend_from_slice(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offsets{p.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());p
}
fn render(data:&[u8],rotate:i32,image:bool)->Bitmap{
    let p=pdf(data,rotate,image);let d=Document::open(&p).unwrap();d.render_page(&d.pages().unwrap()[0],Options::default(),10000).unwrap()
}
fn pixel(b:&Bitmap,x:usize,y:usize)->&[u8]{let i=(y*b.width+x)*4;&b.rgba[i..i+4]}
#[test]fn fill_crop_rotation(){
    let b=render(b"1 0 0 rg 2 3 3 2 re f",0,false);assert_eq!((b.width,b.height),(10,8));
    assert_eq!(pixel(&b,0,7),[255,0,0,255]);assert_eq!(pixel(&b,3,7),[255,255,255,255]);
    for (r,xy) in [(90,(0,0)),(180,(9,0)),(270,(7,9))]{let b=render(b"1 0 0 rg 2 3 3 2 re f",r,false);assert_eq!(pixel(&b,xy.0,xy.1),[255,0,0,255]);}
}
#[test]fn holes_clip_restore(){
    let b=render(b"q 2 3 10 8 re 5 5 4 4 re W* n 0 g 2 3 10 8 re f Q 1 0 0 rg 6 6 1 1 re f",0,false);
    assert_eq!(pixel(&b,0,0),[0,0,0,255]);assert_eq!(pixel(&b,3,3),[255,255,255,255]);assert_eq!(pixel(&b,4,4),[255,0,0,255]);
    let b=render(b"0 g 2 3 0.5 8 re f",0,false);assert_eq!(pixel(&b,0,0),[128,128,128,255]);
}
#[test]fn stroke_curve(){
    let b=render(b"0 G 2 w 1 J 1 j 4 5 m 10 5 l S 0 0 1 rg 3 8 m 3 10 10 10 10 8 c h f",0,false);
    assert_eq!(pixel(&b,5,5),[0,0,0,255]);assert_eq!(pixel(&b,0,0),[255,255,255,255]);
}
#[test]fn image_orientation(){
    let b=render(b"q 4 0 0 4 2 3 cm /Im Do Q",0,true);
    assert_eq!(pixel(&b,0,4),[255,0,0,255]);assert_eq!(pixel(&b,3,4),[0,255,0,255]);assert_eq!(pixel(&b,0,7),[0,0,255,255]);assert_eq!(pixel(&b,3,7),[255,255,0,255]);
}
#[test]fn strict_missing_and_limits(){
    let p=pdf(b"BT /F1 10 Tf (A) Tj ET",0,false);let d=Document::open(&p).unwrap();let pages=d.pages().unwrap();
    assert!(matches!(d.render_page(&pages[0],Options::default(),10000),Err(Error::Invalid(_)|Error::Unsupported(_))));
    let b=d.render_page(&pages[0],Options{scale:1.,allow_missing_text:true},10000).unwrap();assert!(b.missing_text);
    assert!(d.render_page(&pages[0],Options{scale:1e9,allow_missing_text:true},10000).is_err());
    let p=pdf(b"[0 1] 0 d 2 3 m 10 8 l S",0,false);let d=Document::open(&p).unwrap();assert!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000).is_err());
}
#[test]fn png_integrity(){
    let b=render(b"1 0 0 rg 2 3 10 8 re f",0,false);let p=b.png().unwrap();assert_eq!(&p[..8],b"\x89PNG\r\n\x1a\n");
    let n=u32::from_be_bytes(p[33..37].try_into().unwrap()) as usize;assert_eq!(&p[37..41],b"IDAT");
    let mut dict=std::collections::BTreeMap::new();dict.insert(b"Filter".to_vec(),rpcore::Object::Name(b"FlateDecode".to_vec()));
    let obj=rpcore::Object::Stream{dict,data:p[41..41+n].to_vec()};
    let rows=rpcore::filters::decode(&obj,10000).unwrap();
    assert_eq!(rows.len(),b.height*(1+b.width*4));
    for (r,row) in rows.chunks_exact(1+b.width*4).enumerate(){assert_eq!(row[0],0);assert_eq!(&row[1..],&b.rgba[r*b.width*4..(r+1)*b.width*4]);}
}

#[test]fn bounded_local_coverage_many_small_paths(){
    let data=b"2.02 3.02 0.04 0.04 re f\n".repeat(300);
    let p=pdf(&data,0,false);let d=Document::open(&p).unwrap();
    let b=d.render_page(&d.pages().unwrap()[0],Options{scale:50.,allow_missing_text:false},100000).unwrap();
    assert_eq!((b.width,b.height),(500,400));
    assert_eq!(pixel(&b,1,398),[0,0,0,255]);
    assert_eq!(pixel(&b,100,100),[255,255,255,255]);
}
#[test]fn local_masks_outside_clip_and_fractional_edges(){
    let b=render(b"q 100 100 10 10 re W n 2 3 10 8 re f Q -100 -100 1 1 re f 2.5 3 1 1 re f",0,false);
    assert_eq!(pixel(&b,0,7),[128,128,128,255]);assert_eq!(pixel(&b,1,7),[128,128,128,255]);
    assert_eq!(pixel(&b,2,7),[255,255,255,255]);assert_eq!(pixel(&b,0,0),[255,255,255,255]);
}

#[test]fn repeated_local_clips_keep_large_page_within_budget(){
    // 3000 clips on a 500x400 page formerly charged 600m units for
    // full-page allocations alone. The actual clips are 2x2 pixels.
    let data=b"q 2.02 3.02 0.04 0.04 re W n 0 g 2.02 3.02 0.04 0.04 re f Q\n".repeat(3000);
    let p=pdf(&data,0,false);let d=Document::open(&p).unwrap();
    let b=d.render_page(&d.pages().unwrap()[0],Options{scale:50.,allow_missing_text:false},300000).unwrap();
    assert_eq!(pixel(&b,1,398),[0,0,0,255]);assert_eq!(pixel(&b,100,100),[255,255,255,255]);
}
#[test]fn local_nested_clip_and_image(){
    let b=render(b"q 2.5 3 3 4 re W n q 3 3 1 4 re W n 0 g 2 3 10 8 re f Q 4 0 0 4 2 3 cm /Im Do Q",0,true);
    assert_eq!(pixel(&b,0,4),[255,128,128,255]);
    assert_eq!(pixel(&b,1,4),[255,0,0,255]);assert_eq!(pixel(&b,2,4),[0,255,0,255]);
    assert_eq!(pixel(&b,3,4),[128,255,128,255]);assert_eq!(pixel(&b,4,4),[255,255,255,255]);
}
#[test]fn active_edges_many_separated_rectangles(){
    let mut data=String::new();
    for y in 0..200{for x in 0..20{
        data.push_str(&format!("{} {} 0.02 0.02 re ",2.+x as f64*0.4,3.+y as f64*0.04));
    }}data.push_str("f");
    let p=pdf(data.as_bytes(),0,false);let d=Document::open(&p).unwrap();
    let b=d.render_page(&d.pages().unwrap()[0],Options{scale:50.,allow_missing_text:false},300000).unwrap();
    assert_eq!(pixel(&b,0,399),[0,0,0,255]);assert_eq!(pixel(&b,1,399),[255,255,255,255]);
    assert_eq!(pixel(&b,20,397),[0,0,0,255]);assert_eq!(pixel(&b,499,0),[255,255,255,255]);
}

#[test]fn large_paths_under_tiny_clip_do_not_scan_whole_page(){
    // 2000 full-page paths under a 2x2 pixel clip: clipping before the
    // scan conversion avoids >1.6 billion covered subsample visits.
    let mut data=b"q 2.02 3.02 0.04 0.04 re W n\n".to_vec();
    for _ in 0..2000{data.extend_from_slice(b"2 3 10 8 re f\n");}
    data.extend_from_slice(b"Q");
    let p=pdf(&data,0,false);let d=Document::open(&p).unwrap();
    let b=d.render_page(&d.pages().unwrap()[0],Options{scale:50.,allow_missing_text:false},100000).unwrap();
    assert_eq!(pixel(&b,1,398),[0,0,0,255]);assert_eq!(pixel(&b,2,397),[0,0,0,255]);
    assert_eq!(pixel(&b,0,399),[255,255,255,255]);assert_eq!(pixel(&b,100,100),[255,255,255,255]);
}
#[test]fn clipped_winding_preserves_edges_outside_clip(){
    for op in ["f","f*"]{
        let data=format!("q 5 5 2 2 re W n -100 -100 200 200 re {op} Q");
        let b=render(data.as_bytes(),0,false);
        assert_eq!(pixel(&b,3,4),[0,0,0,255]);assert_eq!(pixel(&b,4,5),[0,0,0,255]);
        assert_eq!(pixel(&b,2,4),[255,255,255,255]);
    }
}
#[test]fn decoded_image_reuse_respects_transform_and_clip(){
    let b=render(b"q 4 0 0 4 2 3 cm /Im Do Q q 8 3 2 4 re W n 4 0 0 4 6 3 cm /Im Do Q",0,true);
    assert_eq!(pixel(&b,0,4),[255,0,0,255]);assert_eq!(pixel(&b,2,7),[255,255,0,255]);
    assert_eq!(pixel(&b,4,4),[255,255,255,255]);assert_eq!(pixel(&b,6,4),[0,255,0,255]);assert_eq!(pixel(&b,6,7),[255,255,0,255]);
}

#[test]fn dashed_stroke_period_phase_and_odd_pattern(){
    let a=render(b"2 w [2 1] 0 d 2 7 m 12 7 l S",0,false);
    let b=render(b"2 w 2 7 m 4 7 l 5 7 m 7 7 l 8 7 m 10 7 l 11 7 m 12 7 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
    assert_eq!(pixel(&a,0,3),[0,0,0,255]);assert_eq!(pixel(&a,2,3),[255,255,255,255]);
    let a=render(b"2 w [2 1] 7 d 2 7 m 12 7 l S",0,false);
    let b=render(b"2 w 2 7 m 3 7 l 4 7 m 6 7 l 7 7 m 9 7 l 10 7 m 12 7 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
    assert_eq!(render(b"[2] 0 d 2 7 m 12 7 l S",0,false).rgba,
               render(b"[2 2] 0 d 2 7 m 12 7 l S",0,false).rgba);
}
#[test]fn dashed_stroke_joins_and_subpath_reset(){
    let a=render(b"2 w 0 j [6 2] 0 d 3 4 m 7 4 l 7 8 l 11 8 l S",0,false);
    let b=render(b"2 w 0 j 3 4 m 7 4 l 7 6 l 7 8 m 11 8 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
    let a=render(b"2 w [3 2] 0 d 2 5 m 6 5 l 2 9 m 6 9 l S",0,false);
    let b=render(b"2 w 2 5 m 5 5 l 2 9 m 5 9 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
    // A 16-unit closed square: paint [0,6], [8,14], with no seam
    // continuation for this phase; a long dash covers the whole contour.
    let a=render(b"2 w [100 2] 0 d 4 5 4 4 re S",0,false);
    let b=render(b"2 w 4 5 4 4 re S",0,false);
    assert_eq!(a.rgba,b.rgba);
    // [10 2] on a 16-unit contour wraps the final painted run at the seam.
    let a=render(b"2 w [10 2] 0 d 4 5 4 4 re S",0,false);
    let b=render(b"2 w 4 9 m 4 5 l 8 5 l 8 9 l 6 9 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
}
#[test]fn dashed_stroke_uniform_ctm_and_limits(){
    let a=render(b"q 2 0 0 2 0 0 cm 1 w [1 .5] 0 d 1 3.5 m 6 3.5 l S Q",0,false);
    let b=render(b"2 w [2 1] 0 d 2 7 m 12 7 l S",0,false);
    assert_eq!(a.rgba,b.rgba);
    let p=pdf(b"[0.00001 0.00001] 0 d 2 7 m 12 7 l S",0,false);
    let d=Document::open(&p).unwrap();
    assert!(matches!(d.render_page(&d.pages().unwrap()[0],Options::default(),10000),Err(Error::Limit(_))));
}
