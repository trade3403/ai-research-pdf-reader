//! Scalar and AVX2 INT8 linear with dynamic activation blocks.
//! A block dot is exact i32; scale recovery and cross-block sum use f64,
//! then one checked rounding to FP32 output. No dense weight expansion.
use crate::{
    matmul::avx2_available,
    quantization::{BlockInt8, MAX_BLOCK},
    quantized_table::QuantizedTable,
    tensor::{buffer, layout, Error, Tensor},
};

pub const MAX_DOT: i32 = MAX_BLOCK as i32 * 127 * 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Int8Kernel {
    Scalar,
    Auto,
}

#[derive(Clone, Copy)]
enum Resolved {
    Scalar,
    #[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
    Avx2,
}

impl Resolved {
    fn new(kernel: Int8Kernel) -> Self {
        #[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
        if kernel == Int8Kernel::Auto && avx2_available() {
            return Self::Avx2;
        }
        let _ = (kernel, avx2_available);
        Self::Scalar
    }

    fn dot(self, left: &[i8], right: &[i8]) -> i32 {
        match self {
            Self::Scalar => dot_valid(left, right),
            #[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
            Self::Avx2 => {
                // SAFETY: private variant created only after runtime CPU/OS
                // detection. Callers validate equal lengths and block limits.
                unsafe { dot_avx2(left, right) }
            }
        }
    }
}

/// Public checked scalar primitive. The internal kernel receives validated codes.
pub fn dot_i8(left: &[i8], right: &[i8]) -> Result<i32, Error> {
    dot_i8_with(left, right, Int8Kernel::Scalar)
}

pub fn dot_i8_with(left: &[i8], right: &[i8], kernel: Int8Kernel) -> Result<i32, Error> {
    if left.is_empty() || left.len() != right.len() {
        return Err(Error::Shape);
    }
    if left.len() > MAX_BLOCK {
        return Err(Error::Limit);
    }
    if left.contains(&i8::MIN) || right.contains(&i8::MIN) {
        return Err(Error::Shape);
    }
    Ok(Resolved::new(kernel).dot(left, right))
}

fn dot_valid(left: &[i8], right: &[i8]) -> i32 {
    left.iter()
        .zip(right)
        .map(|(&a, &b)| i32::from(a) * i32::from(b))
        .sum()
}

#[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
#[target_feature(enable = "avx2")]
unsafe fn dot_avx2(left: &[i8], right: &[i8]) -> i32 {
    use std::arch::x86_64::*;

    debug_assert_eq!(left.len(), right.len());
    debug_assert!(left.len() <= MAX_BLOCK);
    let (lv, lt) = left.as_chunks::<16>();
    let (rv, rt) = right.as_chunks::<16>();
    let mut accum = _mm256_setzero_si256();
    for (a, b) in lv.iter().zip(rv) {
        // SAFETY: each chunk contains 16 initialized bytes. Unaligned
        // 128-bit loads are valid; signed extension to i16 avoids the
        // unsigned/signed saturating maddubs instruction altogether.
        let (a, b) = unsafe {
            (
                _mm_loadu_si128(a.as_ptr().cast()),
                _mm_loadu_si128(b.as_ptr().cast()),
            )
        };
        let a = _mm256_cvtepi8_epi16(a);
        let b = _mm256_cvtepi8_epi16(b);
        accum = _mm256_add_epi32(accum, _mm256_madd_epi16(a, b));
    }
    let mut lanes = [0_i32; 8];
    // SAFETY: destination has eight initialized i32 lanes; storeu is unaligned.
    unsafe { _mm256_storeu_si256(lanes.as_mut_ptr().cast(), accum) };
    // Every partial and total absolute sum is bounded by MAX_DOT.
    lanes.into_iter().sum::<i32>() + dot_valid(lt, rt)
}

/// W is stored [out, in], and input is [rows, in]. Quantization is
/// independent for each input row and each matching block of columns.
pub struct QuantizedLinear<'a> {
    pub weight: &'a QuantizedTable,
    pub bias: Option<&'a Tensor>,
}

impl QuantizedLinear<'_> {
    pub fn forward(&self, input: &Tensor, last_only: bool) -> Result<Tensor, Error> {
        self.forward_with(input, last_only, Int8Kernel::Auto)
    }

    pub fn forward_with(
        &self,
        input: &Tensor,
        last_only: bool,
        kernel: Int8Kernel,
    ) -> Result<Tensor, Error> {
        let kernel = Resolved::new(kernel);
        let &[rows, width] = input.shape() else {
            return Err(Error::Shape);
        };
        let &[outputs, features] = self.weight.shape() else {
            return Err(Error::Shape);
        };
        if rows == 0 || width != features || self.bias.is_some_and(|b| b.shape() != [outputs]) {
            return Err(Error::Shape);
        }
        let count = if last_only { 1 } else { rows };
        let (_, size) = layout(&[count, outputs])?;
        if size.checked_mul(width).ok_or(Error::Limit)? > 1 << 30 {
            return Err(Error::Limit);
        }
        let block = self.weight.block_size();
        let activation_budget = width + width / block * 4;
        let start = (rows - count) * width;
        let mut data = buffer(size)?;
        for row in input.data()[start..].chunks_exact(width) {
            // Only one row of activation codes/scales is allocated at a time.
            let q = BlockInt8::quantize(row, block, activation_budget)?;
            for id in 0..outputs {
                let (weights, scales) = self.weight.packed_row(id)?;
                let mut sum = 0_f64;
                for (((a, w), &sa), &sw) in q
                    .codes()
                    .chunks_exact(block)
                    .zip(weights.chunks_exact(block))
                    .zip(q.scales())
                    .zip(scales)
                {
                    let dot = kernel.dot(a, w);
                    sum += f64::from(dot) * f64::from(sa) * f64::from(sw);
                }
                if let Some(bias) = self.bias {
                    sum += f64::from(bias.data()[id]);
                }
                let value = sum as f32;
                if !value.is_finite() {
                    return Err(Error::NonFinite);
                }
                data.push(value);
            }
        }
        Tensor::new(&[count, outputs], data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpmodel::quantized::{write_container, Reader, TensorInput};
    use std::io::Cursor;

    fn table(codes: &[i8], scales: &[f32], shape: &[u64], block: u32) -> QuantizedTable {
        let mut file = Cursor::new(Vec::new());
        write_container(
            &mut file,
            &[TensorInput {
                name: "weight",
                shape,
                block_size: block,
                codes,
                scales,
            }],
            1024,
        )
        .unwrap();
        let mut reader = Reader::open(file, 1024).unwrap();
        QuantizedTable::load(&mut reader, "weight", 1024).unwrap()
    }

    #[test]
    fn signed_dot_bounds_and_errors() {
        assert_eq!(dot_i8(&[127, -127, 0], &[-127, -127, 1]), Ok(0));
        let a = vec![127; MAX_BLOCK];
        let b = vec![-127; MAX_BLOCK];
        assert_eq!(dot_i8(&a, &a), Ok(MAX_DOT));
        assert_eq!(dot_i8(&a, &b), Ok(-MAX_DOT));
        assert_eq!(dot_i8(&[], &[]), Err(Error::Shape));
        assert_eq!(dot_i8(&[1], &[1, 2]), Err(Error::Shape));
        assert_eq!(dot_i8(&[-128], &[1]), Err(Error::Shape));
        let over = vec![0; MAX_BLOCK + 1];
        assert_eq!(dot_i8(&over, &over), Err(Error::Limit));
    }

    #[test]
    fn auto_matches_scalar_signed_extremes_and_unaligned_tails() {
        let mut seed = 0x9384_2a61_u32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 16) % 255) as i16 - 127
        };
        let a: Vec<i8> = (0..MAX_BLOCK + 3).map(|_| next() as i8).collect();
        let b: Vec<i8> = (0..MAX_BLOCK + 5).map(|_| next() as i8).collect();
        for len in [1, 2, 7, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 4095, 4096] {
            let x = &a[1..1 + len];
            let y = &b[3..3 + len];
            assert_eq!(
                dot_i8_with(x, y, Int8Kernel::Auto),
                dot_i8(x, y)
            );
        }
        let high = vec![127; MAX_BLOCK];
        let low = vec![-127; MAX_BLOCK];
        assert_eq!(dot_i8_with(&high, &high, Int8Kernel::Auto), Ok(MAX_DOT));
        assert_eq!(dot_i8_with(&high, &low, Int8Kernel::Auto), Ok(-MAX_DOT));
        assert_eq!(dot_i8_with(&low, &low, Int8Kernel::Auto), Ok(MAX_DOT));
        assert_eq!(dot_i8_with(&[-128], &[1], Int8Kernel::Auto), Err(Error::Shape));
    }

    #[test]
    fn independent_scales_bias_and_last_row() {
        let weight = table(
            &[2, -2, 4, 0, 1, 3, 2, -4],
            &[0.5, 0.25, 1., 0.5],
            &[2, 4],
            2,
        );
        let bias = Tensor::new(&[2], vec![0.5, -0.5]).unwrap();
        let input = Tensor::new(&[2, 4], vec![127., -127., 254., 0., 0., 0., 0., 0.]).unwrap();
        let linear = QuantizedLinear {
            weight: &weight,
            bias: Some(&bias),
        };
        let full = linear.forward(&input, false).unwrap();
        assert_eq!(full.data(), &[508.5, -0.5, 0.5, -0.5]);
        let last = linear.forward(&input, true).unwrap();
        assert_eq!(last.data(), &full.data()[2..]);
    }

    #[test]
    fn shape_and_nonfinite_output() {
        let weight = table(&[127], &[1.], &[1, 1], 1);
        let bad_bias = Tensor::new(&[2], vec![0., 0.]).unwrap();
        let linear = QuantizedLinear {
            weight: &weight,
            bias: None,
        };
        assert_eq!(
            linear
                .forward(&Tensor::zeros(&[0, 1]).unwrap(), false)
                .err(),
            Some(Error::Shape)
        );
        assert_eq!(
            linear
                .forward(&Tensor::zeros(&[1, 2]).unwrap(), false)
                .err(),
            Some(Error::Shape)
        );
        assert_eq!(
            QuantizedLinear {
                weight: &weight,
                bias: Some(&bad_bias)
            }
            .forward(&Tensor::zeros(&[1, 1]).unwrap(), false)
            .err(),
            Some(Error::Shape)
        );
        assert_eq!(
            linear
                .forward(&Tensor::new(&[1, 1], vec![f32::MAX]).unwrap(), false)
                .err(),
            Some(Error::NonFinite)
        );
    }
}
