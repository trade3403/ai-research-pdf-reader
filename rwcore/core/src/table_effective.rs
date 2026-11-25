//! C2.3c-3: direct effective cell properties and checked preferred widths.
use crate::{Appearance, CellVerticalAlign, Error, TableEdge, TableLayout, TableShading, TableWidth};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableDirection { LeftToRight, RightToLeft }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertySource { Table, Cell }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sourced<T> {
    pub value: T,
    pub source: PropertySource,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferredWidth { Twips(u32), Auto, Nil }

/// Percent50 uses 5000 = 100%. Round to nearest twip, ties upward.
/// The reference width is supplied by the caller (page content for tables,
/// final table width for cells); this is not a column allocation algorithm.
/// Values above 100% are not silently clamped. Auto and Nil stay distinct.
pub fn resolve_preferred_width(value: TableWidth, reference_twips: u32) -> Result<PreferredWidth, Error> {
    Ok(match value {
        TableWidth::Twips(v) => PreferredWidth::Twips(v),
        TableWidth::Percent50(v) => {
            let n = (u64::from(v) * u64::from(reference_twips)).checked_add(2500)
                .ok_or(Error::Limit("preferred width arithmetic"))? / 5000;
            PreferredWidth::Twips(u32::try_from(n).map_err(|_| Error::Limit("preferred width overflow"))?)
        }
        TableWidth::Auto => PreferredWidth::Auto,
        TableWidth::Nil => PreferredWidth::Nil,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveCell<'a> {
    /// Physical edges only. Missing edges stay unresolved.
    pub margins: BTreeMap<TableEdge, Sourced<TableWidth>>,
    /// Whole cell shading overrides table shading in this direct subset.
    /// Raw pattern and color are not interpreted as rendered pixels.
    pub shading: Option<Sourced<&'a TableShading>>,
    pub width: Option<Sourced<TableWidth>>,
    pub layout: Option<Sourced<TableLayout>>,
    pub vertical_align: Option<Sourced<CellVerticalAlign>>,
}

fn physical_edge(e: TableEdge, direction: TableDirection) -> Result<TableEdge, Error> {
    use TableDirection::*;
    use TableEdge::*;
    Ok(match (e, direction) {
        (Start, LeftToRight) | (End, RightToLeft) => Left,
        (End, LeftToRight) | (Start, RightToLeft) => Right,
        (Top | Bottom | Left | Right, _) => e,
        _ => return Err(Error::Invalid("nonphysical margin edge")),
    })
}

fn normalize_margins(appearance: &Appearance, direction: TableDirection,
    source: PropertySource) -> Result<BTreeMap<TableEdge, Sourced<TableWidth>>, Error>
{
    let mut out = BTreeMap::new();
    if let Some(margins) = &appearance.margins {
        for (&edge, &value) in margins {
            let physical = physical_edge(edge, direction)?;
            if let Some(previous) = out.insert(physical, Sourced { value, source }) {
                // Equal physical/logical aliases are harmless. Different values
                // within a single layer are ambiguous in this initial subset.
                if previous.value != value { return Err(Error::Invalid("conflicting margin aliases")); }
            }
        }
    }
    Ok(out)
}

/// Resolve direct table defaults and cell overrides, without changing either.
/// Merge margins edge by edge after direction mapping. An empty cell margin
/// container does not clear inherited edges. Explicit zero remains an override.
/// Width is cell-only (never inherit the table's preferred width).
/// Layout is table-only; vertical alignment is cell-only.
/// Border conflict resolution, conditional table styles, theme resolution and
/// complete Word shading semantics are intentionally not implemented here.
pub fn resolve_effective_cell<'a>(table: &'a Appearance, cell: &'a Appearance,
    direction: TableDirection) -> Result<EffectiveCell<'a>, Error>
{
    let mut margins = normalize_margins(table, direction, PropertySource::Table)?;
    margins.extend(normalize_margins(cell, direction, PropertySource::Cell)?);
    let shading = cell.shading.as_ref().map(|value| Sourced { value, source: PropertySource::Cell })
        .or_else(|| table.shading.as_ref().map(|value| Sourced { value, source: PropertySource::Table }));
    Ok(EffectiveCell {
        margins, shading,
        width: cell.width.map(|value| Sourced { value, source: PropertySource::Cell }),
        layout: table.layout.map(|value| Sourced { value, source: PropertySource::Table }),
        vertical_align: cell.vertical_align.map(|value| Sourced { value, source: PropertySource::Cell }),
    })
}
