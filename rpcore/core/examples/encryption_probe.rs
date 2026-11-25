//! CI-only independent encrypted fixture parity probe. Fixed synthetic
//! test credentials; do not use this executable for private documents.
use rpcore::{Document,Error,security::PasswordStatus,raster::Options,content::MAX_CONTENT_BYTES};
#[path="support/urw.rs"] mod urw;
fn run()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.len()!=4{return Err("usage: encryption_probe PLAIN ENCRYPTED FONT_DIR REVISION".into());}
    let expected:u8=args[3].to_str().ok_or("revision")?.parse()?;
    let plain=std::fs::read(&args[0])?;let encrypted=std::fs::read(&args[1])?;
    let info=Document::encryption_info(&encrypted)?.ok_or("fixture not encrypted")?;
    assert_eq!(info.revision,expected);
    assert_eq!(info.permissions,-4);
    assert_eq!(Document::encryption_info(&plain)?,None);
    assert!(matches!(Document::open(&encrypted),Err(Error::Unsupported("encryption"))));
    assert_eq!(Document::authenticate_password(&encrypted,b"incorrect")?,PasswordStatus::Incorrect);
    assert!(matches!(Document::open_with_password(&encrypted,b"incorrect"),Err(Error::Invalid("incorrect PDF password"))));
    let base=Document::open(&plain)?;
    let pages=base.pages()?;assert_eq!(pages.len(),2);
    let fonts=urw::load(std::path::Path::new(&args[2]))?;
    let mut comparisons=0;
    for (pass,role) in [(b"reader".as_slice(),PasswordStatus::User),(b"owner".as_slice(),PasswordStatus::Owner)]{
        assert_eq!(Document::authenticate_password(&encrypted,pass)?,role);
        let doc=Document::open_with_password(&encrypted,pass)?;
        let pp=doc.pages()?;assert_eq!(pp.len(),pages.len());
        for (i,(a,b)) in pages.iter().zip(&pp).enumerate(){
            let t=base.page_text(a,MAX_CONTENT_BYTES)?;let u=doc.page_text(b,MAX_CONTENT_BYTES)?;
            assert_eq!(t.plain_text(),format!("RPCore page {}",i+1));
            assert_eq!(u,t,"text and glyph positions");
            for scale in [1.,2.]{
                let opt=Options{scale,allow_missing_text:false};
                let x=base.render_page_with_standard_fonts(a,opt,MAX_CONTENT_BYTES,&fonts)?;
                let y=doc.render_page_with_standard_fonts(b,opt,MAX_CONTENT_BYTES,&fonts)?;
                assert!(!x.bitmap.missing_text&&!y.bitmap.missing_text);
                assert!(x.bitmap.rgba.chunks_exact(4).any(|p|p[..3]!=[255,255,255]));
                assert_eq!(x.bitmap.width,y.bitmap.width);assert_eq!(x.bitmap.height,y.bitmap.height);
                assert!(x.bitmap.rgba==y.bitmap.rgba,"pixel mismatch");
                comparisons+=1;
            }
        }
    }
    assert_eq!(comparisons,8);
    println!("R{expected}: 8/8 render parity; 4/4 text and glyph parity; wrong password rejected");
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("encryption fixture gate: {e}");std::process::exit(1);}}
