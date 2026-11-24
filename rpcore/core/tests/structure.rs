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

#[test]
fn indirect_length_and_ordered_contents() {
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
        b"<< /Type /Page /Parent 2 0 R /Contents [4 0 R 5 0 R] >>",
        b"<< /Length 6 0 R >>\nstream\nBT /F1 12 Tf\nendstream",
        b"<< /Length 19 /Filter /ASCIIHexDecode >>\nstream\n28486929546a204554>\nendstream",
        b"7 0 R",
        b"12",
    ]);
    let doc = Document::open(&bytes).unwrap();
    let page = &doc.pages().unwrap()[0];
    // Second stream decodes to "(Hi)Tj ET".
    assert_eq!(doc.page_content(page, 100).unwrap(), b"BT /F1 12 Tf\n(Hi)Tj ET");
    assert_eq!(doc.page_operations(page, 100).unwrap().len(), 4);
    assert!(doc.page_content(page, 12).is_err());
}
#[test]
fn indirect_length_cycles_and_non_scalar() {
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Length 3 0 R >>\nstream\nabc\nendstream",
        b"2 0 R",
        b"<< /Length 5 0 R >>\nstream\nabc\nendstream",
        b"1.5",
    ]);
    let doc = Document::open(&bytes).unwrap();
    assert!(doc.resolve(2,0).is_err());
    assert!(doc.resolve(4,0).is_err());
}
#[test]
fn page_operations_and_blank_page() {
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 612 792] >>",
        b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>",
        b"<< /Type /Page /Parent 2 0 R >>",
        b"<< /Length 12 >>\nstream\nBT (Hi) Tj Q\nendstream",
    ]);
    let doc = Document::open(&bytes).unwrap();
    let pages = doc.pages().unwrap();
    let ops = doc.page_operations(&pages[0], 100).unwrap();
    assert_eq!(ops[1].operator, b"Tj");
    assert_eq!(ops[1].operands, vec![Object::String(b"Hi".to_vec())]);
    assert!(doc.page_operations(&pages[1], 100).unwrap().is_empty());
}
