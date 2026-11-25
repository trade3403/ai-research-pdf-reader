use rwcore::*;
fn p16(b:&mut [u8],at:usize,n:u16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn p32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_be_bytes());}
fn format4()->Vec<u8>{
    // A-C via delta; Chinese U+4E00..4E02 via glyph array; FFFF sentinel.
    let mut b=vec![0;46];p16(&mut b,0,4);p16(&mut b,2,46);p16(&mut b,6,6);p16(&mut b,8,4);p16(&mut b,10,1);p16(&mut b,12,2);
    for (i,v) in [0x43,0x4e02,0xffff].into_iter().enumerate(){p16(&mut b,14+i*2,v);}
    for (i,v) in [0x41,0x4e00,0xffff].into_iter().enumerate(){p16(&mut b,22+i*2,v);}
    p16(&mut b,28,1u16.wrapping_sub(0x41));p16(&mut b,30,2);p16(&mut b,32,1);
    p16(&mut b,36,4);p16(&mut b,40,4);p16(&mut b,42,0);p16(&mut b,44,6);b
}
fn format12(groups:&[(u32,u32,u32)])->Vec<u8>{
    let mut b=vec![0;16+groups.len()*12];let len=b.len() as u32;p16(&mut b,0,12);p32(&mut b,4,len);p32(&mut b,12,groups.len() as u32);
    for (i,(a,z,g)) in groups.iter().enumerate(){p32(&mut b,16+i*12,*a);p32(&mut b,20+i*12,*z);p32(&mut b,24+i*12,*g);}b
}
fn cmap(records:&[(u16,u16,Vec<u8>)])->Vec<u8>{
    let mut b=vec![0;4+records.len()*8];p16(&mut b,2,records.len() as u16);
    for (i,(platform,encoding,data)) in records.iter().enumerate(){
        let p=4+i*8;let at=b.len() as u32;p16(&mut b,p,*platform);p16(&mut b,p+2,*encoding);p32(&mut b,p+4,at);b.extend(data);
    }b
}
fn font(cmap:Vec<u8>,glyphs:u16,cff:bool)->Vec<u8>{
    let mut maxp=vec![0;if cff{6}else{32}];p32(&mut maxp,0,if cff{0x5000}else{0x10000});p16(&mut maxp,4,glyphs);
    let mut b=vec![0;44];b[..4].copy_from_slice(&if cff{*b"OTTO"}else{[0,1,0,0]});p16(&mut b,4,2);
    for (i,(tag,data)) in [(*b"maxp",maxp),(*b"cmap",cmap)].into_iter().enumerate(){
        while b.len()%4!=0{b.push(0);}let at=b.len() as u32;let p=12+i*16;
        b[p..p+4].copy_from_slice(&tag);p32(&mut b,p+8,at);p32(&mut b,p+12,data.len() as u32);b.extend(data);
    }b
}
fn parse(b:&[u8],l:CmapLimits)->Result<UnicodeCmap<'_>,Error>{
    let c=parse_font(b,FontLimits::default())?;parse_unicode_cmap(c.face(0).unwrap(),l)
}
fn f4(b:Vec<u8>)->Vec<u8>{font(cmap(&[(3,1,b)]),20,false)}
fn f12(b:Vec<u8>)->Vec<u8>{font(cmap(&[(3,10,b)]),20,false)}
#[test]fn format4_direct_delta_and_missing(){
    let b=f4(format4());let c=parse(&b,CmapLimits::default()).unwrap();
    assert_eq!(c.source(),CmapSource{platform:3,encoding:1,format:CmapFormat::Format4});
    assert_eq!(c.glyph_for_char('A').unwrap(),Some(1));assert_eq!(c.glyph_for_char('C').unwrap(),Some(3));
    assert_eq!(c.glyph_for_char('D').unwrap(),None);assert_eq!(c.glyph_id(0xffff).unwrap(),None);
    assert_eq!(c.glyph_for_char('😀').unwrap(),None);assert_eq!(c.format4_search_fields_valid(),Some(true));
}
#[test]fn format4_relative_offsets_and_zero_before_delta(){
    let b=f4(format4());let c=parse(&b,CmapLimits::default()).unwrap();
    assert_eq!(c.glyph_id(0x4e00).unwrap(),Some(6));assert_eq!(c.glyph_id(0x4e01).unwrap(),None);
    assert_eq!(c.glyph_id(0x4e02).unwrap(),Some(8));assert_eq!(c.glyph_id(0x4e03).unwrap(),None);
}
#[test]fn format4_wrapping_array_delta(){
    let mut t=format4();p16(&mut t,30,0xffff);p16(&mut t,40,2);
    let b=f4(t);let c=parse(&b,CmapLimits::default()).unwrap();assert_eq!(c.glyph_id(0x4e00).unwrap(),Some(1));
}
#[test]fn format12_supplementary_plane_and_cff(){
    let b=font(cmap(&[(0,4,format12(&[(0x41,0x43,2),(0x1f600,0x1f602,10)]))]),20,true);
    let c=parse(&b,CmapLimits::default()).unwrap();assert_eq!(c.source().format,CmapFormat::Format12);
    assert_eq!(c.glyph_for_char('😀').unwrap(),Some(10));assert_eq!(c.glyph_id(0x1f602).unwrap(),Some(12));
    assert_eq!(c.glyph_for_char('A').unwrap(),Some(2));assert_eq!(c.glyph_id(0x1f603).unwrap(),None);
    assert_eq!(c.format4_search_fields_valid(),None);assert_eq!(c.glyph_count(),20);
}
#[test]fn prefer_full_unicode_no_per_character_fallback(){
    let b=font(cmap(&[(3,1,format4()),(3,10,format12(&[(0x1f600,0x1f600,9)]))]),20,false);
    let c=parse(&b,CmapLimits::default()).unwrap();assert_eq!(c.glyph_for_char('A').unwrap(),None);
    assert_eq!(c.glyph_for_char('😀').unwrap(),Some(9));
    let b=font(cmap(&[(0,4,format12(&[(0x41,0x41,8)])),(3,10,format12(&[(0x41,0x41,9)]))]),20,false);
    assert_eq!(parse(&b,CmapLimits::default()).unwrap().glyph_for_char('A').unwrap(),Some(9));
}
#[test]fn preferred_corruption_no_fallback(){
    let mut t=format12(&[(0x41,0x41,2)]);p32(&mut t,4,16);
    let b=font(cmap(&[(3,1,format4()),(3,10,t)]),20,false);assert!(parse(&b,CmapLimits::default()).is_err());
}
#[test]fn invalid_scalar_and_explicit_notdef(){
    let b=f12(format12(&[(0x41,0x42,0)]));let c=parse(&b,CmapLimits::default()).unwrap();
    assert_eq!(c.glyph_for_char('A').unwrap(),None);assert_eq!(c.glyph_for_char('B').unwrap(),Some(1));
    for x in [0xd800,0xdfff,0x110000,u32::MAX]{assert!(c.glyph_id(x).is_err());}
}
#[test]fn format4_advisory_search_fields(){
    let mut t=format4();p16(&mut t,8,0);let b=f4(t);let c=parse(&b,CmapLimits::default()).unwrap();
    assert_eq!(c.format4_search_fields_valid(),Some(false));assert_eq!(c.glyph_for_char('A').unwrap(),Some(1));
}
#[test]fn format4_bad_segments_and_sentinel(){
    for (at,v) in [(6,0),(6,5),(20,1),(22,0x44),(24,0x43),(18,0xfffe)]{
        let mut t=format4();p16(&mut t,at,v);assert!(parse(&f4(t),CmapLimits::default()).is_err(),"field {at}");
    }
}
#[test]fn format4_bad_relative_glyph_offsets(){
    for offset in [1,2,6,0xfffe]{
        let mut t=format4();p16(&mut t,36,offset);assert!(parse(&f4(t),CmapLimits::default()).is_err());
    }
}
#[test]fn validates_all_selected_glyph_ids(){
    let mut t=format4();p16(&mut t,44,18);assert!(parse(&f4(t),CmapLimits::default()).is_err());
    let mut t=format4();p16(&mut t,28,18u16.wrapping_sub(0x41));assert!(parse(&f4(t),CmapLimits::default()).is_err());
    assert!(parse(&f12(format12(&[(0x41,0x43,18)])),CmapLimits::default()).is_err());
    assert!(parse(&f12(format12(&[(0x41,0x42,u32::MAX)])),CmapLimits::default()).is_err());
}
#[test]fn format12_invalid_groups(){
    for groups in [vec![(5,4,1)],vec![(1,3,1),(3,4,2)],vec![(5,6,1),(1,2,2)],vec![(0xd7ff,0xd800,1)],vec![(0x110000,0x110000,1)]]{
        assert!(parse(&f12(format12(&groups)),CmapLimits::default()).is_err());
    }
}
#[test]fn truncated_selected_subtables(){
    for (platform,encoding,t) in [(3,1,format4()),(3,10,format12(&[(0x41,0x42,1)]))]{
        for n in 0..t.len(){let b=font(cmap(&[(platform,encoding,t[..n].to_vec())]),20,false);
            assert!(parse(&b,CmapLimits::default()).is_err(),"truncated {n}");}
    }
}
#[test]fn record_offsets_and_ambiguous_encoding(){
    let mut t=cmap(&[(3,1,format4())]);p32(&mut t,8,4);assert!(parse(&font(t,20,false),CmapLimits::default()).is_err());
    let mut t=cmap(&[(3,1,format4())]);p32(&mut t,8,u32::MAX);assert!(parse(&font(t,20,false),CmapLimits::default()).is_err());
    let t=cmap(&[(3,1,format4()),(3,1,format4())]);assert!(parse(&font(t,20,false),CmapLimits::default()).is_err());
    let mut t=cmap(&[(3,1,format4()),(0,3,format4())]);p32(&mut t,16,20);
    let b=font(t,20,false);assert!(parse(&b,CmapLimits::default()).is_ok());
}
#[test]fn bounded_limits_and_unsupported_encoding(){
    let b=f4(format4());
    for l in [CmapLimits{max_records:0,..Default::default()},CmapLimits{max_segments:2,..Default::default()},CmapLimits{max_validation_codes:6,..Default::default()}]{
        assert!(parse(&b,l).is_err());
    }
    assert!(parse(&f12(format12(&[(0x41,0x42,1)])),CmapLimits{max_groups:0,..Default::default()}).is_err());
    let b=font(cmap(&[(3,0,format4())]),20,false);assert!(matches!(parse(&b,CmapLimits::default()),Err(Error::Unsupported(_))));
}
#[test]fn maxp_count_and_flavor_validation(){
    let b=font(cmap(&[(3,1,format4())]),0,false);assert!(parse(&b,CmapLimits::default()).is_err());
    let b=font(cmap(&[(3,1,format4())]),8,false);assert!(parse(&b,CmapLimits::default()).is_err());
    let mut b=f4(format4());b[..4].copy_from_slice(b"OTTO");assert!(parse(&b,CmapLimits::default()).is_err());
}
