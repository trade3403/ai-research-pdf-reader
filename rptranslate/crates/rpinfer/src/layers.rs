//! FP32 inference layers, with explicit weight layout and checked shapes.
use crate::{
    attention::{scaled_dot_product, Mask},
    matmul::{matmul, Kernel},
    tensor::{buffer, layout, Binary, Error, Tensor},
};

/// Weight layout [out_features, in_features], matching Marian checkpoints.
#[derive(Clone, Copy)]
pub struct Linear<'a> {
    pub weight: &'a Tensor,
    pub bias: Option<&'a Tensor>,
}

impl Linear<'_> {
    pub fn forward(self, x: &Tensor, kernel: Kernel) -> Result<Tensor, Error> {
        let &[_, input] = x.shape() else {
            return Err(Error::Shape);
        };
        let &[output, width] = self.weight.shape() else {
            return Err(Error::Shape);
        };
        if input == 0 || output == 0 || input != width {
            return Err(Error::Shape);
        }
        if self.bias.is_some_and(|b| b.shape() != [output]) {
            return Err(Error::Shape);
        }
        let y = matmul(x, &self.weight.permute(&[1, 0])?, kernel)?;
        match self.bias {
            Some(b) => y.binary(b, Binary::Add),
            None => Ok(y),
        }
    }
}

/// Last-axis LayerNorm with population variance, FP64 statistics and affine
/// parameters; epsilon must be finite and positive.
pub fn layer_norm(
    x: &Tensor,
    weight: &Tensor,
    bias: &Tensor,
    epsilon: f32,
) -> Result<Tensor, Error> {
    let width = *x.shape().last().ok_or(Error::Shape)?;
    if width == 0 || weight.shape() != [width] || bias.shape() != [width] {
        return Err(Error::Shape);
    }
    if !epsilon.is_finite() {
        return Err(Error::NonFinite);
    }
    if epsilon <= 0.0 {
        return Err(Error::Shape);
    }
    let mut data = buffer(x.data().len())?;
    for row in x.data().chunks_exact(width) {
        let mean = row.iter().map(|&v| f64::from(v)).sum::<f64>() / width as f64;
        let variance = row
            .iter()
            .map(|&v| (f64::from(v) - mean).powi(2))
            .sum::<f64>()
            / width as f64;
        let inv = 1.0 / (variance + f64::from(epsilon)).sqrt();
        data.extend(row.iter().zip(weight.data()).zip(bias.data()).map(
            |((&value, &gain), &shift)| {
                ((f64::from(value) - mean) * inv * f64::from(gain) + f64::from(shift)) as f32
            },
        ));
    }
    Tensor::new(x.shape(), data)
}

pub fn residual_norm(
    x: &Tensor,
    branch: &Tensor,
    weight: &Tensor,
    bias: &Tensor,
    epsilon: f32,
) -> Result<Tensor, Error> {
    if x.shape() != branch.shape() {
        return Err(Error::Shape);
    }
    layer_norm(&x.binary(branch, Binary::Add)?, weight, bias, epsilon)
}

/// Single-sequence multi-head self/cross attention. Heads execute serially,
/// reusing the selected matrix backend; probabilities are not retained.
pub struct MultiHead<'a> {
    pub q: Linear<'a>,
    pub k: Linear<'a>,
    pub v: Linear<'a>,
    pub output: Linear<'a>,
    pub heads: usize,
}

impl MultiHead<'_> {
    pub fn forward(
        &self,
        query: &Tensor,
        memory: &Tensor,
        mask: Mask<'_>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let &[queries, model] = query.shape() else {
            return Err(Error::Shape);
        };
        let &[keys, memory_width] = memory.shape() else {
            return Err(Error::Shape);
        };
        if model == 0
            || keys == 0
            || memory_width != model
            || self.heads == 0
            || self.heads > 128
            || !model.is_multiple_of(self.heads)
        {
            return Err(Error::Shape);
        }
        if [self.q, self.k, self.v, self.output]
            .iter()
            .any(|l| l.weight.shape() != [model, model])
        {
            return Err(Error::Shape);
        }
        layout(&[queries, keys])?;
        let work = queries
            .checked_mul(keys)
            .and_then(|v| v.checked_mul(model))
            .ok_or(Error::Limit)?;
        if work > 1 << 30 {
            return Err(Error::Limit);
        }
        let q = self.q.forward(query, kernel)?;
        let k = self.k.forward(memory, kernel)?;
        let v = self.v.forward(memory, kernel)?;
        let width = model / self.heads;
        let mut joined = buffer(q.data().len())?;
        joined.resize(q.data().len(), 0.0);
        for head in 0..self.heads {
            let start = head * width;
            let qh = head_slice(&q, start, width)?;
            let kh = head_slice(&k, start, width)?;
            let vh = head_slice(&v, start, width)?;
            let result = scaled_dot_product(&qh, &kh, &vh, mask, kernel)?;
            for (row, part) in joined
                .chunks_exact_mut(model)
                .zip(result.context.data().chunks_exact(width))
            {
                row[start..start + width].copy_from_slice(part);
            }
        }
        self.output
            .forward(&Tensor::new(&[queries, model], joined)?, kernel)
    }
}

fn head_slice(x: &Tensor, start: usize, width: usize) -> Result<Tensor, Error> {
    let rows = x.shape()[0];
    let model = x.shape()[1];
    let mut data = buffer(rows * width)?;
    for row in x.data().chunks_exact(model) {
        data.extend_from_slice(&row[start..start + width]);
    }
    Tensor::new(&[rows, width], data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(shape: &[usize], values: &[f32]) -> Tensor {
        Tensor::new(shape, values.to_vec()).unwrap()
    }

    fn close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (&a, &b) in actual.iter().zip(expected) {
            assert!((a - b).abs() <= 3e-6 + 3e-6 * b.abs(), "{a} != {b}");
        }
    }

    #[test]
    fn linear_checkpoint_layout() {
        let x = t(&[2, 3], &[1., 2., 3., 4., 5., 6.]);
        let w = t(&[2, 3], &[7., 9., 11., 8., 10., 12.]);
        let b = t(&[2], &[1., -2.]);
        let linear = Linear {
            weight: &w,
            bias: Some(&b),
        };
        let y = linear.forward(&x, Kernel::Auto).unwrap();
        assert_eq!(y.data(), &[59., 62., 140., 152.]);
        assert_eq!(
            linear.forward(&w.permute(&[1, 0]).unwrap(), Kernel::Auto),
            Err(Error::Shape)
        );
    }

    #[test]
    fn numpy_cross_attention_and_residual_norm() {
        let x = t(&[2, 4], &[1., 2., 3., 4., 2., 0., 1., -1.]);
        let memory = t(&[3, 4], &[1., 0., 0., 1., 0., 1., 1., 0., 1., 1., 2., -1.]);
        let wq = t(
            &[4, 4],
            &[
                1., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.5,
            ],
        );
        let wk = t(
            &[4, 4],
            &[
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
        );
        let wv = t(
            &[4, 4],
            &[
                2., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.5, 0., 0., 0., 0., 1.,
            ],
        );
        let wo = t(
            &[4, 4],
            &[
                1., 0., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0., 0., 1.,
            ],
        );
        let bq = t(&[4], &[0.5, 0., 0., 0.]);
        let bo = t(&[4], &[0.1, -0.2, 0.3, 0.]);
        let gain = t(&[4], &[1., 2., 0.5, 1.]);
        let shift = t(&[4], &[0., 0.1, 0., -0.1]);
        let mut layer = MultiHead {
            q: Linear {
                weight: &wq,
                bias: Some(&bq),
            },
            k: Linear {
                weight: &wk,
                bias: None,
            },
            v: Linear {
                weight: &wv,
                bias: None,
            },
            output: Linear {
                weight: &wo,
                bias: Some(&bo),
            },
            heads: 2,
        };
        let mask = Mask {
            key_keep: Some(&[true, true, false]),
            ..Mask::default()
        };
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let y = layer.forward(&x, &memory, mask, kernel).unwrap();
            close(
                y.data(),
                &[
                    0.726_521_25,
                    0.984_418_2,
                    0.591_640_5,
                    0.330_238_46,
                    2.179_767_8,
                    0.203_003_57,
                    2.008_359_4,
                    0.257_183_3,
                ],
            );
            let z = residual_norm(&x, &y, &gain, &shift, 1e-5).unwrap();
            close(
                z.data(),
                &[
                    -1.500_505_1,
                    -0.264_280_83,
                    0.227_135_69,
                    1.128_374_1,
                    1.256_346_5,
                    -1.356_179_5,
                    0.335_902_33,
                    -1.300_061_3,
                ],
            );
        }
        let all = Mask {
            key_keep: Some(&[false; 3]),
            ..Mask::default()
        };
        let y = layer.forward(&x, &memory, all, Kernel::Auto).unwrap();
        assert_eq!(y.data(), &[0.1, -0.2, 0.3, 0., 0.1, -0.2, 0.3, 0.]);
        layer.heads = 3;
        assert_eq!(
            layer.forward(&x, &memory, mask, Kernel::Auto),
            Err(Error::Shape)
        );
    }

    #[test]
    fn norm_edges() {
        let x = t(&[2, 2], &[3., 3., f32::MAX, -f32::MAX]);
        let gain = t(&[2], &[1., 1.]);
        let shift = t(&[2], &[0., 0.]);
        close(
            layer_norm(&x, &gain, &shift, 1e-5).unwrap().data(),
            &[0., 0., 1., -1.],
        );
        assert_eq!(layer_norm(&x, &gain, &shift, 0.), Err(Error::Shape));
        assert_eq!(
            layer_norm(&x, &gain, &shift, f32::NAN),
            Err(Error::NonFinite)
        );
        assert_eq!(
            residual_norm(&x, &gain, &gain, &shift, 1e-5),
            Err(Error::Shape)
        );
        let empty = Tensor::zeros(&[0, 2]).unwrap();
        assert_eq!(
            layer_norm(&empty, &gain, &shift, 1e-5).unwrap().shape(),
            &[0, 2]
        );
    }
}
