//! D1.5a: stable softmax and rank-2 scaled dot-product attention.
use crate::{
    matmul::{matmul, Kernel},
    tensor::{buffer, layout, Error, Tensor},
};

/// True means a position may be attended to. A fully masked row is zero.
#[derive(Clone, Copy, Default)]
pub struct Mask<'a> {
    pub key_keep: Option<&'a [bool]>,
    pub pair_keep: Option<&'a [bool]>,
    /// Query row i may see keys <= i + offset (for incremental decoding).
    pub causal_offset: Option<usize>,
}

impl Mask<'_> {
    fn validate(self, queries: usize, keys: usize) -> Result<(), Error> {
        let (_, size) = layout(&[queries, keys])?;
        if self.key_keep.is_some_and(|m| m.len() != keys)
            || self.pair_keep.is_some_and(|m| m.len() != size)
            || self.causal_offset.is_some_and(|o| o > keys)
        {
            return Err(Error::Shape);
        }
        Ok(())
    }

    fn allows(self, row: usize, col: usize, keys: usize) -> bool {
        self.key_keep.is_none_or(|m| m[col])
            && self.pair_keep.is_none_or(|m| m[row * keys + col])
            && self.causal_offset.is_none_or(|o| col <= row + o)
    }
}

/// Softmax across the last axis, using FP64 exponent and sum after max
/// subtraction. Input Tensor guarantees finite FP32 data.
pub fn softmax(input: &Tensor) -> Result<Tensor, Error> {
    let width = *input.shape().last().ok_or(Error::Shape)?;
    if width == 0 {
        return Err(Error::Shape);
    }
    let mut data = buffer(input.data().len())?;
    for row in input.data().chunks_exact(width) {
        softmax_row(row, |_| true, &mut data);
    }
    Tensor::new(input.shape(), data)
}

fn softmax_row(row: &[f32], keep: impl Fn(usize) -> bool, out: &mut Vec<f32>) {
    let max = row
        .iter()
        .enumerate()
        .filter(|(i, _)| keep(*i))
        .map(|(_, &x)| f64::from(x))
        .fold(f64::NEG_INFINITY, f64::max);
    if max == f64::NEG_INFINITY {
        out.resize(out.len() + row.len(), 0.0);
        return;
    }
    let sum: f64 = row
        .iter()
        .enumerate()
        .filter(|(i, _)| keep(*i))
        .map(|(_, &x)| (f64::from(x) - max).exp())
        .sum();
    out.extend(row.iter().enumerate().map(|(i, &x)| {
        if keep(i) {
            ((f64::from(x) - max).exp() / sum) as f32
        } else {
            0.0
        }
    }));
}

#[derive(Debug, PartialEq)]
pub struct AttentionOutput {
    pub context: Tensor,
    pub probabilities: Tensor,
}

/// Q[queries,d] K[keys,d] V[keys,value_dim] -> context[queries,value_dim].
/// The inputs are projected Q/K/V, not model weights. FP32 matmul errors
/// on nonfinite intermediate scores; the mask is not an additive float bias.
pub fn scaled_dot_product(
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    mask: Mask<'_>,
    kernel: Kernel,
) -> Result<AttentionOutput, Error> {
    let &[queries, dim] = q.shape() else {
        return Err(Error::Shape);
    };
    let &[keys, kd] = k.shape() else {
        return Err(Error::Shape);
    };
    let &[vk, vd] = v.shape() else {
        return Err(Error::Shape);
    };
    if dim == 0 || keys == 0 || vd == 0 || kd != dim || vk != keys {
        return Err(Error::Shape);
    }
    mask.validate(queries, keys)?;
    layout(&[queries, vd])?;
    let transposed = k.permute(&[1, 0])?;
    let scores = matmul(q, &transposed, kernel)?;
    let scale = 1.0 / (dim as f64).sqrt();
    let mut probabilities = buffer(scores.data().len())?;
    let mut scaled = buffer(keys)?;
    for (i, row) in scores.data().chunks_exact(keys).enumerate() {
        scaled.clear();
        scaled.extend(row.iter().map(|&x| (f64::from(x) * scale) as f32));
        softmax_row(&scaled, |j| mask.allows(i, j, keys), &mut probabilities);
    }
    let probabilities = Tensor::new(&[queries, keys], probabilities)?;
    let context = matmul(&probabilities, v, kernel)?;
    Ok(AttentionOutput {
        context,
        probabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (&a, &b) in actual.iter().zip(expected) {
            assert!((a - b).abs() <= 2e-6 + 2e-6 * b.abs(), "{a} != {b}");
        }
    }

    fn fixture() -> (Tensor, Tensor, Tensor) {
        (
            Tensor::new(&[2, 2], vec![1., 0., 0., 1.]).unwrap(),
            Tensor::new(&[3, 2], vec![1., 0., 0., 1., 1., 1.]).unwrap(),
            Tensor::new(&[3, 2], vec![2., 4., 6., 8., 10., 12.]).unwrap(),
        )
    }

    #[test]
    fn stable_softmax_extreme_and_shifted() {
        let a = Tensor::new(
            &[3, 2],
            vec![1000., 1001., -1001., -1000., f32::MAX, -f32::MAX],
        )
        .unwrap();
        let out = softmax(&a).unwrap();
        close(
            out.data(),
            &[0.268_941_43, 0.731_058_6, 0.268_941_43, 0.731_058_6, 1., 0.],
        );
        assert_eq!(softmax(&Tensor::zeros(&[0]).unwrap()), Err(Error::Shape));
        assert_eq!(softmax(&Tensor::zeros(&[]).unwrap()), Err(Error::Shape));
        assert_eq!(
            softmax(&Tensor::zeros(&[0, 3]).unwrap()).unwrap().shape(),
            &[0, 3]
        );
    }

    #[test]
    fn numpy_reference_cross_attention() {
        let (q, k, v) = fixture();
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let out = scaled_dot_product(&q, &k, &v, Mask::default(), kernel).unwrap();
            assert_eq!(out.context.shape(), &[2, 2]);
            assert_eq!(out.probabilities.shape(), &[2, 3]);
            close(
                out.probabilities.data(),
                &[
                    0.401_112_08,
                    0.197_775_81,
                    0.401_112_08,
                    0.197_775_81,
                    0.401_112_08,
                    0.401_112_08,
                ],
            );
            close(out.context.data(), &[6., 8., 6.813_345, 8.813_345]);
        }
    }

    #[test]
    fn padding_and_pair_mask() {
        let (q, k, v) = fixture();
        let mask = Mask {
            key_keep: Some(&[true, true, false]),
            ..Mask::default()
        };
        let out = scaled_dot_product(&q, &k, &v, mask, Kernel::Auto).unwrap();
        close(
            out.context.data(),
            &[3.320_953_8, 5.320_954, 4.679_046, 6.679_046],
        );
        assert_eq!(out.probabilities.data()[2], 0.0);
        assert_eq!(out.probabilities.data()[5], 0.0);
        let mask = Mask {
            pair_keep: Some(&[false, false, false, true, false, false]),
            ..mask
        };
        let out = scaled_dot_product(&q, &k, &v, mask, Kernel::Auto).unwrap();
        assert_eq!(out.context.data(), &[0., 0., 2., 4.]);
        assert_eq!(out.probabilities.data(), &[0., 0., 0., 1., 0., 0.]);
    }

    #[test]
    fn causal_and_cached_offset() {
        let (q, k, v) = fixture();
        let mask = Mask {
            causal_offset: Some(0),
            ..Mask::default()
        };
        let out = scaled_dot_product(&q, &k, &v, mask, Kernel::Auto).unwrap();
        close(out.context.data(), &[2., 4., 4.679_046, 6.679_046]);
        let mask = Mask {
            causal_offset: Some(1),
            ..mask
        };
        let out = scaled_dot_product(&q, &k, &v, mask, Kernel::Auto).unwrap();
        close(
            out.context.data(),
            &[3.320_953_8, 5.320_954, 6.813_345, 8.813_345],
        );
    }

    #[test]
    fn invalid_inputs_and_empty_query() {
        let (q, k, v) = fixture();
        for mask in [
            Mask {
                key_keep: Some(&[true]),
                ..Mask::default()
            },
            Mask {
                pair_keep: Some(&[]),
                ..Mask::default()
            },
            Mask {
                causal_offset: Some(usize::MAX),
                ..Mask::default()
            },
        ] {
            assert_eq!(
                scaled_dot_product(&q, &k, &v, mask, Kernel::Auto),
                Err(Error::Shape)
            );
        }
        let bad = Tensor::zeros(&[2, 3]).unwrap();
        assert_eq!(
            scaled_dot_product(&bad, &k, &v, Mask::default(), Kernel::Auto),
            Err(Error::Shape)
        );
        let empty = Tensor::zeros(&[0, 2]).unwrap();
        let out = scaled_dot_product(&empty, &k, &v, Mask::default(), Kernel::Auto).unwrap();
        assert_eq!(out.context.shape(), &[0, 2]);
        assert_eq!(out.probabilities.shape(), &[0, 3]);
        let huge = Tensor::new(&[1, 2], vec![f32::MAX, f32::MAX]).unwrap();
        assert_eq!(
            scaled_dot_product(&huge, &huge, &huge, Mask::default(), Kernel::Auto),
            Err(Error::NonFinite)
        );
    }
}
