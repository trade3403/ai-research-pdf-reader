//! Portable scalar INT8 linear reference with dynamic activation blocks.
//! A block dot is exact i32; scale recovery and cross-block sum use f64,
//! then one checked rounding to FP32 output. No dense weight expansion.
use crate::{
    quantization::{BlockInt8, MAX_BLOCK},
    quantized_table::QuantizedTable,
    tensor::{buffer, layout, Error, Tensor},
};

pub const MAX_DOT: i32 = MAX_BLOCK as i32 * 127 * 127;

/// Public checked primitive. The internal kernel receives validated codes.
pub fn dot_i8(left: &[i8], right: &[i8]) -> Result<i32, Error> {
    if left.is_empty() || left.len() != right.len() {
        return Err(Error::Shape);
    }
    if left.len() > MAX_BLOCK {
        return Err(Error::Limit);
    }
    if left.contains(&i8::MIN) || right.contains(&i8::MIN) {
        return Err(Error::Shape);
    }
    Ok(dot_valid(left, right))
}

fn dot_valid(left: &[i8], right: &[i8]) -> i32 {
    left.iter()
        .zip(right)
        .map(|(&a, &b)| i32::from(a) * i32::from(b))
        .sum()
}

/// W is stored [out, in], and input is [rows, in]. Quantization is
/// independent for each input row and each matching block of columns.
pub struct QuantizedLinear<'a> {
    pub weight: &'a QuantizedTable,
    pub bias: Option<&'a Tensor>,
}

impl QuantizedLinear<'_> {
    pub fn forward(&self, input: &Tensor, last_only: bool) -> Result<Tensor, Error> {
        let &[rows, width] = input.shape() else {
            return Err(Error::Shape);
        };
        let &[outputs, features] = self.weight.shape() else {
            return Err(Error::Shape);
        };
        if rows == 0
            || width != features
            || self.bias.is_some_and(|b| b.shape() != [outputs])
        {
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
                    let dot = dot_valid(a, w);
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
    fn independent_scales_bias_and_last_row() {
        let weight = table(&[2, -2, 4, 0, 1, 3, 2, -4], &[0.5, 0.25, 1., 0.5], &[2, 4], 2);
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
            linear.forward(&Tensor::zeros(&[0, 1]).unwrap(), false).err(),
            Some(Error::Shape)
        );
        assert_eq!(
            linear.forward(&Tensor::zeros(&[1, 2]).unwrap(), false).err(),
            Some(Error::Shape)
        );
        assert_eq!(
            QuantizedLinear { weight: &weight, bias: Some(&bad_bias) }
                .forward(&Tensor::zeros(&[1, 1]).unwrap(), false).err(),
            Some(Error::Shape)
        );
        assert_eq!(
            linear.forward(&Tensor::new(&[1, 1], vec![f32::MAX]).unwrap(), false).err(),
            Some(Error::NonFinite)
        );
    }
}
