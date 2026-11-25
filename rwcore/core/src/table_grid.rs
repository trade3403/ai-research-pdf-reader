//! C2.3b: checked sparse logical grid and rectangular merge topology.
use crate::{Error, Table, TableCell, VerticalMerge};
use std::collections::BTreeMap;

const MAX_COLUMNS: usize = 256;
const MAX_ROWS: usize = 16384;
const MAX_CELLS: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    pub row: usize,
    /// Index in the source row's cell list, not a grid column.
    pub cell: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalCell<'a> {
    pub source: &'a TableCell,
    pub address: CellAddress,
    pub column: usize,
    pub column_span: usize,
    /// Index into LogicalTable::anchors.
    pub anchor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeAnchor {
    pub address: CellAddress,
    pub column: usize,
    pub column_span: usize,
    pub row_span: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalTable<'a> {
    pub columns: usize,
    pub rows: Vec<Vec<LogicalCell<'a>>>,
    pub anchors: Vec<MergeAnchor>,
}

impl<'a> LogicalTable<'a> {
    /// Locate the source cell covering a grid coordinate. Omitted slots are None.
    pub fn cell_at(&self, row: usize, column: usize) -> Option<&LogicalCell<'a>> {
        if column >= self.columns { return None; }
        let cells = self.rows.get(row)?;
        let i = cells.partition_point(|c| c.column <= column).checked_sub(1)?;
        let cell = &cells[i];
        (column - cell.column < cell.column_span).then_some(cell)
    }

    pub fn anchor_at(&self, row: usize, column: usize) -> Option<&MergeAnchor> {
        self.anchors.get(self.cell_at(row, column)?.anchor)
    }
}

/// Explicit nonempty grids constrain the row's total occupancy; ragged rows
/// within the declared grid retain unoccupied slots. When no grid is supplied,
/// the width is inferred from maximum source row occupancy.
/// Vertical continuations must match a restart or continuation in the
/// immediately preceding row at exactly the same column and width.
/// Legacy hMerge markers are not interpreted. No source content is moved,
/// concatenated or discarded, including content in continuation cells.
/// Storage is proportional to source cells, not the full rectangular grid.
pub fn resolve_table_grid(table: &Table) -> Result<LogicalTable<'_>, Error> {
    if table.grid.len() > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
    if table.rows.len() > MAX_ROWS { return Err(Error::Limit("table rows")); }
    let mut out = LogicalTable { columns: table.grid.len(), rows: Vec::with_capacity(table.rows.len()), anchors: Vec::new() };
    let mut previous: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut total = 0usize;
    for (row_index, row) in table.rows.iter().enumerate() {
        if row.cells.len() > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
        total = total.checked_add(row.cells.len()).ok_or(Error::Limit("table cells"))?;
        if total > MAX_CELLS { return Err(Error::Limit("table cells")); }
        let mut col = usize::from(row.grid_before.unwrap_or(0));
        let mut active = BTreeMap::new();
        let mut cells = Vec::with_capacity(row.cells.len());
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let span = usize::from(cell.grid_span.unwrap_or(1));
            if span == 0 { return Err(Error::Invalid("zero table grid span")); }
            let end = col.checked_add(span).ok_or(Error::Limit("table columns"))?;
            if end > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
            let address = CellAddress { row: row_index, cell: cell_index };
            let key = (col, span);
            let anchor = match cell.vertical_merge {
                Some(VerticalMerge::Continue) => {
                    let index = *previous.get(&key).ok_or(Error::Invalid("unmatched vertical merge continuation"))?;
                    out.anchors[index].row_span += 1;
                    active.insert(key, index);
                    index
                }
                marker => {
                    let index = out.anchors.len();
                    out.anchors.push(MergeAnchor { address, column: col, column_span: span, row_span: 1 });
                    if marker == Some(VerticalMerge::Restart) { active.insert(key, index); }
                    index
                }
            };
            cells.push(LogicalCell { source: cell, address, column: col, column_span: span, anchor });
            col = end;
        }
        let occupied = col.checked_add(usize::from(row.grid_after.unwrap_or(0))).ok_or(Error::Limit("table columns"))?;
        if occupied > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
        if !table.grid.is_empty() && occupied > table.grid.len() {
            return Err(Error::Invalid("row exceeds declared table grid"));
        }
        out.columns = out.columns.max(occupied);
        out.rows.push(cells);
        previous = active;
    }
    Ok(out)
}
