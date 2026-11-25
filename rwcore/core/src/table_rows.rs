//! C2.3f-2: measured-content row constraints and vertical placement.
use crate::{Table, Error, Rect, CellAddress, CellVerticalAlign, RowHeightRule,
    TableDirection, CellInsets, TableGeometry, resolve_table_grid, compute_table_geometry};
use crate::table_geometry::resolve_cell_insets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMeasurement {
    /// Merge anchor source address; not continuation cell address.
    pub address: CellAddress,
    /// Content height excluding cell insets, measured externally in twips.
    pub height: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalPlacement {
    pub address: CellAddress,
    /// Visible-height content rectangle. Does not implement text clipping.
    pub rect: Rect,
    pub overflow_twips: u32,
}
#[derive(Debug)]
pub struct TableRowLayout<'a> {
    pub row_heights: Vec<u32>,
    pub geometry: TableGeometry<'a>,
    pub placements: Vec<VerticalPlacement>,
}
fn add(a:u32,b:u32)->Result<u32,Error>{
    a.checked_add(b).ok_or(Error::Limit("table row arithmetic"))
}

/// Position a measured content block inside its content box. Overflow starts
/// at the top, with the excess reported separately. Center rounds down.
/// Distributed ("both") alignment requires a line layout and is unsupported.
pub fn place_vertical(box_rect:Rect,height:u32,align:CellVerticalAlign)
    ->Result<(Rect,u32),Error>
{
    add(box_rect.y,box_rect.height)?;
    add(box_rect.x,box_rect.width)?;
    let free=box_rect.height.saturating_sub(height);
    let offset=match align{
        CellVerticalAlign::Top=>0,
        CellVerticalAlign::Center=>free/2,
        CellVerticalAlign::Bottom=>free,
        CellVerticalAlign::Both=>return Err(Error::Unsupported("distributed cell alignment")),
    };
    Ok((Rect{y:add(box_rect.y,offset)?,height:height.min(box_rect.height),..box_rect},
        height.saturating_sub(box_rect.height)))
}

/// Measurements must be supplied once per anchor, in logical anchor order.
/// Auto rows use caller's positive minimum; AtLeast rows respect their stored
/// minimum; Exact rows do not grow. Missing rule defaults to Auto.
/// Single-row constraints precede multi-row constraints. A merged deficit is
/// assigned to the last growable row in its span, deterministically in anchor
/// order. This is an initial monotonic policy, not Word's complete layout.
/// Fixed-row overflow is reported by placement; invalid geometry (e.g. insets
/// larger than an exact row) fails explicitly. No font measurement or painting.
pub fn layout_table_rows<'a>(table:&'a Table,column_widths:&[u32],
    measurements:&[CellMeasurement],origin:(u32,u32),direction:TableDirection,
    default_insets:CellInsets,min_auto_height:u32)->Result<TableRowLayout<'a>,Error>
{
    if min_auto_height==0{return Err(Error::Invalid("zero minimum row height"));}
    let grid=resolve_table_grid(table)?;
    if grid.anchors.len()!=measurements.len(){
        return Err(Error::Invalid("cell measurement count"));
    }
    let mut heights=Vec::with_capacity(table.rows.len());
    let mut grow=Vec::with_capacity(table.rows.len());
    for row in &table.rows{
        let h=row.height;
        let rule=h.and_then(|v|v.rule).unwrap_or(RowHeightRule::Auto);
        let value=h.and_then(|v|v.twips);
        let (height,flex)=match rule{
            RowHeightRule::Auto=>(min_auto_height,true),
            RowHeightRule::AtLeast=>(min_auto_height.max(value.ok_or(Error::Invalid("missing minimum row height"))?),true),
            RowHeightRule::Exact=>(value.filter(|v|*v>0).ok_or(Error::Invalid("invalid exact row height"))?,false),
        };
        heights.push(height);
        grow.push(flex);
    }
    let mut requirements=Vec::with_capacity(measurements.len());
    for (anchor,m) in grid.anchors.iter().zip(measurements){
        if anchor.address!=m.address{return Err(Error::Invalid("cell measurement address"));}
        let row=anchor.address.row;
        let source=&table.rows[row].cells[anchor.address.cell];
        let i=resolve_cell_insets(&table.appearance,&source.appearance,direction,default_insets)?;
        requirements.push(add(add(m.height,i.top)?,i.bottom)?);
    }
    for multi in [false,true]{
        for (anchor,&required) in grid.anchors.iter().zip(&requirements){
            if (anchor.row_span>1)!=multi{continue;}
            let start=anchor.address.row;
            let end=start+anchor.row_span;
            let current=heights[start..end].iter().try_fold(0u32,|a,&v|add(a,v))?;
            let deficit=required.saturating_sub(current);
            if deficit>0{
                if let Some(row)=(start..end).rev().find(|&r|grow[r]){
                    heights[row]=add(heights[row],deficit)?;
                }
            }
        }
    }
    let geometry=compute_table_geometry(table,column_widths,&heights,origin,direction,default_insets)?;
    let mut placements=Vec::with_capacity(measurements.len());
    for (cell,m) in geometry.cells().iter().zip(measurements){
        let align=table.rows[cell.address.row].cells[cell.address.cell].appearance.vertical_align
            .unwrap_or(CellVerticalAlign::Top);
        let (rect,overflow_twips)=place_vertical(cell.content,m.height,align)?;
        placements.push(VerticalPlacement{address:m.address,rect,overflow_twips});
    }
    Ok(TableRowLayout{row_heights:heights,geometry,placements})
}
