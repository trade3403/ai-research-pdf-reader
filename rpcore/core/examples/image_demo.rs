#[path="../tests/support/image_fixture.rs"] mod fixture;
use rpcore::{Document,raster::Options};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let dest=std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("usage: image_demo OUTPUT_DIR")?);
    std::fs::create_dir_all(&dest)?;
    let im=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /SMask 6 0 R",&fixture::rgb());
    let mask:Vec<u8>=(0..9).flat_map(|_|(0..17).map(|x|(x*255/16) as u8)).collect();
    let m=fixture::stream("/Subtype /Image /Width 17 /Height 9 /ColorSpace /DeviceGray /BitsPerComponent 8",&mask);
    let p=fixture::pdf(b"0.12 0.24 0.5 rg 0 0 20 20 re f q 18 0 0 12 1 4 cm /Im Do Q q /Half gs 1 1 1 rg 6 1 8 18 re f Q",im,m,"<< /ca 0.5 /BM /Normal >>");
    let d=Document::open(&p)?;let pages=d.pages()?;let b=d.render_page(&pages[0],Options{scale:24.,allow_missing_text:false},1024*1024)?;
    std::fs::write(dest.join("rpcore-image-demo.pdf"),&p)?;std::fs::write(dest.join("rpcore-image-demo.png"),b.png()?)?;
    println!("RPCore JPEG/opacity rendered {}x{}",b.width,b.height);Ok(())
}
