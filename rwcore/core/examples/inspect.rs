use rwcore::{read_docx, Block, Limits};
use std::{fs::File, io::Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("usage: inspect <file.docx>")?;
    let limits = Limits::default();
    let mut bytes = Vec::new();
    File::open(path)?.take(limits.max_archive_bytes as u64 + 1).read_to_end(&mut bytes)?;
    let document = read_docx(&bytes, limits)?;
    for block in document.blocks {
        match block {
            Block::Paragraph(p) => println!("{}", p.text()),
            Block::Unsupported { name } => eprintln!("Unsupported block: {name}"),
        }
    }
    for diagnostic in document.diagnostics { eprintln!("{}", diagnostic.message); }
    Ok(())
}
