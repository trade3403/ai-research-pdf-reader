//! C2.3d: direct collapsed-edge topology and strict three-line recognition.
use crate::{Appearance, Error, LogicalCell, LogicalTable, Table, TableBorder,
    TableDirection, TableEdge, resolve_table_grid};

#[derive(Debug)]
pub struct TableBorderPlan<'a> {
    table: &'a Table,
    grid: LogicalTable<'a>,
    direction: TableDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreeLineBorders<'a> {
    /// Grid row boundary between header and body, zero-based.
    pub header_boundary: usize,
    pub top: &'a TableBorder,
    pub middle: &'a TableBorder,
    pub bottom: &'a TableBorder,
}

fn side(a: &Appearance, edge: TableEdge, direction: TableDirection) -> Result<Option<&TableBorder>, Error> {
    use TableDirection::*;
    use TableEdge::*;
    let Some(b) = &a.borders else { return Ok(None); };
    let alias = match (edge, direction) {
        (Left, LeftToRight) | (Right, RightToLeft) => Some(Start),
        (Right, LeftToRight) | (Left, RightToLeft) => Some(End),
        _ => None,
    };
    let physical = b.get(&edge);
    let logical = alias.and_then(|e| b.get(&e));
    if let (Some(p), Some(l)) = (physical, logical) {
        if p != l { return Err(Error::Invalid("conflicting border aliases")); }
    }
    Ok(physical.or(logical))
}

fn validate(a: &Appearance, table: bool, direction: TableDirection) -> Result<(), Error> {
    use TableEdge::*;
    if let Some(b) = &a.borders {
        for edge in b.keys() {
            if matches!(edge, Tl2Br | Tr2Bl) || (!table && matches!(edge, InsideH | InsideV)) {
                return Err(Error::Unsupported("nonperimeter border topology"));
            }
        }
    }
    side(a, Left, direction)?;
    side(a, Right, direction)?;
    Ok(())
}

fn visible(b: &TableBorder) -> bool { !matches!(b.style.as_str(), "nil" | "none") }

/// Direct subset: a present cell edge overrides the table edge. Two adjacent
/// explicit cell edges must agree exactly, rather than guessing Word's
/// conflict rules. Explicit nil/none suppresses fallback. A shared merge
/// anchor suppresses the internal edge before any conflict check.
fn choose<'a>(a: Option<&LogicalCell<'a>>, a_edge: TableEdge,
    b: Option<&LogicalCell<'a>>, b_edge: TableEdge,
    fallback: Option<&'a TableBorder>, direction: TableDirection) -> Result<Option<&'a TableBorder>, Error>
{
    if let (Some(a), Some(b)) = (a,b) {
        if a.anchor == b.anchor { return Ok(None); }
    }
    let x = match a { Some(c) => side(&c.source.appearance,a_edge,direction)?, None => None };
    let y = match b { Some(c) => side(&c.source.appearance,b_edge,direction)?, None => None };
    if let (Some(x),Some(y)) = (x,y) {
        if x != y { return Err(Error::Unsupported("conflicting adjacent cell borders")); }
    }
    Ok(x.or(y).or(fallback).filter(|b| visible(b)))
}

impl<'a> TableBorderPlan<'a> {
    /// Uses checked logical grid, explicit direction, and collapsed edges.
    /// No cell spacing, style cascade, themes, diagonal edges or painting.
    pub fn new(table: &'a Table, direction: TableDirection) -> Result<Self,Error> {
        let grid = resolve_table_grid(table)?;
        validate(&table.appearance,true,direction)?;
        for row in &table.rows {
            for cell in &row.cells { validate(&cell.appearance,false,direction)?; }
        }
        Ok(Self {table,grid,direction})
    }
    pub fn row_count(&self) -> usize { self.grid.rows.len() }
    pub fn column_count(&self) -> usize { self.grid.columns }

    /// Horizontal unit edge at row boundary 0..=rows, column 0..columns.
    /// On ragged rows, only explicit cell borders apply at an internal gap;
    /// no table inside border is synthesized across omitted slots.
    pub fn horizontal(&self, boundary: usize, column: usize) -> Result<Option<&TableBorder>,Error> {
        use TableEdge::*;
        let rows = self.row_count();
        if boundary > rows || column >= self.column_count() {
            return Err(Error::Invalid("horizontal border coordinate"));
        }
        let a = boundary.checked_sub(1).and_then(|r|self.grid.cell_at(r,column));
        let b = self.grid.cell_at(boundary,column);
        let fallback = if boundary == 0 && b.is_some() { Some(Top) }
            else if boundary == rows && a.is_some() { Some(Bottom) }
            else if a.is_some() && b.is_some() { Some(InsideH) } else { None };
        let fallback = match fallback {
            Some(e) => side(&self.table.appearance,e,self.direction)?, None => None,
        };
        choose(a,Bottom,b,Top,fallback,self.direction)
    }

    /// Vertical unit edge at row 0..rows, column boundary 0..=columns.
    pub fn vertical(&self, row: usize, boundary: usize) -> Result<Option<&TableBorder>,Error> {
        use TableEdge::*;
        if row >= self.row_count() || boundary > self.column_count() {
            return Err(Error::Invalid("vertical border coordinate"));
        }
        let a = boundary.checked_sub(1).and_then(|c|self.grid.cell_at(row,c));
        let b = self.grid.cell_at(row,boundary);
        let fallback = if boundary == 0 && b.is_some() { Some(Left) }
            else if boundary == self.column_count() && a.is_some() { Some(Right) }
            else if a.is_some() && b.is_some() { Some(InsideV) } else { None };
        let fallback = match fallback {
            Some(e) => side(&self.table.appearance,e,self.direction)?, None => None,
        };
        choose(a,Right,b,Left,fallback,self.direction)
    }

    /// Strict recognition of exactly three complete uniform single-rule
    /// horizontal lines (top/header separator/bottom) and no vertical lines.
    /// Caller identifies header row count; no semantic header inference.
    /// Sparse/ragged tables and incomplete or split lines do not match.
    /// Scan work is bounded independently of the sparse source model.
    pub fn three_line(&self, header_rows: usize) -> Result<Option<ThreeLineBorders<'_>>,Error> {
        let rows = self.row_count();
        let cols = self.column_count();
        if header_rows == 0 || header_rows >= rows || cols == 0 {
            return Err(Error::Invalid("three-line header boundary"));
        }
        if rows.checked_mul(cols).is_none_or(|n| n > 1_000_000) {
            return Err(Error::Limit("three-line grid scan"));
        }
        for row in 0..rows {
            for col in 0..cols {
                if self.grid.cell_at(row,col).is_none() { return Ok(None); }
            }
            for boundary in 0..=cols {
                if self.vertical(row,boundary)?.is_some() { return Ok(None); }
            }
        }
        let mut lines = [None;3];
        for boundary in 0..=rows {
            let index = if boundary == 0 { Some(0) }
                else if boundary == header_rows { Some(1) }
                else if boundary == rows { Some(2) } else { None };
            for col in 0..cols {
                let edge = self.horizontal(boundary,col)?;
                match (index,edge) {
                    (Some(i),Some(b)) if b.style == "single" => {
                        if lines[i].is_some_and(|prev| prev != b) { return Ok(None); }
                        lines[i] = Some(b);
                    }
                    (None,None) => (),
                    _ => return Ok(None),
                }
            }
        }
        match lines {
            [Some(top),Some(middle),Some(bottom)] =>
                Ok(Some(ThreeLineBorders{header_boundary:header_rows,top,middle,bottom})),
            _ => Ok(None),
        }
    }
}
