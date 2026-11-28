// Original synthetic outline fixture. No external fonts, no host font dependency.
fn u16b(v:&mut Vec<u8>,n:u16){v.extend_from_slice(&n.to_be_bytes());}
fn i16b(v:&mut Vec<u8>,n:i16){v.extend_from_slice(&n.to_be_bytes());}
fn u32b(v:&mut Vec<u8>,n:u32){v.extend_from_slice(&n.to_be_bytes());}
fn simple(contours:&[&[(i16,i16,bool)]])->Vec<u8>{
    let mut g=Vec::new();i16b(&mut g,contours.len() as i16);
    for n in [0,0,600,700]{i16b(&mut g,n);}
    let mut n=0;for c in contours{n+=c.len();u16b(&mut g,(n-1) as u16);}
    u16b(&mut g,0);
    let points:Vec<_>=contours.iter().flat_map(|c|c.iter().copied()).collect();
    for &(_,_,on) in &points{g.push(u8::from(on));}
    let(mut x,mut y)=(0,0);
    for &(a,_,_) in &points{i16b(&mut g,a-x);x=a;}
    for &(_,a,_) in &points{i16b(&mut g,a-y);y=a;}
    g
}
pub fn font(cycle:bool)->Vec<u8>{font_with_cmap(cycle,None)}
pub fn font_with_cmap(cycle:bool,cmap:Option<Vec<u8>>)->Vec<u8>{
    let g1=simple(&[&[(0,0,true),(600,0,true),(600,700,true),(0,700,true)],&[(200,200,true),(200,500,true),(400,500,true),(400,200,true)]]);
    let g2=simple(&[&[(0,0,true),(0,600,false),(600,600,false),(600,0,true)]]);
    let mut g3=Vec::new();i16b(&mut g3,-1);for n in [100,50,400,400]{i16b(&mut g3,n);}
    u16b(&mut g3,11);u16b(&mut g3,if cycle{3}else{1});i16b(&mut g3,100);i16b(&mut g3,50);i16b(&mut g3,8192);
    let mut glyf=Vec::new();let mut loca=Vec::new();u32b(&mut loca,0);u32b(&mut loca,0);
    for g in [g1,g2,g3]{glyf.extend(g);while glyf.len()%4!=0{glyf.push(0);}u32b(&mut loca,glyf.len() as u32);}
    let mut head=vec![0;54];head[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());head[12..16].copy_from_slice(&0x5f0f3cf5u32.to_be_bytes());head[18..20].copy_from_slice(&1000u16.to_be_bytes());head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut maxp=vec![0;32];maxp[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());maxp[4..6].copy_from_slice(&4u16.to_be_bytes());
    let mut hhea=vec![0;36];hhea[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());hhea[34..36].copy_from_slice(&2u16.to_be_bytes());
    let mut hmtx=Vec::new();u16b(&mut hmtx,650);i16b(&mut hmtx,0);u16b(&mut hmtx,650);i16b(&mut hmtx,0);i16b(&mut hmtx,0);i16b(&mut hmtx,100);
    let mut tables=vec![(b"glyf",glyf),(b"head",head),(b"hhea",hhea),(b"hmtx",hmtx),(b"loca",loca),(b"maxp",maxp)];if let Some(c)=cmap{tables.push((b"cmap",c));}
    let mut f=Vec::new();u32b(&mut f,0x00010000);u16b(&mut f,tables.len() as u16);u16b(&mut f,64);u16b(&mut f,2);u16b(&mut f,32);
    f.resize(12+16*tables.len(),0);
    for(i,(tag,data))in tables.into_iter().enumerate(){
        let off=f.len();let p=12+i*16;f[p..p+4].copy_from_slice(tag);
        f[p+8..p+12].copy_from_slice(&(off as u32).to_be_bytes());f[p+12..p+16].copy_from_slice(&(data.len() as u32).to_be_bytes());f.extend(data);while f.len()%4!=0{f.push(0);}
    }f
}
fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut o=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();o.extend_from_slice(data);o.extend_from_slice(b"\nendstream");o
}
pub fn pdf(content:&[u8])->Vec<u8>{
    pdf_impl(content,font(false),None)
}
pub fn simple_pdf(content:&[u8],font:Vec<u8>,encoding:&str,flags:u32)->Vec<u8>{pdf_impl(content,font,Some((encoding,flags)))}
fn pdf_impl(content:&[u8],font:Vec<u8>,simple:Option<(&str,u32)>)->Vec<u8>{
    let mut objects=vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 140 90] /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        stream("",content),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /RPCoreFixture /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /RPCoreFixture /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 650 /CIDToGIDMap /Identity /FontDescriptor 7 0 R >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /RPCoreFixture /Flags 4 /FontBBox [0 0 600 700] /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 100 /FontFile2 8 0 R >>".to_vec(),
        stream(&format!("/Length1 {}",font.len()),&font),
        stream("",b"1 begincodespacerange <0000> <ffff> endcodespacerange 3 beginbfchar <0001> <004f> <0002> <0044> <0003> <006f> endbfchar")
    ];
    if let Some((encoding,flags))=simple{
        objects[4]=format!("<< /Type /Font /Subtype /TrueType /BaseFont /RPCoreFixture {encoding} /FirstChar 65 /LastChar 66 /Widths [650 650] /FontDescriptor 7 0 R >>").into_bytes();
        objects[5]=b"null".to_vec();
        objects[6]=format!("<< /Type /FontDescriptor /FontName /RPCoreFixture /Flags {flags} /FontBBox [0 0 600 700] /ItalicAngle 0 /Ascent 700 /Descent 0 /CapHeight 700 /StemV 100 /FontFile2 8 0 R >>").into_bytes();
    }
    let mut p=b"%PDF-1.5\n".to_vec();let mut offs=Vec::new();
    for(i,o)in objects.iter().enumerate(){offs.push(p.len());p.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());p.extend_from_slice(o);p.extend_from_slice(b"\nendobj\n");}
    let x=p.len();p.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offs{p.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    p.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());p
}

pub fn cmap_table(platform:u16,encoding:u16,sub:&[u8])->Vec<u8>{
    let mut c=vec![0,0,0,1];u16b(&mut c,platform);u16b(&mut c,encoding);u32b(&mut c,12);c.extend(sub);c
}
pub fn cmap6(start:u16,gids:&[u16])->Vec<u8>{
    let mut c=Vec::new();for x in [6,(10+2*gids.len()) as u16,0,start,gids.len()as u16]{u16b(&mut c,x);}
    for &g in gids{u16b(&mut c,g);}c
}
