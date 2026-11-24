// Minimal original PDF fixture with nested isolated Form transparency groups.
pub fn stream(dict:&str,data:&[u8])->Vec<u8>{
    let mut out=format!("<< {dict} /Length {} >>\nstream\n",data.len()).into_bytes();
    out.extend_from_slice(data);out.extend_from_slice(b"\nendstream");out
}
pub fn pdf(page:&[u8],group:&str,body:&[u8],nested:&[u8],outer_gs:&str,inner_gs:&str)->Vec<u8>{
    pdf_nested(page,group,body,nested,outer_gs,inner_gs,"<< /S /Transparency /I true /CS /DeviceRGB >>")
}
pub fn pdf_nested(page:&[u8],group:&str,body:&[u8],nested:&[u8],outer_gs:&str,inner_gs:&str,nested_group:&str)->Vec<u8>{
    let resources="/Resources << /XObject << /G 5 0 R /N 6 0 R >> /ExtGState << /Outer 7 0 R /Inner 8 0 R >> >>";
    let objects=[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 20 20] >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /Contents 4 0 R {resources} >>").into_bytes(),
        stream("",page),
        stream(&format!("/Type /XObject /Subtype /Form /BBox [0 0 15 15] /Group {group} {resources}"),body),
        stream(&format!("/Type /XObject /Subtype /Form /BBox [0 0 15 15] /Group {nested_group} {resources}"),nested),
        outer_gs.as_bytes().to_vec(),inner_gs.as_bytes().to_vec(),
    ];
    let mut out=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for (i,o) in objects.iter().enumerate(){offsets.push(out.len());out.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());out.extend_from_slice(o);out.extend_from_slice(b"\nendobj\n");}
    let x=out.len();out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes());
    for o in offsets{out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());}
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",objects.len()+1).as_bytes());out
}
pub const GROUP:&str="<< /Type /Group /S /Transparency /I true /K false /CS /DeviceRGB >>";
pub const OVERLAP:&[u8]=b"1 0 0 rg 0 0 10 10 re f 0 0 1 rg 5 0 10 10 re f";
