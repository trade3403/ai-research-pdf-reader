//! Bounded JPEG 2000 Part 1 tile-component geometry and LRCP/RLCP
//! packet order. Coordinates are half-open in the relevant sample grid.
//! Packet headers and entropy payloads are not decoded.
use super::codestream::{inspect_codestream,Codestream,CodingStyle,Progression};
use crate::{Error,Result};

const MAX_NODES:usize=262_144;
const MAX_PACKETS:usize=1_000_000;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Rect{pub x0:i64,pub y0:i64,pub x1:i64,pub y1:i64}
impl Rect{
    pub fn width(self)->i64{(self.x1-self.x0).max(0)}
    pub fn height(self)->i64{(self.y1-self.y0).max(0)}
    fn empty(self)->bool{self.width()==0||self.height()==0}
    fn clip(self,b:Self)->Self{
        let x0=self.x0.max(b.x0);let y0=self.y0.max(b.y0);
        Self{x0,y0,x1:self.x1.min(b.x1).max(x0),y1:self.y1.min(b.y1).max(y0)}
    }
    fn project(self,dx:i64,dy:i64,ox:i64,oy:i64)->Self{
        Self{x0:ceil(self.x0-ox,dx),y0:ceil(self.y0-oy,dy),x1:ceil(self.x1-ox,dx),y1:ceil(self.y1-oy,dy)}
    }
}
fn ceil(x:i64,d:i64)->i64{x.div_euclid(d)+i64::from(x.rem_euclid(d)!=0)}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Band{Ll,Hl,Lh,Hh}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct BandPrecinct{
    pub band:Band,pub bounds:Rect,
    /// Raster-ordered intersections with the globally anchored code-block grid.
    pub blocks:Vec<Rect>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Precinct{
    pub bounds:Rect,pub bands:Vec<BandPrecinct>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Subband{pub band:Band,pub bounds:Rect}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Resolution{
    pub bounds:Rect,pub precinct_columns:usize,pub precinct_rows:usize,
    pub subbands:Vec<Subband>,pub precincts:Vec<Precinct>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TileComponent{pub bounds:Rect,pub resolutions:Vec<Resolution>}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Packet{pub layer:u16,pub resolution:u8,pub component:u8,pub precinct:usize}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct TilePlan{
    pub tile:u16,pub bounds:Rect,pub components:Vec<TileComponent>,
    pub packets:Vec<Packet>,pub codeblock_count:usize,
}
fn charge(left:&mut usize,n:usize)->Result<()>{
    *left=left.checked_sub(n).ok_or(Error::Limit("JPX geometry nodes"))?;Ok(())
}
fn grid(b:Rect,sx:i64,sy:i64)->Result<(i64,i64,usize,usize)>{
    let x=b.x0.div_euclid(sx);let y=b.y0.div_euclid(sy);
    if b.empty(){return Ok((x,y,0,0));}
    let w=usize::try_from(ceil(b.x1,sx)-x).map_err(|_|Error::Limit("JPX grid columns"))?;
    let h=usize::try_from(ceil(b.y1,sy)-y).map_err(|_|Error::Limit("JPX grid rows"))?;
    Ok((x,y,w,h))
}
fn cell(x:i64,y:i64,sx:i64,sy:i64)->Rect{Rect{x0:x*sx,y0:y*sy,x1:(x+1)*sx,y1:(y+1)*sy}}
fn product(w:usize,h:usize)->Result<usize>{w.checked_mul(h).ok_or(Error::Limit("JPX grid count"))}
fn blocks(b:Rect,sx:i64,sy:i64,left:&mut usize,total:&mut usize)->Result<Vec<Rect>>{
    let (x,y,w,h)=grid(b,sx,sy)?;
    let n=product(w,h)?;charge(left,n)?;
    *total=total.checked_add(n).ok_or(Error::Limit("JPX codeblocks"))?;
    let mut out=Vec::with_capacity(n);
    for j in 0..h{for i in 0..w{out.push(cell(x+i as i64,y+j as i64,sx,sy).clip(b));}}
    Ok(out)
}
fn resolution(b:Rect,s:&CodingStyle,r:usize,left:&mut usize,total:&mut usize)->Result<Resolution>{
    charge(left,1)?;
    let shift=usize::from(s.levels)-r;
    let d=1i64<<shift;
    let bounds=b.project(d,d,0,0);
    let (ex,ey)=s.precincts[r];
    let (px,py)=(1i64<<ex,1i64<<ey);
    let mut subbands=Vec::new();
    if r==0{subbands.push(Subband{band:Band::Ll,bounds});}
    else{
        for (band,ox,oy) in [(Band::Hl,d,0),(Band::Lh,0,d),(Band::Hh,d,d)]{
            subbands.push(Subband{band,bounds:b.project(2*d,2*d,ox,oy)});
        }
    }
    charge(left,subbands.len())?;
    let (gx,gy,cols,rows)=grid(bounds,px,py)?;
    let n=product(cols,rows)?;
    charge(left,n)?;
    let divisor=if r==0{1}else{2};
    let (bx,by)=(px/divisor,py/divisor);
    let (cbx,cby)=(bx.min(i64::from(s.codeblock_width)),by.min(i64::from(s.codeblock_height)));
    let mut precincts=Vec::with_capacity(n);
    for j in 0..rows{for i in 0..cols{
        let nominal=cell(gx+i as i64,gy+j as i64,px,py);
        // For higher resolutions the nominal precinct is projected to
        // each detail subband's grid. Its origin is aligned to 2 samples.
        let band_cell=nominal.project(divisor,divisor,0,0);
        charge(left,subbands.len())?;
        let mut bands=Vec::with_capacity(subbands.len());
        for sb in &subbands{
            let clipped=band_cell.clip(sb.bounds);
            bands.push(BandPrecinct{band:sb.band,bounds:clipped,blocks:blocks(clipped,cbx,cby,left,total)?});
        }
        precincts.push(Precinct{bounds:nominal.clip(bounds),bands});
    }}
    Ok(Resolution{bounds,precinct_columns:cols,precinct_rows:rows,subbands,precincts})
}
fn tile_bounds(c:&Codestream,tile:u16)->Result<Rect>{
    let h=&c.geometry;
    let i=u64::from(tile);
    if i>=u64::from(h.tiles_x)*u64::from(h.tiles_y){return Err(Error::Invalid("JPX tile index"));}
    let x=i64::from(tile as u32%h.tiles_x);let y=i64::from(tile as u32/h.tiles_x);
    let sx=i64::from(h.tile_width);let sy=i64::from(h.tile_height);
    let tx=i64::from(h.tile_x_origin)+x*sx;let ty=i64::from(h.tile_y_origin)+y*sy;
    Ok(Rect{x0:tx,y0:ty,x1:tx+sx,y1:ty+sy}.clip(Rect{
        x0:i64::from(h.x_origin),y0:i64::from(h.y_origin),
        x1:i64::from(h.x_origin)+i64::from(h.width),y1:i64::from(h.y_origin)+i64::from(h.height),
    }))
}
fn plan(c:&Codestream,tile:u16)->Result<TilePlan>{
    // Spatial progression orders need a reference-grid traversal across
    // differently sampled components; never substitute a local sort.
    if !matches!(c.coding.progression,Progression::Lrcp|Progression::Rlcp){return Err(Error::Unsupported("JPX spatial packet progression"));}
    let bounds=tile_bounds(c,tile)?;
    let mut left=MAX_NODES;let mut total=0;
    let mut components=Vec::new();let mut max_res=0;let mut per_layer=0usize;
    for (g,s) in c.geometry.components.iter().zip(&c.component_coding){
        charge(&mut left,1)?;
        let b=bounds.project(i64::from(g.x_sampling),i64::from(g.y_sampling),0,0);
        let mut resolutions=Vec::new();
        for r in 0..=usize::from(s.levels){
            let res=resolution(b,s,r,&mut left,&mut total)?;
            per_layer=per_layer.checked_add(res.precincts.len()).ok_or(Error::Limit("JPX packet count"))?;
            resolutions.push(res);
        }
        max_res=max_res.max(resolutions.len());components.push(TileComponent{bounds:b,resolutions});
    }
    let n=product(per_layer,usize::from(c.coding.layers))?;
    if n>MAX_PACKETS{return Err(Error::Limit("JPX packet count"));}
    let mut packets=Vec::with_capacity(n);
    // Enumerate nonempty (resolution,component,precinct) positions once
    // so absent high resolutions do not cause layers*components work.
    let mut positions=Vec::with_capacity(per_layer);
    for r in 0..max_res{for (ci,comp) in components.iter().enumerate(){
        if let Some(res)=comp.resolutions.get(r){
            for p in 0..res.precincts.len(){positions.push(Packet{layer:0,resolution:r as u8,component:ci as u8,precinct:p});}
        }
    }}
    if c.coding.progression==Progression::Lrcp{
        for l in 0..c.coding.layers{for p in &positions{packets.push(Packet{layer:l,..*p});}}
    }else{
        let mut start=0;
        while start<positions.len(){
            let mut end=start+1;
            while end<positions.len()&&positions[end].resolution==positions[start].resolution{end+=1;}
            for l in 0..c.coding.layers{for p in &positions[start..end]{packets.push(Packet{layer:l,..*p});}}
            start=end;
        }
    }
    Ok(TilePlan{tile,bounds,components,packets,codeblock_count:total})
}
/// Re-validate source bytes and plan one tile. Resource caps are per call,
/// not an aggregate document limit. No packet header/body bytes are consumed.
pub fn inspect_tile(data:&[u8],tile:u16)->Result<TilePlan>{plan(&inspect_codestream(data)?,tile)}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::filters::jpx::codestream::Wavelet;
    fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|p|u8::from_str_radix(std::str::from_utf8(p).unwrap(),16).unwrap()).collect()}
    // Independently encoded OpenJPEG fixtures retained from B11.2b.
    const GRAY:&str="ff4fff51002900000000000c000000090000000000000000000000080000000800000000000000000001070101ff52000c00000001000104040001ff5c00074040484850ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a00000000002c0001ff93df8080120a4b899e9b0e1790c691ddf4cded37c01140f901802217037e97ff90000a0001000000240001ff93cfb424111df46a1ccbe3ba53c01140f9010017cf036eff90000a0002000000190001ff93c7d408065eaa47c010800eff90000a0003000000180001ff93c3e7060828bfc010800affd9";
    const RGB:&str="ff4fff51002f00000000000c0000000900000000000000000000000c0000000900000000000000000003070101070101070101ff52000e010100010101010100003344ff5c000b42482457d357d35762ff640025000143726561746564206279204f70656e4a5045472076657273696f6e20322e352e34ff90000a0000000001020001ff93cfc0841289bf64d7b33e30291626d80a861e346fca9e9dc125a2833a40269493e4f3fd9fcfc0843ab81599e1e5a571c7d7f78e5ba8a7debf6b807648db9a1117b1fedb391268c83fcfc0780168f484c2b86f7abc55d96f99d41b3b3668a10a6912db7a36167616e7e7c1f50543ed1583ea063140259cdb742a75272b0385db2d968875dcbdde1aea8d63d73a0163bdc77f042d61e78bd4c0f9c647e02d07d4120bb4093f2244a9d583d077bf014fcb841bca7f10b2573e0b15f205a6696c78c8087f0c2216a6623668ff6fc0f9c443ed1281f386014370cdeac16ec1015ceb6999ce6f9cdd1918704a6a3eb3590f01694c82dca7ffd9";
    fn rect(x0:i64,y0:i64,x1:i64,y1:i64)->Rect{Rect{x0,y0,x1,y1}}
    fn area(b:Rect)->i64{b.width()*b.height()}
    #[test]fn reversible_edge_tiles(){
        let b=hex(GRAY);
        for (i,bounds) in [rect(0,0,8,8),rect(8,0,12,8),rect(0,8,8,9),rect(8,8,12,9)].into_iter().enumerate(){
            let p=inspect_tile(&b,i as u16).unwrap();
            assert_eq!(p.bounds,bounds);assert_eq!(p.components[0].bounds,bounds);
            assert_eq!(p.packets.len(),2);
            assert_eq!(p.components[0].resolutions[0].bounds,bounds.project(2,2,0,0));
            let sum:i64=p.components[0].resolutions.iter().flat_map(|r|&r.subbands).map(|b|area(b.bounds)).sum();
            assert_eq!(sum,area(bounds));
        }
        assert!(inspect_tile(&b,4).is_err());
    }
    #[test]fn irreversible_rgb_precincts(){
        let p=inspect_tile(&hex(RGB),0).unwrap();
        assert_eq!(p.bounds,rect(0,0,12,9));assert_eq!(p.components.len(),3);
        for c in &p.components{
            assert_eq!(c.resolutions[0].bounds,rect(0,0,6,5));
            assert_eq!(c.resolutions[1].subbands.iter().map(|b|b.bounds).collect::<Vec<_>>(),vec![rect(0,0,6,5),rect(0,0,6,4),rect(0,0,6,4)]);
            for r in &c.resolutions{assert_eq!((r.precinct_columns,r.precinct_rows),(1,1));}
        }
        assert_eq!(p.codeblock_count,12);assert_eq!(p.packets.len(),6);
    }
    #[test]fn packet_order_two_layers(){
        let mut c=inspect_codestream(&hex(RGB)).unwrap();c.coding.layers=2;
        c.coding.progression=Progression::Lrcp;
        let a=plan(&c,0).unwrap().packets;
        let key=|p:&Packet|(p.layer,p.resolution,p.component,p.precinct);
        assert_eq!(a.len(),12);assert!(a.windows(2).all(|w|key(&w[0])<key(&w[1])));
        assert_eq!((a[5].layer,a[5].resolution,a[5].component),(0,1,2));
        assert_eq!((a[6].layer,a[6].resolution,a[6].component),(1,0,0));
        c.coding.progression=Progression::Rlcp;
        let a=plan(&c,0).unwrap().packets;let key=|p:&Packet|(p.resolution,p.layer,p.component,p.precinct);
        assert!(a.windows(2).all(|w|key(&w[0])<key(&w[1])));
        assert_eq!((a[5].layer,a[5].resolution,a[5].component),(1,0,2));
        assert_eq!((a[6].layer,a[6].resolution,a[6].component),(0,1,0));
        for order in [Progression::Rpcl,Progression::Pcrl,Progression::Cprl]{c.coding.progression=order;assert!(plan(&c,0).is_err());}
    }
    fn style(levels:u8,precinct:(u8,u8))->CodingStyle{CodingStyle{levels,codeblock_width:4,codeblock_height:4,codeblock_style:0,wavelet:Wavelet::Reversible53,precincts:vec![precinct;usize::from(levels)+1]}}
    #[test]fn nonzero_origin_precinct_and_block_coverage(){
        // Independently enumerate each sample coordinate in every band and
        // require exactly one code-block owner, including clipped edges.
        for b in [rect(3,5,22,18),rect(1,1,2,2),rect(7,9,7,14),rect(4,4,40,29)]{
            let s=style(2,(3,3));let mut left=MAX_NODES;let mut total=0;
            let mut samples=0;
            for r in 0..=2{
                let res=resolution(b,&s,r,&mut left,&mut total).unwrap();
                for sb in &res.subbands{
                    samples+=area(sb.bounds);
                    let blocks:Vec<_>=res.precincts.iter().flat_map(|p|&p.bands).filter(|p|p.band==sb.band).flat_map(|p|&p.blocks).collect();
                    for y in sb.bounds.y0..sb.bounds.y1{for x in sb.bounds.x0..sb.bounds.x1{
                        assert_eq!(blocks.iter().filter(|q|q.x0<=x&&x<q.x1&&q.y0<=y&&y<q.y1).count(),1);
                    }}
                    assert_eq!(blocks.iter().map(|q|area(**q)).sum::<i64>(),area(sb.bounds));
                    assert!(blocks.iter().all(|q|q.width()<=4&&q.height()<=4));
                }
            }
            assert_eq!(samples,area(b));
        }
    }
    #[test]fn subsampled_components_and_different_levels(){
        let mut c=inspect_codestream(&hex(RGB)).unwrap();
        c.geometry.x_origin=3;c.geometry.y_origin=5;c.geometry.width=9;c.geometry.height=4;
        c.geometry.components[1].x_sampling=2;c.geometry.components[1].y_sampling=3;
        c.component_coding[1]=style(0,(2,2));
        let p=plan(&c,0).unwrap();
        assert_eq!(p.components[1].bounds,rect(2,2,6,3));
        assert_eq!(p.components[1].resolutions.len(),1);
        assert_eq!(p.components[1].resolutions[0].precinct_columns,2);
        assert!(!p.packets.iter().any(|p|p.component==1&&p.resolution==1));
    }
    #[test]fn bounds_before_allocating(){
        let mut c=inspect_codestream(&hex(GRAY)).unwrap();
        c.geometry.width=10000;c.geometry.height=10000;c.geometry.tile_width=10000;c.geometry.tile_height=10000;c.geometry.tiles_x=1;c.geometry.tiles_y=1;
        c.component_coding[0]=style(0,(0,0));
        assert!(matches!(plan(&c,0),Err(Error::Limit("JPX geometry nodes"))));
        let mut c=inspect_codestream(&hex(RGB)).unwrap();
        c.component_coding[0]=style(0,(0,0));c.coding.layers=u16::MAX;
        assert!(matches!(plan(&c,0),Err(Error::Limit("JPX packet count"))));
    }
}
