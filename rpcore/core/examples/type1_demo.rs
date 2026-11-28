#[path="../tests/support/type1_fixture.rs"] mod fixture;
use rpcore::{Document,raster::Options};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let dest=std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("usage: type1_demo OUTPUT_DIR")?);
    std::fs::create_dir_all(&dest)?;
    let p=fixture::pdf(b"0.96 0.97 1 rg 0 0 140 90 re f 0.12 0.24 0.5 rg BT /F1 50 Tf 15 22 Td (AB) Tj ET","/Encoding /StandardEncoding");
    let d=Document::open(&p)?;let pages=d.pages()?;
    let b=d.render_page(&pages[0],Options{scale:4.,allow_missing_text:false},1024*1024)?;
    std::fs::write(dest.join("rpcore-type1-demo.pdf"),&p)?;std::fs::write(dest.join("rpcore-type1-demo.png"),b.png()?)?;
    println!("RPCore Type1 rendered {}x{} RGBA, missing_text={}",b.width,b.height,b.missing_text);Ok(())
}
