//! Generate a reproducible, text-free PDF and render it through RPCore.
use rpcore::{Document,raster::Options};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let dest=std::env::args_os().nth(1).ok_or("usage: render_demo OUTPUT_DIR")?;
    std::fs::create_dir_all(&dest)?;let dest=std::path::PathBuf::from(dest);
    let contents=b"0.95 0.97 1 rg 0 0 420 300 re f 0.12 0.23 0.42 rg 20 245 380 35 re f 0.2 0.48 0.78 rg 35 45 75 160 re f 0.25 0.67 0.55 rg 135 45 75 115 re f 0.87 0.48 0.25 rg 235 45 75 190 re f 0.13 0.24 0.4 RG 3 w 1 J 1 j 25 38 m 395 38 l S 25 38 m 25 225 l S 0.9 0.3 0.3 RG 3 w 1 J 330 50 m 330 200 395 200 395 50 c S q 330 180 65 50 re W n 0.2 0.55 0.8 rg 320 170 90 80 re 340 190 30 20 re f* Q";
    let mut stream=format!("<< /Length {} >>\nstream\n",contents.len()).into_bytes();stream.extend_from_slice(contents);stream.extend_from_slice(b"\nendstream");
    let objects=[b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 420 300] >>".to_vec(),b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),stream];
    let mut pdf=b"%PDF-1.4\n".to_vec();let mut offsets=Vec::new();
    for(i,o)in objects.iter().enumerate(){offsets.push(pdf.len());pdf.extend_from_slice(format!("{} 0 obj\n",i+1).as_bytes());pdf.extend_from_slice(o);pdf.extend_from_slice(b"\nendobj\n");}
    let x=pdf.len();pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for off in offsets{pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());}
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n").as_bytes());
    let doc=Document::open(&pdf)?;let page=doc.pages()?;
    let bitmap=doc.render_page(&page[0],Options{scale:2.,allow_missing_text:false},1024*1024)?;
    std::fs::write(dest.join("rpcore-vector-demo.pdf"),&pdf)?;
    std::fs::write(dest.join("rpcore-vector-demo.png"),bitmap.png()?)?;
    println!("RPCore rendered {}x{} RGBA, missing_text={}",bitmap.width,bitmap.height,bitmap.missing_text);
    Ok(())
}
