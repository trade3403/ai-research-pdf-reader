//! B-P0 isolated, release-mode in-memory benchmark probe. No PDFium linkage.
use rpcore::{Document,content::MAX_CONTENT_BYTES,raster::Options};
use std::{io::Read,time::Instant, hint::black_box};
fn run()->Result<(),Box<dyn std::error::Error>>{
    let a:Vec<String>=std::env::args().skip(1).collect();
    if a.len()!=5{return Err("usage: perf_probe FILE inspect|render|text DPI cold|warm OUTPUT".into());}
    let dpi:u32=a[2].parse()?;
    if ![72,96,150,200,300].contains(&dpi)||!matches!(a[3].as_str(),"cold"|"warm"){return Err("invalid benchmark parameters".into());}
    let mut bytes=Vec::new();
    std::fs::File::open(&a[0])?.take((rpcore::MAX_BYTES+1)as u64).read_to_end(&mut bytes)?;
    if bytes.len()>rpcore::MAX_BYTES{return Err("input limit".into());}
    let t=Instant::now();
    let doc=Document::open(&bytes)?;
    let pages=doc.pages()?;
    let open_ns=t.elapsed().as_nanos();
    if pages.len()!=1{return Err("B-P0a expects one page".into());}
    let p=&pages[0];
    if p.rotation!=0{return Err("B-P0a rotation unsupported".into());}
    println!("pages={}\nopen_ns={open_ns}",pages.len());
    match a[1].as_str(){
        "inspect"=>{
            if a[3]!="cold"{return Err("inspect requires cold".into());}
            let [x0,y0,x1,y1]=p.crop_box;
            println!("elapsed_ns={open_ns}\nwidth_pt={}\nheight_pt={}",x1-x0,y1-y0);
        }
        "render"=>{
            let opt=Options{scale:f64::from(dpi)/72.,allow_missing_text:false};
            if a[3]=="warm"{black_box(doc.render_page(p,opt,MAX_CONTENT_BYTES)?);}
            let t=Instant::now();
            let bmp=black_box(doc.render_page(p,opt,MAX_CONTENT_BYTES)?);
            let ns=t.elapsed().as_nanos();
            if bmp.missing_text{return Err("strict render missing text".into());}
            // RGB normalization and file output outside timed operation.
            let mut rgb=Vec::with_capacity(bmp.width*bmp.height*3);
            for px in bmp.rgba.chunks_exact(4){if px[3]!=255{return Err("nonopaque result".into());}rgb.extend_from_slice(&px[..3]);}
            std::fs::write(&a[4],rgb)?;
            println!("elapsed_ns={ns}\nwidth={}\nheight={}",bmp.width,bmp.height);
        }
        "text"=>{
            if a[3]=="warm"{black_box(doc.page_text(p,MAX_CONTENT_BYTES)?);}
            let t=Instant::now();
            let v=black_box(doc.page_text(p,MAX_CONTENT_BYTES)?);
            // Complete UTF-8 result construction is part of the operation.
            let mut output=String::new();
            for g in &v.glyphs{
                let s=g.unicode.as_deref().ok_or("unmapped glyph")?;
                for ch in s.chars().filter(|c|!c.is_whitespace()){
                    use std::fmt::Write;
                    writeln!(&mut output,"{}\t{}\t{}",u32::from(ch),g.origin[0],g.origin[1])?;
                }
            }
            let ns=t.elapsed().as_nanos();
            if !v.warnings.is_empty(){return Err("text warnings".into());}
            std::fs::write(&a[4],output)?;
            println!("elapsed_ns={ns}");
        }
        _=>return Err("unknown operation".into()),
    }
    println!("status=ok");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
