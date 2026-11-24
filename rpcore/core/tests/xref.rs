use rpcore::{Document,Object,Error};
fn object(pdf:&mut Vec<u8>,id:u32,gen:u16,body:&[u8])->usize{
    let off=pdf.len();
    pdf.extend_from_slice(format!("{id} {gen} obj\n").as_bytes());
    pdf.extend_from_slice(body);
    pdf.extend_from_slice(b"\nendobj\n");
    off
}
fn stream(pdf:&mut Vec<u8>,id:u32,dict:&str,data:&[u8])->usize{
    let mut body=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    object(pdf,id,0,&body)
}
fn row(out:&mut Vec<u8>,kind:u8,b:usize,c:u16){
    out.push(kind);
    out.extend_from_slice(&(b as u32).to_be_bytes());
    out.extend_from_slice(&c.to_be_bytes());
}
fn tail(pdf:&mut Vec<u8>,xref:usize){
    pdf.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
}
// RFC1950 stored block generated from the exact test payload, not a hand-written
// compressed hex string. Used to exercise the Flate + XRef integration.
fn zlib(data:&[u8])->Vec<u8>{
    let n=u16::try_from(data.len()).unwrap();
    let mut out=vec![0x78,0x01,1];
    out.extend_from_slice(&n.to_le_bytes());out.extend_from_slice(&(!n).to_le_bytes());
    out.extend_from_slice(data);
    let (mut a,mut b)=(1u32,0u32);
    for &v in data{a=(a+u32::from(v))%65521;b=(b+a)%65521;}
    out.extend_from_slice(&((b<<16)|a).to_be_bytes());
    out
}
fn modern(wrong_index:bool,hybrid:bool)->Vec<u8>{
    let mut pdf=b"%PDF-1.5\n".to_vec();
    let bodies=[b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>",b"12"];
    let mut header=String::new();
    let mut payload=Vec::new();
    for (id,body) in [1,2,3,6].into_iter().zip(bodies){
        header.push_str(&format!("{id} {} ",payload.len()));
        payload.extend_from_slice(body);payload.push(b'\n');
    }
    let first=header.len();
    let mut container=header.into_bytes();
    container.extend(payload);
    let o4=stream(&mut pdf,4,&format!("/Type /ObjStm /N 4 /First {first} /Filter /FlateDecode"),&zlib(&container));
    let o5=object(&mut pdf,5,0,b"<< /Length 6 0 R >>\nstream\nBT (Hi) Tj Q\nendstream");
    let o7=pdf.len();
    let mut rows=Vec::new();
    row(&mut rows,0,0,65535);
    row(&mut rows,2,4,0);row(&mut rows,2,4,1);row(&mut rows,2,4,if wrong_index{1}else{2});
    row(&mut rows,1,o4,0);row(&mut rows,1,o5,0);row(&mut rows,2,4,3);row(&mut rows,1,o7,0);
    let x=stream(&mut pdf,7,"/Type /XRef /Size 8 /Root 1 0 R /W [1 4 2] /Filter /FlateDecode",&zlib(&rows));
    assert_eq!(x,o7);
    if hybrid{
        let table=pdf.len();
        pdf.extend_from_slice(format!("xref\n0 8\n0000000000 65535 f \n0000000000 00000 f \n0000000000 00000 f \n0000000000 00000 f \n{o4:010} 00000 n \n{o5:010} 00000 n \n0000000000 00000 f \n{o7:010} 00000 n \ntrailer\n<< /Size 8 /Root 1 0 R /XRefStm {x} >>\n").as_bytes());
        tail(&mut pdf,table);
    }else{tail(&mut pdf,x);}
    pdf
}
#[test]
fn modern_pdf_and_hybrid(){
    for hybrid in [false,true]{
        let bytes=modern(false,hybrid);
        let doc=Document::open(&bytes).unwrap();
        assert_eq!(doc.page_count().unwrap(),1);
        let pages=doc.pages().unwrap();
        assert_eq!(pages[0].media_box,[0.,0.,612.,792.]);
        assert_eq!(doc.page_content(&pages[0],100).unwrap(),b"BT (Hi) Tj Q");
        assert_eq!(doc.resolve(6,0).unwrap(),Object::Number(12.));
        assert!(doc.resolve(6,1).is_err());
        assert!(doc.resolve(0,65535).is_err());
    }
}
#[test]
fn compressed_identity_check(){
    let bytes=modern(true,false);
    let doc=Document::open(&bytes).unwrap();
    assert!(doc.pages().is_err());
}
fn old_pdf()->(Vec<u8>,usize){
    let mut pdf=b"%PDF-1.4\n".to_vec();
    let a=object(&mut pdf,1,0,b"<< /Type /Catalog /Pages 2 0 R >>");
    let b=object(&mut pdf,2,0,b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 100 200] >>");
    let c=object(&mut pdf,3,0,b"<< /Type /Page /Parent 2 0 R >>");
    let d=object(&mut pdf,4,0,b"42");
    let x=pdf.len();
    pdf.extend_from_slice(format!("xref\n0 5\n0000000000 65535 f \n{a:010} 00000 n \n{b:010} 00000 n \n{c:010} 00000 n \n{d:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\n").as_bytes());
    tail(&mut pdf,x);
    (pdf,x)
}
#[test]
fn incremental_generation_and_tombstones(){
    let (mut pdf,prev)=old_pdf();
    let a=object(&mut pdf,3,1,b"<< /Type /Page /Parent 2 0 R /Rotate 90 >>");
    let b=object(&mut pdf,2,0,b"<< /Type /Pages /Kids [3 1 R] /Count 1 /MediaBox [0 0 100 200] >>");
    let x=pdf.len();
    pdf.extend_from_slice(format!("xref\n2 3\n{b:010} 00000 n \n{a:010} 00001 n \n0000000000 00001 f \ntrailer\n<< /Size 5 /Prev {prev} >>\n").as_bytes());
    tail(&mut pdf,x);
    let doc=Document::open(&pdf).unwrap();
    assert_eq!(doc.pages().unwrap()[0].rotation,90);
    assert!(doc.resolve(3,0).is_err());
    assert!(doc.resolve(4,0).is_err());
}
#[test]
fn incremental_xref_stream_sparse_and_default_type(){
    let (mut pdf,prev)=old_pdf();
    let a=object(&mut pdf,4,0,b"99");
    let x=pdf.len();
    let mut data=Vec::new();
    data.extend_from_slice(&(a as u32).to_be_bytes());data.extend_from_slice(&(x as u32).to_be_bytes());
    stream(&mut pdf,5,&format!("/Type /XRef /Size 6 /W [0 4 0] /Index [4 1 5 1] /Prev {prev}"),&data);
    tail(&mut pdf,x);
    let doc=Document::open(&pdf).unwrap();
    assert_eq!(doc.resolve(4,0).unwrap(),Object::Number(99.));
    assert_eq!(doc.page_count().unwrap(),1);
}
#[test]
fn malformed_xref_and_cycle(){
    let (mut pdf,_)=old_pdf();
    let x=pdf.len();
    pdf.extend_from_slice(format!("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {x} >>\n").as_bytes());
    tail(&mut pdf,x);
    assert!(matches!(Document::open(&pdf),Err(Error::Invalid(_))));
    for (params,data) in [
        ("/W [1 4 2] /Index [0 2]",vec![0;7]),
        ("/W [1 4 2] /Index [1 2 2 1]",vec![0;21]),
        ("/W [9 0 0]",vec![0;18]),
    ]{
        let mut p=b"%PDF-1.5\n".to_vec();
        let off=stream(&mut p,1,&format!("/Type /XRef /Size 3 /Root 1 0 R {params}"),&data);
        tail(&mut p,off);
        assert!(Document::open(&p).is_err());
    }
}
