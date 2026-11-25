//! Isolated process probe for corpus diagnostics. No external rendering engine.
use rpcore::{Document,Error,PageInfo,raster::{Options,TextQuality},content::MAX_CONTENT_BYTES};
use std::{io::Read,time::Instant};
#[path="support/urw.rs"] mod urw;
fn classify(e:Error){
    let (kind,reason)=match e{Error::Invalid(s)=>("invalid",s),Error::Unsupported(s)=>("unsupported",s),Error::Limit(s)=>("limit",s)};
    println!("status={kind}\nreason={reason}");
}
fn report_icc(v:&[rpcore::icc::ProfileDiagnostic]){
    use rpcore::icc::{ProfileUse,ProfilePath};
    let mut counts=[0;4];
    for (i,p) in v.iter().enumerate(){
        let source=match p.source{ProfileUse::Vector=>"vector",ProfileUse::Image=>"image"};
        let (path,reason)=match p.path{
            ProfilePath::RgbMatrix=>("rgb_matrix".to_owned(),""),
            ProfilePath::GrayTrc=>("gray_trc".to_owned(),""),
            ProfilePath::CmykMab{tag}=>(format!("mab_{}",String::from_utf8_lossy(&tag)),""),
            ProfilePath::CmykLut{tag}=>(format!("cmyk_{}",String::from_utf8_lossy(&tag)),""),
            ProfilePath::Alternate{reason}=>("alternate".to_owned(),reason),
        };
        let base=if p.source==ProfileUse::Vector{0}else{2};
        counts[base+usize::from(matches!(p.path,ProfilePath::Alternate{..}))]+=1;
        println!("icc_diag_{i}={source};{}:{};{};{path};{reason}",p.object_id.0,p.object_id.1,p.components);
    }
    println!("icc_vector_native={}\nicc_vector_alternate={}\nicc_image_native={}\nicc_image_alternate={}",counts[0],counts[1],counts[2],counts[3]);
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    let mut args=std::env::args_os().skip(1);
    let path=args.next().ok_or("usage: corpus_probe FILE inspect|content|text|render|math [PAGE [OUTPUT.png or normalized rectangle]]")?;
    let stage=args.next().ok_or("missing stage")?.into_string().map_err(|_|"invalid stage")?;
    let page=if stage=="inspect"{None}else{Some(args.next().ok_or("missing page")?.into_string().map_err(|_|"invalid page")?.parse::<usize>()?)};
    let output=args.next();if args.next().is_some()||output.is_some()&&!matches!(stage.as_str(),"render"|"math"){return Err("unexpected arguments".into());}
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
    let mut quality=if stage=="render"{
        match std::env::var("RPCORE_TEXT_AA"){
            Ok(v)=>match v.as_str(){"2"=>TextQuality::Standard,"4"=>TextQuality::Fine,_=>return Err("unsupported text AA (2 or 4)".into())},
            Err(std::env::VarError::NotPresent)=>TextQuality::Standard,Err(e)=>return Err(e.into()),
        }
    }else{TextQuality::Standard};
    if stage=="render"{
        match std::env::var("RPCORE_TEXT_GRID"){
            Ok(v) if v=="1"&&quality==TextQuality::Fine=>quality=TextQuality::FineGrid,
            Ok(v) if v=="0"=>{},
            Ok(_)=>return Err("grid fitting requires text AA 4 and grid 0 or 1".into()),
            Err(std::env::VarError::NotPresent)=>{},
            Err(e)=>return Err(e.into()),
        }
    }
    if stage=="render"{
        match std::env::var("RPCORE_TEXT_AREA"){
            Ok(v) if v=="1"&&quality==TextQuality::Fine=>quality=TextQuality::Area,
            Ok(v) if v=="0"=>{},
            Ok(_)=>return Err("area requires text AA 4 without grid".into()),
            Err(std::env::VarError::NotPresent)=>{},Err(e)=>return Err(e.into()),
        }
    }
    let result=match stage.as_str(){
        "content"=>doc.page_operations(p,MAX_CONTENT_BYTES).map(|v|println!("operations={}",v.len())),
        "text"=>doc.page_text(p,MAX_CONTENT_BYTES).map(|v|{
            println!("glyphs={}\nmapped={}\nwarnings={}",v.glyphs.len(),v.glyphs.iter().filter(|g|g.unicode.as_ref().is_some_and(|s|!s.is_empty())).count(),v.warnings.len());
        }),
        "math"=>{
            // Normalized top-left CropBox rectangle, independent of page dimensions.
            let raw=output.ok_or("missing math region")?.into_string().map_err(|_|"invalid math region")?;
            let v:Vec<f64>=raw.split(',').map(str::parse::<f64>).collect::<Result<_,_>>()?;
            let r:[f64;4]=v.try_into().map_err(|_|"math region needs four coordinates")?;
            if r.iter().any(|v|!v.is_finite()||*v<0.||*v>1.)||r[0]>=r[2]||r[1]>=r[3]{return Err("invalid normalized math region".into());}
            let view=rpcore::raster::PageViewport::new(p,1.)?;
            let [w,h]=view.content_size();
            let region=view.view_rect_to_pdf([r[0]*w,r[1]*h,r[2]*w,r[3]*h])?;
            let fonts=std::env::var_os("RPCORE_STANDARD_FONT_DIR").map(|dir|urw::load(std::path::Path::new(&dir))).transpose()?;
            match doc.page_math_geometry(p,MAX_CONTENT_BYTES,fonts.as_ref(),region){
                Ok((g,m))=>{
                    println!("math_geometry=ok\nselected={}\nmapped={}\nscripts={}\nfractions={}\nradicals={}",
                        m.glyph_indices.len(),m.glyph_indices.iter().filter(|&&i|g.glyphs[i].unicode.as_ref().is_some_and(|s|!s.is_empty())).count(),
                        m.scripts.len(),m.fractions.len(),m.radicals.len());
                    println!("embedded_name_mapped={}",m.glyph_indices.iter().filter(|&&i|g.glyphs[i].unicode_from_embedded_name).count());
                    for &i in &m.glyph_indices{
                        let x=&g.glyphs[i];
                        println!("glyph_diag_{i}=code={:?}; font={:?}; name={:?}; unicode={:?}; fallback={}; origin={:?}; advance={:?}; em={:?}; bounds={:?}; command={}",
                            x.code,x.base_font.as_ref().map(|n|String::from_utf8_lossy(n)),x.embedded_glyph_name.as_ref().map(|n|String::from_utf8_lossy(n)),
                            x.unicode,x.unicode_from_embedded_name,x.origin,x.advance_end,x.em_matrix.0,x.outline_bounds,x.command_index);
                    }
                    println!("script_diag={:?}",m.scripts);
                    println!("fraction_diag={:?}",m.fractions);
                    println!("radical_diag={:?}",m.radicals);
                    println!("substituted_fonts={}",g.substituted_fonts.iter().map(|f|std::str::from_utf8(f.name()).unwrap()).collect::<Vec<_>>().join(","));
                    if m.glyph_indices.is_empty(){Err(Error::Unsupported("empty selected formula"))}
                    else{match rpcore::raster::math_tree::build_tree(&g,&m){
                        Ok(t)=>{
                            println!("math_tree=ok");
                            t.to_latex(&g).map(|latex|println!("latex={latex}"))
                        }
                        Err(e)=>Err(e),
                    }}
                }
                Err(e)=>Err(e),
            }
        },
        "render"=>match if let Some(dir)=std::env::var_os("RPCORE_STANDARD_FONT_DIR"){
            let fonts=urw::load(std::path::Path::new(&dir))?;
            doc.render_page_with_text_quality(p,Options{scale,allow_missing_text:false},MAX_CONTENT_BYTES,Some(&fonts),quality).map(|v|{
                let names=v.substituted_fonts.iter().map(|f|std::str::from_utf8(f.name()).unwrap()).collect::<Vec<_>>().join(",");
                println!("substituted_fonts={names}");report_icc(&v.icc_profiles);v.bitmap
            })
        }else{doc.render_page_with_text_quality(p,Options{scale,allow_missing_text:false},MAX_CONTENT_BYTES,None,quality).map(|v|{report_icc(&v.icc_profiles);v.bitmap})}{
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
