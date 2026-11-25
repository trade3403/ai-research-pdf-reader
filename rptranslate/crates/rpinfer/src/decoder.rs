//! Inference-only post-norm Transformer decoder; full-prefix recomputation.
use crate::{
    attention::Mask,
    encoder::Norm,
    layers::MultiHead,
    matmul::Kernel,
    tensor::{Error, Tensor},
    transformer::{input_embeddings, FeedForward},
    vocabulary::RowTable,
};

#[derive(Clone, Copy, Default)]
pub struct DecoderMask<'a> {
    pub target_keep: Option<&'a [bool]>,
    pub source_keep: Option<&'a [bool]>,
}

pub struct DecoderLayer<'a> {
    pub self_attention: MultiHead<'a>,
    pub self_norm: Norm<'a>,
    pub cross_attention: MultiHead<'a>,
    pub cross_norm: Norm<'a>,
    pub ffn: FeedForward<'a>,
    pub final_norm: Norm<'a>,
}

impl DecoderLayer<'_> {
    pub fn self_stage(
        &self,
        x: &Tensor,
        target_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let mask = Mask {
            key_keep: target_keep,
            causal_offset: Some(0),
            ..Mask::default()
        };
        let branch = self.self_attention.forward(x, x, mask, kernel)?;
        self.self_norm.residual(x, &branch)
    }

    pub fn cross_stage(
        &self,
        x: &Tensor,
        memory: &Tensor,
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let mask = Mask {
            key_keep: source_keep,
            ..Mask::default()
        };
        let branch = self.cross_attention.forward(x, memory, mask, kernel)?;
        self.cross_norm.residual(x, &branch)
    }

    pub fn forward(
        &self,
        x: &Tensor,
        memory: &Tensor,
        masks: DecoderMask<'_>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let x = self.self_stage(x, masks.target_keep, kernel)?;
        let x = self.cross_stage(&x, memory, masks.source_keep, kernel)?;
        let branch = self.ffn.forward(&x, kernel)?;
        self.final_norm.residual(&x, &branch)
    }
}

/// Full target prefix, no cache. Source and target masks are independent.
pub fn decode_hidden(
    x: &Tensor,
    memory: &Tensor,
    layers: &[DecoderLayer<'_>],
    masks: DecoderMask<'_>,
    kernel: Kernel,
) -> Result<Tensor, Error> {
    let &[target, hidden] = x.shape() else {
        return Err(Error::Shape);
    };
    let &[source, memory_width] = memory.shape() else {
        return Err(Error::Shape);
    };
    if target == 0 || source == 0 || hidden == 0 || memory_width != hidden || layers.is_empty() {
        return Err(Error::Shape);
    }
    if layers.len() > 64 {
        return Err(Error::Limit);
    }
    if masks.target_keep.is_some_and(|m| m.len() != target)
        || masks.source_keep.is_some_and(|m| m.len() != source)
    {
        return Err(Error::Shape);
    }
    let mut iter = layers.iter();
    let first = iter.next().ok_or(Error::Shape)?;
    let mut state = first.forward(x, memory, masks, kernel)?;
    for layer in iter {
        state = layer.forward(&state, memory, masks, kernel)?;
    }
    Ok(state)
}

pub struct Decoder<'a> {
    pub embeddings: &'a dyn RowTable,
    pub embedding_scale: f32,
    pub max_positions: usize,
    pub layers: &'a [DecoderLayer<'a>],
}

impl Decoder<'_> {
    /// Caller supplies target keep flags explicitly: the Marian start token
    /// may share the PAD ID and must not be automatically hidden.
    pub fn forward(
        &self,
        ids: &[u32],
        memory: &Tensor,
        masks: DecoderMask<'_>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let x = input_embeddings(
            self.embeddings,
            ids,
            self.embedding_scale,
            0,
            self.max_positions,
        )?;
        decode_hidden(&x, memory, self.layers, masks, kernel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layers::Linear, transformer::Activation};

    fn t(shape: &[usize], data: &[f32]) -> Tensor {
        Tensor::new(shape, data.to_vec()).unwrap()
    }

    fn close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (&a, &b) in actual.iter().zip(expected) {
            assert!((a - b).abs() <= 1e-5 + 1e-5 * b.abs(), "{a} != {b}");
        }
    }

    #[test]
    fn reference_and_causal_isolation() {
        let x = t(&[3, 4], &[1., 2., 3., 4., 2., 0., 1., -1., 3., -2., 1., 2.]);
        let memory = t(&[3, 4], &[1., 0., 2., 1., 0., 1., 1., 0., 9., 8., 7., 6.]);
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
        let linear = |weight| Linear { weight, bias: None };
        let attention = || MultiHead {
            q: linear(&q),
            k: linear(&k),
            v: linear(&v),
            output: linear(&o),
            heads: 2,
        };
        let make = || DecoderLayer {
            self_attention: attention(),
            self_norm: norm,
            cross_attention: attention(),
            cross_norm: norm,
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
        let masks = DecoderMask {
            source_keep: Some(&[true, true, false]),
            target_keep: None,
        };
        let changed = t(&[3, 4], &[1., 2., 3., 4., 2., 0., 1., -1., -9., 3., 2., 1.]);
        let changed_memory = t(&[3, 4], &[1., 0., 2., 1., 0., 1., 1., 0., -2., 3., 5., 7.]);
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let a = layers[0].self_stage(&x, None, kernel).unwrap();
            close(
                a.data(),
                &[
                    -1.147_077, 1.5912, -0.420_595, 0.894_133, 1.320_254, -0.906_716, 0.284_351,
                    -1.317_812, 1.218_329, -2.230_805, 0.158_773, -0.082_005,
                ],
            );
            let c = layers[0]
                .cross_stage(&a, &memory, masks.source_keep, kernel)
                .unwrap();
            close(
                c.data(),
                &[
                    -0.899_432, 2.348_565, -0.458_955, 0.218_298, 1.400_256, -1.111_518, 0.240_982,
                    -1.174_541, 1.316_649, -1.987_107, 0.221_058, -0.46736,
                ],
            );
            let first = layers[0].forward(&x, &memory, masks, kernel).unwrap();
            close(
                first.data(),
                &[
                    -1.559_385, -0.161_838, 0.36785, 0.898_244, 1.533_797, -1.248_498, 0.123_652,
                    -0.982_101, 1.483_771, -1.682_426, 0.133_113, -0.661_713,
                ],
            );
            let out = decode_hidden(&x, &memory, &layers, masks, kernel).unwrap();
            close(
                out.data(),
                &[
                    -1.414_739, -0.278_608, 0.155_766, 1.255_613, 1.50173, -1.518_659, 0.14209,
                    -0.806_804, 1.492_554, -1.575_968, 0.144_277, -0.763_796,
                ],
            );
            let altered = decode_hidden(&changed, &memory, &layers, masks, kernel).unwrap();
            close(&out.data()[..8], &altered.data()[..8]);
            let altered = decode_hidden(&x, &changed_memory, &layers, masks, kernel).unwrap();
            close(out.data(), altered.data());
            let prefix = t(&[2, 4], &x.data()[..8]);
            let short = decode_hidden(&prefix, &memory, &layers, masks, kernel).unwrap();
            close(&out.data()[..8], short.data());
        }
        let target_mask = DecoderMask {
            target_keep: Some(&[true, false, true]),
            ..masks
        };
        let changed_middle = t(&[3, 4], &[1., 2., 3., 4., 8., 9., 1., 3., 3., -2., 1., 2.]);
        let a = decode_hidden(&x, &memory, &layers, target_mask, Kernel::Auto).unwrap();
        let b =
            decode_hidden(&changed_middle, &memory, &layers, target_mask, Kernel::Auto).unwrap();
        close(&a.data()[8..], &b.data()[8..]);
        let invalid = DecoderMask {
            target_keep: Some(&[true]),
            ..masks
        };
        assert_eq!(
            decode_hidden(&x, &memory, &layers, invalid, Kernel::Auto),
            Err(Error::Shape)
        );
        assert_eq!(
            decode_hidden(&x, &memory, &[], masks, Kernel::Auto),
            Err(Error::Shape)
        );
        let decoder = Decoder {
            embeddings: &x,
            embedding_scale: 1.,
            max_positions: 4,
            layers: &layers,
        };
        let ids = [0, 1, 2];
        let hidden = input_embeddings(&x, &ids, 1., 0, 4).unwrap();
        let expected = decode_hidden(&hidden, &memory, &layers, masks, Kernel::Auto).unwrap();
        assert_eq!(
            decoder.forward(&ids, &memory, masks, Kernel::Auto),
            Ok(expected)
        );
        assert_eq!(
            decoder.forward(&[3], &memory, masks, Kernel::Auto),
            Err(Error::Index)
        );
        assert_eq!(
            decoder.forward(&[], &memory, masks, Kernel::Auto),
            Err(Error::Shape)
        );
    }
}
