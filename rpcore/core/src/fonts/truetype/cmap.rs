//! Bounded sfnt character-to-glyph maps. Distinct from PDF ToUnicode CMaps.
use crate::{Error,Result};
use super::{u16at,u32at};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CmapKind{Unicode,MacRoman,Symbol}
#[derive(Debug)]
enum Mapping{Dense{start:u32,gids:Vec<u16>},Groups(Vec<(u32,u32,u16)>)}
#[derive(Debug)]
pub(super) struct Maps{unicode:Option<Mapping>,mac:Option<Mapping>,symbol:Option<Mapping>}
impl Mapping{
    fn get(&self,c:u32)->Option<u16>{
        let g=match self{
            Self::Dense{start,gids}=>*gids.get(c.checked_sub(*start)? as usize)?,
            Self::Groups(groups)=>{
                let i=groups.partition_point(|g|g.1<c);
                let &(a,b,g)=groups.get(i)?;
                if c<a||c>b{return None;}
                u16::try_from(u32::from(g)+c-a).ok()?
            }
        };(g!=0).then_some(g)
    }
}
fn checked_gid(g:u16,n:usize)->Result<u16>{
    if usize::from(g)>=n{return Err(Error::Invalid("TrueType cmap glyph ID"));}Ok(g)
}
fn subtable(d:&[u8],n:usize)->Result<Mapping>{
    let fmt=u16at(d,0)?;
    let len=if fmt==12{u32at(d,4)? as usize}else{usize::from(u16at(d,2)?)};
    let d=d.get(..len).ok_or(Error::Invalid("TrueType cmap length"))?;
    match fmt{
        0=>{
            if len!=262{return Err(Error::Invalid("TrueType cmap format 0 length"));}
            let gids=d[6..].iter().map(|&g|checked_gid(u16::from(g),n)).collect::<Result<Vec<_>>>()?;
            Ok(Mapping::Dense{start:0,gids})
        }
        6=>{
            let start=u32::from(u16at(d,6)?);let count=usize::from(u16at(d,8)?);
            if start+count as u32>65536||len!=10+2*count{return Err(Error::Invalid("TrueType cmap format 6 range"));}
            let gids=(0..count).map(|i|checked_gid(u16at(d,10+2*i)?,n)).collect::<Result<Vec<_>>>()?;
            Ok(Mapping::Dense{start,gids})
        }
        4=>{
            let twice=usize::from(u16at(d,6)?);
            if twice==0||twice%2!=0{return Err(Error::Invalid("TrueType cmap segment count"));}
            let seg=twice/2;if seg>8192{return Err(Error::Limit("TrueType cmap segments"));}
            if len<16+8*seg||u16at(d,14+2*seg)?!=0{return Err(Error::Invalid("TrueType cmap format 4 layout"));}
            let ends=14;let starts=16+2*seg;let deltas=16+4*seg;let offsets=16+6*seg;
            let mut gids=vec![0;65536];let mut prev=None;
            for i in 0..seg{
                let a=u16at(d,starts+2*i)?;let b=u16at(d,ends+2*i)?;
                let delta=u16at(d,deltas+2*i)?;let off=usize::from(u16at(d,offsets+2*i)?);
                if a>b||prev.is_some_and(|p|a<=p)||off%2!=0{return Err(Error::Invalid("TrueType cmap segment range"));}
                for c in a..=b{
                    let g=if off==0{c.wrapping_add(delta)}else{
                        let p=offsets+2*i+off+2*usize::from(c-a);
                        if p<16+8*seg{return Err(Error::Invalid("TrueType cmap glyph array offset"));}
                        let v=u16at(d,p)?;if v==0{0}else{v.wrapping_add(delta)}
                    };
                    gids[usize::from(c)]=checked_gid(g,n)?;
                }
                prev=Some(b);
            }
            if prev!=Some(65535){return Err(Error::Invalid("TrueType cmap sentinel"));}
            Ok(Mapping::Dense{start:0,gids})
        }
        12=>{
            if u16at(d,2)?!=0{return Err(Error::Invalid("TrueType cmap format 12 reserved"));}
            let count=u32at(d,12)? as usize;if count>65536{return Err(Error::Limit("TrueType cmap groups"));}
            if len!=16+12*count{return Err(Error::Invalid("TrueType cmap format 12 length"));}
            let mut groups=Vec::with_capacity(count);let mut prev=None;
            for i in 0..count{
                let p=16+12*i;let a=u32at(d,p)?;let b=u32at(d,p+4)?;let g=u32at(d,p+8)?;
                if a>b||b>0x10ffff||prev.is_some_and(|p|a<=p)||g.checked_add(b-a).is_none_or(|v|v>=n as u32){return Err(Error::Invalid("TrueType cmap group range"));}
                groups.push((a,b,g as u16));prev=Some(b);
            }
            Ok(Mapping::Groups(groups))
        }
        _=>Err(Error::Unsupported("TrueType cmap format")),
    }
}
impl Maps{
    pub(super) fn parse(d:&[u8],n:usize)->Result<Self>{
        if u16at(d,0)?!=0{return Err(Error::Invalid("TrueType cmap version"));}
        let count=usize::from(u16at(d,2)?);
        if count>128{return Err(Error::Limit("TrueType cmap records"));}
        let end=4+8*count;if end>d.len(){return Err(Error::Invalid("TrueType cmap records"));}
        let mut choice:[Option<(u16,usize)>;3]=[None;3];
        for i in 0..count{
            let p=4+8*i;let platform=u16at(d,p)?;let encoding=u16at(d,p+2)?;let offset=u32at(d,p+4)? as usize;
            if offset<end||offset>=d.len(){return Err(Error::Invalid("TrueType cmap subtable offset"));}
            let fmt=u16at(d,offset)?;
            let rank=match fmt{12=>40,4=>30,6=>20,0=>10,_=>continue};
            let (slot,priority)=match (platform,encoding){
                (3,10)=>(0,4),(0,0..=6)=>(0,3),(3,1)=>(0,2),(1,0)=>(1,0),(3,0)=>(2,0),_=>continue,
            };
            let rank=rank+priority;
            if choice[slot].is_none_or(|(r,_)|rank>r){choice[slot]=Some((rank,offset));}
        }
        let load=|v:Option<(u16,usize)>|v.map(|(_,o)|subtable(&d[o..],n)).transpose();
        Ok(Self{unicode:load(choice[0])?,mac:load(choice[1])?,symbol:load(choice[2])?})
    }
    pub(super) fn get(&self,kind:CmapKind,c:u32)->Option<u16>{
        match kind{CmapKind::Unicode=>&self.unicode,CmapKind::MacRoman=>&self.mac,CmapKind::Symbol=>&self.symbol}.as_ref()?.get(c)
    }
}
