// Original synthetic Type1 program; no external fonts.
fn encrypt(d:&[u8],mut key:u16)->Vec<u8>{
    let mut v=Vec::new();
    for &p in d{let c=p^(key>>8) as u8;v.push(c);key=(u16::from(c).wrapping_add(key)).wrapping_mul(52845).wrapping_add(22719);}
    v
}
fn operand(v:&mut Vec<u8>,n:i32){
    if (-107..=107).contains(&n){v.push((n+139) as u8);}
    else{v.push(255);v.extend_from_slice(&n.to_be_bytes());}
}
fn op(v:&mut Vec<u8>,a:&[i32],o:u8){for &n in a{operand(v,n);}v.push(o);}
fn charstring(v:&[u8],len_iv:i32)->Vec<u8>{
    if len_iv==-1{return v.to_vec();}
    let mut p=vec![17;len_iv as usize];p.extend(v);encrypt(&p,4330)
}
pub fn sections(len_iv:i32,recursive:bool)->(Vec<u8>,Vec<u8>,Vec<u8>){
    let clear=b"%!PS-AdobeFont-1.0: RPCoreType1 1.0\n11 dict begin\n/FontName /RPCoreType1 def\n/FontType 1 def\n/FontMatrix [0.001 0 0 0.001 0 0] readonly def\n/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\ndup 65 /A put\ndup 66 /B put\nreadonly def\ncurrentdict end\ncurrentfile eexec\n".to_vec();
    let mut a=Vec::new();op(&mut a,&[20,650],13);op(&mut a,&[0,0],21);op(&mut a,&[0],10);
    op(&mut a,&[700],7);op(&mut a,&[-600],6);op(&mut a,&[-700],7);a.push(9);
    op(&mut a,&[200,200],21);op(&mut a,&[300],7);op(&mut a,&[200],6);op(&mut a,&[-300],7);op(&mut a,&[-200],6);a.extend([9,14]);
    let mut b=Vec::new();op(&mut b,&[20,650],13);op(&mut b,&[0,0],21);op(&mut b,&[0,700,600,0,0,-700],8);op(&mut b,&[-600],6);b.extend([9,14]);
    let mut sub=Vec::new();if recursive{op(&mut sub,&[0],10);}else{op(&mut sub,&[600],6);}sub.push(11);
    let mut nd=Vec::new();op(&mut nd,&[0,650],13);nd.push(14);
    let mut private=vec![0,0,0,0];private.extend(format!("dup /Private 8 dict dup begin\n/lenIV {len_iv} def\n/RD {{string currentfile exch readstring pop}} executeonly def\n/Subrs 1 array\n").as_bytes());
    let s=charstring(&sub,len_iv);private.extend(format!("dup 0 {} RD ",s.len()).as_bytes());private.extend(s);private.extend(b" NP\nreadonly def\n/CharStrings 3 dict dup begin\n");
    for (name,g) in [(".notdef",nd),("A",a),("B",b)]{
        let s=charstring(&g,len_iv);
        private.extend(format!("/{name} {} -| ",s.len()).as_bytes());private.extend(s);private.extend(b" ND\n");
    }
    private.extend(b"end\nend\ncurrentfile closefile\n");
    let enc=encrypt(&private,55665);let mut tail=vec![b'0';512];tail.extend(b"\ncleartomark\n");(clear,enc,tail)
}
pub fn pfa()->Vec<u8>{
    let (mut c,e,t)=sections(4,false);
    for b in e{c.extend(format!("{b:02x}").as_bytes());}c.push(b'\n');c.extend(t);c
}
pub fn pfb()->Vec<u8>{
    let (c,e,t)=sections(4,false);let mut out=Vec::new();
    for (kind,data) in [(1u8,c),(2,e),(1,t)]{out.extend([0x80,kind]);out.extend((data.len() as u32).to_le_bytes());out.extend(data);}
    out.extend([0x80,3]);out
}
fn stream(dict:&str,d:&[u8])->Vec<u8>{
    let mut o=format!("<< {dict} /Length {} >>\nstream\n",d.len()).into_bytes();o.extend(d);o.extend(b"\nendstream");o
}
pub fn pdf(content:&[u8],encoding:&str)->Vec<u8>{
    let (mut c,e,t)=sections(4,false);let dict=format!("/Length1 {} /Length2 {} /Length3 {}",c.len(),e.len(),t.len());c.extend(e);c.extend(t);
    let objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 140 90] /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),stream("",content),
        format!("<< /Type /Font /Subtype /Type1 /BaseFont /RPCoreType1 {encoding} /FirstChar 65 /LastChar 66 /Widths [650 650] /FontDescriptor 6 0 R >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /RPCoreType1 /Flags 4 /FontBBox [0 0 650 700] /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 100 /FontFile 7 0 R >>".to_vec(),stream(&dict,&c)];
    let mut p=b"%PDF-1.4\n".to_vec();let mut offs=Vec::new();
    for(i,o)in objects.iter().enumerate(){offs.push(p.len());p.extend(format!("{} 0 obj\n",i+1).as_bytes());p.extend(o);p.extend(b"\nendobj\n");}
    let x=p.len();p.extend(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offs{p.extend(format!("{o:010} 00000 n \n").as_bytes());}
    p.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());p
}
