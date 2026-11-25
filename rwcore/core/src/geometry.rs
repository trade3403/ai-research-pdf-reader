//! C2.2c: checked integer page and column geometry, independent of rendering.
use crate::{Error, SectionProperties};

/// Explicit gutter placement supplied by the caller, not inferred from settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterSide { Left, Right, Top }

/// Coordinates in twips from the physical page's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageGeometry {
    pub page: Rect,
    pub content: Rect,
    pub columns: Vec<Rect>,
    /// True when a separator is requested; drawing is a later responsibility.
    pub column_separator: bool,
    /// Unallocated content width after the final explicit column.
    pub trailing_width: u32,
}

fn required<T: Copy>(value: Option<T>) -> Result<T, Error> {
    value.ok_or(Error::Invalid("missing explicit page geometry property"))
}
fn add(a: u32, b: u32) -> Result<u32, Error> {
    a.checked_add(b).ok_or(Error::Limit("page geometry arithmetic"))
}
fn subtract_positive(a: u32, b: u32) -> Result<u32, Error> {
    a.checked_sub(b).filter(|v| *v > 0).ok_or(Error::Invalid("nonpositive content dimension"))
}
fn margin(value: Option<i32>) -> Result<u32, Error> {
    u32::try_from(required(value)?).map_err(|_| Error::Unsupported("negative vertical margin geometry"))
}

/// Compute the supported static geometry subset.
/// Page dimensions and four margins must be explicit; orientation is descriptive
/// and never swaps stored dimensions. Gutter defaults to zero, with caller-selected
/// side. Missing columns default to a single column; equal columns default to a
/// 720-twip gap. Extra integer twips are assigned left-to-right.
/// Explicit unequal columns retain their widths and gaps, and any trailing space.
/// No header/footer collision, mirrored margins, RTL, continuous-section flow,
/// inherited geometry or pagination is resolved here.
pub fn compute_page_geometry(section: &SectionProperties, gutter_side: GutterSide)
    -> Result<PageGeometry, Error>
{
    let width = required(section.width_twips)?;
    let height = required(section.height_twips)?;
    let mut left = required(section.margins.left)?;
    let mut right = required(section.margins.right)?;
    let mut top = margin(section.margins.top)?;
    let bottom = margin(section.margins.bottom)?;
    let gutter = section.margins.gutter.unwrap_or(0);
    match gutter_side {
        GutterSide::Left => left = add(left, gutter)?,
        GutterSide::Right => right = add(right, gutter)?,
        GutterSide::Top => top = add(top, gutter)?,
    }
    let content = Rect {
        x: left, y: top,
        width: subtract_positive(width, add(left, right)?)?,
        height: subtract_positive(height, add(top, bottom)?)?,
    };
    let page = Rect { x: 0, y: 0, width, height };
    let mut columns = Vec::new();
    let mut offset = 0u32;
    let c = section.columns.as_ref();
    let separator = c.and_then(|v| v.separator).unwrap_or(false);
    if let Some(c) = c.filter(|v| v.equal_width == Some(false)) {
        let count = c.explicit.len();
        if !(1..=64).contains(&count) { return Err(Error::Invalid("invalid explicit column count")); }
        if c.count.is_some_and(|v| usize::from(v) != count) {
            return Err(Error::Invalid("explicit column count mismatch"));
        }
        for (i, col) in c.explicit.iter().enumerate() {
            let w = required(col.width_twips)?;
            if w == 0 { return Err(Error::Invalid("zero column width")); }
            let end = add(offset, w)?;
            if end > content.width { return Err(Error::Invalid("columns exceed content width")); }
            columns.push(Rect { x: add(content.x, offset)?, y: content.y, width: w, height: content.height });
            offset = end;
            if i + 1 < count {
                offset = add(offset, col.spacing_twips.or(c.spacing_twips).unwrap_or(720))?;
                if offset >= content.width { return Err(Error::Invalid("columns exceed content width")); }
            }
        }
    } else {
        let count = u32::from(c.and_then(|v| v.count).unwrap_or(1));
        if !(1..=64).contains(&count) { return Err(Error::Limit("section column count")); }
        let gap = c.and_then(|v| v.spacing_twips).unwrap_or(720);
        let gaps = gap.checked_mul(count - 1).ok_or(Error::Limit("page geometry arithmetic"))?;
        let usable = subtract_positive(content.width, gaps)?;
        let base = usable / count;
        let remainder = usable % count;
        if base == 0 { return Err(Error::Invalid("zero column width")); }
        for i in 0..count {
            let w = base + u32::from(i < remainder);
            columns.push(Rect { x: add(content.x, offset)?, y: content.y, width: w, height: content.height });
            offset = add(offset, w)?;
            if i + 1 < count { offset = add(offset, gap)?; }
        }
    }
    Ok(PageGeometry { page, content, columns, column_separator: separator,
        trailing_width: content.width.checked_sub(offset).ok_or(Error::Invalid("columns exceed content width"))? })
}
