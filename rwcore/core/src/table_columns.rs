//! C2.3f-3: bounded integer column allocation from measured widths.
use crate::{Table, Error, CellAddress, CellInsets, TableDirection, TableLayout, TableWidth,
    PreferredWidth, resolve_preferred_width, resolve_table_grid};
use crate::table_geometry::resolve_cell_insets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellWidthMeasurement {
    pub address: CellAddress,
    /// Measured content widths, excluding insets.
    pub minimum: u32,
    pub preferred: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellWidthResult {
    pub address: CellAddress,
    pub content_width: u32,
    pub minimum_overflow: u32,
    pub preferred_shortfall: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumnLayout {
    pub layout: TableLayout,
    pub width: u32,
    pub column_widths: Vec<u32>,
    pub cells: Vec<CellWidthResult>,
}
fn add(a:u32,b:u32)->Result<u32,Error>{
    a.checked_add(b).ok_or(Error::Limit("table column arithmetic"))
}
fn sum(v:&[u32])->u64{v.iter().map(|&x|u64::from(x)).sum()}
fn preferred(value:Option<TableWidth>,base:u32)->Result<Option<u32>,Error>{
    match value {
        Some(v)=>match resolve_preferred_width(v,base)?{
            PreferredWidth::Twips(w)=>Ok(Some(w)),
            PreferredWidth::Auto|PreferredWidth::Nil=>Ok(None),
        },
        None=>Ok(None),
    }
}
/// Allocate integer amount in proportion to nonnegative weights, using largest
/// remainder with lower column index as the tie breaker. Positive sum required.
fn apportion(amount:u32,weights:&[u32],out:&mut [u32])->Result<(),Error>{
    let total=sum(weights);
    if amount==0{return Ok(());}
    if total==0{return Err(Error::Invalid("zero column weights"));}
    let mut used=0u32;
    let mut remainders=Vec::with_capacity(weights.len());
    for (i,&weight) in weights.iter().enumerate(){
        let product=u64::from(amount)*u64::from(weight);
        let share=(product/total) as u32;
        out[i]=add(out[i],share)?;
        used=add(used,share)?;
        remainders.push((product%total,i));
    }
    remainders.sort_unstable_by(|a,b|b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_,i) in remainders.iter().take((amount-used) as usize){
        out[i]=add(out[i],1)?;
    }
    Ok(())
}

/// Fixed and autofit allocation subset. Table dxa/pct width is an explicit
/// target; missing/auto/nil uses the available width. A target outside the
/// positive available budget fails rather than silently resizing the page.
/// Every column has a positive minimum. Anchor measurements must be in grid
/// anchor order. In both modes insets are hard constraints; in autofit the
/// measured minimum is also hard. Fixed mode reports measured overflow.
/// Interval lower bounds are solved by a forward longest-path recurrence over
/// prefix coordinates (minimal feasible total width). Remaining width first
/// satisfies soft preferences proportionally, then uses source grid weights
/// (missing/zero weight = 1). Cell dxa/pct preferences are soft, split evenly
/// across spans; autofit also uses measured preferred width plus insets.
/// This deterministic policy does not reproduce Word's full autofit semantics.
pub fn allocate_table_columns(table:&Table,available_width:u32,
    measurements:&[CellWidthMeasurement],direction:TableDirection,
    default_insets:CellInsets,min_column_width:u32)->Result<TableColumnLayout,Error>
{
    if available_width==0 || min_column_width==0{
        return Err(Error::Invalid("zero column allocation budget or minimum"));
    }
    let grid=resolve_table_grid(table)?;
    if grid.columns==0 || grid.rows.is_empty(){return Err(Error::Invalid("empty table geometry"));}
    if measurements.len()!=grid.anchors.len(){return Err(Error::Invalid("cell width measurement count"));}
    let target=preferred(table.appearance.width,available_width)?.unwrap_or(available_width);
    if target==0 || target>available_width{return Err(Error::Invalid("table width outside budget"));}
    let layout=table.appearance.layout.unwrap_or(TableLayout::Autofit);
    let n=grid.columns;
    let weights:Vec<u32>=(0..n).map(|c|table.grid.get(c).copied().flatten().unwrap_or(1).max(1)).collect();
    let mut desired:Vec<u32>=(0..n).map(|c|table.grid.get(c).copied().flatten().unwrap_or(0)).collect();
    let mut constraints:Vec<Vec<(usize,u32)>>=vec![Vec::new();n+1];
    let mut insets=Vec::with_capacity(measurements.len());
    for (anchor,m) in grid.anchors.iter().zip(measurements){
        if anchor.address!=m.address{return Err(Error::Invalid("cell width measurement address"));}
        if m.minimum>m.preferred{return Err(Error::Invalid("inverted measured width range"));}
        let source=&table.rows[anchor.address.row].cells[anchor.address.cell];
        let i=resolve_cell_insets(&table.appearance,&source.appearance,direction,default_insets)?;
        let padding=add(i.left,i.right)?;
        insets.push(padding);
        let required=if layout==TableLayout::Autofit{add(padding,m.minimum)?}else{padding};
        let start=anchor.column;
        let end=start+anchor.column_span;
        constraints[end].push((start,required));
        let mut wish=preferred(source.appearance.width,target)?.unwrap_or(0).max(required);
        if layout==TableLayout::Autofit{wish=wish.max(add(padding,m.preferred)?);}
        let span=anchor.column_span as u32;
        let each=wish/span;
        let extra=wish%span;
        for (offset,d) in desired[start..end].iter_mut().enumerate(){
            *d=(*d).max(each+u32::from((offset as u32)<extra));
        }
    }
    let mut prefix=vec![0u32;n+1];
    for end in 1..=n{
        let mut v=add(prefix[end-1],min_column_width)?;
        for &(start,required) in &constraints[end]{
            v=v.max(add(prefix[start],required)?);
        }
        prefix[end]=v;
    }
    if prefix[n]>target{return Err(Error::Invalid("minimum table width exceeds budget"));}
    let mut widths:Vec<u32>=prefix.windows(2).map(|p|p[1]-p[0]).collect();
    let mut remaining=target-prefix[n];
    let gaps:Vec<u32>=desired.iter().zip(&widths).map(|(&d,&w)|d.saturating_sub(w)).collect();
    let stage=u64::from(remaining).min(sum(&gaps)) as u32;
    apportion(stage,&gaps,&mut widths)?;
    remaining-=stage;
    apportion(remaining,&weights,&mut widths)?;
    let mut cells=Vec::with_capacity(measurements.len());
    for ((anchor,m),padding) in grid.anchors.iter().zip(measurements).zip(insets){
        let outer=u32::try_from(sum(&widths[anchor.column..anchor.column+anchor.column_span]))
            .map_err(|_|Error::Limit("table column arithmetic"))?;
        let content_width=outer.checked_sub(padding).ok_or(Error::Invalid("cell margins exceed geometry"))?;
        cells.push(CellWidthResult{address:m.address,content_width,
            minimum_overflow:m.minimum.saturating_sub(content_width),
            preferred_shortfall:m.preferred.saturating_sub(content_width)});
    }
    Ok(TableColumnLayout{layout,width:target,column_widths:widths,cells})
}
