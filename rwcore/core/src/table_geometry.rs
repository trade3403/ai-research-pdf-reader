//! C2.3f-1: fixed grid geometry in integer twips, independent of painting.
use crate::{Error, Table, Rect, CellAddress, LogicalTable, TableDirection, TableEdge,
    TableWidth, resolve_table_grid, resolve_effective_cell};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellInsets {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellGeometry {
    pub address: CellAddress,
    pub column: usize,
    pub column_span: usize,
    pub row_span: usize,
    pub outer: Rect,
    pub content: Rect,
    pub insets: CellInsets,
}
#[derive(Debug)]
pub struct TableGeometry<'a> {
    grid: LogicalTable<'a>,
    bounds: Rect,
    x_offsets: Vec<u32>,
    y_offsets: Vec<u32>,
    cells: Vec<CellGeometry>,
}
impl TableGeometry<'_> {
    pub fn bounds(&self) -> Rect { self.bounds }
    /// Offsets are relative to the table origin, including terminal edge.
    pub fn column_offsets(&self) -> &[u32] { &self.x_offsets }
    pub fn row_offsets(&self) -> &[u32] { &self.y_offsets }
    /// Exactly one geometry per logical merge anchor, in anchor order.
    pub fn cells(&self) -> &[CellGeometry] { &self.cells }
    /// Omitted slots have no geometry. Continuation slots resolve to the anchor.
    pub fn cell_at(&self,row:usize,column:usize)->Option<&CellGeometry>{
        self.cells.get(self.grid.cell_at(row,column)?.anchor)
    }
}
fn add(a:u32,b:u32)->Result<u32,Error>{
    a.checked_add(b).ok_or(Error::Limit("table geometry arithmetic"))
}
fn offsets(sizes:&[u32])->Result<Vec<u32>,Error>{
    let mut out=Vec::with_capacity(sizes.len()+1);
    let mut total=0;
    out.push(total);
    for &size in sizes{
        if size==0{return Err(Error::Invalid("zero table geometry dimension"));}
        total=add(total,size)?;
        out.push(total);
    }
    Ok(out)
}
fn inset_rect(rect:Rect,i:CellInsets)->Result<Rect,Error>{
    let width=rect.width.checked_sub(add(i.left,i.right)?)
        .ok_or(Error::Invalid("cell margins exceed geometry"))?;
    let height=rect.height.checked_sub(add(i.top,i.bottom)?)
        .ok_or(Error::Invalid("cell margins exceed geometry"))?;
    Ok(Rect{x:add(rect.x,i.left)?,y:add(rect.y,i.top)?,width,height})
}

/// Compute fixed geometry using explicit resolved positive column widths and
/// row heights supplied by the caller. No resizing, font measurement, or
/// pagination occurs. Grid order is physical left-to-right; direction maps
/// margin aliases only, not column order. Insets use the merge anchor's
/// effective direct margins (continuation properties stay in the source).
/// Missing edges use caller-provided defaults; Nil means zero in this subset.
/// Auto and percentage margins are unsupported rather than guessed.
/// Zero-sized content is allowed; overlarge margins fail. Borders are not
/// subtracted, and nested tables are not recursively positioned.
pub fn compute_table_geometry<'a>(table:&'a Table, column_widths:&[u32],
    row_heights:&[u32], origin:(u32,u32), direction:TableDirection,
    default_insets:CellInsets)->Result<TableGeometry<'a>,Error>
{
    let grid=resolve_table_grid(table)?;
    if grid.columns==0 || grid.rows.is_empty(){
        return Err(Error::Invalid("empty table geometry"));
    }
    if column_widths.len()!=grid.columns || row_heights.len()!=grid.rows.len(){
        return Err(Error::Invalid("table geometry dimension count"));
    }
    let x_offsets=offsets(column_widths)?;
    let y_offsets=offsets(row_heights)?;
    let width=*x_offsets.last().unwrap();
    let height=*y_offsets.last().unwrap();
    add(origin.0,width)?;
    add(origin.1,height)?;
    let bounds=Rect{x:origin.0,y:origin.1,width,height};
    let mut cells=Vec::with_capacity(grid.anchors.len());
    for anchor in &grid.anchors{
        let r=anchor.address.row;
        let c=anchor.column;
        let outer=Rect{x:add(origin.0,x_offsets[c])?,y:add(origin.1,y_offsets[r])?,
            width:x_offsets[c+anchor.column_span]-x_offsets[c],
            height:y_offsets[r+anchor.row_span]-y_offsets[r]};
        let source=&table.rows[r].cells[anchor.address.cell];
        let effective=resolve_effective_cell(&table.appearance,&source.appearance,direction)?;
        let margin=|edge,default|match effective.margins.get(&edge).map(|m|m.value){
            None=>Ok(default),
            Some(TableWidth::Twips(v))=>Ok(v),
            Some(TableWidth::Nil)=>Ok(0),
            Some(TableWidth::Auto|TableWidth::Percent50(_))=>Err(Error::Unsupported("nonfixed cell margin")),
        };
        let insets=CellInsets{
            left:margin(TableEdge::Left,default_insets.left)?,
            right:margin(TableEdge::Right,default_insets.right)?,
            top:margin(TableEdge::Top,default_insets.top)?,
            bottom:margin(TableEdge::Bottom,default_insets.bottom)?,
        };
        let content=inset_rect(outer,insets)?;
        cells.push(CellGeometry{address:anchor.address,column:c,column_span:anchor.column_span,
            row_span:anchor.row_span,outer,content,insets});
    }
    Ok(TableGeometry{grid,bounds,x_offsets,y_offsets,cells})
}

/// Convenience entry point: all source grid columns must have explicit widths.
/// Caller still supplies resolved row heights; preferred width metadata is not
/// interpreted as the final layout width.
pub fn compute_declared_grid_geometry<'a>(table:&'a Table,row_heights:&[u32],
    origin:(u32,u32),direction:TableDirection,default_insets:CellInsets)
    ->Result<TableGeometry<'a>,Error>
{
    if table.grid.len()>256{return Err(Error::Limit("table columns"));}
    let widths=table.grid.iter().map(|w|w.ok_or(Error::Invalid("missing declared column width")))
        .collect::<Result<Vec<_>,_>>()?;
    compute_table_geometry(table,&widths,row_heights,origin,direction,default_insets)
}
