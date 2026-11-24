use std::io::Read;
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args=std::env::args_os().skip(1);
    let path=args.next().ok_or("usage: rpcore FILE.pdf [--render PAGE OUTPUT.png SCALE]")?;
    let mode=args.next();
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take((rpcore::MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    let doc = rpcore::Document::open(&bytes)?;
    let pages = doc.pages()?;
    if let Some(mode)=mode{
        if mode!="--render"{return Err("unknown option".into());}
        let page:usize=args.next().ok_or("missing page number")?.to_str().ok_or("invalid page number")?.parse()?;
        let output=args.next().ok_or("missing output path")?;
        let scale:f64=args.next().ok_or("missing scale")?.to_str().ok_or("invalid scale")?.parse()?;
        if args.next().is_some(){return Err("unexpected argument".into());}
        let p=pages.get(page.checked_sub(1).ok_or("page numbers start at 1")?).ok_or("page out of range")?;
        let bitmap=doc.render_page(p,rpcore::raster::Options{scale,allow_missing_text:false},rpcore::content::MAX_CONTENT_BYTES)?;
        std::fs::write(output,bitmap.png()?)?;
        println!("rendered page={} width={} height={}",page,bitmap.width,bitmap.height);
        return Ok(());
    }
    println!("pages={}", pages.len());
    for (i, page) in pages.iter().enumerate() {
        println!("page={} media={:?} crop={:?} rotate={}", i+1, page.media_box, page.crop_box, page.rotation);
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
