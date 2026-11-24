use rpcore::{filters,Object,Document};
fn stream(dict: &[u8], data: &[u8]) -> Object {
    let Object::Dict(dict)=rpcore::parse_object(dict).unwrap() else { panic!("dict") };
    Object::Stream {dict,data:data.to_vec()}
}
fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
#[test]
fn ascii85_flate_chain() {
    let s=stream(b"<< /Filter [/ASCII85Decode /FlateDecode] /DecodeParms [null null] >>",&[71,97,113,69,35,56,108,33,60,90,33,45,55,33,40,48,96,126,62]);
    assert_eq!(filters::decode(&s,24).unwrap(),b"abc".repeat(8));
    assert!(filters::decode(&s,23).is_err());
}
#[test]
fn flate_predictor() {
    let s=stream(b"<< /Filter /FlateDecode /DecodeParms << /Predictor 15 /Columns 3 >> >>",&hex("789c63e4e2e262626462060000d60028"));
    assert_eq!(filters::decode(&s,6).unwrap(),[10,20,30,11,22,33]);
    assert!(filters::decode(&s,5).is_err());
}
#[test]
fn compressed_page_content() {
    let compressed=hex("789c730a51d0773354303452084953d0f048cdc9c9d75408c952700d010054a906b6");
    let mut pdf=b"%PDF-1.4\n".to_vec();
    let mut offsets=Vec::new();
    for (i,o) in [b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>"].iter().enumerate(){
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());
        pdf.extend_from_slice(o);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n",compressed.len()).as_bytes());
    pdf.extend_from_slice(&compressed);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let x=pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for o in offsets {pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").as_bytes());
    let doc=Document::open(&pdf).unwrap();
    let pages=doc.pages().unwrap();
    let ops=doc.page_operations(&pages[0],100).unwrap();
    assert_eq!(ops.iter().map(|o|o.operator.as_slice()).collect::<Vec<_>>(),vec![b"BT".as_slice(),b"Tf",b"Tj",b"ET"]);
}
