// JPEG fixtures generated with Pillow/libjpeg: quality=90, optimize=True.
// RGB uses subsampling=2. Input: L(9x9,85), RGB(17x9,200,39,70),
// L(8x8) with each row [0,30,60,90,120,150,180,210].
// Reference decoded pixels verified with Pillow; no runtime dependencies.
fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|a|u8::from_str_radix(std::str::from_utf8(a).unwrap(),16).unwrap()).collect()}
pub fn gray()->Vec<u8>{hex(concat!(
    "ffd8ffe000104a46494600010100000100010000ffdb00430003020203020203",
    "03030304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d",
    "0e110e0b0b1016101113141515150c0f171816141812141514ffc0000b080009",
    "000901011100ffc4001500010100000000000000000000000000000007ffc400",
    "14100100000000000000000000000000000000ffda0008010100003f008600ff",
    "d9",
))}
pub fn rgb()->Vec<u8>{hex(concat!(
    "ffd8ffe000104a46494600010100000100010000ffdb00430003020203020203",
    "03030304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d",
    "0e110e0b0b1016101113141515150c0f171816141812141514ffdb0043010304",
    "0405040509050509140d0b0d1414141414141414141414141414141414141414",
    "141414141414141414141414141414141414141414141414141414141414ffc0",
    "0011080009001103012200021101031101ffc400150001010000000000000000",
    "0000000000000007ffc40014100100000000000000000000000000000000ffc4",
    "00160101010100000000000000000000000000000608ffc40014110100000000",
    "000000000000000000000000ffda000c03010002110311003f008e009f6d0000",
    "7fffd9",
))}
pub fn gradient()->Vec<u8>{hex(concat!(
    "ffd8ffe000104a46494600010100000100010000ffdb00430003020203020203",
    "03030304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d",
    "0e110e0b0b1016101113141515150c0f171816141812141514ffc0000b080008",
    "000801011100ffc40014000100000000000000000000000000000006ffc40017",
    "100003010000000000000000000000000000094582ffda0008010100003f0004",
    "bba664ffd9",
))}
pub fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut b=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();b.extend_from_slice(data);b.extend_from_slice(b"\nendstream");b
}
pub fn pdf(content:&[u8],im:Vec<u8>,mask:Vec<u8>,gs:&str)->Vec<u8>{
    pdf_extra(content,im,mask,gs,Vec::new())
}
pub fn pdf_extra(content:&[u8],im:Vec<u8>,mask:Vec<u8>,gs:&str,extra:Vec<Vec<u8>>)->Vec<u8>{
    let mut objs=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 20 20] >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /XObject << /Im 5 0 R >> /ExtGState << /Half 7 0 R >> >> >>".to_vec(),stream("",content),im,mask,gs.as_bytes().to_vec()];
    objs.extend(extra);
    let mut p=b"%PDF-1.4\n".to_vec();let mut offs=Vec::new();
    for(i,o)in objs.iter().enumerate(){offs.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objs.len()+1).as_bytes());
    for off in offs{p.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objs.len()+1).as_bytes());p
}

// Pillow/libjpeg quality=90 optimize=True; reference pixels independently decoded.
pub fn progressive_gradient()->Vec<u8>{hex(concat!(
    "ffd8ffe000104a46494600010100000100010000ffdb00430003020203020203",
    "03030304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d",
    "0e110e0b0b1016101113141515150c0f171816141812141514ffc2000b080008",
    "000801011100ffc40014000100000000000000000000000000000005ffda0008",
    "01010000000103ffc4001510010100000000000000000000000000000007ffda",
    "00080101000105029dbfffc40015100101000000000000000000000000000000",
    "03ffda0008010100063f029bffc4001510010100000000000000000000000000",
    "0000c1ffda0008010100013f218bffda0008010100000010ff00ffc400141001",
    "00000000000000000000000000000000ffda0008010100013f106fffd9",
))}

// Pillow/libjpeg quality=90 optimize=True; reference pixels independently decoded.
pub fn cmyk()->Vec<u8>{hex(concat!(
    "ffd8ffee000e41646f626500640000000000ffdb004300030202030202030303",
    "0304030304050805050404050a070706080c0a0c0c0b0a0b0b0d0e12100d0e11",
    "0e0b0b1016101113141515150c0f171816141812141514ffc000140800080008",
    "044311004d11005911004b1100ffc40014000100000000000000000000000000",
    "000009ffc40014100100000000000000000000000000000000ffda000e044300",
    "4d0059004b00003f0054c550aa2a6fffd9",
))}
