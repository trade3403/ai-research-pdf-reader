use rwcore::*;
fn put16(b:&mut [u8],at:usize,n:u16){b[at..at+2].copy_from_slice(&n.to_be_bytes());}
fn put32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_be_bytes());}
fn put_tag(b:&mut [u8],at:usize,t:[u8;4]){b[at..at+4].copy_from_slice(&t);}
fn directory(b:&mut [u8],at:usize,signature:[u8;4],records:&[([u8;4],u32,u32,u32)]){
    put_tag(b,at,signature);put16(b,at+4,records.len() as u16);
    let p=1usize<<(usize::BITS-1-records.len().leading_zeros());
    put16(b,at+6,(p*16) as u16);put16(b,at+8,p.trailing_zeros() as u16);put16(b,at+10,((records.len()-p)*16) as u16);
    for (i,(t,c,o,l)) in records.iter().enumerate(){let p=at+12+i*16;put_tag(b,p,*t);put32(b,p+4,*c);put32(b,p+8,*o);put32(b,p+12,*l);}
}
fn sfnt()->Vec<u8>{
    let mut b=vec![0;64];directory(&mut b,0,[0,1,0,0],&[(*b"maxp",0x01020300,44,3),(*b"head",3,48,12)]);
    b[44..47].copy_from_slice(&[1,2,3]);put32(&mut b,48,1);put32(&mut b,52,2);put32(&mut b,56,0x12345678);b
}
fn ttc(v2:bool)->Vec<u8>{
    let mut b=vec![0;108];put_tag(&mut b,0,*b"ttcf");put32(&mut b,4,if v2{0x20000}else{0x10000});put32(&mut b,8,2);
    put32(&mut b,12,32);put32(&mut b,16,60);
    directory(&mut b,32,[0,1,0,0],&[(*b"head",3,88,12)]);
    directory(&mut b,60,*b"OTTO",&[(*b"head",3,88,12)]);
    put32(&mut b,88,1);put32(&mut b,92,2);
    b
}
fn parse(b:&[u8])->Result<FontContainer<'_>,Error>{parse_font(b,FontLimits::default())}
#[test]fn single_true_type_and_sorted_borrowed_tables(){
    let b=sfnt();let c=parse(&b).unwrap();assert_eq!(c.kind(),FontContainerKind::Single);
    let f=c.face(0).unwrap();assert_eq!(f.flavor(),FontFlavor::TrueType);assert!(f.search_fields_valid());
    assert_eq!(f.tables()[0].tag,*b"head");assert_eq!(f.table(*b"maxp"),Some(&b[44..47]));
    assert!(f.table(*b"cmap").is_none());assert!(c.face(1).is_none());
}
#[test]fn opentype_cff_signature(){
    let mut b=sfnt();put_tag(&mut b,0,*b"OTTO");assert_eq!(parse(&b).unwrap().face(0).unwrap().flavor(),FontFlavor::OpenTypeCff);
}
#[test]fn table_checksums_padding_and_head_adjustment(){
    let mut b=sfnt();let c=parse(&b).unwrap();let f=c.face(0).unwrap();
    assert!(f.verify_table_checksum(*b"maxp").unwrap());assert!(f.verify_table_checksum(*b"head").unwrap());
    assert!(f.verify_table_checksum(*b"cmap").is_err());
    b[45]=9;assert!(!parse(&b).unwrap().face(0).unwrap().verify_table_checksum(*b"maxp").unwrap());
}
#[test]fn search_fields_not_trusted(){
    let mut b=sfnt();put16(&mut b,6,0);let c=parse(&b).unwrap();assert!(!c.face(0).unwrap().search_fields_valid());
    assert!(c.face(0).unwrap().table(*b"head").is_some());
}
#[test]fn collection_absolute_shared_table(){
    let b=ttc(false);let c=parse(&b).unwrap();assert_eq!(c.kind(),FontContainerKind::CollectionV1);assert_eq!(c.faces().len(),2);
    assert_eq!(c.face(1).unwrap().offset(),60);assert_eq!(c.face(0).unwrap().table(*b"head"),c.face(1).unwrap().table(*b"head"));
    assert!(c.face(1).unwrap().verify_table_checksum(*b"head").unwrap());
}
#[test]fn collection_v2_signature_raw_and_absent(){
    let mut b=ttc(true);assert!(parse(&b).unwrap().collection_signature().is_none());
    put_tag(&mut b,20,*b"DSIG");put32(&mut b,24,8);put32(&mut b,28,100);b[100]=7;
    let c=parse(&b).unwrap();assert_eq!(c.kind(),FontContainerKind::CollectionV2);assert_eq!(c.collection_signature(),Some(&b[100..108]));
    put32(&mut b,28,88);assert!(parse(&b).is_err());
}
#[test]fn rejects_truncation_at_all_required_prefixes(){
    let b=sfnt();for n in 0..60{assert!(parse(&b[..n]).is_err(),"prefix {n}");}
    assert!(parse(&b[..60]).is_ok());
}
#[test]fn rejects_duplicate_tags_and_bad_tag(){
    let mut b=sfnt();put_tag(&mut b,28,*b"maxp");assert!(parse(&b).is_err());
    let mut b=sfnt();b[12]=0;assert!(parse(&b).is_err());
}
#[test]fn rejects_overflow_unaligned_and_directory_overlap(){
    for offset in [u32::MAX,45,12,60]{let mut b=sfnt();put32(&mut b,36,offset);assert!(parse(&b).is_err());}
    let mut b=sfnt();put32(&mut b,24,u32::MAX);assert!(parse(&b).is_err());
}
#[test]fn rejects_partial_alias_and_shared_checksum_mismatch(){
    let mut b=ttc(false);put32(&mut b,80,92);put32(&mut b,84,8);assert!(parse(&b).is_err());
    let mut b=ttc(false);put32(&mut b,76,4);assert!(parse(&b).is_err());
}
#[test]fn rejects_duplicate_face_and_bad_collection_headers(){
    let mut b=ttc(false);put32(&mut b,16,32);assert!(parse(&b).is_err());
    let mut b=ttc(false);put32(&mut b,4,0x30000);assert!(parse(&b).is_err());
    let mut b=ttc(false);put32(&mut b,8,0);assert!(parse(&b).is_err());
    let mut b=ttc(true);put32(&mut b,24,1);assert!(parse(&b).is_err());
}
#[test]fn limits_before_directory_allocation(){
    let b=ttc(false);for l in [FontLimits{max_bytes:100,..Default::default()},FontLimits{max_faces:1,..Default::default()},
        FontLimits{max_tables_per_face:0,..Default::default()},FontLimits{max_total_tables:1,..Default::default()}]{
        assert!(parse_font(&b,l).is_err());
    }
    assert!(parse_font(&sfnt(),FontLimits{max_faces:0,..Default::default()}).is_err());
}
#[test]fn rejects_unsupported_compressed_container_and_empty_directory(){
    let mut b=sfnt();put_tag(&mut b,0,*b"wOF2");assert!(matches!(parse(&b),Err(Error::Unsupported(_))));
    let mut b=sfnt();put16(&mut b,4,0);assert!(parse(&b).is_err());
}
#[test]fn short_head_checksum_rejected_and_empty_table_supported(){
    let mut b=sfnt();put32(&mut b,40,8);let c=parse(&b).unwrap();assert!(c.face(0).unwrap().verify_table_checksum(*b"head").is_err());
    let mut b=sfnt();put32(&mut b,24,0);put32(&mut b,16,0);let c=parse(&b).unwrap();
    assert_eq!(c.face(0).unwrap().table(*b"maxp"),Some(&b[44..44]));assert!(c.face(0).unwrap().verify_table_checksum(*b"maxp").unwrap());
}
