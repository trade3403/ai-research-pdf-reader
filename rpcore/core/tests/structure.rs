use rpcore::{Document,Object,filters};
fn pdf(objects: &[&[u8]]) -> Vec<u8> {
    let mut data=b"%PDF-1.4\n".to_vec();
    let mut offsets=Vec::new();
    for (i,obj) in objects.iter().enumerate() {
        offsets.push(data.len());
        data.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());
        data.extend_from_slice(obj);
        data.extend_from_slice(b"\nendobj\n");
    }
    let x=data.len();
    data.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offsets {data.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    data.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());
    data
}
#[test]
fn inherited_page_and_stream() {
    let bytes=pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] /Rotate -90 /Resources << >> >>",
        b"<< /Type /Page /Parent 2 0 R /CropBox [10 20 600 780] /Contents 4 0 R >>",
        b"<< /Length 5 /Filter /ASCIIHexDecode >>\nstream\n4142>\nendstream",
    ]);
    let doc=Document::open(&bytes).unwrap();
    let p=doc.pages().unwrap();
    assert_eq!(p[0].media_box,[0.0,0.0,612.0,792.0]);
    assert_eq!(p[0].crop_box,[10.0,20.0,600.0,780.0]);
    assert_eq!(p[0].rotation,270);
    assert!(matches!(p[0].resources,Some(Object::Dict(_))));
    assert_eq!(filters::decode(&doc.resolve(4,0).unwrap(),10).unwrap(),b"AB");
}
#[test]
fn truncated_stream() {
    let bytes=pdf(&[b"<< /Type /Catalog /Pages 2 0 R >>",b"<< /Length 999999 >>\nstream\nabc\nendstream"]);
    assert!(Document::open(&bytes).unwrap().resolve(2,0).is_err());
}
