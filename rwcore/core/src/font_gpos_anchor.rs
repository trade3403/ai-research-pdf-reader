//! Anchor formats 1–3 and checked design-unit/pixel adjustment math.
//! Outline point resolution and variation-store evaluation are separate.
use crate::{Error,GposDevice,GposValueLimits,GposValueRecord};
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct GposAnchor{
    pub x:i16,pub y:i16,pub detail:GposAnchorDetail,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum GposAnchorDetail{
    Coordinate,
    ContourPoint(u16),
    Device{ x:Option<GposDevice>,y:Option<GposDevice> },
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{
    let end=p.checked_add(2).ok_or(Error::Limit("gpos anchor offset"))?;
    Ok(u16::from_be_bytes(b.get(p..end).ok_or(Error::Invalid("gpos anchor truncated"))?.try_into().unwrap()))
}
/// The anchor's Device offsets are relative to its own beginning.
/// ContourPoint is retained as an index, not resolved to an outline point.
pub fn parse_gpos_anchor(b:&[u8],at:usize,limits:GposValueLimits)->Result<GposAnchor,Error>{
    if b.len()>limits.max_bytes{return Err(Error::Limit("gpos anchor bytes"));}
    let format=word(b,at)?;
    let size=match format{1=>6,2=>8,3=>10,_=>return Err(Error::Invalid("gpos anchor format"))};
    let end=at.checked_add(size).ok_or(Error::Limit("gpos anchor offset"))?;
    if end>b.len(){return Err(Error::Invalid("gpos anchor truncated"));}
    let x=word(b,at+2)? as i16;let y=word(b,at+4)? as i16;
    let detail=match format{
        1=>GposAnchorDetail::Coordinate,
        2=>GposAnchorDetail::ContourPoint(word(b,at+6)?),
        3=>{
            let mut left=limits.max_device_entries;
            let mut read=|p|{
                let rel=usize::from(word(b,p)?);
                if rel==0{return Ok(None);}
                if rel<size{return Err(Error::Invalid("gpos anchor device into header"));}
                let target=at.checked_add(rel).ok_or(Error::Limit("gpos anchor device offset"))?;
                crate::font_gpos_value::decode_device(b,target,&mut left).map(Some)
            };
            GposAnchorDetail::Device{x:read(at+6)?,y:read(at+8)?}
        }
        _=>unreachable!(),
    };
    Ok(GposAnchor{x,y,detail})
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
pub struct GposAdjustment{
    pub x_placement:i32,pub y_placement:i32,pub x_advance:i32,pub y_advance:i32,
}
impl GposAdjustment{
    /// All four fields are checked before committing any result.
    pub fn checked_add(self,other:Self)->Result<Self,Error>{
        let add=|a:i32,b:i32|a.checked_add(b).ok_or(Error::Limit("gpos adjustment overflow"));
        Ok(Self{x_placement:add(self.x_placement,other.x_placement)?,
            y_placement:add(self.y_placement,other.y_placement)?,
            x_advance:add(self.x_advance,other.x_advance)?,
            y_advance:add(self.y_advance,other.y_advance)?})
    }
}
impl GposValueRecord{
    pub fn design_adjustment(&self)->GposAdjustment{
        GposAdjustment{x_placement:i32::from(self.x_placement),y_placement:i32::from(self.y_placement),
            x_advance:i32::from(self.x_advance),y_advance:i32::from(self.y_advance)}
    }
    /// Pixel deltas are returned separately from design units. Refuse
    /// unresolved variation indices rather than treating them as zero.
    pub fn pixel_adjustment(&self,x_ppem:u16,y_ppem:u16)->Result<GposAdjustment,Error>{
        Ok(GposAdjustment{x_placement:pixel(&self.x_placement_device,x_ppem)?,
            y_placement:pixel(&self.y_placement_device,y_ppem)?,
            x_advance:pixel(&self.x_advance_device,x_ppem)?,
            y_advance:pixel(&self.y_advance_device,y_ppem)?})
    }
}
fn pixel(device:&Option<GposDevice>,ppem:u16)->Result<i32,Error>{
    match device{
        None=>Ok(0),
        Some(GposDevice::VariationIndex{..})=>Err(Error::Unsupported("gpos unresolved variation index")),
        Some(d)=>d.pixel_delta(ppem).map(i32::from).ok_or(Error::Invalid("gpos malformed device")),
    }
}
impl GposAnchor{
    /// Nominal design-coordinate difference, ignoring contour hinting
    /// and Device/VariationIndex corrections. Does not move glyphs.
    pub fn design_offset_from(&self,other:&Self)->(i32,i32){
        (i32::from(self.x)-i32::from(other.x),i32::from(self.y)-i32::from(other.y))
    }
    pub fn pixel_offset(&self,x_ppem:u16,y_ppem:u16)->Result<(i32,i32),Error>{
        match &self.detail{
            GposAnchorDetail::Device{x,y}=>Ok((pixel(x,x_ppem)?,pixel(y,y_ppem)?)),
            GposAnchorDetail::Coordinate=>Ok((0,0)),
            GposAnchorDetail::ContourPoint(_)=>Err(Error::Unsupported("gpos unresolved contour point")),
        }
    }
}
