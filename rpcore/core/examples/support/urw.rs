//! Explicit optional URW Base35 directory adapter for examples/diagnostics.
// Font files and licensing remain with caller/package manager, not RPCore.
use rpcore::{fonts::standard::StandardFont,raster::StandardFontSet};
use std::{path::Path,io::Read};
pub const URW:[(StandardFont,&str);14]=[
    (StandardFont::Courier,"NimbusMonoPS-Regular.otf"),(StandardFont::CourierBold,"NimbusMonoPS-Bold.otf"),
    (StandardFont::CourierOblique,"NimbusMonoPS-Italic.otf"),(StandardFont::CourierBoldOblique,"NimbusMonoPS-BoldItalic.otf"),
    (StandardFont::Helvetica,"NimbusSans-Regular.otf"),(StandardFont::HelveticaBold,"NimbusSans-Bold.otf"),
    (StandardFont::HelveticaOblique,"NimbusSans-Italic.otf"),(StandardFont::HelveticaBoldOblique,"NimbusSans-BoldItalic.otf"),
    (StandardFont::TimesRoman,"NimbusRoman-Regular.otf"),(StandardFont::TimesBold,"NimbusRoman-Bold.otf"),
    (StandardFont::TimesItalic,"NimbusRoman-Italic.otf"),(StandardFont::TimesBoldItalic,"NimbusRoman-BoldItalic.otf"),
    (StandardFont::Symbol,"StandardSymbolsPS.otf"),(StandardFont::ZapfDingbats,"D050000L.otf"),
];
pub fn load(dir:&Path)->Result<StandardFontSet,Box<dyn std::error::Error>>{
    let mut set=StandardFontSet::new();
    for (face,name) in URW{
        let p=dir.join(name);let mut data=Vec::new();
        std::fs::File::open(&p)?.take(16*1024*1024+1).read_to_end(&mut data)?;
        set.insert(face,data)?;
    }
    Ok(set)
}
