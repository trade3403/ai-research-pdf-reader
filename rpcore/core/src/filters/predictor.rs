//! PDF TIFF horizontal differencing and PNG row predictors.
use crate::{Error,Object,MAX_BYTES};
use std::collections::BTreeMap;
pub(super) struct Predictor { kind: usize, colors: usize, bits: usize, samples: usize, row: usize, bpp: usize }
fn param(d: &BTreeMap<Vec<u8>,Object>, k: &[u8], default: usize) -> Result<usize,Error> {
    match d.get(k) {
        None => Ok(default),
        Some(Object::Number(n)) if *n >= 1.0 && n.fract() == 0.0 && *n <= MAX_BYTES as f64 => Ok(*n as usize),
        _ => Err(Error::Invalid("predictor parameter")),
    }
}
impl Predictor {
    pub(super) fn new(p: Option<&Object>) -> Result<Self,Error> {
        let empty = BTreeMap::new();
        let d = match p { None | Some(Object::Null) => &empty, Some(Object::Dict(d)) => d, _ => return Err(Error::Unsupported("DecodeParms reference or type")) };
        let kind = param(d,b"Predictor",1)?;
        if !matches!(kind,1|2|10..=15) { return Err(Error::Unsupported("predictor")); }
        let colors = param(d,b"Colors",1)?;
        let bits = param(d,b"BitsPerComponent",8)?;
        let columns = param(d,b"Columns",1)?;
        if !matches!(bits,1|2|4|8|16) { return Err(Error::Invalid("predictor bit depth")); }
        let samples = colors.checked_mul(columns).ok_or(Error::Limit("predictor samples"))?;
        let row = samples.checked_mul(bits).and_then(|v|v.checked_add(7)).ok_or(Error::Limit("predictor row"))? / 8;
        let bpp = colors.checked_mul(bits).and_then(|v|v.checked_add(7)).ok_or(Error::Limit("predictor pixel"))? / 8;
        if row > MAX_BYTES { return Err(Error::Limit("predictor row")); }
        Ok(Self { kind,colors,bits,samples,row,bpp })
    }
    pub(super) fn encoded_limit(&self, limit: usize) -> usize {
        if self.kind >= 10 { limit.saturating_add(limit / self.row).min(MAX_BYTES) } else { limit }
    }
    pub(super) fn apply(&self, mut data: Vec<u8>, limit: usize) -> Result<Vec<u8>,Error> {
        if self.kind == 1 {
            if data.len() > limit { return Err(Error::Limit("decoded stream")); }
            return Ok(data);
        }
        if self.kind == 2 {
            if data.len() % self.row != 0 { return Err(Error::Invalid("TIFF predictor row")); }
            if data.len() > limit { return Err(Error::Limit("decoded stream")); }
            // Fast byte case; packed 1/2/4-bit and big-endian 16-bit samples
            // use modular sample addition rather than byte addition.
            for row in data.chunks_exact_mut(self.row) {
                if self.bits == 8 {
                    for i in self.colors..self.samples { row[i] = row[i].wrapping_add(row[i-self.colors]); }
                } else {
                    let mask = (1u32 << self.bits) - 1;
                    for i in self.colors..self.samples {
                        let value = (sample(row,i,self.bits) + sample(row,i-self.colors,self.bits)) & mask;
                        write_sample(row,i,self.bits,value);
                    }
                }
            }
            return Ok(data);
        }
        let stride = self.row + 1;
        if data.len() % stride != 0 { return Err(Error::Invalid("PNG predictor row")); }
        let size = data.len() / stride * self.row;
        if size > limit { return Err(Error::Limit("decoded stream")); }
        let mut out = Vec::with_capacity(size);
        for line in data.chunks_exact(stride) {
            let tag = line[0];
            if tag > 4 { return Err(Error::Invalid("PNG predictor tag")); }
            let start = out.len();
            for (i,&v) in line[1..].iter().enumerate() {
                let a = if i >= self.bpp { out[start+i-self.bpp] } else { 0u8 };
                let b = if start >= self.row { out[start+i-self.row] } else { 0u8 };
                let c = if start >= self.row && i >= self.bpp { out[start+i-self.row-self.bpp] } else { 0u8 };
                let pred = match tag { 0=>0,1=>a,2=>b,3=>((u16::from(a)+u16::from(b))/2) as u8,4=>paeth(a,b,c),_=>unreachable!() };
                out.push(v.wrapping_add(pred));
            }
        }
        Ok(out)
    }
}
fn sample(row: &[u8], i: usize, bits: usize) -> u32 {
    let start = i * bits;
    let mut v = 0;
    for k in start..start+bits { v = (v << 1) | u32::from((row[k/8] >> (7-k%8)) & 1); }
    v
}
fn write_sample(row: &mut [u8], i: usize, bits: usize, v: u32) {
    let start = i * bits;
    for (j,k) in (start..start+bits).enumerate() {
        let mask = 1u8 << (7-k%8);
        row[k/8] = (row[k/8] & !mask) | (((v >> (bits-1-j)) as u8 & 1) * mask);
    }
}
fn paeth(a:u8,b:u8,c:u8)->u8 {
    let (a,b,c)=(i32::from(a),i32::from(b),i32::from(c));
    let p=a+b-c;
    let (pa,pb,pc)=((p-a).abs(),(p-b).abs(),(p-c).abs());
    if pa<=pb && pa<=pc {a as u8} else if pb<=pc {b as u8} else {c as u8}
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(s:&[u8])->Predictor { Predictor::new(Some(&crate::parse_object(s).unwrap())).unwrap() }
    #[test]
    fn png_all_rows() {
        let p=p(b"<< /Predictor 15 /Columns 3 >>");
        let data=vec![0,10,20,30,1,11,10,10,2,1,1,1,3,3,7,7,4,1,1,1];
        assert_eq!(p.apply(data,15).unwrap(),vec![10,20,30,11,21,31,12,22,32,9,22,34,10,23,35]);
        assert!(p.apply(vec![5,1,2,3],3).is_err());
        assert!(p.apply(vec![1,2],3).is_err());
        assert!(p.apply(vec![1,2,3,4],2).is_err());
    }
    #[test]
    fn tiff_packed_and_16() {
        assert_eq!(p(b"<< /Predictor 2 /Columns 3 >>").apply(vec![10,10,10],3).unwrap(),[10,20,30]);
        assert_eq!(p(b"<< /Predictor 2 /Columns 4 /BitsPerComponent 2 >>").apply(vec![0x55],1).unwrap(),[0x6c]);
        assert_eq!(p(b"<< /Predictor 2 /Columns 2 /BitsPerComponent 16 >>").apply(vec![0,255,0,2],4).unwrap(),[0,255,1,1]);
        assert_eq!(p(b"<< /Predictor 2 /Columns 2 /Colors 3 >>").apply(vec![1,2,3,4,5,6],6).unwrap(),[1,2,3,5,7,9]);
    }
}
