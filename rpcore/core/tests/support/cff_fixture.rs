// Original synthetic CFF1 fixtures, no external fonts.
fn index(items:&[Vec<u8>])->Vec<u8>{
    let mut b=(items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty(){return b;}
    b.push(2);let mut off=1u16;b.extend_from_slice(&off.to_be_bytes());
    for v in items{off+=v.len() as u16;b.extend_from_slice(&off.to_be_bytes());}
    for v in items{b.extend_from_slice(v);}b
}
fn di(b:&mut Vec<u8>,n:usize){b.push(29);b.extend_from_slice(&(n as u32).to_be_bytes());}
fn op(b:&mut Vec<u8>,args:&[i16],opcode:u8){for n in args{b.push(28);b.extend_from_slice(&n.to_be_bytes());}b.push(opcode);}
fn top(cid:bool,charset:usize,chars:usize,fd:usize,select:usize,private:usize)->Vec<u8>{
    let mut b=Vec::new();
    if cid{for n in [391,392,0]{di(&mut b,n);}b.extend([12,30]);}
    di(&mut b,charset);b.push(15);di(&mut b,chars);b.push(17);
    if cid{di(&mut b,fd);b.extend([12,36]);di(&mut b,select);b.extend([12,37]);}
    else{di(&mut b,6);di(&mut b,private);b.push(18);}
    b
}
fn fd(private:usize)->Vec<u8>{let mut d=Vec::new();di(&mut d,6);di(&mut d,private);d.push(18);index(&[d])}
pub fn font(cid:bool,recursive:bool)->Vec<u8>{
    let mut first=Vec::new();
    op(&mut first,&[0,0],21);op(&mut first,&[-107],29);
    op(&mut first,&[0,700,-600,0,0,-700],5);op(&mut first,&[200,200],21);
    op(&mut first,&[0,300,200,0,0,-300,-200,0],5);first.push(14);
    let mut second=Vec::new();op(&mut second,&[0,0],21);op(&mut second,&[-107],10);op(&mut second,&[0,-700],5);second.push(14);
    let mut global=Vec::new();if recursive{op(&mut global,&[-107],29);}else{op(&mut global,&[600,0],5);}global.push(11);
    let mut local=Vec::new();op(&mut local,&[0,700,600,0,0,-700],8);local.push(11);
    let chars=index(&[vec![14],first,second]);
    let mut charset=vec![0];for n in if cid{[42u16,77]}else{[391u16,392]}{charset.extend_from_slice(&n.to_be_bytes());}
    let strings=if cid{index(&[b"Adobe".to_vec(),b"Identity".to_vec()])}else{index(&[b"outlineA".to_vec(),b"outlineB".to_vec()])};
    let mut prefix=vec![1,0,4,2];prefix.extend(index(&[b"RPCoreCFF".to_vec()]));
    let top_len=index(&[top(cid,0,0,0,0,0)]).len();
    let globals=index(&[global]);
    let charset_off=prefix.len()+top_len+strings.len()+globals.len();
    let chars_off=charset_off+charset.len();let fd_off=chars_off+chars.len();
    let fds=if cid{fd(0).len()}else{0};
    // FDSelect format 3: one range [0,3) with FD 0.
    let select=if cid{vec![3,0,1,0,0,0,0,3]}else{vec![]};
    let select_off=fd_off+fds;let private_off=select_off+select.len();
    prefix.extend(index(&[top(cid,charset_off,chars_off,fd_off,select_off,private_off)]));
    prefix.extend(strings);prefix.extend(globals);prefix.extend(charset);prefix.extend(chars);
    if cid{prefix.extend(fd(private_off));prefix.extend(select);}
    di(&mut prefix,6);prefix.push(19);prefix.extend(index(&[local]));prefix
}
fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut o=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();o.extend_from_slice(data);o.extend_from_slice(b"\nendstream");o
}
pub fn pdf(cid:bool,content:&[u8])->Vec<u8>{
    let f=font(cid,false);
    let mut objects=vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 140 90] /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),stream("",content)];
    if cid{
        objects.push(b"<< /Type /Font /Subtype /Type0 /BaseFont /RPCoreCFF /Encoding /Identity-H /DescendantFonts [6 0 R] >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /RPCoreCFF /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 650 /FontDescriptor 7 0 R >>".to_vec());
    }else{
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /RPCoreCFF /Encoding << /Differences [65 /outlineA /outlineB] >> /FirstChar 65 /LastChar 66 /Widths [650 650] /FontDescriptor 7 0 R >>".to_vec());
        objects.push(b"null".to_vec());
    }
    objects.push(b"<< /Type /FontDescriptor /FontName /RPCoreCFF /Flags 4 /FontBBox [0 0 600 700] /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 100 /FontFile3 8 0 R >>".to_vec());
    objects.push(stream(if cid{"/Subtype /CIDFontType0C"}else{"/Subtype /Type1C"},&f));
    let mut p=b"%PDF-1.5\n".to_vec();let mut offs=Vec::new();
    for(i,o)in objects.iter().enumerate(){offs.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend_from_slice(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offs{p.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());p
}
