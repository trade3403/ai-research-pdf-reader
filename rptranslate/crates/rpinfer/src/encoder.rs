//! Inference-only, post-norm Transformer encoder.
use crate::{
    attention::Mask,
    layers::{residual_norm, MultiHead},
    matmul::Kernel,
    tensor::{Error, Tensor},
    transformer::{input_embeddings, FeedForward},
};

#[derive(Clone, Copy)]
pub struct Norm<'a> {
    pub weight: &'a Tensor,
    pub bias: &'a Tensor,
    pub epsilon: f32,
}

impl Norm<'_> {
    pub fn residual(self, x: &Tensor, branch: &Tensor) -> Result<Tensor, Error> {
        residual_norm(x, branch, self.weight, self.bias, self.epsilon)
    }
}

pub struct EncoderLayer<'a> {
    pub attention: MultiHead<'a>,
    pub attention_norm: Norm<'a>,
    pub ffn: FeedForward<'a>,
    pub final_norm: Norm<'a>,
}

impl EncoderLayer<'_> {
    /// Exposes the first post-norm state for layer-by-layer validation.
    pub fn attention_stage(
        &self,
        x: &Tensor,
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let mask = Mask {
            key_keep: source_keep,
            ..Mask::default()
        };
        let attended = self.attention.forward(x, x, mask, kernel)?;
        self.attention_norm.residual(x, &attended)
    }

    pub fn forward(
        &self,
        x: &Tensor,
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let x = self.attention_stage(x, source_keep, kernel)?;
        let ff = self.ffn.forward(&x, kernel)?;
        self.final_norm.residual(&x, &ff)
    }
}

/// Execute 1..=64 layers in order. No hidden-state accumulation or dropout.
pub fn encode_hidden(
    x: &Tensor,
    layers: &[EncoderLayer<'_>],
    source_keep: Option<&[bool]>,
    kernel: Kernel,
) -> Result<Tensor, Error> {
    let &[length, hidden] = x.shape() else {
        return Err(Error::Shape);
    };
    if length == 0 || hidden == 0 || layers.is_empty() {
        return Err(Error::Shape);
    }
    if layers.len() > 64 {
        return Err(Error::Limit);
    }
    if source_keep.is_some_and(|m| m.len() != length) {
        return Err(Error::Shape);
    }
    let mut iter = layers.iter();
    let first = iter.next().ok_or(Error::Shape)?;
    let mut state = first.forward(x, source_keep, kernel)?;
    for layer in iter {
        state = layer.forward(&state, source_keep, kernel)?;
    }
    Ok(state)
}

pub struct Encoder<'a> {
    pub embeddings: &'a Tensor,
    pub embedding_scale: f32,
    pub max_positions: usize,
    pub layers: &'a [EncoderLayer<'a>],
}

impl Encoder<'_> {
    /// Source padding masks hide key positions, not query rows.
    pub fn forward(
        &self,
        ids: &[u32],
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let x = input_embeddings(
            self.embeddings,
            ids,
            self.embedding_scale,
            0,
            self.max_positions,
        )?;
        encode_hidden(&x, self.layers, source_keep, kernel)
    }
}

/// Construct an explicit keep mask from IDs; callers may provide custom masks.
pub fn padding_mask(ids: &[u32], pad_id: u32) -> Result<Vec<bool>, Error> {
    let mut mask = Vec::new();
    mask.try_reserve_exact(ids.len())
        .map_err(|_| Error::Allocation)?;
    mask.extend(ids.iter().map(|&id| id != pad_id));
    Ok(mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layers::Linear, transformer::Activation};

    fn t(shape: &[usize], data: &[f32]) -> Tensor {
        Tensor::new(shape, data.to_vec()).unwrap()
    }

    fn close(a: &[f32], b: &[f32]) {
        assert_eq!(a.len(), b.len());
        for (&x, &y) in a.iter().zip(b) {
            assert!((x - y).abs() <= 1e-5 + 1e-5 * y.abs(), "{x} != {y}");
        }
    }

    #[test]
    fn two_layer_numpy_reference_and_mask_isolation() {
        let x = t(&[3, 4], &[1., 2., 3., 4., 2., 0., 1., -1., 9., 8., 7., 6.]);
        let changed = t(&[3, 4], &[1., 2., 3., 4., 2., 0., 1., -1., -9., 3., 2., 1.]);
        let q = t(
            &[4, 4],
            &[
                1., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.5,
            ],
        );
        let k = t(
            &[4, 4],
            &[
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
        );
        let v = t(
            &[4, 4],
            &[
                2., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.5, 0., 0., 0., 0., 1.,
            ],
        );
        let o = t(
            &[4, 4],
            &[
                1., 0., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0., 0., 1.,
            ],
        );
        let w1 = t(&[3, 4], &[1., -1., 0., 0., 0., 1., 1., 0., 0., 0., -1., 1.]);
        let w2 = t(
            &[4, 3],
            &[1., 0., 0.5, 0., -1., 0., 0.5, 1., 0., 0., 0., 1.],
        );
        let b1 = t(&[3], &[0.2, -0.1, 0.3]);
        let b2 = t(&[4], &[0.1, 0., -0.2, 0.1]);
        let gain = t(&[4], &[1., 1.5, 0.5, 1.]);
        let bias = t(&[4], &[0., 0.1, 0., -0.1]);
        let norm = Norm {
            weight: &gain,
            bias: &bias,
            epsilon: 1e-5,
        };
        let make = || EncoderLayer {
            attention: MultiHead {
                q: Linear {
                    weight: &q,
                    bias: None,
                },
                k: Linear {
                    weight: &k,
                    bias: None,
                },
                v: Linear {
                    weight: &v,
                    bias: None,
                },
                output: Linear {
                    weight: &o,
                    bias: None,
                },
                heads: 2,
            },
            attention_norm: norm,
            ffn: FeedForward {
                first: Linear {
                    weight: &w1,
                    bias: Some(&b1),
                },
                second: Linear {
                    weight: &w2,
                    bias: Some(&b2),
                },
                activation: Activation::Relu,
            },
            final_norm: norm,
        };
        let layers = [make(), make()];
        let mask = [true, true, false];
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let a = layers[0].attention_stage(&x, Some(&mask), kernel).unwrap();
            close(
                a.data(),
                &[
                    -1.147_988_6,
                    1.584_579_5,
                    -0.420_057_27,
                    0.898_383_44,
                    1.320_254_1,
                    -0.906_716,
                    0.284_351_08,
                    -1.317_812_2,
                    0.568_074_35,
                    2.088_259_7,
                    -0.599_633_93,
                    -0.794_313,
                ],
            );
            let first = layers[0].forward(&x, Some(&mask), kernel).unwrap();
            close(
                first.data(),
                &[
                    -1.004_121_9,
                    -0.342_411_1,
                    -0.182_793_78,
                    1.564_650_2,
                    1.521_986_2,
                    -1.126_990_7,
                    0.133_564_41,
                    -1.071_121_2,
                    0.668_001_3,
                    1.044_177_4,
                    0.213_658_57,
                    -1.82477,
                ],
            );
            let out = encode_hidden(&x, &layers, Some(&mask), kernel).unwrap();
            close(
                out.data(),
                &[
                    -0.535_994_95,
                    -0.349_807_08,
                    -0.430_704_65,
                    1.597_275_7,
                    1.475_076_1,
                    -1.538_538_6,
                    0.168_342_28,
                    -0.819_401_56,
                    1.376_005_8,
                    -0.535_743_6,
                    0.191_146_69,
                    -1.434_470_1,
                ],
            );
            let altered = encode_hidden(&changed, &layers, Some(&mask), kernel).unwrap();
            close(&out.data()[..8], &altered.data()[..8]);
        }
        assert_eq!(
            encode_hidden(&x, &layers, Some(&[true]), Kernel::Auto),
            Err(Error::Shape)
        );
        assert_eq!(
            encode_hidden(&x, &[], None, Kernel::Auto),
            Err(Error::Shape)
        );
        let encoder = Encoder {
            embeddings: &x,
            embedding_scale: 1.,
            max_positions: 4,
            layers: &layers,
        };
        let ids = [0, 1, 2];
        let keep = padding_mask(&ids, 2).unwrap();
        assert_eq!(keep, mask);
        let hidden = input_embeddings(&x, &ids, 1., 0, 4).unwrap();
        let expected = encode_hidden(&hidden, &layers, Some(&keep), Kernel::Auto).unwrap();
        assert_eq!(
            encoder.forward(&ids, Some(&keep), Kernel::Auto),
            Ok(expected)
        );
        assert_eq!(encoder.forward(&[3], None, Kernel::Auto), Err(Error::Index));
        assert_eq!(encoder.forward(&[], None, Kernel::Auto), Err(Error::Shape));
    }
}
