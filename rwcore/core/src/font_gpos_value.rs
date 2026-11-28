//! GPOS ValueRecord and Device/VariationIndex decoding. Design-unit
//! values remain separate from pixel deltas; variation indices are
//! opaque until an ItemVariationStore evaluator is available.
use crate::Error;
#[derive(Debug,Clone,Copy)]
pub struct GposValueLimits{pub max_bytes:usize,pub max_device_entries:usize}
impl Default for GposValueLimits{
    fn default()->Self{Self{max_bytes:32*1024*1024,max_device_entries:65536}}
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum GposDevice{
    Pixels{start:u16,end:u16,deltas:Vec<i8>},
    VariationIndex{outer:u16,inner:u16},
}
impl GposDevice{
    pub fn pixel_delta(&self,ppem:u16)->Option<i8>{
        match self{
            Self::Pixels{start,end,deltas}=>if ppem<*start||ppem>*end{Some(0)}else{deltas.get(usize::from(ppem-*start)).copied()},
            Self::VariationIndex{..}=>None,
        }
    }
}
#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct GposValueRecord{
    pub x_placement:i16,pub y_placement:i16,pub x_advance:i16,pub y_advance:i16,
    pub x_placement_device:Option<GposDevice>,pub y_placement_device:Option<GposDevice>,
    pub x_advance_device:Option<GposDevice>,pub y_advance_device:Option<GposDevice>,
}
fn bytes(b:&[u8],p:usize,n:usize)->Result<&[u8],Error>{
    b.get(p..p.checked_add(n).ok_or(Error::Limit("gpos value offset"))?).ok_or(Error::Invalid("gpos value truncated"))
}
fn word(b:&[u8],p:usize)->Result<u16,Error>{Ok(u16::from_be_bytes(bytes(b,p,2)?.try_into().unwrap()))}
fn advance(p:&mut usize,n:usize)->Result<(),Error>{
    *p=p.checked_add(n).ok_or(Error::Limit("gpos value offset"))?;Ok(())
}
pub fn gpos_value_record_size(format:u16)->Result<usize,Error>{
    if format&0xff00!=0{return Err(Error::Invalid("gpos value reserved format"));}
    Ok(format.count_ones() as usize*2)
}
pub(crate) fn decode_device(b:&[u8],at:usize,left:&mut usize)->Result<GposDevice,Error>{
    bytes(b,at,6)?;
    let start=word(b,at)?;let end=word(b,at+2)?;let format=word(b,at+4)?;
    if format==0x8000{return Ok(GposDevice::VariationIndex{outer:start,inner:end});}
    let bits=match format{1=>2usize,2=>4,3=>8,_=>return Err(Error::Invalid("gpos device format"))};
    if end<start{return Err(Error::Invalid("gpos device range"));}
    let n=usize::from(end)-usize::from(start)+1;
    *left=left.checked_sub(n).ok_or(Error::Limit("gpos device entries"))?;
    let per=16/bits;let words=(n+per-1)/per;
    bytes(b,at+6,words*2)?;
    let mut deltas=Vec::new();deltas.try_reserve_exact(n).map_err(|_|Error::Limit("gpos device allocation"))?;
    let mask=(1u16<<bits)-1;let sign=1u16<<(bits-1);
    for i in 0..n{
        let packed=word(b,at+6+2*(i/per))?;
        let raw=(packed>>(16-bits*(i%per+1)))&mask;
        let value=if raw&sign!=0{i16::from(raw as u8)-(1i16<<bits)}else{raw as i16};
        deltas.push(value as i8);
    }
    Ok(GposDevice::Pixels{start,end,deltas})
}
pub fn parse_gpos_device(b:&[u8],at:usize,limits:GposValueLimits)->Result<GposDevice,Error>{
    if b.len()>limits.max_bytes{return Err(Error::Limit("gpos value bytes"));}
    let mut left=limits.max_device_entries;
    decode_device(b,at,&mut left)
}
/// Decode a record at an absolute offset. Device offsets are relative
/// to the caller-supplied parent base (PairSet for PairPos format 1,
/// positioning subtable for SinglePos and PairPos format 2).
/// min_device_offset is an absolute lower bound covering all parent
/// inline records; caller must supply it. Zero offsets mean absent.
pub fn parse_gpos_value_record(b:&[u8],at:usize,parent:usize,min_device_offset:usize,format:u16,
    limits:GposValueLimits)->Result<(GposValueRecord,usize),Error>{
    if b.len()>limits.max_bytes{return Err(Error::Limit("gpos value bytes"));}
    let mut left=limits.max_device_entries;
    decode_value_record(b,at,parent,min_device_offset,format,&mut left)
}
pub(crate) fn decode_value_record(b:&[u8],at:usize,parent:usize,min_device_offset:usize,format:u16,left:&mut usize)
    ->Result<(GposValueRecord,usize),Error>{
    let size=gpos_value_record_size(format)?;
    bytes(b,at,size)?;
    let end=at.checked_add(size).ok_or(Error::Limit("gpos value offset"))?;
    if parent>at||min_device_offset<end||min_device_offset>b.len(){return Err(Error::Invalid("gpos value parent range"));}
    let mut p=at;let mut vals=[0i16;4];
    for (i,v) in vals.iter_mut().enumerate(){
        if format&(1<<i)!=0{*v=word(b,p)? as i16;advance(&mut p,2)?;}
    }
    let mut devices:[Option<GposDevice>;4]=[None,None,None,None];
    for (i,v) in devices.iter_mut().enumerate(){
        if format&(0x10<<i)!=0{
            let rel=usize::from(word(b,p)?);advance(&mut p,2)?;
            if rel!=0{
                let target=parent.checked_add(rel).ok_or(Error::Limit("gpos device offset"))?;
                if target<min_device_offset{return Err(Error::Invalid("gpos device into inline records"));}
                *v=Some(decode_device(b,target,left)?);
            }
        }
    }
    let [x_placement_device,y_placement_device,x_advance_device,y_advance_device]=devices;
    Ok((GposValueRecord{x_placement:vals[0],y_placement:vals[1],x_advance:vals[2],y_advance:vals[3],
        x_placement_device,y_placement_device,x_advance_device,y_advance_device},end))
}
