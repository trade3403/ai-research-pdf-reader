//! C4.3f: same-face unhinted Unicode-to-TrueType grayscale bitmap pipeline.
use crate::{Error,FontFace,FontMetrics,FontMetricLimits,UnicodeCmap,CmapLimits,TrueTypeOutlines,OutlineLimits,
    CompositeLimits,PathLimits,FlattenLimits,FlattenOptions,GlyphFillRule,RasterLimits,RasterOptions,
    RasterViewport,RasterTransform,GlyphBitmap,GlyphPathBounds,GlyphHorizontalMetric,MissingGlyphPolicy,
    PATH_ONE,parse_font_metrics,parse_unicode_cmap,parse_true_type_outlines,true_type_glyph_path,
    flatten_glyph_path,rasterize_flat_glyph};

const DEVICE_LIMIT:i128=1<<40;
const PIXEL:i128=65536;
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub struct RenderFontLimits{
    pub metrics:FontMetricLimits,pub cmap:CmapLimits,pub outlines:OutlineLimits,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub struct GlyphRenderLimits{
    pub assembly:CompositeLimits,pub path:PathLimits,pub flatten:FlattenLimits,pub raster:RasterLimits,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GlyphRenderOptions{
    pub ppem_26_6:u32,pub origin_x_26_6:i64,pub origin_y_26_6:i64,
    /// Componentwise flattening plus device-vertex rounding error budget,
    /// in 16.16 pixel units; NOT a guarantee of alpha coverage accuracy.
    pub tolerance_16_16:u32,
    pub samples_per_axis:u8,pub fill:GlyphFillRule,pub limits:GlyphRenderLimits,
}
impl Default for GlyphRenderOptions{
    fn default()->Self{Self{ppem_26_6:16*64,origin_x_26_6:0,origin_y_26_6:0,
        tolerance_16_16:4096,samples_per_axis:4,fill:GlyphFillRule::NonZero,limits:GlyphRenderLimits::default()}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct RenderedGlyph{
    pub glyph_id:u16,pub missing:bool,
    /// Original glyph hmtx data. Composite metric-source metadata is separate.
    pub metric:GlyphHorizontalMetric,pub metrics_source:u16,
    pub advance_26_6:i64,pub left_bearing_26_6:i64,
    /// Absolute bitmap viewport and row-major coverage.
    pub bitmap:GlyphBitmap,
    pub origin_x_26_6:i64,pub origin_y_26_6:i64,
    pub flatten_tolerance_path_units:i64,
}
#[derive(Debug)]
pub struct TrueTypeRenderer<'a>{metrics:FontMetrics<'a>,cmap:UnicodeCmap<'a>,outlines:TrueTypeOutlines<'a>}
pub fn parse_true_type_renderer<'a>(face:&FontFace<'a>,l:RenderFontLimits)->Result<TrueTypeRenderer<'a>,Error>{
    Ok(TrueTypeRenderer{metrics:parse_font_metrics(face,l.metrics)?,
        cmap:parse_unicode_cmap(face,l.cmap)?,outlines:parse_true_type_outlines(face,l.outlines)?})
}
fn round(n:i128,d:i128)->i128{
    let q=n/d;let r=n%d;q+if r.abs()*2>=d{n.signum()}else{0}
}
fn ceil(n:i128,d:i128)->i128{
    let q=n.div_euclid(d);q+if n.rem_euclid(d)!=0{1}else{0}
}
fn device(v:i128)->Result<(),Error>{
    if !(-DEVICE_LIMIT..=DEVICE_LIMIT).contains(&v){return Err(Error::Limit("render device coordinate"));}
    Ok(())
}
pub(crate) fn validate(o:GlyphRenderOptions,units:u16)->Result<i64,Error>{
    if o.ppem_26_6==0||o.tolerance_16_16==0{return Err(Error::Invalid("render scale or tolerance"));}
    if !matches!(o.samples_per_axis,1|2|4|8|16){return Err(Error::Invalid("render samples"));}
    device(i128::from(o.origin_x_26_6)*1024)?;device(i128::from(o.origin_y_26_6)*1024)?;
    // Reserve half a device-grid unit for rounding each transformed vertex.
    // floor((tolerance-1/2)*den/scale) never exceeds the requested budget.
    let den=i128::from(units)*i128::from(PATH_ONE);
    let scale=i128::from(o.ppem_26_6)*1024;
    let t=(2*i128::from(o.tolerance_16_16)-1)*den/(2*scale);
    if t<1{return Err(Error::Limit("render tolerance below path resolution"));}
    i64::try_from(t).map_err(|_|Error::Limit("render tolerance"))
}
fn viewport(b:Option<GlyphPathBounds>,t:RasterTransform)->Result<RasterViewport,Error>{
    let den=i128::from(t.units_per_em)*i128::from(PATH_ONE);
    let scale=i128::from(t.ppem_26_6)*1024;
    let ox=i128::from(t.origin_x_26_6)*1024;
    let oy=i128::from(t.origin_y_26_6)*1024;
    let Some(b)=b else{
        return Ok(RasterViewport{left:i32::try_from(ox.div_euclid(PIXEL)).map_err(|_|Error::Limit("render left"))?,
            top:i32::try_from(oy.div_euclid(PIXEL)).map_err(|_|Error::Limit("render top"))?,width:0,height:0});
    };
    // Bounds are already outward-rounded in path units. Exact scale and
    // y inversion, then floor/ceil to pixel edges. The rasterizer rounds
    // vertices to 16.16; they cannot cross these integer pixel edges.
    let x0=ox*den+i128::from(b.x_min)*scale;
    let x1=ox*den+i128::from(b.x_max)*scale;
    let y0=oy*den-i128::from(b.y_max)*scale;
    let y1=oy*den-i128::from(b.y_min)*scale;
    for n in [x0,x1,y0,y1]{
        if n.abs()>DEVICE_LIMIT*den{return Err(Error::Limit("render glyph bounds"));}
    }
    let d=den*PIXEL;
    let left=x0.div_euclid(d);let right=ceil(x1,d);
    let top=y0.div_euclid(d);let bottom=ceil(y1,d);
    Ok(RasterViewport{left:i32::try_from(left).map_err(|_|Error::Limit("render left"))?,
        top:i32::try_from(top).map_err(|_|Error::Limit("render top"))?,
        width:u32::try_from(right-left).map_err(|_|Error::Limit("render width"))?,
        height:u32::try_from(bottom-top).map_err(|_|Error::Limit("render height"))?})
}
impl TrueTypeRenderer<'_>{
    pub fn metrics(&self)->&FontMetrics<'_>{&self.metrics}
    pub fn cmap(&self)->&UnicodeCmap<'_>{&self.cmap}
    pub fn render_char(&self,c:char,missing:MissingGlyphPolicy,o:GlyphRenderOptions)->Result<RenderedGlyph,Error>{
        match self.cmap.glyph_for_char(c)?{
            Some(id)=>self.render_glyph(id,o),
            None if missing==MissingGlyphPolicy::Notdef=>{
                let mut g=self.render_glyph(0,o)?;g.missing=true;Ok(g)
            },
            None=>Err(Error::Invalid("render missing glyph")),
        }
    }
    pub fn render_glyph(&self,id:u16,o:GlyphRenderOptions)->Result<RenderedGlyph,Error>{
        let units=self.metrics.head().units_per_em;
        let tolerance=validate(o,units)?;
        let metric=self.metrics.glyph(id)?;
        let path=true_type_glyph_path(&self.outlines,id,o.limits.assembly,o.limits.path)?;
        let transform=RasterTransform{units_per_em:units,ppem_26_6:o.ppem_26_6,
            origin_x_26_6:o.origin_x_26_6,origin_y_26_6:o.origin_y_26_6};
        let viewport=viewport(path.bounds(),transform)?;
        // Check dimensions before potentially expensive subdivision.
        let pixels=u64::from(viewport.width)*u64::from(viewport.height);
        if viewport.width>o.limits.raster.max_width||viewport.height>o.limits.raster.max_height||
            usize::try_from(pixels).map_or(true,|p|p>o.limits.raster.max_pixels){
            return Err(Error::Limit("render bitmap dimensions"));
        }
        let flat=flatten_glyph_path(&path,FlattenOptions{tolerance,limits:o.limits.flatten})?;
        let bitmap=rasterize_flat_glyph(&flat,RasterOptions{transform,viewport,fill:o.fill,
            samples_per_axis:o.samples_per_axis,limits:o.limits.raster})?;
        let scaled=|v:i128|->Result<i64,Error>{
            i64::try_from(round(v*i128::from(o.ppem_26_6),i128::from(units))).map_err(|_|Error::Limit("render metric"))
        };
        Ok(RenderedGlyph{glyph_id:id,missing:false,metric,metrics_source:path.metrics_glyph(),
            advance_26_6:scaled(i128::from(metric.advance_width))?,left_bearing_26_6:scaled(i128::from(metric.left_bearing))?,
            bitmap,origin_x_26_6:o.origin_x_26_6,origin_y_26_6:o.origin_y_26_6,
            flatten_tolerance_path_units:tolerance})
    }
}
