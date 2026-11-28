use rwcore::{inspect_package, read_docx, Limits};
use std::{fs::File, io::Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.is_empty() { return Err("usage: corpus_probe <file.docx> ...".into()); }
    let mut failed = 0usize;
    for (i, path) in paths.iter().enumerate() {
        let result = (|| -> Result<_, Box<dyn std::error::Error>> {
            let limits = Limits::default();
            let mut bytes = Vec::new();
            File::open(path)?.take(limits.max_archive_bytes as u64 + 1).read_to_end(&mut bytes)?;
            let graph = inspect_package(&bytes, limits)?;
            let document = read_docx(&bytes, limits)?;
            Ok((graph.parts.len(), graph.relationships.len(), document.blocks.len(), document.diagnostics.len()))
        })();
        match result {
            Ok((parts, edges, blocks, warnings)) => println!("sample {}: OK parts={parts} edges={edges} blocks={blocks} warnings={warnings}", i+1),
            Err(err) => { failed += 1; eprintln!("sample {}: FAILED {err:?}", i+1); }
        }
    }
    println!("total={} failed={failed}", paths.len());
    if failed > 0 { return Err("corpus failures".into()); }
    Ok(())
}
