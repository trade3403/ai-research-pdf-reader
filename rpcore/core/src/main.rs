use std::io::Read;
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("usage: rpcore FILE.pdf")?;
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take((rpcore::MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    let doc = rpcore::Document::open(&bytes)?;
    println!("pages={}", doc.page_count()?);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
