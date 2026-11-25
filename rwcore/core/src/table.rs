//! C2.3a: bounded source-order table, row, cell and grid parsing.
use crate::{Block, Paragraph, Run, Error, Limits};
use crate::opc::{xml, named, parse_paragraph_node, parse_run_node};
use roxmltree::Node;
use crate::table_properties::{Appearance, appearance_property, RowHeight, row_height};

const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const MAX_COLUMNS: usize = 256;
const MAX_ROWS: usize = 16384;
const MAX_CELLS: usize = 65536;
const MAX_NESTING: usize = 16;
const MAX_TOTAL_TABLES: usize = 4096;
const MAX_TOTAL_ROWS: usize = 32768;

/// Shared by a whole standalone table tree or by the main document body.
#[derive(Default)]
pub(crate) struct TableBudget {
    tables: usize,
    rows: usize,
    cells: usize,
}
fn spend(used: &mut usize, limit: usize, label: &'static str) -> Result<(), Error> {
    if *used >= limit { return Err(Error::Limit(label)); }
    *used += 1;
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    /// Source grid widths, in integer twips; missing width stays unspecified.
    pub grid: Vec<Option<u32>>,
    pub appearance: Appearance,
    pub rows: Vec<TableRow>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub grid_before: Option<u16>,
    pub grid_after: Option<u16>,
    pub height: Option<RowHeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalMerge { Restart, Continue }

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableCell {
    pub blocks: Vec<Block>,
    /// Raw span and merge markers. No logical merge resolution yet.
    pub grid_span: Option<u16>,
    pub vertical_merge: Option<VerticalMerge>,
    pub appearance: Appearance,
}

fn bounded_columns(value: Option<&str>, positive: bool) -> Result<u16, Error> {
    let n: u16 = value.ok_or(Error::Invalid("missing table grid count"))?
        .parse().map_err(|_| Error::Invalid("invalid table grid count"))?;
    if usize::from(n) > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
    if positive && n == 0 { return Err(Error::Invalid("zero table grid span")); }
    Ok(n)
}

fn paragraph(node: Node<'_, '_>, ns: &str, warnings: &mut Vec<String>) -> Paragraph {
    let mut p = Paragraph::default();
    let mut seen = false;
    for child in node.children().filter(|n| n.is_element()) {
        if named(child, ns, "pPr") {
            if seen { warnings.push("Duplicate cell paragraph properties".into()); continue; }
            seen = true;
            p.properties = parse_paragraph_node(child, ns);
            warnings.extend(p.properties.diagnostics.iter().cloned());
        } else if named(child, ns, "r") {
            let mut run = Run::default();
            let mut seen = false;
            for item in child.children().filter(|n| n.is_element()) {
                if named(item, ns, "rPr") {
                    if seen { warnings.push("Duplicate cell run properties".into()); continue; }
                    seen = true;
                    run.properties = parse_run_node(item, ns);
                    warnings.extend(run.properties.diagnostics.iter().cloned());
                } else if named(item, ns, "t") {
                    for text in item.children().filter(|n| n.is_text()) {
                        run.text.push_str(text.text().unwrap_or_default());
                    }
                    if item.children().any(|n| n.is_element()) {
                        warnings.push("Unsupported nested cell text".into());
                    }
                } else if named(item, ns, "tab") {
                    run.text.push('\t');
                } else if named(item, ns, "br") && matches!(item.attribute((ns,"type")), None | Some("textWrapping")) {
                    run.text.push('\n');
                } else {
                    warnings.push(format!("Unsupported cell run element: {}",item.tag_name().name()));
                }
            }
            p.runs.push(run);
        } else {
            warnings.push(format!("Unsupported cell paragraph element: {}",child.tag_name().name()));
        }
    }
    p
}

fn cell(node: Node<'_, '_>, ns: &str, warnings: &mut Vec<String>, depth: usize, budget: &mut TableBudget) -> Result<TableCell, Error> {
    let mut out = TableCell::default();
    let mut seen = false;
    for child in node.children().filter(|n| n.is_element()) {
        if named(child, ns, "tcPr") {
            if seen { return Err(Error::Invalid("duplicate table cell properties")); }
            seen = true;
            for prop in child.children().filter(|n| n.is_element()) {
                if named(prop, ns, "gridSpan") {
                    if out.grid_span.is_some() { return Err(Error::Invalid("duplicate gridSpan")); }
                    out.grid_span = Some(bounded_columns(prop.attribute((ns,"val")), true)?);
                } else if named(prop, ns, "vMerge") {
                    if out.vertical_merge.is_some() { return Err(Error::Invalid("duplicate vMerge")); }
                    out.vertical_merge = Some(match prop.attribute((ns,"val")) {
                        Some("restart") => VerticalMerge::Restart,
                        None | Some("continue") => VerticalMerge::Continue,
                        _ => return Err(Error::Invalid("invalid vertical merge marker")),
                    });
                } else if !appearance_property(prop, ns, false, &mut out.appearance, warnings)? {
                    warnings.push(format!("Unsupported table cell property: {}",prop.tag_name().name()));
                }
            }
        } else if named(child, ns, "p") {
            out.blocks.push(Block::Paragraph(paragraph(child, ns, warnings)));
        } else if named(child, ns, "tbl") {
            let nested = parse_table_node_budget(child, ns, depth + 1, budget)?;
            warnings.extend(nested.diagnostics.iter().cloned());
            out.blocks.push(Block::Table(Box::new(nested)));
        } else {
            let name = child.tag_name().name().to_owned();
            warnings.push(format!("Unsupported table cell block: {name}"));
            out.blocks.push(Block::Unsupported { name });
        }
    }
    Ok(out)
}

/// Parse a bounded table tree, retaining nested tables without resolving merges.
/// Missing grid metadata and ragged rows are retained, not silently repaired.
pub fn parse_table(bytes: &[u8], limits: Limits) -> Result<Table, Error> {
    if bytes.len() > limits.max_part_bytes { return Err(Error::Limit("part bytes")); }
    let tree = xml(bytes, limits)?;
    let root = tree.root_element();
    let ns = root.tag_name().namespace().ok_or(Error::Invalid("table namespace"))?;
    if !matches!(ns, WORD | STRICT) || !named(root, ns, "tbl") {
        return Err(Error::Invalid("table root"));
    }
    parse_table_node_budget(root, ns, 1, &mut TableBudget::default())
}

pub(crate) fn parse_table_node_budget(root: Node<'_, '_>, ns: &str, depth: usize, budget: &mut TableBudget) -> Result<Table, Error> {
    if depth > MAX_NESTING { return Err(Error::Limit("table nesting depth")); }
    spend(&mut budget.tables, MAX_TOTAL_TABLES, "total tables")?;
    let mut out = Table::default();
    let mut grid_seen = false;
    let mut props_seen = false;
    let mut cells = 0usize;
    for child in root.children().filter(|n| n.is_element()) {
        if named(child, ns, "tblGrid") {
            if grid_seen { return Err(Error::Invalid("duplicate table grid")); }
            grid_seen = true;
            for col in child.children().filter(|n| n.is_element()) {
                if !named(col, ns, "gridCol") {
                    out.diagnostics.push(format!("Unsupported table grid element: {}",col.tag_name().name()));
                    continue;
                }
                if out.grid.len() >= MAX_COLUMNS { return Err(Error::Limit("table columns")); }
                let width = col.attribute((ns,"w")).map(|s| s.parse::<u32>()
                    .map_err(|_| Error::Invalid("invalid table grid width"))).transpose()?;
                out.grid.push(width);
            }
        } else if named(child, ns, "tblPr") {
            if props_seen { return Err(Error::Invalid("duplicate table properties")); }
            props_seen = true;
            for prop in child.children().filter(|n| n.is_element()) {
                if !appearance_property(prop, ns, true, &mut out.appearance, &mut out.diagnostics)? {
                    out.diagnostics.push(format!("Unsupported table property: {}",prop.tag_name().name()));
                }
            }
        } else if named(child, ns, "tr") {
            if out.rows.len() >= MAX_ROWS { return Err(Error::Limit("table rows")); }
            spend(&mut budget.rows, MAX_TOTAL_ROWS, "total table rows")?;
            let mut row = TableRow::default();
            let mut props_seen = false;
            for item in child.children().filter(|n| n.is_element()) {
                if named(item, ns, "trPr") {
                    if props_seen { return Err(Error::Invalid("duplicate table row properties")); }
                    props_seen = true;
                    for prop in item.children().filter(|n| n.is_element()) {
                        if named(prop, ns, "trHeight") {
                            row_height(prop, ns, &mut row.height)?;
                            continue;
                        }
                        let slot = if named(prop, ns, "gridBefore") { &mut row.grid_before }
                            else if named(prop, ns, "gridAfter") { &mut row.grid_after }
                            else {
                                out.diagnostics.push(format!("Unsupported table row property: {}",prop.tag_name().name()));
                                continue;
                            };
                        if slot.is_some() { return Err(Error::Invalid("duplicate row grid offset")); }
                        *slot = Some(bounded_columns(prop.attribute((ns,"val")), false)?);
                    }
                } else if named(item, ns, "tc") {
                    if row.cells.len() >= MAX_COLUMNS { return Err(Error::Limit("table columns")); }
                    if cells >= MAX_CELLS { return Err(Error::Limit("table cells")); }
                    cells += 1;
                    spend(&mut budget.cells, MAX_CELLS, "total table cells")?;
                    row.cells.push(cell(item, ns, &mut out.diagnostics, depth, budget)?);
                } else {
                    out.diagnostics.push(format!("Unsupported table row element: {}",item.tag_name().name()));
                }
            }
            let occupied = row.cells.iter().map(|c| usize::from(c.grid_span.unwrap_or(1))).sum::<usize>()
                + usize::from(row.grid_before.unwrap_or(0)) + usize::from(row.grid_after.unwrap_or(0));
            if occupied > MAX_COLUMNS { return Err(Error::Limit("table columns")); }
            out.rows.push(row);
        } else {
            out.diagnostics.push(format!("Unsupported table element: {}",child.tag_name().name()));
        }
    }
    if !out.grid.is_empty() {
        for (i,row) in out.rows.iter().enumerate() {
            let occupied = row.cells.iter().map(|c| usize::from(c.grid_span.unwrap_or(1))).sum::<usize>()
                + usize::from(row.grid_before.unwrap_or(0)) + usize::from(row.grid_after.unwrap_or(0));
            if occupied != out.grid.len() {
                out.diagnostics.push(format!("Table row {i} differs from declared grid"));
            }
        }
    }
    Ok(out)
}
