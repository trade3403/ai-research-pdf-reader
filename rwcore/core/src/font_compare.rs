//! Bounded comparison of independent straight-alpha glyph bitmaps in absolute pixel coordinates.
use crate::{Error, GlyphBitmap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitmapComparison {
    pub pixels: u64,
    pub differing_pixels: u64,
    pub absolute_error: u64,
    pub squared_error: u64,
    pub max_error: u8,
    pub actual_ink: u64,
    pub reference_ink: u64,
}

fn validate(b: &GlyphBitmap, max: u64) -> Result<(), Error> {
    let n = u64::from(b.viewport.width) * u64::from(b.viewport.height);
    if n > max { return Err(Error::Limit("comparison input pixels")); }
    if n != b.alpha.len() as u64 { return Err(Error::Invalid("comparison bitmap length")); }
    Ok(())
}

fn sample(b: &GlyphBitmap, x: i64, y: i64) -> u8 {
    let dx = x - i64::from(b.viewport.left);
    let dy = y - i64::from(b.viewport.top);
    if dx < 0 || dy < 0 || dx >= i64::from(b.viewport.width) || dy >= i64::from(b.viewport.height) {
        return 0;
    }
    b.alpha[(dy as u64 * u64::from(b.viewport.width) + dx as u64) as usize]
}

/// Compare on the bounding rectangle of nonempty viewports, treating absent
/// pixels as transparent. Empty rectangles do not affect the comparison domain.
/// The caller must supply independently generated reference coverage.
pub fn compare_glyph_bitmaps(a: &GlyphBitmap, b: &GlyphBitmap, max_pixels: u64) -> Result<BitmapComparison, Error> {
    // Bound all accumulators even when the caller requests an enormous budget.
    let max = max_pixels.min(u64::MAX / 65025);
    validate(a, max)?;
    validate(b, max)?;
    let mut bounds: Option<(i64,i64,i64,i64)> = None;
    for bitmap in [a,b] {
        let v = bitmap.viewport;
        if v.width == 0 || v.height == 0 { continue; }
        let x = i64::from(v.left);
        let y = i64::from(v.top);
        let r = x + i64::from(v.width);
        let d = y + i64::from(v.height);
        bounds = Some(match bounds {
            None => (x,y,r,d),
            Some((l,t,rr,dd)) => (l.min(x),t.min(y),rr.max(r),dd.max(d)),
        });
    }
    let mut report = BitmapComparison { pixels:0, differing_pixels:0, absolute_error:0,
        squared_error:0, max_error:0, actual_ink:0, reference_ink:0 };
    let Some((l,t,r,d)) = bounds else { return Ok(report); };
    report.pixels = ((r-l) as u64).checked_mul((d-t) as u64).ok_or(Error::Limit("comparison area"))?;
    if report.pixels > max { return Err(Error::Limit("comparison area")); }
    for y in t..d {
        for x in l..r {
            let av = sample(a,x,y);
            let bv = sample(b,x,y);
            let delta = av.abs_diff(bv);
            report.actual_ink += u64::from(av);
            report.reference_ink += u64::from(bv);
            report.absolute_error += u64::from(delta);
            report.squared_error += u64::from(delta) * u64::from(delta);
            report.max_error = report.max_error.max(delta);
            report.differing_pixels += if delta != 0 { 1 } else { 0 };
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RasterViewport;
    fn bitmap(left:i32, top:i32, width:u32, height:u32, alpha:&[u8]) -> GlyphBitmap {
        GlyphBitmap {viewport:RasterViewport{left,top,width,height},alpha:alpha.to_vec()}
    }
    #[test] fn identical() {
        let a=bitmap(-2,-1,2,1,&[40,255]);
        let r=compare_glyph_bitmaps(&a,&a,2).unwrap();
        assert_eq!((r.pixels,r.absolute_error,r.actual_ink),(2,0,295));
    }
    #[test] fn translated() {
        let a=bitmap(-1,0,2,1,&[100,200]);
        let b=bitmap(0,0,2,1,&[200,50]);
        let r=compare_glyph_bitmaps(&a,&b,3).unwrap();
        assert_eq!((r.pixels,r.differing_pixels,r.absolute_error,r.squared_error,r.max_error),(3,2,150,12500,100));
        assert_eq!((r.actual_ink,r.reference_ink),(300,250));
    }
    #[test] fn vertical_alignment() {
        let a=bitmap(0,-1,1,2,&[255,30]);
        let b=bitmap(0,0,1,1,&[30]);
        assert_eq!(compare_glyph_bitmaps(&a,&b,2).unwrap().absolute_error,255);
    }
    #[test] fn empty_ignored() {
        let a=bitmap(i32::MAX,i32::MIN,0,10,&[]);
        let b=bitmap(0,0,1,1,&[7]);
        assert_eq!(compare_glyph_bitmaps(&a,&b,1).unwrap().pixels,1);
        assert_eq!(compare_glyph_bitmaps(&a,&a,0).unwrap().pixels,0);
    }
    #[test] fn malformed_length() {
        let a=bitmap(0,0,2,1,&[1]);
        assert!(compare_glyph_bitmaps(&a,&a,2).is_err());
    }
    #[test] fn distant_area_rejected() {
        let a=bitmap(i32::MIN,0,1,1,&[1]);
        let b=bitmap(i32::MAX,0,1,1,&[1]);
        assert!(compare_glyph_bitmaps(&a,&b,100).is_err());
    }
    #[test] fn input_budget() {
        let a=bitmap(0,0,2,1,&[0,0]);
        assert!(compare_glyph_bitmaps(&a,&a,1).is_err());
    }
    #[test] fn padded_transparency() {
        let a=bitmap(-1,0,3,1,&[0,255,0]);
        let b=bitmap(0,0,1,1,&[255]);
        let r=compare_glyph_bitmaps(&a,&b,3).unwrap();
        assert_eq!((r.pixels,r.differing_pixels,r.absolute_error),(3,0,0));
    }
}
