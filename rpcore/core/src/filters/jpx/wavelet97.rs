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

    // B11.2g4: fixed vectors computed independently with JavaScript BigInt
    // decimal fixed-point (50 fractional digits), forward lifting on absolute
    // coordinate maps, then vertical/horizontal subband analysis and LL recursion.
    // Signal: ((i*73+seed*31)%257-128) divided by 8 for seeds divisible by 3.
    // Existing close() tolerance is unchanged. These are not codestream fixtures.
    #[test]fn closure97_line_even(){
        close(&inverse97_line(&[-6.0499999999999595e+1],&[7.3000000000000384e+1],false).unwrap(),&[-9.7000000000000000e+1,-2.3999999999999996e+1]);
        close(&inverse97_line(&[-5.2341193826465165e+1,6.6341193826465116e+1],&[-1.8656053938400111e-14],false).unwrap(),&[-6.5999999999999986e+1,7.0000000000000000e+0,8.0000000000000000e+1]);
        close(&inverse97_line(&[-1.9656183141737951e+0,7.3066242575113414e+0,-3.5101545706718146e-1,-2.4455992867145553e+0],&[1.1977303900452634e+0,-2.0843191170135867e+1,1.9645460780090591e+1],false).unwrap(),&[-4.3750000000000000e+0,4.7499999999999991e+0,1.3874999999999998e+1,-9.1249999999999982e+0,0.0000000000000000e+0,9.1249999999999982e+0,-1.3874999999999998e+1]);
        close(&inverse97_line(&[5.5481812482511081e+1,-7.2379334281231621e+1,4.7198060915972434e+1,-2.1809632875995444e+1],&[1.3808184312036261e+2,-1.4788686240724333e+1,1.2996300882604430e+2,8.0487668588636780e+1],false).unwrap(),&[-4.0000000000000000e+0,6.9000000000000000e+1,-1.1499999999999999e+2,-4.2000000000000000e+1,3.1000000000000000e+1,1.0400000000000000e+2,-8.0000000000000000e+1,-7.0000000000000000e+0]);
        close(&inverse97_line(&[8.6481812482510989e+1,-3.7045255841412882e+1,1.3519301058131578e+1,-6.9103379497950613e+1,4.5978948843372152e+1,-1.6808123656537404e+1,-3.3564794293716389e+1],&[1.3808184312036252e+2,2.9316656188914457e-14,-1.4328868624072470e+2,1.4788686240724381e+1,-1.6674552936108688e+2,1.5716368624072479e+2],false).unwrap(),&[2.6999999999999996e+1,9.9999999999999986e+1,-8.4000000000000000e+1,-1.0999999999999998e+1,6.2000000000000000e+1,-1.2199999999999999e+2,-4.8999999999999993e+1,2.3999999999999996e+1,9.7000000000000000e+1,-8.7000000000000000e+1,-1.4000000000000000e+1,5.8999999999999993e+1,-1.2499999999999999e+2]);
        close(&inverse97_line(&[-2.4607930496408876e+0,-8.7869069801766422e+0,6.1066724372437884e+0,-4.7629224372438390e+0,9.6223686054215065e+0,2.1249999999999929e+0,-5.0012295963465325e+0,7.3961644959220738e+0],&[-1.6713355390045308e+1,1.8485857800905490e+0,-1.7911085780090595e+1,1.8485857800905372e+0,-2.0843191170135871e+1,2.0843191170135860e+1,-5.8219006688033703e-1,7.8939193536701344e+0],false).unwrap(),&[7.2499999999999991e+0,-1.5749999999999998e+1,-6.6249999999999991e+0,2.5000000000000000e+0,1.1624999999999998e+1,-1.1374999999999998e+1,-2.2500000000000000e+0,6.8749999999999991e+0,1.6000000000000000e+1,-7.0000000000000000e+0,2.1249999999999996e+0,1.1249999999999998e+1,-1.1750000000000000e+1,-2.6250000000000000e+0,6.5000000000000000e+0,1.5624999999999998e+1]);
    }
    #[test]fn closure97_line_odd(){
        close(&inverse97_line(&[-6.0500000000000021e+1],&[-7.3000000000000071e+1],true).unwrap(),&[-9.7000000000000000e+1,-2.3999999999999996e+1]);
        close(&inverse97_line(&[6.9999999999999769e+0],&[-8.9802709658722080e+1,8.9802709658722037e+1],true).unwrap(),&[-6.5999999999999986e+1,7.0000000000000000e+0,8.0000000000000000e+1]);
        close(&inverse97_line(&[5.5074541094994682e+0,-2.2056088004441747e+0,1.4481546909446916e+0],&[-1.3758130133760694e+1,1.9576795456925645e+1,1.2663957132102115e+0,-2.7928252206511040e+1],true).unwrap(),&[-4.3750000000000000e+0,4.7499999999999991e+0,1.3874999999999998e+1,-9.1249999999999982e+0,0.0000000000000000e+0,9.1249999999999982e+0,-1.3874999999999998e+1]);
        close(&inverse97_line(&[1.0809632875995479e+1,-5.8198060915972391e+1,6.1379334281231650e+1,-6.6481812482511046e+1],&[-8.0487668588636751e+1,-1.2996300882604427e+2,1.4788686240724363e+1,-1.3808184312036258e+2],true).unwrap(),&[-4.0000000000000000e+0,6.9000000000000000e+1,-1.1499999999999999e+2,-4.2000000000000000e+1,3.1000000000000000e+1,1.0400000000000000e+2,-8.0000000000000000e+1,-7.0000000000000000e+0]);
        close(&inverse97_line(&[3.4935202221417235e+1,-7.0946814180217199e+0,-7.0978948843372066e+1,4.4103379497950691e+1,-3.5550188985531598e+1,-2.4147624724424221e+0],&[-8.0487668588636822e+1,-1.5341985194640648e+2,1.6674552936108697e+2,-1.4788686240724314e+1,1.4328868624072476e+2,1.0131165705681731e+1,-2.2342601765208829e+2],true).unwrap(),&[2.6999999999999996e+1,9.9999999999999986e+1,-8.4000000000000000e+1,-1.0999999999999998e+1,6.2000000000000000e+1,-1.2199999999999999e+2,-4.8999999999999993e+1,2.3999999999999996e+1,9.7000000000000000e+1,-8.7000000000000000e+1,-1.4000000000000000e+1,5.8999999999999993e+1,-1.2499999999999999e+2]);
        close(&inverse97_line(&[-8.6149144959220241e+0,4.6417834281688508e+0,-5.8566724372437893e+0,9.3879224372438248e+0,-9.3991263226644961e-1,5.1899126322664362e+0,-5.4889378943110199e+0,1.4299137924128466e+1],&[2.7928252206511058e+1,-3.1149814933007498e+0,1.7911085780090595e+1,-1.8485857800905496e+0,1.7911085780090584e+1,-5.6634449455857476e-15,-1.7911085780090595e+1,6.5085539004526227e-1],true).unwrap(),&[7.2499999999999991e+0,-1.5749999999999998e+1,-6.6249999999999991e+0,2.5000000000000000e+0,1.1624999999999998e+1,-1.1374999999999998e+1,-2.2500000000000000e+0,6.8749999999999991e+0,1.6000000000000000e+1,-7.0000000000000000e+0,2.1249999999999996e+0,1.1249999999999998e+1,-1.1750000000000000e+1,-2.6250000000000000e+0,6.5000000000000000e+0,1.5624999999999998e+1]);
    }
    #[test]fn closure97_plane_reference(){
        close(&inverse97_plane(2,3,false,false,&[2.7437134297047716e-1,2.8725628657029013e+1],&[-5.5499999999999254e+1,-5.5499999999999325e+1],&[7.0575276787844353e-13],&[2.5700000000000176e+2]).unwrap(),&[8.9000000000000000e+1,-9.4999999999999986e+1,-2.1999999999999996e+1,5.1000000000000000e+1,1.2400000000000000e+2,-5.9999999999999993e+1]);
        close(&inverse97_plane(3,2,true,false,&[-1.8749999999999616e+1],&[1.9677466282272687e+2,6.0225337177273211e+1],&[-3.7999999999999950e+1],&[-4.2315838195315619e-13,-5.9233869882619947e-14]).unwrap(),&[1.1999999999999999e+2,-6.4000000000000000e+1,9.0000000000000000e+0,8.2000000000000000e+1,-1.0200000000000000e+2,-2.8999999999999996e+1]);
        close(&inverse97_plane(4,3,false,true,&[-5.9942201985124646e+0,4.1924225992562150e+0],&[2.0019796582328393e+0,-1.8972709316465636e+1],&[-6.5902303971424763e+0,4.1776405251184503e+0,5.5067107871877976e+0,4.3953692798589286e+0],&[-2.0613340687350963e+0,3.9944839697651531e+1,2.1274828864452436e-1,-4.1226681374701490e+0]).unwrap(),&[-1.3249999999999998e+1,-4.1249999999999991e+0,5.0000000000000000e+0,1.4124999999999998e+1,-8.8749999999999982e+0,2.5000000000000000e-1,9.3750000000000000e+0,-1.3625000000000000e+1,-4.5000000000000000e+0,4.6250000000000000e+0,1.3749999999999998e+1,-9.2500000000000000e+0]);
        close(&inverse97_plane(3,4,true,true,&[-2.3134521126157278e+1,-3.4355957747684080e+1],&[-3.7742670863795027e+1,8.4429643580603994e+1,2.3594327147359866e+2,5.6182783092784533e+1],&[3.7207386433184965e+1,3.2521306783407809e+1],&[1.9331499010300256e+1,-1.9951852510250525e+0,1.3362293673557474e+2,-1.3791093615212315e+1]).unwrap(),&[-7.5000000000000000e+1,-2.0000000000000000e+0,7.0999999999999986e+1,-1.1299999999999999e+2,-4.0000000000000000e+1,3.2999999999999993e+1,1.0599999999999999e+2,-7.7999999999999986e+1,-5.0000000000000000e+0,6.7999999999999986e+1,-1.1599999999999999e+2,-4.2999999999999993e+1]);
        close(&inverse97_plane(1,7,true,false,&[],&[-2.4724946513390329e+1,4.9452994060090759e+1,-1.1808123656537422e+1,-2.8564794293716414e+1],&[],&[9.5818431203621319e+0,-1.6674552936108691e+2,1.5716368624072476e+2]).unwrap(),&[-4.3999999999999993e+1,2.8999999999999996e+1,1.0200000000000000e+2,-8.2000000000000000e+1,-9.0000000000000000e+0,6.4000000000000000e+1,-1.1999999999999999e+2]);
        close(&inverse97_plane(7,1,false,true,&[],&[],&[5.8102265603138887e+0,-1.0172416785153947e+1,4.9119069801766564e+0,-1.4142069503590995e+0],&[1.7260230390045329e+1,-1.8485857800905388e+0,1.6713355390045319e+1]).unwrap(),&[-1.6250000000000000e+0,7.4999999999999991e+0,-1.5500000000000000e+1,-6.3750000000000000e+0,2.7499999999999996e+0,1.1874999999999998e+1,-1.1125000000000000e+1]);
    }
    #[test]fn closure97_levels_reference(){
        close(&inverse97_levels(7,5,0,0,&[1.4106974060828689e+1],&[Detail97{hl:&[1.6358155635025899e+1],lh:&[-8.1274324046227591e+0],hh:&[-2.1936116648349283e-14]},Detail97{hl:&[-1.2895562435729258e+2,-4.1233399616087659e+1,-1.2895562435729255e+2,-4.1233399616087645e+1],lh:&[-1.5798736008077008e-14,-5.9395683702332512e-14],hh:&[3.4368615480996012e-13,1.0989321818590574e-13]},Detail97{hl:&[1.3808184312036209e+2,-1.4788686240724342e+1,1.3370684312036204e+2,1.3808184312036212e+2,-1.4788686240724330e+1,1.3370684312036204e+2,1.3808184312036212e+2,-1.4788686240724315e+1,1.3370684312036207e+2],lh:&[-7.5000000000019129e-1,-7.4999999999985045e-1,-7.5000000000017208e-1,-7.5000000000003730e-1,7.4999999999981026e-1,7.5000000000015099e-1,7.4999999999982936e-1,7.4999999999996425e-1],hh:&[-3.6601003895905516e-13,4.1412938376975667e-14,-3.5435000524755489e-13,-3.7000776480299803e-13,3.7415212533032768e-14,-3.5834773109149781e-13]}]).unwrap(),&[1.8000000000000000e+1,9.0999999999999986e+1,-9.2999999999999986e+1,-2.0000000000000000e+1,5.2999999999999993e+1,1.2599999999999999e+2,-5.7999999999999993e+1,1.4999999999999998e+1,8.7999999999999986e+1,-9.5999999999999986e+1,-2.3000000000000000e+1,4.9999999999999993e+1,1.2300000000000000e+2,-6.0999999999999993e+1,1.1999999999999998e+1,8.5000000000000000e+1,-9.9000000000000000e+1,-2.6000000000000000e+1,4.7000000000000000e+1,1.1999999999999999e+2,-6.4000000000000000e+1,9.0000000000000000e+0,8.2000000000000000e+1,-1.0200000000000000e+2,-2.8999999999999996e+1,4.3999999999999993e+1,1.1700000000000000e+2,-6.7000000000000000e+1,5.9999999999999991e+0,7.9000000000000000e+1,-1.0499999999999999e+2,-3.2000000000000000e+1,4.1000000000000000e+1,1.1400000000000000e+2,-7.0000000000000000e+1]);
        close(&inverse97_levels(6,4,3,7,&[1.6056633848896421e+1],&[Detail97{hl:&[-2.6230075742932613e+0],lh:&[],hh:&[]},Detail97{hl:&[4.7744371232501130e+1],lh:&[-2.2567950173649610e+1,1.3645816396048728e+1],hh:&[1.3526882566709230e+2]},Detail97{hl:&[-1.0840092658116770e+2,4.7795148078452527e+1,-2.4171199896463054e+0,1.1505463179461414e+2,-2.3488263912865076e+1,2.6058184191107183e+0],lh:&[-2.7272724300606246e+0,3.3540318113551685e+1,-7.5052227035134152e+1,-3.4938053533945507e+1,4.7947694862860466e+1,-8.6306214823756434e+1],hh:&[8.8458821055245338e+1,-3.6136477030792906e+2,2.2688088914139223e+2,-1.4927256740726108e+2,-3.5524457966398984e+1,-2.3021202301051576e+2]}]).unwrap(),&[4.8999999999999993e+1,1.2199999999999999e+2,-6.2000000000000000e+1,1.0999999999999998e+1,8.4000000000000000e+1,-9.9999999999999986e+1,-2.6999999999999996e+1,4.6000000000000000e+1,1.1900000000000000e+2,-6.5000000000000000e+1,8.0000000000000000e+0,8.0999999999999986e+1,-1.0299999999999999e+2,-2.9999999999999996e+1,4.2999999999999993e+1,1.1599999999999999e+2,-6.7999999999999986e+1,5.0000000000000000e+0,7.7999999999999986e+1,-1.0599999999999999e+2,-3.2999999999999993e+1,4.0000000000000000e+1,1.1299999999999999e+2,-7.0999999999999986e+1]);
        close(&inverse97_levels(4,5,14,9,&[],&[Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[-6.7484714954486236e-1],hh:&[]},Detail97{hl:&[-5.9240828972689812e+0],lh:&[7.2745649685533453e-1],hh:&[9.4706094691030351e+0]},Detail97{hl:&[-1.4497279333651688e+1,1.4025808667303426e+1,2.8688452725155518e+0,-2.0706440545031061e+1],lh:&[-4.8935805342845935e+0,-5.2381560146067576e+0,-1.1403580930724692e+1,5.7017904653623965e+0,7.2474855749362312e+0,4.0612034942809263e+0],hh:&[1.6187564671272243e+0,-6.9346844944354693e+0,9.5596522341172321e+0,1.6702867091946878e+1,-3.5920413254067909e+0,3.4869110906324798e+0]}]).unwrap(),&[1.0000000000000000e+1,-1.3000000000000000e+1,-3.8750000000000000e+0,5.2500000000000000e+0,1.4374999999999998e+1,-8.6250000000000000e+0,5.0000000000000000e-1,9.6250000000000000e+0,-1.3375000000000000e+1,-4.2499999999999991e+0,4.8749999999999991e+0,1.4000000000000000e+1,-9.0000000000000000e+0,1.2500000000000000e-1,9.2500000000000000e+0,-1.3749999999999998e+1,-4.6250000000000000e+0,4.5000000000000000e+0,1.3625000000000000e+1,-9.3750000000000000e+0]);
        close(&inverse97_levels(1,7,7,2,&[],&[Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[1.9564794293716442e+1,2.8081236565374517e+0,-5.8452994060090731e+1,1.5724946513390361e+1],lh:&[],hh:&[-1.5716368624072473e+2,1.6674552936108694e+2,-9.5818431203621071e+0]}]).unwrap(),&[1.1099999999999999e+2,-7.2999999999999986e+1,0.0000000000000000e+0,7.2999999999999986e+1,-1.1099999999999999e+2,-3.7999999999999993e+1,3.5000000000000000e+1]);
        close(&inverse97_levels(7,1,2,7,&[],&[Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[-1.0439310339302776e+2,5.3911503154488123e+1,-3.4327424714668972e+1,4.6724946513390258e+1],hh:&[-5.2068431203621248e+0,1.4328868624072476e+2,-9.5818431203621888e+0]}]).unwrap(),&[-1.1499999999999999e+2,-4.2000000000000000e+1,3.1000000000000000e+1,1.0400000000000000e+2,-8.0000000000000000e+1,-7.0000000000000000e+0,6.5999999999999986e+1]);
        close(&inverse97_levels(3,3,4294967291,4294967291,&[],&[Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[],lh:&[],hh:&[-4.1171874999999503e+0]},Detail97{hl:&[],lh:&[],hh:&[]},Detail97{hl:&[-2.2697958172948911e+0,1.0301045817294959e+1],lh:&[4.9190341082962217e+0,3.1122158917038458e+0],hh:&[-4.1226681374701943e+0,4.2549657728904505e-1,3.9944839697651489e+1,-4.1226681374702228e+0]}]).unwrap(),&[-1.0500000000000000e+1,-1.3749999999999998e+0,7.7500000000000000e+0,-1.5249999999999998e+1,-6.1249999999999991e+0,2.9999999999999996e+0,1.2125000000000000e+1,-1.0875000000000000e+1,-1.7500000000000000e+0]);
    }
}
