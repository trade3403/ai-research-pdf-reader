//! C4.3g: bounded nominal single-face text placement and alpha compositing.
use crate::{Error,TrueTypeRenderer,GlyphRenderOptions,MissingGlyphPolicy,GlyphBitmap,RasterViewport,GlyphHorizontalMetric};
use crate::font_render::validate;

const PEN_LIMIT:i64=1<<30;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct TextRenderLimits{
    pub max_text_bytes:usize,pub max_scalars:usize,
    /// Sum of retained individual glyph bitmap pixels, including repeats.
    pub max_glyph_pixels:usize,pub max_total_samples:u64,
    pub max_bitmap_width:u32,pub max_bitmap_height:u32,pub max_bitmap_pixels:usize,
}
impl Default for TextRenderLimits{
    fn default()->Self{Self{max_text_bytes:1024*1024,max_scalars:4096,max_glyph_pixels:8*1024*1024,
        max_total_samples:128*1024*1024,max_bitmap_width:16384,max_bitmap_height:4096,max_bitmap_pixels:8*1024*1024}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct TextRenderOptions{
    pub glyph:GlyphRenderOptions,pub missing:MissingGlyphPolicy,pub limits:TextRenderLimits,
}
impl Default for TextRenderOptions{
    fn default()->Self{Self{glyph:GlyphRenderOptions::default(),missing:MissingGlyphPolicy::Reject,limits:TextRenderLimits::default()}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PlacedGlyph{
    pub character:char,pub byte_start:usize,pub byte_end:usize,
    pub glyph_id:u16,pub missing:bool,pub metric:GlyphHorizontalMetric,pub metrics_source:u16,
    pub origin_x_26_6:i64,pub origin_y_26_6:i64,pub end_x_26_6:i64,
    pub start_units:i64,pub end_units:i64,pub viewport:RasterViewport,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct RenderedText{
    pub glyphs:Vec<PlacedGlyph>,pub bitmap:GlyphBitmap,
    pub origin_x_26_6:i64,pub origin_y_26_6:i64,pub end_x_26_6:i64,
    pub total_units:i64,pub advance_26_6:i64,pub missing_count:usize,
}
fn pen(origin:i64,units:i64,ppem:u32,upem:u16)->Result<i64,Error>{
    let n=i128::from(units)*i128::from(ppem);let d=i128::from(upem);
    // Advances are nonnegative; round the cumulative relative displacement.
    let v=i128::from(origin)+(n+d/2)/d;
    if !(-i128::from(PEN_LIMIT)..=i128::from(PEN_LIMIT)).contains(&v){return Err(Error::Limit("text render pen"));}
    Ok(v as i64)
}
fn bitmap_size(v:RasterViewport,l:TextRenderLimits)->Result<usize,Error>{
    if v.width>l.max_bitmap_width||v.height>l.max_bitmap_height{return Err(Error::Limit("text bitmap dimensions"));}
    let n=usize::try_from(u64::from(v.width)*u64::from(v.height)).map_err(|_|Error::Limit("text bitmap pixels"))?;
    if n>l.max_bitmap_pixels{return Err(Error::Limit("text bitmap pixels"));}
    Ok(n)
}
fn union(a:Option<RasterViewport>,b:RasterViewport)->Result<Option<RasterViewport>,Error>{
    if b.width==0||b.height==0{return Ok(a);}
    let Some(a)=a else{return Ok(Some(b));};
    let left=a.left.min(b.left);let top=a.top.min(b.top);
    let right=(i64::from(a.left)+i64::from(a.width)).max(i64::from(b.left)+i64::from(b.width));
    let bottom=(i64::from(a.top)+i64::from(a.height)).max(i64::from(b.top)+i64::from(b.height));
    Ok(Some(RasterViewport{left,top,
        width:u32::try_from(right-i64::from(left)).map_err(|_|Error::Limit("text bitmap width"))?,
        height:u32::try_from(bottom-i64::from(top)).map_err(|_|Error::Limit("text bitmap height"))?}))
}
fn composite(dst:&mut GlyphBitmap,src:&GlyphBitmap)->Result<(),Error>{
    if src.viewport.width==0||src.viewport.height==0{return Ok(());}
    let x=usize::try_from(i64::from(src.viewport.left)-i64::from(dst.viewport.left)).map_err(|_|Error::Invalid("text bitmap x"))?;
    let y=usize::try_from(i64::from(src.viewport.top)-i64::from(dst.viewport.top)).map_err(|_|Error::Invalid("text bitmap y"))?;
    let sw=src.viewport.width as usize;let sh=src.viewport.height as usize;let dw=dst.viewport.width as usize;
    // Both bitmaps were constructed and bounded by the rasterizer / union.
    for row in 0..sh{
        let start=(y+row)*dw+x;
        let d=dst.alpha.get_mut(start..start+sw).ok_or(Error::Invalid("text bitmap destination"))?;
        let s=src.alpha.get(row*sw..(row+1)*sw).ok_or(Error::Invalid("text bitmap source"))?;
        for (d,&s) in d.iter_mut().zip(s){
            // Straight coverage source-over, rounded per layer to 8 bits.
            *d=(u32::from(s)+(u32::from(*d)*(255-u32::from(s))+127)/255) as u8;
        }
    }
    Ok(())
}
/// Render logical-order Unicode scalars from one TrueType face. Positions
/// derive from cumulative unrounded hmtx design units, not per-glyph rounded
/// advances. No shaping, bidi, kerning, fallback font, line break or tab layout.
/// Alpha compositing is a layer approximation, not an exact union of outlines.
/// Per-glyph stage budgets plus text/pixel/sample budgets bound this reference
/// implementation; no claim of global allocator or exact CPU-work accounting.
pub fn render_nominal_text(font:&TrueTypeRenderer<'_>,text:&str,o:TextRenderOptions)->Result<RenderedText,Error>{
    if text.len()>o.limits.max_text_bytes{return Err(Error::Limit("text render bytes"));}
    let upem=font.metrics().head().units_per_em;
    validate(o.glyph,upem)?;
    let ox=o.glyph.origin_x_26_6;let oy=o.glyph.origin_y_26_6;
    let mut total_units=0i64;let mut x=ox;let mut missing_count=0usize;
    let mut pixels_left=o.limits.max_glyph_pixels;let mut samples_left=o.limits.max_total_samples;
    let samples=u64::from(o.glyph.samples_per_axis).pow(2);
    let mut glyphs=Vec::new();let mut layers=Vec::new();let mut extent=None;
    for (byte_start,c) in text.char_indices(){
        if glyphs.len()>=o.limits.max_scalars{return Err(Error::Limit("text render scalars"));}
        if c.is_control()||matches!(c,'\u{2028}'|'\u{2029}'){return Err(Error::Unsupported("text render control or line separator"));}
        let mut options=o.glyph;options.origin_x_26_6=x;
        options.limits.raster.max_pixels=options.limits.raster.max_pixels.min(pixels_left);
        options.limits.raster.max_samples=options.limits.raster.max_samples.min(samples_left);
        let g=font.render_char(c,o.missing,options)?;
        let n=g.bitmap.alpha.len();
        pixels_left=pixels_left.checked_sub(n).ok_or(Error::Limit("text glyph pixels"))?;
        let charge=u64::try_from(n).ok().and_then(|n|n.checked_mul(samples)).ok_or(Error::Limit("text render samples"))?;
        samples_left=samples_left.checked_sub(charge).ok_or(Error::Limit("text render samples"))?;
        let end_units=total_units.checked_add(i64::from(g.metric.advance_width)).ok_or(Error::Limit("text render advance sum"))?;
        let end_x=pen(ox,end_units,o.glyph.ppem_26_6,upem)?;
        extent=union(extent,g.bitmap.viewport)?;
        if let Some(v)=extent{bitmap_size(v,o.limits)?;}
        if g.missing{missing_count+=1;}
        glyphs.push(PlacedGlyph{character:c,byte_start,byte_end:byte_start+c.len_utf8(),
            glyph_id:g.glyph_id,missing:g.missing,metric:g.metric,metrics_source:g.metrics_source,
            origin_x_26_6:x,origin_y_26_6:oy,end_x_26_6:end_x,
            start_units:total_units,end_units,viewport:g.bitmap.viewport});
        layers.push(g.bitmap);total_units=end_units;x=end_x;
    }
    let viewport=extent.unwrap_or(RasterViewport{left:(ox.div_euclid(64)) as i32,top:(oy.div_euclid(64)) as i32,width:0,height:0});
    let size=bitmap_size(viewport,o.limits)?;
    let mut bitmap=GlyphBitmap{viewport,alpha:vec![0;size]};
    for layer in &layers{composite(&mut bitmap,layer)?;}
    Ok(RenderedText{glyphs,bitmap,origin_x_26_6:ox,origin_y_26_6:oy,end_x_26_6:x,
        total_units,advance_26_6:x-ox,missing_count})
}
