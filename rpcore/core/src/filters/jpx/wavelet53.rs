//! Reversible JPEG 2000 5/3 inverse lifting in one and two dimensions.
//! Inputs are compact low/high subbands in increasing coordinate order.
//! Integer arithmetic uses floor division and symmetric boundary extension.
use crate::{Error,Result};

const MAX_LINE:usize=1_048_576;

/// Recover samples on a line whose first reference-grid coordinate has
/// parity `odd_origin`. The low band occupies even grid coordinates.
/// For an isolated odd-coordinate sample the high band is scaled by two.
pub fn inverse53_line(low:&[i32],high:&[i32],odd_origin:bool)->Result<Vec<i32>>{
    let n=low.len().checked_add(high.len()).ok_or(Error::Limit("JPX 5/3 line length"))?;
    if n>MAX_LINE{return Err(Error::Limit("JPX 5/3 line length"));}
    let p=usize::from(odd_origin);
    let expected_low=n/2+usize::from(n%2!=0&&!odd_origin);
    if low.len()!=expected_low{return Err(Error::Invalid("JPX 5/3 subband lengths"));}
    if n==0{return Ok(Vec::new());}
    if n==1{return Ok(vec![if odd_origin{high[0].div_euclid(2)}else{low[0]}]);}
    let mut a=vec![0i64;n];
    for (j,&v) in low.iter().enumerate(){a[2*j+p]=i64::from(v);}
    for (j,&v) in high.iter().enumerate(){a[2*j+1-p]=i64::from(v);}
    // Symmetric extension duplicates the available opposite-parity neighbour.
    let adjacent=|i:usize| (if i==0{1}else{i-1},if i+1==n{n-2}else{i+1});
    // Undo update (low), then undo prediction (high).
    for i in (p..n).step_by(2){
        let (l,r)=adjacent(i);
        a[i]-=(a[l]+a[r]+2).div_euclid(4);
    }
    for i in (1-p..n).step_by(2){
        let (l,r)=adjacent(i);
        a[i]+=(a[l]+a[r]).div_euclid(2);
    }
    a.into_iter().map(|v|i32::try_from(v).map_err(|_|Error::Limit("JPX 5/3 sample overflow"))).collect()
}

const MAX_PLANE:usize=4_194_304;
/// Reconstruct one resolution from compact row-major subbands.
/// HL is high in X, LH is high in Y. Undo horizontal lifting first,
/// then vertical; reversing this order changes integer rounding.
/// The origin flags refer to the output resolution's reference grid.
pub fn inverse53_plane(width:usize,height:usize,odd_x:bool,odd_y:bool,
    ll:&[i32],hl:&[i32],lh:&[i32],hh:&[i32])->Result<Vec<i32>>{
    let area=width.checked_mul(height).ok_or(Error::Limit("JPX 5/3 plane area"))?;
    if width==0||height==0{return Err(Error::Invalid("JPX 5/3 empty plane"));}
    if width>MAX_LINE||height>MAX_LINE||area>MAX_PLANE{return Err(Error::Limit("JPX 5/3 plane area"));}
    let lw=width/2+usize::from(width%2!=0&&!odd_x);
    let lhgt=height/2+usize::from(height%2!=0&&!odd_y);
    let hw=width-lw;
    let hhgt=height-lhgt;
    if ll.len()!=lw*lhgt||hl.len()!=hw*lhgt||lh.len()!=lw*hhgt||hh.len()!=hw*hhgt{
        return Err(Error::Invalid("JPX 5/3 subband plane dimensions"));
    }
    let mut rows=Vec::with_capacity(area);
    for (lo,hi,count) in [(ll,hl,lhgt),(lh,hh,hhgt)]{
        for y in 0..count{
            rows.extend(inverse53_line(&lo[y*lw..(y+1)*lw],&hi[y*hw..(y+1)*hw],odd_x)?);
        }
    }
    let mut out=vec![0;area];
    let mut low=Vec::with_capacity(lhgt);
    let mut high=Vec::with_capacity(hhgt);
    for x in 0..width{
        low.clear();high.clear();
        for y in 0..lhgt{low.push(rows[y*width+x]);}
        for y in lhgt..height{high.push(rows[y*width+x]);}
        let col=inverse53_line(&low,&high,odd_y)?;
        for (y,v) in col.into_iter().enumerate(){out[y*width+x]=v;}
    }
    Ok(out)
}

const MAX_LEVELS:usize=32;
const MAX_RECONSTRUCTION_WORK:usize=MAX_PLANE*4;
/// Detail bands of one resolution, ordered from coarsest to finest.
#[derive(Debug,Clone,Copy)]
pub struct Detail53<'a>{pub hl:&'a[i32],pub lh:&'a[i32],pub hh:&'a[i32]}
#[derive(Clone,Copy)]
struct Grid{ x0:u64,y0:u64,x1:u64,y1:u64 }
impl Grid{
    fn low(self)->Self{
        Self{x0:(self.x0+1)/2,y0:(self.y0+1)/2,x1:(self.x1+1)/2,y1:(self.y1+1)/2}
    }
    fn size(self)->(usize,usize){((self.x1-self.x0)as usize,(self.y1-self.y0)as usize)}
}
/// Reconstruct all requested resolutions from a compact coarsest LL band.
/// Origins are absolute component-grid coordinates, not local tile offsets.
/// Detail levels are ordered coarse to fine. Each level derives its low/high
/// geometry by ceiling projection; no caller-supplied intermediate sizes.
pub fn inverse53_levels(width:usize,height:usize,x_origin:u32,y_origin:u32,
    ll:&[i32],details:&[Detail53<'_>])->Result<Vec<i32>>{
    let area=width.checked_mul(height).ok_or(Error::Limit("JPX 5/3 plane area"))?;
    if width==0||height==0{return Err(Error::Invalid("JPX 5/3 empty plane"));}
    if width>MAX_LINE||height>MAX_LINE||area>MAX_PLANE||details.len()>MAX_LEVELS{
        return Err(Error::Limit("JPX 5/3 reconstruction dimensions"));
    }
    let mut grids=Vec::with_capacity(details.len()+1);
    let mut g=Grid{x0:u64::from(x_origin),y0:u64::from(y_origin),
        x1:u64::from(x_origin)+width as u64,y1:u64::from(y_origin)+height as u64};
    if g.x1>u64::from(u32::MAX)||g.y1>u64::from(u32::MAX){
        return Err(Error::Limit("JPX 5/3 grid coordinates"));
    }
    grids.push(g);
    for _ in details{g=g.low();grids.push(g);}
    let (cw,ch)=g.size();
    if ll.len()!=cw*ch{return Err(Error::Invalid("JPX 5/3 coarsest LL dimensions"));}
    // Validate all bands and total work before allocating reconstructed planes.
    let mut work=ll.len();
    for (g,d) in grids[..details.len()].iter().rev().zip(details){
        let (w,h)=g.size();let (lw,lh)=g.low().size();
        let (hw,hh)=(w-lw,h-lh);
        if d.hl.len()!=hw*lh||d.lh.len()!=lw*hh||d.hh.len()!=hw*hh{
            return Err(Error::Invalid("JPX 5/3 detail dimensions"));
        }
        work=work.checked_add(w*h).ok_or(Error::Limit("JPX 5/3 reconstruction work"))?;
        if work>MAX_RECONSTRUCTION_WORK{return Err(Error::Limit("JPX 5/3 reconstruction work"));}
    }
    let mut current=ll.to_vec();
    for (g,d) in grids[..details.len()].iter().rev().zip(details){
        let (w,h)=g.size();
        if w==0||h==0{continue;}
        current=inverse53_plane(w,h,g.x0%2!=0,g.y0%2!=0,&current,d.hl,d.lh,d.hh)?;
    }
    Ok(current)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn inverse53_fixed_even_origin(){
        assert_eq!(inverse53_line(&[-3,6,0],&[-13,10],false).unwrap(),[3,-8,7,11,-5]);
        assert_eq!(inverse53_line(&[-32,47],&[192,1],false).unwrap(),[-128,127,-1,0]);
    }
    #[test]fn inverse53_fixed_odd_origin(){
        assert_eq!(inverse53_line(&[-4,9],&[11,6,-16],true).unwrap(),[3,-8,7,11,-5]);
        assert_eq!(inverse53_line(&[47,-32],&[-255,-64],true).unwrap(),[-128,127,-1,0]);
    }
    #[test]fn inverse53_short_lines(){
        for odd in [false,true]{assert!(inverse53_line(&[],&[],odd).unwrap().is_empty());}
        assert_eq!(inverse53_line(&[-7],&[],false).unwrap(),[-7]);
        assert_eq!(inverse53_line(&[],&[-14],true).unwrap(),[-7]);
        assert_eq!(inverse53_line(&[2],&[-11],false).unwrap(),[7,-4]);
        assert_eq!(inverse53_line(&[2],&[11],true).unwrap(),[7,-4]);
    }
    #[test]fn inverse53_constant_and_impulse(){
        for n in 2..=64{for odd in [false,true]{
            let l=n/2+usize::from(n%2!=0&&!odd);
            assert_eq!(inverse53_line(&vec![-17;l],&vec![0;n-l],odd).unwrap(),vec![-17;n]);
        }}
        assert_eq!(inverse53_line(&[0,4,0],&[0,0],false).unwrap(),[0,2,4,2,0]);
    }
    #[test]fn inverse53_limits(){
        assert!(inverse53_line(&[0,0],&[],false).is_err());
        assert!(inverse53_line(&[0],&[],true).is_err());
        assert!(inverse53_line(&[i32::MAX],&[i32::MIN],false).is_err());
        assert!(inverse53_line(&[i32::MIN],&[i32::MAX],true).is_err());
        assert!(inverse53_line(&vec![0;MAX_LINE+1],&[],false).is_err());
    }

    #[test]fn plane_reference_real_subbands(){
        assert_eq!(inverse53_plane(4,4,false,false,&[-3,11,22,-29],&[59,8,-114,4],
            &[-8,20,-29,52],&[115,10,192,0]).unwrap(),
            [0,1,-1,2,-3,12,-12,0,127,-128,0,1,2,3,4,5]);
    }
    #[test]fn plane_reference_odd_shapes(){
        let want=[-50,-13,24,-40,-3,34,-30,7,44,-20,17,-47,-10,27,-37];
        assert_eq!(inverse53_plane(5,3,false,false,&[-30,15,-15,12,-6,2],&[-13,-24,-63,77],
            &[39,7,26],&[-25,51]).unwrap(),want);
        assert_eq!(inverse53_plane(5,3,true,true,&[-17,13],&[39,13,-39],
            &[5,-46,-17,-17],&[-101,51,101,0,0,0]).unwrap(),want);
    }
    #[test]fn plane_reference_mixed_origins(){
        let want=[-50,-13,24,-40,-3,34,-30,7,44,-20,17,-47];
        assert_eq!(inverse53_plane(3,4,true,false,&[-13,4],&[-37,37,-37,11],
            &[0,-15],&[0,0,0,-101]).unwrap(),want);
        assert_eq!(inverse53_plane(4,3,false,true,&[4,-5],&[13,-13],
            &[-72,16,-3,-3],&[-50,-101,-101,-101]).unwrap(),want);
    }
    #[test]fn plane_degenerate_axes(){
        let want=[-50,-13,24,-40,-3];
        assert_eq!(inverse53_plane(1,5,true,false,&[],&[-100,24,-56],&[],&[0,-100]).unwrap(),want);
        assert_eq!(inverse53_plane(5,1,false,true,&[],&[],&[-100,23,-56],&[0,-101]).unwrap(),want);
        assert_eq!(inverse53_plane(1,1,true,true,&[],&[],&[],&[-200]).unwrap(),[-50]);
        assert_eq!(inverse53_plane(1,1,false,false,&[7],&[],&[],&[]).unwrap(),[7]);
    }
    #[test]fn plane_limits_and_overflow(){
        assert!(inverse53_plane(0,1,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse53_plane(usize::MAX,2,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse53_plane(4096,4096,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse53_plane(2,2,false,false,&[0],&[],&[0],&[0]).is_err());
        assert!(inverse53_plane(2,1,false,false,&[i32::MAX],&[i32::MIN],&[],&[]).is_err());
    }
    #[test]fn levels_reference_even(){
        let details=[Detail53{hl:&[19,36,28,17],lh:&[-3,15],hh:&[-6,12]},
            Detail53{hl:&[26,1,25,0,1,38,-25,1,-51],lh:&[77,-12,-37,26,26,-37,-25,51],hh:&[51,101,-50,-50,101,0]}];
        assert_eq!(inverse53_levels(7,5,0,0,&[-1,0,4,4],&details).unwrap(),[-50,-13,24,-40,-3,34,-30,7,44,-20,17,-47,-10,27,-37,0,37,-27,10,47,-17,20,-44,-7,30,-34,3,40,-24,13,50,-14,23,-41,-4]);
    }
    #[test]fn levels_reference_odd(){
        let details=[Detail53{hl:&[-6],lh:&[9,-23],hh:&[-45]},
            Detail53{hl:&[-50,-6,-25,-14,27,32,-13,12],lh:&[-32,-19,32,7,-18,39,45,-6,-19],hh:&[0,101,51,-101,-50,76,51,-101,-101,51,101,0]}];
        assert_eq!(inverse53_levels(7,5,3,5,&[-2,2],&details).unwrap(),[-50,-13,24,-40,-3,34,-30,7,44,-20,17,-47,-10,27,-37,0,37,-27,10,47,-17,20,-44,-7,30,-34,3,40,-24,13,50,-14,23,-41,-4]);
    }
    #[test]fn levels_reference_mixed(){
        let details=[Detail53{hl:&[15],lh:&[-3],hh:&[-30]},
            Detail53{hl:&[4,-5,26,-14,8,-7],lh:&[23,-9],hh:&[-24,-23,58]},
            Detail53{hl:&[50,-6,-25,19,-38,38,38,0,50,-38,-75,-12],lh:&[-55,46,-17,-17,46,26,19,-44,32,-37,-50,32,-31,7,-12],hh:&[-50,-50,101,-50,51,25,0,-75,0,-75,-50,76]}];
        assert_eq!(inverse53_levels(9,6,2,3,&[2],&details).unwrap(),[-50,-13,24,-40,-3,34,-30,7,44,-20,17,-47,-10,27,-37,0,37,-27,10,47,-17,20,-44,-7,30,-34,3,40,-24,13,50,-14,23,-41,-4,33,-31,6,43,-21,16,-48,-11,26,-38,-1,36,-28,9,46,-18,19,-45,-8]);
    }
    #[test]fn levels_zero_and_empty_coarsest(){
        assert_eq!(inverse53_levels(2,2,1,2,&[1,2,3,4],&[]).unwrap(),[1,2,3,4]);
        let d=Detail53{hl:&[],lh:&[],hh:&[-200]};
        assert_eq!(inverse53_levels(1,1,1,1,&[],&[d]).unwrap(),[-50]);
        let empty=Detail53{hl:&[],lh:&[],hh:&[]};
        assert_eq!(inverse53_levels(1,1,1,1,&[],&[empty,d]).unwrap(),[-50]);
    }
    #[test]fn levels_dimensions_and_limits(){
        let empty=Detail53{hl:&[],lh:&[],hh:&[]};
        assert!(inverse53_levels(2,2,0,0,&[1],&[empty]).is_err());
        assert!(inverse53_levels(2,2,0,0,&[],&[]).is_err());
        assert!(inverse53_levels(1,1,0,0,&[0],&[empty;MAX_LEVELS+1]).is_err());
        assert!(inverse53_levels(1,1,u32::MAX,0,&[0],&[]).is_err());
        assert!(inverse53_levels(usize::MAX,2,0,0,&[],&[]).is_err());
        assert!(inverse53_levels(0,1,0,0,&[],&[]).is_err());
    }
}
