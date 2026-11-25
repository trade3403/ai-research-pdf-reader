//! D1.9a: symmetric signed INT8, independent contiguous block scales.
//! No model format, activation quantization or integer matmul in this stage.
use crate::tensor::{buffer, Error};

pub const MAX_ELEMENTS: usize = 16 * 1024 * 1024;
pub const MAX_BLOCK: usize = 4096;
pub const QMAX: i8 = 127;

fn inventory(len: usize, block: usize, budget: usize) -> Result<(usize, usize), Error> {
    if len == 0 || block == 0 {
        return Err(Error::Shape);
    }
    if len > MAX_ELEMENTS || block > MAX_BLOCK {
        return Err(Error::Limit);
    }
    let count = len.div_ceil(block);
    let bytes = count
        .checked_mul(4)
        .and_then(|v| v.checked_add(len))
        .ok_or(Error::Limit)?;
    if bytes > budget {
        return Err(Error::Limit);
    }
    Ok((count, bytes))
}

fn scale_for(values: &[f32]) -> f32 {
    let peak = values.iter().fold(0_f32, |a, &b| a.max(b.abs()));
    if peak == 0.0 {
        return 1.0;
    }
    // Compute the scale in f64 then round to the stored f32. Preserve the
    // smallest subnormal; avoid overflow when reconstructing +/-127.
    let mut scale = ((f64::from(peak) / 127.0) as f32).max(f32::from_bits(1));
    if f64::from(scale) * 127.0 > f64::from(f32::MAX) {
        scale = f32::from_bits(scale.to_bits() - 1);
    }
    scale
}

fn quantize_value(value: f32, scale: f32) -> i8 {
    (f64::from(value) / f64::from(scale))
        .round_ties_even()
        .clamp(-127.0, 127.0) as i8
}

/// Owns packed codes and one finite, positive FP32 scale per block.
/// Zero point is zero; -128 is invalid. The last block can be shorter.
/// Payload budget counts codes and scales, not allocations or metadata.
#[derive(Debug, PartialEq)]
pub struct BlockInt8 {
    codes: Vec<i8>,
    scales: Vec<f32>,
    block_size: usize,
    payload_bytes: usize,
}

impl BlockInt8 {
    pub fn quantize(values: &[f32], block_size: usize, budget: usize) -> Result<Self, Error> {
        let (count, _) = inventory(values.len(), block_size, budget)?;
        if values.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        let mut codes = Vec::new();
        codes
            .try_reserve_exact(values.len())
            .map_err(|_| Error::Allocation)?;
        let mut scales = buffer(count)?;
        for block in values.chunks(block_size) {
            let scale = scale_for(block);
            scales.push(scale);
            codes.extend(block.iter().map(|&v| quantize_value(v, scale)));
        }
        Self::from_parts(codes, scales, block_size, budget)
    }

    /// Validate imported raw parts without copying. Caller-owned input
    /// allocations already exist; the budget limits retained payload.
    pub fn from_parts(
        codes: Vec<i8>,
        scales: Vec<f32>,
        block_size: usize,
        budget: usize,
    ) -> Result<Self, Error> {
        let (count, payload_bytes) = inventory(codes.len(), block_size, budget)?;
        if scales.len() != count || codes.contains(&i8::MIN) {
            return Err(Error::Shape);
        }
        for (&scale, block) in scales.iter().zip(codes.chunks(block_size)) {
            if !scale.is_finite() || scale <= 0.0 {
                return Err(Error::NonFinite);
            }
            let peak = block.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
            if f64::from(scale) * f64::from(peak) > f64::from(f32::MAX) {
                return Err(Error::NonFinite);
            }
        }
        Ok(Self {
            codes,
            scales,
            block_size,
            payload_bytes,
        })
    }

    pub fn codes(&self) -> &[i8] {
        &self.codes
    }

    pub fn scales(&self) -> &[f32] {
        &self.scales
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }

    /// Dequantize a checked range, including a partial first/last block.
    /// The caller selects an output budget in bytes before allocation.
    pub fn dequantize_range(
        &self,
        start: usize,
        count: usize,
        budget: usize,
    ) -> Result<Vec<f32>, Error> {
        let end = start.checked_add(count).ok_or(Error::Limit)?;
        if end > self.codes.len() {
            return Err(Error::Index);
        }
        if count.checked_mul(4).ok_or(Error::Limit)? > budget {
            return Err(Error::Limit);
        }
        let mut out = buffer(count)?;
        for (offset, &code) in self.codes[start..end].iter().enumerate() {
            let scale = self.scales[(start + offset) / self.block_size];
            out.push((f64::from(code) * f64::from(scale)) as f32);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_ties_and_zero_block() {
        let q =
            BlockInt8::quantize(&[-127., -2.5, -1.5, 0., 1.5, 2.5, 127., 0., 0.], 7, 17).unwrap();
        assert_eq!(q.codes(), &[-127, -2, -2, 0, 2, 2, 127, 0, 0]);
        assert_eq!(q.scales(), &[1., 1.]);
        assert_eq!(q.payload_bytes(), 17);
        assert_eq!(q.dequantize_range(5, 4, 16).unwrap(), [2., 127., 0., 0.]);
    }

    #[test]
    fn independent_blocks_and_tail() {
        let input = [1., -0.25, 0.5, 100., -25., 50., 0.125];
        let q = BlockInt8::quantize(&input, 3, 19).unwrap();
        assert_eq!(q.scales().len(), 3);
        assert_eq!(q.block_size(), 3);
        assert_eq!(q.codes()[0], 127);
        assert_eq!(q.codes()[3], 127);
        assert_eq!(q.codes()[6], 127);
        let all = q.dequantize_range(0, input.len(), 28).unwrap();
        for (i, (&actual, &expected)) in all.iter().zip(&input).enumerate() {
            let scale = f64::from(q.scales()[i / 3]);
            let error = (f64::from(actual) - f64::from(expected)).abs();
            assert!(error <= scale / 2.0 + f64::from(expected.abs()) * 1e-6);
        }
        assert_eq!(q.dequantize_range(2, 4, 16).unwrap(), all[2..6]);
    }

    #[test]
    fn extreme_finite_and_subnormal() {
        let tiny = f32::from_bits(1);
        let input = [f32::MAX, -f32::MAX, 0., tiny, -tiny, tiny * 2.];
        let q = BlockInt8::quantize(&input, 3, 14).unwrap();
        let actual = q.dequantize_range(0, 6, 24).unwrap();
        assert!(actual.iter().all(|v| v.is_finite()));
        assert_eq!(&actual[3..], &input[3..]);
        assert_eq!(q.codes()[0], 127);
        assert_eq!(q.codes()[1], -127);
    }

    #[test]
    fn malformed_parts_and_budget() {
        assert_eq!(BlockInt8::quantize(&[1.], 1, 4).err(), Some(Error::Limit));
        assert_eq!(BlockInt8::quantize(&[], 1, 100).err(), Some(Error::Shape));
        assert_eq!(BlockInt8::quantize(&[1.], 0, 100).err(), Some(Error::Shape));
        assert_eq!(
            BlockInt8::quantize(&[1.], MAX_BLOCK + 1, 100).err(),
            Some(Error::Limit)
        );
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                BlockInt8::quantize(&[value], 1, 5).err(),
                Some(Error::NonFinite)
            );
        }
        assert!(BlockInt8::from_parts(vec![-128], vec![1.], 1, 5).is_err());
        assert!(BlockInt8::from_parts(vec![127], vec![f32::MAX], 1, 5).is_err());
        assert!(BlockInt8::from_parts(vec![1], vec![], 1, 5).is_err());
        for scale in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(BlockInt8::from_parts(vec![1], vec![scale], 1, 5).is_err());
        }
        let q = BlockInt8::quantize(&[1., 2.], 2, 6).unwrap();
        assert_eq!(q.dequantize_range(1, 2, 8).err(), Some(Error::Index));
        assert_eq!(q.dequantize_range(0, 2, 7).err(), Some(Error::Limit));
        assert_eq!(
            q.dequantize_range(usize::MAX, 1, 4).err(),
            Some(Error::Limit)
        );
        assert!(q.dequantize_range(2, 0, 0).unwrap().is_empty());
    }

    #[test]
    fn standard_block_payloads() {
        let values = vec![0.125; 4097];
        for block in [32, 64, 128, 256, 512, 4096] {
            let bytes = values.len() + values.len().div_ceil(block) * 4;
            let q = BlockInt8::quantize(&values, block, bytes).unwrap();
            assert_eq!(q.payload_bytes(), bytes);
            assert!(q.codes().iter().all(|&v| v == 127));
        }
    }
}
