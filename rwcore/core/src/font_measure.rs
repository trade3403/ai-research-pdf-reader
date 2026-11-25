//! C4.2c: nominal single-face Unicode measurement, without shaping.
use crate::{Error,FontFace,FontMetrics,FontMetricLimits,FontScale,UnicodeCmap,CmapLimits,
    GlyphHorizontalMetric,parse_font_metrics,parse_unicode_cmap};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum MissingGlyphPolicy{Reject,Notdef}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum MeasureUnit{Twips,Pixels26_6{dpi:u32}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct MeasureLimits{pub max_text_bytes:usize,pub max_scalars:usize}
impl Default for MeasureLimits{
    fn default()->Self{Self{max_text_bytes:4*1024*1024,max_scalars:262144}}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct MeasureOptions{
    pub font_size_twips:u32,pub unit:MeasureUnit,pub missing:MissingGlyphPolicy,pub limits:MeasureLimits,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct NominalGlyph{
    pub character:char,pub byte_start:usize,pub byte_end:usize,
    pub glyph_id:u16,pub missing:bool,pub metric:GlyphHorizontalMetric,
    /// Unrounded cumulative design units; not ink bounds.
    pub start_units:i64,pub end_units:i64,
    /// Rounded cumulative positions in the selected MeasureUnit.
    pub start:i64,pub end:i64,
    /// Scaled left bearing as an independent metric, not a painted edge.
    pub left_bearing:i64,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct NominalLineMetrics{
    pub ascender:i64,pub descender:i64,pub line_gap:i64,pub line_height:i64,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TextMeasurement{
    pub unit:MeasureUnit,pub glyphs:Vec<NominalGlyph>,
    pub total_units:i64,pub width:i64,pub missing_count:usize,
    pub line:NominalLineMetrics,
}
/// A single face supplies both cmap and hmtx. Independent tables from
/// different faces cannot be combined by this public interface.
#[derive(Debug)]
pub struct NominalFont<'a>{metrics:FontMetrics<'a>,cmap:UnicodeCmap<'a>}
impl<'a> NominalFont<'a>{
    pub fn metrics(&self)->&FontMetrics<'a>{&self.metrics}
    pub fn cmap(&self)->&UnicodeCmap<'a>{&self.cmap}
    pub fn measure(&self,text:&str,options:MeasureOptions)->Result<TextMeasurement,Error>{
        if text.len()>options.limits.max_text_bytes{return Err(Error::Limit("measure text bytes"));}
        let scale=self.metrics.scale(options.font_size_twips)?;
        // Validate unit and DPI even for empty input.
        let _=convert(scale,options.unit,0)?;
        let header=self.metrics.horizontal_header();
        let line=NominalLineMetrics{
            ascender:convert(scale,options.unit,i64::from(header.ascender))?,
            descender:convert(scale,options.unit,i64::from(header.descender))?,
            line_gap:convert(scale,options.unit,i64::from(header.line_gap))?,
            line_height:convert(scale,options.unit,i64::from(header.line_height_units()))?,
        };
        let mut glyphs=Vec::new();
        let mut total_units=0i64;
        let mut width=0i64;
        let mut missing_count=0usize;
        for (byte_start,character) in text.char_indices(){
            if glyphs.len()>=options.limits.max_scalars{return Err(Error::Limit("measure scalar count"));}
            // This is a single nominal span, not a line breaker. Do not
            // pretend to measure tabs, newlines, C0/C1 or Unicode separators.
            if character.is_control()||matches!(character,'\u{2028}'|'\u{2029}'){
                return Err(Error::Unsupported("measure control or line separator"));
            }
            let mapped=self.cmap.glyph_for_char(character)?;
            let missing=mapped.is_none();
            let glyph_id=match mapped{
                Some(id)=>id,
                None if options.missing==MissingGlyphPolicy::Notdef=>0,
                None=>return Err(Error::Invalid("measure missing glyph")),
            };
            let metric=self.metrics.glyph(glyph_id)?;
            let end_units=total_units.checked_add(i64::from(metric.advance_width)).ok_or(Error::Limit("measure advance sum"))?;
            // Round cumulative endpoints, not individual advances. The
            // resulting widths telescope exactly to the final width.
            let end=convert(scale,options.unit,end_units)?;
            let left_bearing=convert(scale,options.unit,i64::from(metric.left_bearing))?;
            glyphs.push(NominalGlyph{character,byte_start,byte_end:byte_start+character.len_utf8(),
                glyph_id,missing,metric,start_units:total_units,end_units,start:width,end,left_bearing});
            if missing{missing_count+=1;}
            total_units=end_units;width=end;
        }
        Ok(TextMeasurement{unit:options.unit,glyphs,total_units,width,missing_count,line})
    }
}
fn convert(scale:FontScale,unit:MeasureUnit,units:i64)->Result<i64,Error>{
    match unit{MeasureUnit::Twips=>scale.twips(units),MeasureUnit::Pixels26_6{dpi}=>scale.pixels_26_6(units,dpi)}
}
/// Parse matching default-instance metrics and Unicode mapping from one face.
/// Nominal measurement maps each scalar independently in logical order:
/// no shaping, kerning, variation-selector handling, normalization, bidi,
/// grapheme segmentation, line breaking, font fallback or ink geometry.
pub fn parse_nominal_font<'a>(face:&FontFace<'a>,metric_limits:FontMetricLimits,cmap_limits:CmapLimits)->Result<NominalFont<'a>,Error>{
    let metrics=parse_font_metrics(face,metric_limits)?;
    let cmap=parse_unicode_cmap(face,cmap_limits)?;
    Ok(NominalFont{metrics,cmap})
}
