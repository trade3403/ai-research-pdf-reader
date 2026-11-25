//! Reversible JPEG 2000 5/3 inverse lifting in one dimension.
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
}
