#[path="../tests/support/group_fixture.rs"] mod fixture;
use rpcore::{Document,raster::Options};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let dest=std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("usage: group_demo OUTPUT_DIR")?);
    std::fs::create_dir_all(&dest)?;
    let p=fixture::pdf(b"0.94 0.96 1 rg 0 0 20 20 re f q /Outer gs 1 0 0 1 2 4 cm /G Do Q",fixture::GROUP,fixture::OVERLAP,b"","<< /ca 0.5 >>","<< >>");
    let d=Document::open(&p)?;let pages=d.pages()?;
    let b=d.render_page(&pages[0],Options{scale:24.,allow_missing_text:false},1024*1024)?;
    std::fs::write(dest.join("rpcore-group-demo.pdf"),&p)?;
    std::fs::write(dest.join("rpcore-group-demo.png"),b.png()?)?;
    println!("RPCore isolated group rendered {}x{}",b.width,b.height);Ok(())
}
