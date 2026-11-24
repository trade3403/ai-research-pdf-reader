//! Isolated process probe for corpus diagnostics. No external rendering engine.
use rpcore::{Document,Error,PageInfo,raster::{Options,TextQuality},content::MAX_CONTENT_BYTES};
use std::{io::Read,time::Instant};
#[path="support/urw.rs"] mod urw;
fn classify(e:Error){
    let (kind,reason)=match e{Error::Invalid(s)=>("invalid",s),Error::Unsupported(s)=>("unsupported",s),Error::Limit(s)=>("limit",s)};
    println!("status={kind}\nreason={reason}");
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    let mut args=std::env::args_os().skip(1);
    let path=args.next().ok_or("usage: corpus_probe FILE inspect|content|text|render [PAGE [OUTPUT.png]]")?;
    let stage=args.next().ok_or("missing stage")?.into_string().map_err(|_|"invalid stage")?;
    let page=if stage=="inspect"{None}else{Some(args.next().ok_or("missing page")?.into_string().map_err(|_|"invalid page")?.parse::<usize>()?)};
    let output=args.next();if args.next().is_some()||output.is_some()&&stage!="render"{return Err("unexpected arguments".into());}
    let start=Instant::now();let mut bytes=Vec::new();
    std::fs::File::open(path)?.take((rpcore::MAX_BYTES+1)as u64).read_to_end(&mut bytes)?;
    if bytes.len()>rpcore::MAX_BYTES{classify(Error::Limit("input bytes"));return Ok(());}
    let doc=match Document::open(&bytes){Ok(d)=>d,Err(e)=>{println!("failed_at=open");classify(e);return Ok(());}};
    let pages=match doc.pages(){Ok(p)=>p,Err(e)=>{println!("failed_at=pages");classify(e);return Ok(());}};
    if stage=="inspect"{
        println!("status=ok\npages={}\nbytes={}\nengine_ms={}",pages.len(),bytes.len(),start.elapsed().as_millis());return Ok(());
    }
    let p:&PageInfo=pages.get(page.unwrap().checked_sub(1).ok_or("page starts at 1")?).ok_or("page out of range")?;
    let scale=if stage=="render"{
        match std::env::var("RPCORE_RENDER_DPI"){
            Ok(v)=>match v.as_str(){"36"=>0.5,"72"=>1.,"144"=>2.,_=>return Err("unsupported render DPI (36, 72, 144)".into())},
            Err(std::env::VarError::NotPresent)=>0.5,
            Err(e)=>return Err(e.into()),
        }
    }else{0.5};
    let quality=if stage=="render"{
        match std::env::var("RPCORE_TEXT_AA"){
            Ok(v)=>match v.as_str(){"2"=>TextQuality::Standard,"4"=>TextQuality::Fine,_=>return Err("unsupported text AA (2 or 4)".into())},
            Err(std::env::VarError::NotPresent)=>TextQuality::Standard,Err(e)=>return Err(e.into()),
        }
    }else{TextQuality::Standard};
    let result=match stage.as_str(){
        "content"=>doc.page_operations(p,MAX_CONTENT_BYTES).map(|v|println!("operations={}",v.len())),
        "text"=>doc.page_text(p,MAX_CONTENT_BYTES).map(|v|{
            println!("glyphs={}\nmapped={}\nwarnings={}",v.glyphs.len(),v.glyphs.iter().filter(|g|g.unicode.as_ref().is_some_and(|s|!s.is_empty())).count(),v.warnings.len());
        }),
        "render"=>match if let Some(dir)=std::env::var_os("RPCORE_STANDARD_FONT_DIR"){
            let fonts=urw::load(std::path::Path::new(&dir))?;
            doc.render_page_with_text_quality(p,Options{scale,allow_missing_text:false},MAX_CONTENT_BYTES,Some(&fonts),quality).map(|v|{
                let names=v.substituted_fonts.iter().map(|f|std::str::from_utf8(f.name()).unwrap()).collect::<Vec<_>>().join(",");
                println!("substituted_fonts={names}");v.bitmap
            })
        }else{doc.render_page_with_text_quality(p,Options{scale,allow_missing_text:false},MAX_CONTENT_BYTES,None,quality).map(|v|v.bitmap)}{
            Ok(b)=>{
                if b.missing_text{Err(Error::Invalid("strict render returned missing text"))}else{
                    let png=match b.png(){Ok(v)=>v,Err(e)=>{classify(e);return Ok(());}};
                    if let Some(path)=output{std::fs::write(path,&png)?;}
                    // Stable noncryptographic pixel fingerprint for repeatability, not visual accuracy.
                    let hash=b.rgba.iter().fold(0xcbf29ce484222325u64,|h,&v|(h^u64::from(v)).wrapping_mul(0x100000001b3));
                    println!("width={}\nheight={}\nnonwhite_pixels={}\npng_bytes={}\nrgba_fnv64={hash:016x}",b.width,b.height,b.rgba.chunks_exact(4).filter(|p|p[..3]!=[255,255,255]).count(),png.len());
                    Ok(())
                }
            }
            Err(e)=>Err(e),
        },
        _=>return Err("unknown stage".into()),
    };
    match result{Ok(())=>println!("status=ok"),Err(e)=>{println!("failed_at={stage}");classify(e);}}
    println!("engine_ms={}",start.elapsed().as_millis());Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
