//! C2.3c-1: direct width, border and margin metadata, independent of layout.
use crate::{Error, opc::named};
use roxmltree::Node;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableWidth { Twips(u32), Percent50(u32), Auto, Nil }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableEdge { Top, Bottom, Left, Right, Start, End, InsideH, InsideV, Tl2Br, Tr2Bl }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableColor { Auto, Rgb(u32) }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableBorder {
    pub style: String,
    pub size_eighth_points: Option<u32>,
    pub space_points: Option<u32>,
    pub color: Option<TableColor>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Appearance {
    pub width: Option<TableWidth>,
    pub borders: Option<BTreeMap<TableEdge,TableBorder>>,
    pub margins: Option<BTreeMap<TableEdge,TableWidth>>,
}
fn set<T>(slot: &mut Option<T>, value: T) -> Result<(), Error> {
    if slot.is_some() { return Err(Error::Invalid("duplicate table appearance property")); }
    *slot=Some(value); Ok(())
}
fn number(n: Node<'_, '_>, ns: &str, key: &str) -> Result<Option<u32>, Error> {
    n.attribute((ns,key)).map(|s|s.parse().map_err(|_|Error::Invalid("invalid table property integer"))).transpose()
}
fn width(n: Node<'_, '_>, ns: &str) -> Result<TableWidth, Error> {
    let w=number(n,ns,"w")?;
    match n.attribute((ns,"type")).unwrap_or("dxa") {
        "dxa"=>Ok(TableWidth::Twips(w.ok_or(Error::Invalid("missing table width"))?)),
        "pct"=>Ok(TableWidth::Percent50(w.ok_or(Error::Invalid("missing table width"))?)),
        "auto" if w.unwrap_or(0)==0=>Ok(TableWidth::Auto),
        "nil" if w.unwrap_or(0)==0=>Ok(TableWidth::Nil),
        "auto"|"nil"=>Err(Error::Invalid("nonzero auto/nil width")),
        _=>Err(Error::Unsupported("table width unit")),
    }
}
fn edge(n: Node<'_, '_>, ns: &str) -> Option<TableEdge> {
    if n.tag_name().namespace()!=Some(ns) { return None; }
    Some(match n.tag_name().name() {
        "top"=>TableEdge::Top,"bottom"=>TableEdge::Bottom,"left"=>TableEdge::Left,
        "right"=>TableEdge::Right,"start"=>TableEdge::Start,"end"=>TableEdge::End,
        "insideH"=>TableEdge::InsideH,"insideV"=>TableEdge::InsideV,
        "tl2br"=>TableEdge::Tl2Br,"tr2bl"=>TableEdge::Tr2Bl,_=>return None,
    })
}
fn color(s: &str) -> Result<TableColor, Error> {
    if s=="auto" {return Ok(TableColor::Auto);}
    if s.len()!=6 || !s.bytes().all(|b|b.is_ascii_hexdigit()) {return Err(Error::Invalid("invalid table color"));}
    Ok(TableColor::Rgb(u32::from_str_radix(s,16).map_err(|_|Error::Invalid("invalid table color"))?))
}
pub(crate) fn appearance_property(n: Node<'_, '_>, ns: &str, table: bool, out: &mut Appearance, warnings: &mut Vec<String>) -> Result<bool, Error> {
    if named(n,ns,if table {"tblW"} else {"tcW"}) {
        set(&mut out.width,width(n,ns)?)?;
    } else if named(n,ns,if table {"tblBorders"} else {"tcBorders"}) {
        let mut map=BTreeMap::new();
        for item in n.children().filter(|n|n.is_element()) {
            let Some(e)=edge(item,ns) else {warnings.push(format!("Unsupported border edge: {}",item.tag_name().name()));continue;};
            if map.contains_key(&e) {return Err(Error::Invalid("duplicate table border edge"));}
            let style=item.attribute((ns,"val")).ok_or(Error::Invalid("missing border style"))?;
            if style.is_empty() || style.len()>128 || style.chars().any(char::is_whitespace) {return Err(Error::Invalid("invalid border style"));}
            for key in ["themeColor","themeTint","themeShade","shadow","frame"] {
                if item.attribute((ns,key)).is_some() {warnings.push(format!("Unsupported border attribute: {key}"));}
            }
            map.insert(e,TableBorder{style:style.into(),size_eighth_points:number(item,ns,"sz")?,space_points:number(item,ns,"space")?,color:item.attribute((ns,"color")).map(color).transpose()?});
        }
        set(&mut out.borders,map)?;
    } else if named(n,ns,if table {"tblCellMar"} else {"tcMar"}) {
        let mut map=BTreeMap::new();
        for item in n.children().filter(|n|n.is_element()) {
            let Some(e @ (TableEdge::Top|TableEdge::Bottom|TableEdge::Left|TableEdge::Right|TableEdge::Start|TableEdge::End))=edge(item,ns) else {warnings.push(format!("Unsupported margin edge: {}",item.tag_name().name()));continue;};
            if map.contains_key(&e) {return Err(Error::Invalid("duplicate table margin edge"));}
            map.insert(e,width(item,ns)?);
        }
        set(&mut out.margins,map)?;
    } else {return Ok(false);}
    Ok(true)
}
