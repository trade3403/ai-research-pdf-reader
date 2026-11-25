//! C4.2a: native default-instance horizontal font metrics and integer scaling.
use crate::{Error,FontFace,FontFlavor,FontTag};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontBounds{pub x_min:i16,pub y_min:i16,pub x_max:i16,pub y_max:i16}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontHead{
    pub revision_raw:i32,pub flags:u16,pub units_per_em:u16,pub bounds:FontBounds,
    pub mac_style:u16,pub lowest_rec_ppem:u16,pub loca_format:i16,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct TrueTypeMaxima{
    pub points:u16,pub contours:u16,pub composite_points:u16,pub composite_contours:u16,
    pub zones:u16,pub twilight_points:u16,pub storage:u16,pub function_defs:u16,
    pub instruction_defs:u16,pub stack_elements:u16,pub instruction_bytes:u16,
    pub component_elements:u16,pub component_depth:u16,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontMaxp{pub glyph_count:u16,pub true_type:Option<TrueTypeMaxima>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct HorizontalHeader{
    pub ascender:i16,pub descender:i16,pub line_gap:i16,pub advance_width_max:u16,
    pub min_left_bearing:i16,pub min_right_bearing:i16,pub x_max_extent:i16,
    pub caret_slope_rise:i16,pub caret_slope_run:i16,pub caret_offset:i16,
    pub long_metric_count:u16,
}
impl HorizontalHeader{
    /// Raw hhea line metric, not a Word line-spacing policy.
    pub fn line_height_units(self)->i32{
        i32::from(self.ascender)-i32::from(self.descender)+i32::from(self.line_gap)
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GlyphHorizontalMetric{pub advance_width:u16,pub left_bearing:i16}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontMetricLimits{pub max_glyphs:usize}
impl Default for FontMetricLimits{fn default()->Self{Self{max_glyphs:65535}}}
#[derive(Debug)]
pub struct FontMetrics<'a>{
    head:FontHead,maxp:FontMaxp,hhea:HorizontalHeader,hmtx:&'a [u8],
}
impl<'a> FontMetrics<'a>{
    pub fn head(&self)->FontHead{self.head}
    pub fn maxp(&self)->FontMaxp{self.maxp}
    pub fn horizontal_header(&self)->HorizontalHeader{self.hhea}
    /// The final long metric supplies advance width for remaining glyphs;
    /// their left-side bearings are stored in a separate signed array.
    pub fn glyph(&self,glyph:u16)->Result<GlyphHorizontalMetric,Error>{
        if glyph>=self.maxp.glyph_count{return Err(Error::Invalid("font glyph index"));}
        let g=usize::from(glyph);let n=usize::from(self.hhea.long_metric_count);
        let advance_at=g.min(n-1)*4;
        let bearing_at=if g<n{g*4+2}else{n*4+(g-n)*2};
        Ok(GlyphHorizontalMetric{advance_width:u16_at(self.hmtx,advance_at)?,left_bearing:i16_at(self.hmtx,bearing_at)?})
    }
    pub fn scale(&self,font_size_twips:u32)->Result<FontScale,Error>{
        FontScale::new(self.head.units_per_em,font_size_twips)
    }
}
fn bytes_at<const N:usize>(b:&[u8],at:usize)->Result<[u8;N],Error>{
    let end=at.checked_add(N).ok_or(Error::Limit("font metric offset"))?;
    b.get(at..end).ok_or(Error::Invalid("font truncated metric table"))?
        .try_into().map_err(|_|Error::Invalid("font metric field"))
}
fn u16_at(b:&[u8],at:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(bytes_at(b,at)?))}
fn i16_at(b:&[u8],at:usize)->Result<i16,Error>{Ok(i16::from_be_bytes(bytes_at(b,at)?))}
fn u32_at(b:&[u8],at:usize)->Result<u32,Error>{Ok(u32::from_be_bytes(bytes_at(b,at)?))}
fn required<'a>(face:&FontFace<'a>,tag:FontTag)->Result<&'a [u8],Error>{
    face.table(tag).ok_or(Error::Invalid("font missing metric table"))
}
fn exact_size(b:&[u8],n:usize)->Result<(),Error>{
    if b.len()!=n{return Err(Error::Invalid("font metric table length"));}
    Ok(())
}
fn units_valid(n:u16)->bool{(16..=16384).contains(&n)}
fn parse_head(b:&[u8],flavor:FontFlavor)->Result<FontHead,Error>{
    exact_size(b,54)?;
    if u32_at(b,0)?!=0x00010000{return Err(Error::Unsupported("font head version"));}
    if u32_at(b,12)?!=0x5f0f3cf5{return Err(Error::Invalid("font head magic"));}
    let units_per_em=u16_at(b,18)?;
    if !units_valid(units_per_em){return Err(Error::Invalid("font units per em"));}
    let bounds=FontBounds{x_min:i16_at(b,36)?,y_min:i16_at(b,38)?,x_max:i16_at(b,40)?,y_max:i16_at(b,42)?};
    if bounds.x_min>bounds.x_max||bounds.y_min>bounds.y_max{return Err(Error::Invalid("font reversed bounds"));}
    let loca_format=i16_at(b,50)?;
    if !matches!(loca_format,0|1)||flavor==FontFlavor::OpenTypeCff&&loca_format!=0{
        return Err(Error::Unsupported("font loca format"));
    }
    if i16_at(b,52)?!=0{return Err(Error::Unsupported("font glyph data format"));}
    Ok(FontHead{revision_raw:i32::from_be_bytes(bytes_at(b,4)?),flags:u16_at(b,16)?,
        units_per_em,bounds,mac_style:u16_at(b,44)?,lowest_rec_ppem:u16_at(b,46)?,loca_format})
}
pub(crate) fn parse_maxp(b:&[u8],flavor:FontFlavor,limits:FontMetricLimits)->Result<FontMaxp,Error>{
    let version=u32_at(b,0)?;
    let glyph_count=u16_at(b,4)?;
    if glyph_count==0{return Err(Error::Invalid("font zero glyphs"));}
    if usize::from(glyph_count)>limits.max_glyphs{return Err(Error::Limit("font glyph count"));}
    let true_type=match (flavor,version){
        (FontFlavor::OpenTypeCff,0x00005000)=>{exact_size(b,6)?;None},
        (FontFlavor::TrueType,0x00010000)=>{
            exact_size(b,32)?;
            Some(TrueTypeMaxima{points:u16_at(b,6)?,contours:u16_at(b,8)?,
                composite_points:u16_at(b,10)?,composite_contours:u16_at(b,12)?,zones:u16_at(b,14)?,
                twilight_points:u16_at(b,16)?,storage:u16_at(b,18)?,function_defs:u16_at(b,20)?,
                instruction_defs:u16_at(b,22)?,stack_elements:u16_at(b,24)?,instruction_bytes:u16_at(b,26)?,
                component_elements:u16_at(b,28)?,component_depth:u16_at(b,30)?})
        },
        _=>return Err(Error::Unsupported("font maxp flavor or version")),
    };
    Ok(FontMaxp{glyph_count,true_type})
}
fn parse_hhea(b:&[u8],glyph_count:u16)->Result<HorizontalHeader,Error>{
    exact_size(b,36)?;
    if u32_at(b,0)?!=0x00010000{return Err(Error::Unsupported("font hhea version"));}
    if i16_at(b,32)?!=0{return Err(Error::Unsupported("font horizontal metric format"));}
    let long_metric_count=u16_at(b,34)?;
    if long_metric_count==0||long_metric_count>glyph_count{return Err(Error::Invalid("font long metric count"));}
    Ok(HorizontalHeader{ascender:i16_at(b,4)?,descender:i16_at(b,6)?,line_gap:i16_at(b,8)?,
        advance_width_max:u16_at(b,10)?,min_left_bearing:i16_at(b,12)?,min_right_bearing:i16_at(b,14)?,
        x_max_extent:i16_at(b,16)?,caret_slope_rise:i16_at(b,18)?,caret_slope_run:i16_at(b,20)?,
        caret_offset:i16_at(b,22)?,long_metric_count})
}
/// Parse the four required horizontal metric tables with strict lengths.
/// Borrow hmtx, no per-glyph allocation or outline/variation evaluation.
/// Header extrema and maxp resource claims are exposed, not independently
/// validated against outlines. Table checksums remain explicitly opt-in.
pub fn parse_font_metrics<'a>(face:&FontFace<'a>,limits:FontMetricLimits)->Result<FontMetrics<'a>,Error>{
    let head=parse_head(required(face,*b"head")?,face.flavor())?;
    let maxp=parse_maxp(required(face,*b"maxp")?,face.flavor(),limits)?;
    let hhea=parse_hhea(required(face,*b"hhea")?,maxp.glyph_count)?;
    let hmtx=required(face,*b"hmtx")?;
    let n=usize::from(hhea.long_metric_count);let g=usize::from(maxp.glyph_count);
    let length=n.checked_mul(4).and_then(|a|(g-n).checked_mul(2).and_then(|b|a.checked_add(b)))
        .ok_or(Error::Limit("font hmtx length"))?;
    exact_size(hmtx,length)?;
    Ok(FontMetrics{head,maxp,hhea,hmtx})
}
/// Positive font size in twips (20 twips per point); no implied device DPI.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FontScale{units_per_em:u16,font_size_twips:u32}
fn rounded_ratio(n:i128,d:i128)->Result<i64,Error>{
    // Positive denominator; round nearest, exact ties away from zero.
    let q=n/d;let r=n%d;
    let adjust=if r.abs()*2>=d{n.signum()}else{0};
    i64::try_from(q.checked_add(adjust).ok_or(Error::Limit("font scale result"))?)
        .map_err(|_|Error::Limit("font scale result"))
}
fn product(a:i128,b:i128)->Result<i128,Error>{
    a.checked_mul(b).ok_or(Error::Limit("font scale arithmetic"))
}
impl FontScale{
    pub fn new(units_per_em:u16,font_size_twips:u32)->Result<Self,Error>{
        if !units_valid(units_per_em)||font_size_twips==0{return Err(Error::Invalid("font scale parameters"));}
        Ok(Self{units_per_em,font_size_twips})
    }
    pub fn units_per_em(self)->u16{self.units_per_em}
    pub fn font_size_twips(self)->u32{self.font_size_twips}
    pub fn twips(self,units:i64)->Result<i64,Error>{
        rounded_ratio(product(i128::from(units),i128::from(self.font_size_twips))?,i128::from(self.units_per_em))
    }
    /// Convert directly to signed 26.6 device pixels at caller-provided DPI.
    /// Do not round to twips first. 1440 twips = one inch.
    pub fn pixels_26_6(self,units:i64,dpi:u32)->Result<i64,Error>{
        if dpi==0{return Err(Error::Invalid("font zero dpi"));}
        let n=product(product(product(i128::from(units),i128::from(self.font_size_twips))?,i128::from(dpi))?,64)?;
        rounded_ratio(n,i128::from(self.units_per_em)*1440)
    }
}
