//! Transformer input embeddings and inference-only feed-forward layers.
use crate::{
    layers::Linear,
    matmul::Kernel,
    tensor::{buffer, layout, Binary, Error, Tensor},
};

/// Gather rows from [vocab, hidden] with explicit embedding scaling.
/// Padding is not silently zeroed: the checkpoint's actual row is used.
pub fn embedding(table: &Tensor, ids: &[u32], scale: f32) -> Result<Tensor, Error> {
    let &[vocab, hidden] = table.shape() else {
        return Err(Error::Shape);
    };
    if vocab == 0 || hidden == 0 {
        return Err(Error::Shape);
    }
    if !scale.is_finite() {
        return Err(Error::NonFinite);
    }
    let (_, size) = layout(&[ids.len(), hidden])?;
    let mut data = buffer(size)?;
    for &id in ids {
        let id = usize::try_from(id).map_err(|_| Error::Index)?;
        if id >= vocab {
            return Err(Error::Index);
        }
        data.extend(
            table.data()[id * hidden..(id + 1) * hidden]
                .iter()
                .map(|&x| x * scale),
        );
    }
    Tensor::new(&[ids.len(), hidden], data)
}

/// Marian uses grouped sin/cos halves, not interleaved sin/cos.
/// Odd widths put the extra sine channel in the first half.
pub fn sinusoidal(
    length: usize,
    hidden: usize,
    offset: usize,
    max_positions: usize,
) -> Result<Tensor, Error> {
    if hidden == 0 || max_positions == 0 || max_positions > 1_048_576 {
        return Err(Error::Shape);
    }
    let end = offset.checked_add(length).ok_or(Error::Limit)?;
    if end > max_positions {
        return Err(Error::Index);
    }
    let (_, size) = layout(&[length, hidden])?;
    let mut data = buffer(size)?;
    let split = hidden.div_ceil(2);
    let mut frequencies = Vec::new();
    frequencies
        .try_reserve_exact(split)
        .map_err(|_| Error::Allocation)?;
    for j in 0..split {
        frequencies.push(10000_f64.powf(-2.0 * j as f64 / hidden as f64));
    }
    for pos in offset..end {
        data.extend(frequencies.iter().map(|&f| (pos as f64 * f).sin() as f32));
        data.extend(
            frequencies[..hidden / 2]
                .iter()
                .map(|&f| (pos as f64 * f).cos() as f32),
        );
    }
    Tensor::new(&[length, hidden], data)
}

/// The caller selects the model's scale, e.g. sqrt(hidden) or 1.
pub fn input_embeddings(
    table: &Tensor,
    ids: &[u32],
    scale: f32,
    offset: usize,
    max_positions: usize,
) -> Result<Tensor, Error> {
    let tokens = embedding(table, ids, scale)?;
    let positions = sinusoidal(ids.len(), tokens.shape()[1], offset, max_positions)?;
    tokens.binary(&positions, Binary::Add)
}

#[derive(Clone, Copy)]
pub enum Activation {
    Relu,
    /// Tanh-approximate GELU (not the exact erf-based GELU).
    GeluTanh,
    Silu,
}

pub fn activate(x: &Tensor, kind: Activation) -> Result<Tensor, Error> {
    let mut data = buffer(x.data().len())?;
    data.extend(x.data().iter().map(|&x| {
        let v = f64::from(x);
        match kind {
            Activation::Relu => x.max(0.0),
            Activation::GeluTanh => {
                (0.5 * v * (1.0 + (0.797_884_560_802_865_4 * (v + 0.044715 * v.powi(3))).tanh()))
                    as f32
            }
            Activation::Silu => (v / (1.0 + (-v).exp())) as f32,
        }
    }));
    Tensor::new(x.shape(), data)
}

/// FFN: x -> affine -> activation -> affine; residual/norm are separate.
pub struct FeedForward<'a> {
    pub first: Linear<'a>,
    pub second: Linear<'a>,
    pub activation: Activation,
}

impl FeedForward<'_> {
    pub fn forward(&self, x: &Tensor, kernel: Kernel) -> Result<Tensor, Error> {
        let &[_, hidden] = x.shape() else {
            return Err(Error::Shape);
        };
        let &[inner, input] = self.first.weight.shape() else {
            return Err(Error::Shape);
        };
        if hidden == 0 || inner == 0 || input != hidden {
            return Err(Error::Shape);
        }
        if self.second.weight.shape() != [hidden, inner] {
            return Err(Error::Shape);
        }
        let intermediate = self.first.forward(x, kernel)?;
        self.second
            .forward(&activate(&intermediate, self.activation)?, kernel)
    }
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
            assert!((a - b).abs() <= 2e-6 + 2e-6 * b.abs(), "{a} != {b}");
        }
    }

    #[test]
    fn embedding_lookup_scale_and_bounds() {
        let table = t(&[3, 2], &[1., 2., 3., 4., 5., 6.]);
        let result = embedding(&table, &[2, 0, 2], 2.).unwrap();
        assert_eq!(result.shape(), &[3, 2]);
        assert_eq!(result.data(), &[10., 12., 2., 4., 10., 12.]);
        assert_eq!(embedding(&table, &[3], 1.), Err(Error::Index));
        assert_eq!(embedding(&table, &[0], f32::NAN), Err(Error::NonFinite));
        assert_eq!(embedding(&table, &[], 1.).unwrap().shape(), &[0, 2]);
    }

    #[test]
    fn marian_position_reference_and_offset() {
        let p = sinusoidal(3, 4, 0, 512).unwrap();
        close(
            p.data(),
            &[
                0.,
                0.,
                1.,
                1.,
                0.841_470_96,
                0.009_999_833,
                0.540_302_3,
                0.99995,
                0.909_297_4,
                0.019_998_666,
                -0.416_146_84,
                0.9998,
            ],
        );
        let tail = sinusoidal(2, 4, 1, 512).unwrap();
        assert_eq!(tail.data(), &p.data()[4..]);
        let odd = sinusoidal(1, 3, 0, 512).unwrap();
        assert_eq!(odd.data(), &[0., 0., 1.]);
        assert_eq!(sinusoidal(2, 4, 511, 512), Err(Error::Index));
        assert_eq!(sinusoidal(2, 4, usize::MAX, 512), Err(Error::Limit));
        assert_eq!(sinusoidal(1, 0, 0, 512), Err(Error::Shape));
        let table = t(&[1, 4], &[1., 2., 3., 4.]);
        let x = input_embeddings(&table, &[0], 2., 0, 512).unwrap();
        assert_eq!(x.data(), &[2., 4., 7., 9.]);
    }

    #[test]
    fn activation_reference() {
        let x = t(&[3], &[-1., 0., 1.]);
        assert_eq!(
            activate(&x, Activation::Relu).unwrap().data(),
            &[0., 0., 1.]
        );
        close(
            activate(&x, Activation::GeluTanh).unwrap().data(),
            &[-0.158_808, 0., 0.841_192],
        );
        close(
            activate(&x, Activation::Silu).unwrap().data(),
            &[-0.268_941_43, 0., 0.731_058_6],
        );
        let extreme = t(&[2], &[-f32::MAX, f32::MAX]);
        for kind in [Activation::Relu, Activation::GeluTanh, Activation::Silu] {
            assert_eq!(activate(&extreme, kind).unwrap().data(), &[0., f32::MAX]);
        }
    }

    #[test]
    fn numpy_ffn_reference() {
        let x = t(&[2, 2], &[1., -2., 3., 4.]);
        let w1 = t(&[3, 2], &[1., 2., -1., 1., 0.5, -0.5]);
        let b1 = t(&[3], &[0.5, -0.5, 1.]);
        let w2 = t(&[2, 3], &[1., -2., 0.5, 0., 1., 2.]);
        let b2 = t(&[2], &[0.25, -0.25]);
        let mut ffn = FeedForward {
            first: Linear {
                weight: &w1,
                bias: Some(&b1),
            },
            second: Linear {
                weight: &w2,
                bias: Some(&b2),
            },
            activation: Activation::Relu,
        };
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let y = ffn.forward(&x, kernel).unwrap();
            assert_eq!(y.shape(), &[2, 2]);
            assert_eq!(y.data(), &[1.5, 4.75, 11., 1.25]);
        }
        ffn.second.weight = &w1;
        assert_eq!(ffn.forward(&x, Kernel::Auto), Err(Error::Shape));
    }
}
