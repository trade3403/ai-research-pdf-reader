//! Irreversible JPEG 2000 9/7 inverse lifting, scalar f64 reference core.
//! Part 1 (1,2) normalization: analysis low DC gain 1, high Nyquist gain 2.
//! Quantization and component reconstruction are deliberately separate.
use crate::{Error,Result};

const MAX_LINE:usize=1_048_576;
const ALPHA:f64=-1.586134342059924;
const BETA:f64=-0.052980118572961;
const GAMMA:f64=0.882911075530934;
const DELTA:f64=0.443506852043971;
const K:f64=1.230174104914001;

/// Compact low/high input in increasing reference-grid order.
/// The low band occupies even reference-grid coordinates.
/// An isolated sample bypasses lifting and scaling, for either parity.
/// Reject nonfinite inputs and intermediates; no implicit clipping.
pub fn inverse97_line(low:&[f64],high:&[f64],odd_origin:bool)->Result<Vec<f64>>{
    let n=low.len().checked_add(high.len()).ok_or(Error::Limit("JPX 9/7 line length"))?;
    if n>MAX_LINE{return Err(Error::Limit("JPX 9/7 line length"));}
    let p=usize::from(odd_origin);
    if low.len()!=n/2+usize::from(n%2!=0&&!odd_origin){
        return Err(Error::Invalid("JPX 9/7 subband lengths"));
    }
    if low.iter().chain(high).any(|v|!v.is_finite()){
        return Err(Error::Invalid("JPX 9/7 nonfinite coefficient"));
    }
    if n==0{return Ok(Vec::new());}
    if n==1{return Ok(vec![if odd_origin{high[0]}else{low[0]}]);}
    let mut a=vec![0.0;n];
    for (j,&v) in low.iter().enumerate(){a[2*j+p]=v*K;}
    for (j,&v) in high.iter().enumerate(){a[2*j+1-p]=v/K;}
    if a.iter().any(|v|!v.is_finite()){return Err(Error::Limit("JPX 9/7 scaling overflow"));}
    // Reverse the four analysis lifting steps. Whole-sample symmetric
    // extension duplicates the opposite-parity neighbour at each edge.
    for (parity,k) in [(p,DELTA),(1-p,GAMMA),(p,BETA),(1-p,ALPHA)]{
        for i in (parity..n).step_by(2){
            let l=if i==0{1}else{i-1};
            let r=if i+1==n{n-2}else{i+1};
            let v=a[i]-k*(a[l]+a[r]);
            if !v.is_finite(){return Err(Error::Limit("JPX 9/7 lifting overflow"));}
            a[i]=v;
        }
    }
    Ok(a)
}

const MAX_PLANE:usize=4_194_304;
/// Reconstruct one resolution from row-major LL, HL, LH and HH bands.
/// HL is high in X; LH is high in Y. Undo horizontal lifting first,
/// then vertical, with the output reference-grid parity on each axis.
/// Keep floating-point samples unrounded and unclipped.
pub fn inverse97_plane(width:usize,height:usize,odd_x:bool,odd_y:bool,
    ll:&[f64],hl:&[f64],lh:&[f64],hh:&[f64])->Result<Vec<f64>>{
    let area=width.checked_mul(height).ok_or(Error::Limit("JPX 9/7 plane area"))?;
    if width==0||height==0{return Err(Error::Invalid("JPX 9/7 empty plane"));}
    if width>MAX_LINE||height>MAX_LINE||area>MAX_PLANE{
        return Err(Error::Limit("JPX 9/7 plane area"));
    }
    let lw=width/2+usize::from(width%2!=0&&!odd_x);
    let ly=height/2+usize::from(height%2!=0&&!odd_y);
    let hw=width-lw;let hy=height-ly;
    if ll.len()!=lw*ly||hl.len()!=hw*ly||lh.len()!=lw*hy||hh.len()!=hw*hy{
        return Err(Error::Invalid("JPX 9/7 subband plane dimensions"));
    }
    if [ll,hl,lh,hh].into_iter().flatten().any(|v|!v.is_finite()){
        return Err(Error::Invalid("JPX 9/7 nonfinite coefficient"));
    }
    let mut rows=Vec::with_capacity(area);
    for (lo,hi,count) in [(ll,hl,ly),(lh,hh,hy)]{
        for y in 0..count{
            rows.extend(inverse97_line(&lo[y*lw..(y+1)*lw],&hi[y*hw..(y+1)*hw],odd_x)?);
        }
    }
    let mut out=vec![0.;area];
    let mut low=Vec::with_capacity(ly);
    let mut high=Vec::with_capacity(hy);
    for x in 0..width{
        low.clear();high.clear();
        for y in 0..ly{low.push(rows[y*width+x]);}
        for y in ly..height{high.push(rows[y*width+x]);}
        let col=inverse97_line(&low,&high,odd_y)?;
        for (y,v) in col.into_iter().enumerate(){out[y*width+x]=v;}
    }
    Ok(out)
}


const MAX_LEVELS:usize=32;
const MAX_RECONSTRUCTION_WORK:usize=MAX_PLANE*4;

/// Detail subbands for one resolution, ordered coarse to fine.
#[derive(Debug,Clone,Copy)]
pub struct Detail97<'a>{pub hl:&'a[f64],pub lh:&'a[f64],pub hh:&'a[f64]}

#[derive(Clone,Copy)]
struct Grid97{x0:u64,y0:u64,x1:u64,y1:u64}
impl Grid97{
    fn low(self)->Self{
        Self{x0:(self.x0+1)/2,y0:(self.y0+1)/2,x1:(self.x1+1)/2,y1:(self.y1+1)/2}
    }
    fn size(self)->(usize,usize){((self.x1-self.x0)as usize,(self.y1-self.y0)as usize)}
}

/// Reconstruct an irreversible component from its coarsest LL and successive
/// detail bands. Coordinates are absolute half-open component-grid coordinates.
/// Empty intermediate resolutions are valid, but the final plane is nonempty.
/// All geometry, finite input and cumulative work are checked before synthesis.
/// No quantization, sample rounding or clipping is performed here.
pub fn inverse97_levels(width:usize,height:usize,x_origin:u32,y_origin:u32,
    ll:&[f64],details:&[Detail97<'_>])->Result<Vec<f64>>{
    let area=width.checked_mul(height).ok_or(Error::Limit("JPX 9/7 plane area"))?;
    if width==0||height==0{return Err(Error::Invalid("JPX 9/7 empty plane"));}
    if width>MAX_LINE||height>MAX_LINE||area>MAX_PLANE||details.len()>MAX_LEVELS{
        return Err(Error::Limit("JPX 9/7 reconstruction dimensions"));
    }
    let mut g=Grid97{x0:u64::from(x_origin),y0:u64::from(y_origin),
        x1:u64::from(x_origin)+width as u64,y1:u64::from(y_origin)+height as u64};
    if g.x1>u64::from(u32::MAX)||g.y1>u64::from(u32::MAX){
        return Err(Error::Limit("JPX 9/7 grid coordinates"));
    }
    let mut grids=Vec::with_capacity(details.len()+1);
    grids.push(g);
    for _ in details{g=g.low();grids.push(g);}
    let (cw,ch)=g.size();
    if ll.len()!=cw*ch{return Err(Error::Invalid("JPX 9/7 coarsest LL dimensions"));}
    if ll.iter().any(|v|!v.is_finite()){return Err(Error::Invalid("JPX 9/7 nonfinite coefficient"));}
    let mut work=ll.len();
    for (g,d) in grids[..details.len()].iter().rev().zip(details){
        let (w,h)=g.size();let (lw,lh)=g.low().size();
        let (hw,hh)=(w-lw,h-lh);
        if d.hl.len()!=hw*lh||d.lh.len()!=lw*hh||d.hh.len()!=hw*hh{
            return Err(Error::Invalid("JPX 9/7 detail dimensions"));
        }
        work=work.checked_add(w*h).ok_or(Error::Limit("JPX 9/7 reconstruction work"))?;
        if work>MAX_RECONSTRUCTION_WORK{return Err(Error::Limit("JPX 9/7 reconstruction work"));}
        if [d.hl,d.lh,d.hh].into_iter().flatten().any(|v|!v.is_finite()){
            return Err(Error::Invalid("JPX 9/7 nonfinite coefficient"));
        }
    }
    let mut current=ll.to_vec();
    for (g,d) in grids[..details.len()].iter().rev().zip(details){
        let (w,h)=g.size();
        if w==0||h==0{continue;}
        current=inverse97_plane(w,h,g.x0%2!=0,g.y0%2!=0,&current,d.hl,d.lh,d.hh)?;
    }
    Ok(current)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn close(a:&[f64],b:&[f64]){
        assert_eq!(a.len(),b.len());
        for (i,(x,y)) in a.iter().zip(b).enumerate(){
            assert!((x-y).abs()<=1e-10*(1.0+y.abs()),"sample {}: {} != {}",i,x,y);
        }
    }
    // Fixed vectors generated by a separate Python Decimal forward
    // lifting calculation at 45 digits of working precision.
    #[test]fn reference97_even(){
        close(&inverse97_line(&[-4.194602751634156,5.501572264463832,2.1914582227064465],&[-14.823501103255529,11.823501103255508],false).unwrap(),&[3.,-8.,7.,11.,-5.]);
    }
    #[test]fn reference97_odd(){
        close(&inverse97_line(&[-1.8348763348970185,6.334876334897013],&[14.007978788513524,6.420696419656003,-20.849371627825533],true).unwrap(),&[3.,-8.,7.,11.,-5.]);
    }
    #[test]fn reference97_two_and_impulse(){
        close(&inverse97_line(&[-0.500000000000737],&[-255.0000000000008],false).unwrap(),&[127.,-128.]);
        close(&inverse97_line(&[-0.4999999999992598],&[255.0000000000008],true).unwrap(),&[127.,-128.]);
        close(&inverse97_line(&[0.533728236885751,0.25000000000000067,-0.016864118442874828],&[1.0575435262285016,-0.05754352622849979,0.],false).unwrap(),&[0.,1.,0.,0.,0.,0.]);
    }
    #[test]fn reference97_dc_and_nyquist(){
        for n in 2..=65{for odd in [false,true]{
            let l=n/2+usize::from(n%2!=0&&!odd);
            close(&inverse97_line(&vec![17.;l],&vec![0.;n-l],odd).unwrap(),&vec![17.;n]);
            let expected:Vec<_>=(0..n).map(|i|if (i%2!=0)==odd{1.}else{-1.}).collect();
            close(&inverse97_line(&vec![0.;l],&vec![-2.;n-l],odd).unwrap(),&expected);
        }}
    }
    #[test]fn reference97_short_and_invalid(){
        for odd in [false,true]{assert!(inverse97_line(&[],&[],odd).unwrap().is_empty());}
        assert_eq!(inverse97_line(&[-7.],&[],false).unwrap(),[-7.]);
        assert_eq!(inverse97_line(&[],&[-7.],true).unwrap(),[-7.]);
        assert!(inverse97_line(&[1.],&[],true).is_err());
        assert!(inverse97_line(&[f64::NAN],&[],false).is_err());
        assert!(inverse97_line(&[],&[f64::INFINITY],true).is_err());
        assert!(inverse97_line(&[f64::MAX],&[0.],false).is_err());
        assert!(inverse97_line(&[0.],&[f64::MAX],false).is_err());
        assert!(inverse97_line(&vec![0.;MAX_LINE+1],&[],false).is_err());
    }

    // Separate Python Decimal forward lifting, 45-digit arithmetic;
    // vertical analysis then horizontal. Reference input (i*37 % 101)-50.
    #[test]fn plane97_reference_even(){
        let a=inverse97_plane(4,4,false,false,
            &[-14.821272703659382,-0.7537151798561157,18.346602473807124,-10.841125471060586],
            &[35.09651724122752,-22.795183640581723,-3.984640195851484,-10.729645029232488],
            &[35.77108197894249,-3.310075458136462,-14.144313116011768,-2.0787745046624813],
            &[65.69016791652082,111.90121609242061,111.9012160924206,97.63446396423927]).unwrap();
        let expected:Vec<_>=(0..16).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn plane97_reference_mixed_x(){
        let a=inverse97_plane(5,3,true,false,
            &[-7.90390113420691,-0.1835594969921918,-15.662172846796079,13.874633477995536],
            &[-5.418763194131768,36.96574899698358,-2.2116349609793304,101.1195616850103,-8.809800922444511,-99.30105967897757],
            &[12.227987520143019,25.647012479856922],
            &[65.53034422361785,-28.15594807453948,-59.71844807453948]).unwrap();
        let expected:Vec<_>=(0..15).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn plane97_reference_mixed_y(){
        let a=inverse97_plane(3,5,false,true,
            &[-34.90328890868197,28.66541780754459,0.6000268952430935,-4.237155794105359],
            &[-2.1002884425874404,39.97528844258738],
            &[-5.766482041696528,-20.75370708198258,-8.809800922444468,36.96574899698353,7.584984047729345,13.123308926871394],
            &[9.218448074539264,-28.15594807453948,-3.406551925460893]).unwrap();
        let expected:Vec<_>=(0..15).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn plane97_reference_odd(){
        let a=inverse97_plane(3,3,true,true,
            &[-2.999999999999981],
            &[-45.51644188181788,45.5164418818179],
            &[-12.301741049139963,12.301741049139979],
            &[1.540941627686054e-13,-8.852217861175207e-14,8.852217861175204e-14,-1.5409416276860545e-13]).unwrap();
        let expected:Vec<_>=(0..9).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn plane97_degenerate_axes(){
        for x in [false,true]{for y in [false,true]{
            let mut b=[&[][..];4];
            b[usize::from(x)+2*usize::from(y)]=&[-7.25];
            close(&inverse97_plane(1,1,x,y,b[0],b[1],b[2],b[3]).unwrap(),&[-7.25]);
        }}
        for odd in [false,true]{
            close(&inverse97_plane(1,2,true,odd,&[],&[3.5],&[],&[0.]).unwrap(),&[3.5,3.5]);
            close(&inverse97_plane(2,1,odd,true,&[],&[],&[3.5],&[0.]).unwrap(),&[3.5,3.5]);
        }
        close(&inverse97_plane(4,4,false,false,&[12.5;4],&[0.;4],&[0.;4],&[0.;4]).unwrap(),&[12.5;16]);
    }
    #[test]fn plane97_limits_and_nonfinite(){
        assert!(inverse97_plane(0,1,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse97_plane(usize::MAX,2,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse97_plane(MAX_LINE+1,1,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse97_plane(2049,2049,false,false,&[],&[],&[],&[]).is_err());
        assert!(inverse97_plane(2,2,false,false,&[1.],&[0.],&[0.],&[]).is_err());
        assert!(inverse97_plane(2,2,false,false,&[1.],&[0.],&[0.],&[f64::NAN]).is_err());
        assert!(inverse97_plane(2,2,false,false,&[f64::MAX],&[0.],&[0.],&[0.]).is_err());
    }

    // Fixed independent forward vectors: Python Decimal, precision 45,
    // vertical then horizontal analysis, recursive LL ceiling projection.
    // Source samples (i*37 % 101)-50, stored as row-major f64.
    #[test]fn levels97_reference_even(){
        let d=[
            Detail97{hl:&[6.534075391286449],lh:&[3.4391614923352853,3.962211598459144],hh:&[-39.20445234772009]},
            Detail97{hl:&[-24.08893837066667,-8.063263012999867,-32.09570292647538,48.17180361830881],
                lh:&[29.43332653493182,8.113131348325524,30.613198440146434,40.93444676899272,1.4726374644929554,38.57470295856339],
                hh:&[-45.423037322715835,72.37631328544629,141.34607464543205,-94.25262657089266]}];
        let a=inverse97_levels(5,4,0,0,&[-6.834808418749882,2.6012462369509923],&d).unwrap();
        let expected:Vec<_>=(0..20).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn levels97_reference_odd_absolute(){
        let d=[
            Detail97{hl:&[],lh:&[],hh:&[]},
            Detail97{hl:&[],lh:&[-2.468749999999959],hh:&[18.628573981003154]},
            Detail97{hl:&[47.85039924543889,14.077974037269694,-50.756347319978104],
                lh:&[-6.3575813390964315,-36.28434331744513,-18.09839370118948,-15.009681642268784],
                hh:&[-146.14407677361353,62.79266629514089,133.18253648148948,15.083388326377317,-6.480770146062078,-13.745640332409979]}];
        let a=inverse97_levels(5,3,3,5,&[],&d).unwrap();
        let expected:Vec<_>=(0..15).map(|i|((i*37%101)-50)as f64).collect();
        close(&a,&expected);
    }
    #[test]fn levels97_reference_empty_coarse_axis(){
        let empty=Detail97{hl:&[],lh:&[],hh:&[]};
        let d=[empty,empty,Detail97{hl:&[-39.65199785375914,6.6505499194279825,-29.14910198509694],
            lh:&[],hh:&[5.843448074539255,-56.34344807453934]}];
        let a=inverse97_levels(1,5,7,2,&[],&d).unwrap();
        close(&a,&[-50.,-13.,24.,-40.,-3.]);
    }
    #[test]fn levels97_identity_and_deep_singletons(){
        assert_eq!(inverse97_levels(2,1,7,3,&[-1.5,9.],&[]).unwrap(),[-1.5,9.]);
        let empty=Detail97{hl:&[],lh:&[],hh:&[]};
        assert_eq!(inverse97_levels(1,1,0,0,&[2.25],&[empty;32]).unwrap(),[2.25]);
        assert_eq!(inverse97_levels(1,1,1,1,&[],&[empty,Detail97{hl:&[],lh:&[],hh:&[-3.]}]).unwrap(),[-3.]);
    }
    #[test]fn levels97_invalid_geometry_and_float(){
        let empty=Detail97{hl:&[],lh:&[],hh:&[]};
        assert!(inverse97_levels(0,1,0,0,&[],&[]).is_err());
        assert!(inverse97_levels(usize::MAX,2,0,0,&[],&[]).is_err());
        assert!(inverse97_levels(MAX_LINE+1,1,0,0,&[],&[]).is_err());
        assert!(inverse97_levels(2049,2049,0,0,&[],&[]).is_err());
        assert!(inverse97_levels(1,1,u32::MAX,0,&[0.],&[]).is_err());
        assert!(inverse97_levels(1,1,0,0,&[0.],&[empty;33]).is_err());
        assert!(inverse97_levels(1,1,0,0,&[],&[]).is_err());
        assert!(inverse97_levels(1,1,0,0,&[f64::NAN],&[]).is_err());
        assert!(inverse97_levels(2,2,0,0,&[0.],&[empty]).is_err());
        let d=[Detail97{hl:&[f64::INFINITY],lh:&[0.],hh:&[0.]}];
        assert!(inverse97_levels(2,2,0,0,&[0.],&d).is_err());
        let d=[Detail97{hl:&[0.],lh:&[0.],hh:&[0.]}];
        assert!(inverse97_levels(2,2,0,0,&[f64::MAX],&d).is_err());
    }
}
