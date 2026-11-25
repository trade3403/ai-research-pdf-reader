//! C2.3f-4: bounded recursive layout with external measurements.
use crate::{Table, Paragraph, Block, Error, Rect, CellAddress, CellInsets, TableDirection,
    TableColumnLayout, TableRowLayout, CellWidthMeasurement, CellMeasurement,
    allocate_table_columns, layout_table_rows, resolve_table_grid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedLayoutOptions {
    pub direction: TableDirection,
    pub default_insets: CellInsets,
    pub min_column_width: u32,
    pub min_auto_height: u32,
}
/// Widths must account for the entire anchor content, including nested tables.
/// Height is measured after actual content width is allocated.
pub trait TableMeasurer {
    fn cell_widths(&mut self, table:&Table, address:CellAddress)->Result<(u32,u32),Error>;
    fn paragraph_height(&mut self, paragraph:&Paragraph, available_width:u32)->Result<u32,Error>;
}
#[derive(Debug)]
pub struct NestedTableLayout<'a> {
    /// Local table layout, always with origin (0,0).
    pub columns: TableColumnLayout,
    pub rows: TableRowLayout<'a>,
    pub cells: Vec<NestedCellLayout<'a>>,
    /// Absolute table rectangle; local geometry is not mutated.
    pub bounds: Rect,
}
#[derive(Debug)]
pub struct NestedCellLayout<'a> {
    pub address: CellAddress,
    pub blocks: Vec<NestedBlockLayout<'a>>,
}
#[derive(Debug)]
pub enum NestedBlockLayout<'a> {
    Paragraph { source:&'a Paragraph, bounds:Rect },
    Table(Box<NestedTableLayout<'a>>),
}
#[derive(Default)]
struct Budget { tables:usize, rows:usize, cells:usize, blocks:usize }
fn spend(used:&mut usize,amount:usize,limit:usize,label:&'static str)->Result<(),Error>{
    *used=used.checked_add(amount).filter(|v|*v<=limit).ok_or(Error::Limit(label))?;
    Ok(())
}
fn add(a:u32,b:u32)->Result<u32,Error>{
    a.checked_add(b).ok_or(Error::Limit("nested table coordinates"))
}
fn translate(rect:Rect,x:u32,y:u32)->Result<Rect,Error>{
    let rect=Rect{x:add(rect.x,x)?,y:add(rect.y,y)?,..rect};
    add(rect.x,rect.width)?;
    add(rect.y,rect.height)?;
    Ok(rect)
}
impl NestedTableLayout<'_>{
    fn translate(&mut self,x:u32,y:u32)->Result<(),Error>{
        self.bounds=translate(self.bounds,x,y)?;
        for cell in &mut self.cells{
            for block in &mut cell.blocks{
                match block{
                    NestedBlockLayout::Paragraph{bounds,..}=>*bounds=translate(*bounds,x,y)?,
                    NestedBlockLayout::Table(child)=>child.translate(x,y)?,
                }
            }
        }
        Ok(())
    }
}
/// Layout mixed paragraph/table blocks in source order, left aligned.
/// Each child receives its parent's actual content width, then its height
/// contributes to the parent's measured height. Vertical positioning uses
/// the existing row solver. Absolute block bounds describe full measured
/// content; exact-row overflow is reported in rows.placements, not clipped.
/// Nonempty continuation content and unsupported blocks fail explicitly.
/// No font shaping, automatic intrinsic nested width measurement, painting,
/// inter-block spacing, or pagination is implemented by this coordinator.
pub fn layout_nested_table<'a,M:TableMeasurer>(table:&'a Table,available_width:u32,
    origin:(u32,u32),options:NestedLayoutOptions,measurer:&mut M)
    ->Result<NestedTableLayout<'a>,Error>
{
    let mut result=layout_local(table,available_width,options,measurer,1,&mut Budget::default())?;
    result.translate(origin.0,origin.1)?;
    Ok(result)
}
fn layout_local<'a,M:TableMeasurer>(table:&'a Table,available_width:u32,
    o:NestedLayoutOptions,measurer:&mut M,depth:usize,budget:&mut Budget)
    ->Result<NestedTableLayout<'a>,Error>
{
    if depth>16{return Err(Error::Limit("nested layout depth"));}
    spend(&mut budget.tables,1,4096,"nested layout tables")?;
    spend(&mut budget.rows,table.rows.len(),32768,"nested layout rows")?;
    let grid=resolve_table_grid(table)?;
    for row in &grid.rows{
        spend(&mut budget.cells,row.len(),65536,"nested layout cells")?;
        for cell in row{
            spend(&mut budget.blocks,cell.source.blocks.len(),200000,"nested layout blocks")?;
            if grid.anchors[cell.anchor].address!=cell.address &&
                cell.source.blocks.iter().any(|b|!matches!(b,Block::Paragraph(p) if p.runs.is_empty())){
                return Err(Error::Unsupported("nonempty merge continuation layout"));
            }
        }
    }
    let mut widths=Vec::with_capacity(grid.anchors.len());
    for anchor in &grid.anchors{
        let (minimum,preferred)=measurer.cell_widths(table,anchor.address)?;
        widths.push(CellWidthMeasurement{address:anchor.address,minimum,preferred});
    }
    let columns=allocate_table_columns(table,available_width,&widths,o.direction,o.default_insets,o.min_column_width)?;
    let mut heights=Vec::with_capacity(grid.anchors.len());
    let mut cells=Vec::with_capacity(grid.anchors.len());
    for (anchor,width) in grid.anchors.iter().zip(&columns.cells){
        let source=&table.rows[anchor.address.row].cells[anchor.address.cell];
        let mut blocks=Vec::with_capacity(source.blocks.len());
        let mut height=0;
        for block in &source.blocks{
            match block{
                Block::Paragraph(p)=>{
                    let h=measurer.paragraph_height(p,width.content_width)?;
                    blocks.push(NestedBlockLayout::Paragraph{source:p,bounds:Rect{
                        x:0,y:height,width:width.content_width,height:h}});
                    height=add(height,h)?;
                }
                Block::Table(child)=>{
                    let mut child=layout_local(child,width.content_width,o,measurer,depth+1,budget)?;
                    let h=child.bounds.height;
                    child.translate(0,height)?;
                    height=add(height,h)?;
                    blocks.push(NestedBlockLayout::Table(Box::new(child)));
                }
                Block::Unsupported{..}=>return Err(Error::Unsupported("nested layout block")),
            }
        }
        heights.push(CellMeasurement{address:anchor.address,height});
        cells.push(NestedCellLayout{address:anchor.address,blocks});
    }
    let rows=layout_table_rows(table,&columns.column_widths,&heights,(0,0),o.direction,o.default_insets,o.min_auto_height)?;
    for (cell,placement) in cells.iter_mut().zip(&rows.placements){
        for block in &mut cell.blocks{
            match block{
                NestedBlockLayout::Paragraph{bounds,..}=>*bounds=translate(*bounds,placement.rect.x,placement.rect.y)?,
                NestedBlockLayout::Table(child)=>child.translate(placement.rect.x,placement.rect.y)?,
            }
        }
    }
    let bounds=rows.geometry.bounds();
    Ok(NestedTableLayout{columns,rows,cells,bounds})
}
