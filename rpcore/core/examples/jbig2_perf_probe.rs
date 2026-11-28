//! Development-only native JBIG2 image decode benchmark. No PDFium linkage.
use rpcore::filters::jbig2::page::{self,Limits};
use std::{io::Read,time::Instant,hint::black_box};
fn run()->Result<(),Box<dyn std::error::Error>>{
    let a:Vec<String>=std::env::args().skip(1).collect();
    if a.len()!=2{return Err("usage: jbig2_perf_probe SEED OUTPUT".into());}
    let t=Instant::now();let mut bytes=Vec::new();
    std::fs::File::open(&a[0])?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len()<2||bytes.len()>4096{return Err("invalid seed length".into());}
    let split=usize::from(u16::from_le_bytes([bytes[0],bytes[1]]));
    if split>=bytes.len()-2{return Err("invalid seed split".into());}
    let (g,p)=bytes[2..].split_at(split);
    let open_ns=t.elapsed().as_nanos();
    let strict=page::decode_pdf_page_reclaimed(g,p,Limits::default())?;
    let baseline=page::fuzz_pdf_image(g,p,Limits::default())?;
    if baseline.bitmap!=strict.bitmap{return Err("strict/compatible bitmap mismatch".into());}
    let mut times=Vec::with_capacity(20);
    for _ in 0..20{
        let t=Instant::now();
        let d=black_box(page::fuzz_pdf_image(black_box(g),black_box(p),Limits::default())?);
        times.push(t.elapsed().as_nanos());
        if d!=baseline{return Err("nonrepeatable decoded page".into());}
    }
    let s=baseline.stats;
    std::fs::write(&a[1],baseline.bitmap.packed())?;
    times.sort_unstable();
    println!("status=ok\nopen_ns={open_ns}\nelapsed_ns={}\np95_ns={}\nsamples_ns={}\nwidth={}\nheight={}\npacked_bytes={}\nwork_used={}\nregion_pixels={}\ndecisions={}\nhuffman_bits={}\ndictionary_work={}\ncomposed_pixels={}",
        (times[9]+times[10])/2,times[18],times.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        baseline.bitmap.width(),baseline.bitmap.height(),s.retained_packed_bytes,s.work_used,s.region_pixels,s.decisions,s.huffman_bits,s.dictionaries.work_used,s.composed_pixels);
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
