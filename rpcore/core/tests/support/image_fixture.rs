// JPEG fixtures independently encoded using Pillow/libjpeg; no runtime dependencies.
fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|a|u8::from_str_radix(std::str::from_utf8(a).unwrap(),16).unwrap()).collect()}
pub fn gray()->Vec<u8>{hex("ffd8ffe000104a46494600010100000100010000ffdb0043000503040404030504040405050506070c08070707070f0b0b090c110f1212110f111113161c1713141a1511111821181a1d1d1f1f1f13172224221e241c1e1f1effc0000b080009000901011100ffc4001500010100000000000000000000000000000007ffc40014100100000000000000000000000000000000ffda0008010100003f009d00ffd9")}
pub fn rgb()->Vec<u8>{hex("ffd8ffe000104a46494600010100000100010000ffdb0043000503040404030504040405050506070c08070707070f0b0b090c110f1212110f111113161c1713141a1511111821181a1d1d1f1f1f13172224221e241c1e1f1effdb0043010505050706070e08080e1e1411141e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1effc00011080009001103012200021101031101ffc4001500010100000000000000000000000000000006ffc40014100100000000000000000000000000000000ffc400160101010100000000000000000000000000000507ffc40014110100000000000000000000000000000000ffda000c03010002110311003f00840131be8003ffd9")}
pub fn gradient()->Vec<u8>{hex("ffd8ffe000104a46494600010100000100010000ffdb0043000302020302020303030304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d0e110e0b0b1016101113141515150c0f171816141812141514ffc0000b080008000801011100ffc40014000100000000000000000000000000000006ffc40017100003010000000000000000000000000000094582ffda0008010100003f0004bba664ffd9")}
pub fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut b=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();b.extend_from_slice(data);b.extend_from_slice(b"\nendstream");b
}
pub fn pdf(content:&[u8],im:Vec<u8>,mask:Vec<u8>,gs:&str)->Vec<u8>{
    let objs=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 20 20] >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /XObject << /Im 5 0 R >> /ExtGState << /Half 7 0 R >> >> >>".to_vec(),stream("",content),im,mask,gs.as_bytes().to_vec()];
    let mut p=b"%PDF-1.4\n".to_vec();let mut offs=Vec::new();
    for(i,o)in objs.iter().enumerate(){offs.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objs.len()+1).as_bytes());
    for off in offs{p.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objs.len()+1).as_bytes());p
}
