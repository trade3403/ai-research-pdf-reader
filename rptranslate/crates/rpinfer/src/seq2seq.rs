//! Joint encoder/decoder forward inference, no autoregressive search yet.
use crate::{
    attention::softmax,
    decoder::{Decoder, DecoderMask},
    encoder::Encoder,
    matmul::Kernel,
    tensor::{buffer, layout, Error, Tensor},
};

/// None ties the output weight to the decoder embedding table.
pub struct OutputHead<'a> {
    pub weight: Option<&'a Tensor>,
    pub bias: Option<&'a Tensor>,
}

impl OutputHead<'_> {
    /// Contiguous vocabulary rows: avoid transposing or duplicating the
    /// full embedding table for a single next-token projection.
    pub fn project(
        &self,
        hidden: &Tensor,
        decoder_embeddings: &Tensor,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        let &[rows, width] = hidden.shape() else {
            return Err(Error::Shape);
        };
        let weight = self.weight.unwrap_or(decoder_embeddings);
        let &[vocab, features] = weight.shape() else {
            return Err(Error::Shape);
        };
        if rows == 0
            || width == 0
            || vocab == 0
            || features != width
            || decoder_embeddings.shape() != [vocab, width]
            || self.bias.is_some_and(|b| b.shape() != [vocab])
        {
            return Err(Error::Shape);
        }
        let count = if last_only { 1 } else { rows };
        let (_, size) = layout(&[count, vocab])?;
        if size.checked_mul(width).ok_or(Error::Limit)? > 1 << 30 {
            return Err(Error::Limit);
        }
        let start = if last_only { (rows - 1) * width } else { 0 };
        let mut data = buffer(size)?;
        for row in hidden.data()[start..].chunks_exact(width) {
            for (id, w) in weight.data().chunks_exact(width).enumerate() {
                let mut sum = 0.0_f32;
                for (&x, &y) in row.iter().zip(w) {
                    sum += x * y;
                }
                if let Some(bias) = self.bias {
                    sum += bias.data()[id];
                }
                data.push(sum);
            }
        }
        Tensor::new(&[count, vocab], data)
    }
}

pub struct Seq2Seq<'a> {
    pub encoder: Encoder<'a>,
    pub decoder: Decoder<'a>,
    pub head: OutputHead<'a>,
}

/// Holds the source memory and a copied mask, bound to the same model.
/// Decoder prefixes may be evaluated repeatedly without re-encoding source.
pub struct Encoded<'m, 'a> {
    model: &'m Seq2Seq<'a>,
    memory: Tensor,
    source_keep: Option<Vec<bool>>,
    kernel: Kernel,
}

impl<'a> Seq2Seq<'a> {
    pub fn encode<'m>(
        &'m self,
        source: &[u32],
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Encoded<'m, 'a>, Error> {
        let memory = self.encoder.forward(source, source_keep, kernel)?;
        let source_keep = if let Some(keep) = source_keep {
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(keep.len())
                .map_err(|_| Error::Allocation)?;
            owned.extend_from_slice(keep);
            Some(owned)
        } else {
            None
        };
        Ok(Encoded {
            model: self,
            memory,
            source_keep,
            kernel,
        })
    }
}

impl Encoded<'_, '_> {
    pub fn memory(&self) -> &Tensor {
        &self.memory
    }

    pub fn hidden(&self, prefix: &[u32], target_keep: Option<&[bool]>) -> Result<Tensor, Error> {
        let masks = DecoderMask {
            target_keep,
            source_keep: self.source_keep.as_deref(),
        };
        self.model
            .decoder
            .forward(prefix, &self.memory, masks, self.kernel)
    }

    /// Teacher-forced logits for all supplied target positions.
    pub fn logits(&self, prefix: &[u32], target_keep: Option<&[bool]>) -> Result<Tensor, Error> {
        let hidden = self.hidden(prefix, target_keep)?;
        self.model
            .head
            .project(&hidden, self.model.decoder.embeddings, false)
    }

    /// Project only the last decoder row, for search in the next stage.
    pub fn next_logits(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
    ) -> Result<Tensor, Error> {
        let hidden = self.hidden(prefix, target_keep)?;
        self.model
            .head
            .project(&hidden, self.model.decoder.embeddings, true)
    }

    pub fn next_probabilities(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
    ) -> Result<Tensor, Error> {
        softmax(&self.next_logits(prefix, target_keep)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoder::DecoderLayer,
        encoder::{EncoderLayer, Norm},
        layers::{Linear, MultiHead},
        transformer::{Activation, FeedForward},
    };

    fn t(shape: &[usize], values: &[f32]) -> Tensor {
        Tensor::new(shape, values.to_vec()).unwrap()
    }

    fn close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (&a, &b) in actual.iter().zip(expected) {
            assert!((a - b).abs() <= 1e-5 + 1e-5 * b.abs(), "{a} != {b}");
        }
    }

    #[test]
    fn joint_reference_and_reusable_source() {
        let table = t(
            &[5, 4],
            &[
                0., 1., 2., 3., 1., -1., 0.5, 2., 2., 1., 0., -1., -1., 2., 1., 0., 0.5, -0.5, 1.,
                -1.,
            ],
        );
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
        let shift = t(&[4], &[0., 0.1, 0., -0.1]);
        let bias = t(&[5], &[0.1, -0.2, 0., 0.3, -0.1]);
        let norm = Norm {
            weight: &gain,
            bias: &shift,
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
        let ffn = || FeedForward {
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
        let enc = [EncoderLayer {
            attention: attention(),
            attention_norm: norm,
            ffn: ffn(),
            final_norm: norm,
        }];
        let dec = [DecoderLayer {
            self_attention: attention(),
            self_norm: norm,
            cross_attention: attention(),
            cross_norm: norm,
            ffn: ffn(),
            final_norm: norm,
        }];
        let model = Seq2Seq {
            encoder: Encoder {
                embeddings: &table,
                embedding_scale: 2.,
                max_positions: 8,
                layers: &enc,
            },
            decoder: Decoder {
                embeddings: &table,
                embedding_scale: 2.,
                max_positions: 8,
                layers: &dec,
            },
            head: OutputHead {
                weight: None,
                bias: Some(&bias),
            },
        };
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let source = model
                .encode(&[1, 2, 4], Some(&[true, true, false]), kernel)
                .unwrap();
            close(
                source.memory().data(),
                &[
                    1.091_053, -2.259_698, -0.044_802, 0.471_684, 1.522_265, -0.952_558, 0.121_519,
                    -1.163_598, 1.453_135, -1.555_087, 0.188_643, -0.827_031,
                ],
            );
            let hidden = source.hidden(&[4, 3], None).unwrap();
            close(
                hidden.data(),
                &[
                    1.421_722, -1.724_493, 0.193_128, -0.691_650, 1.042_575, -0.644_021, 0.424_343,
                    -1.495_247,
                ],
            );
            let logits = source.logits(&[4, 3], None).unwrap();
            assert_eq!(logits.shape(), &[2, 5]);
            close(
                logits.data(),
                &[
                    -3.313_187, 1.659_478, 1.810_601, -4.377_579, 2.357_886, -4.181_075,
                    -1.291_726, 2.936_375, -1.606_273, 2.662_887,
                ],
            );
            let last = source.next_logits(&[4, 3], None).unwrap();
            assert_eq!(last.shape(), &[1, 5]);
            assert_eq!(last.data(), &logits.data()[5..]);
            close(
                source.next_probabilities(&[4, 3], None).unwrap().data(),
                &[0.000_454, 0.008_160, 0.559_673, 0.005_958, 0.425_755],
            );
            close(
                source.next_logits(&[4], None).unwrap().data(),
                &logits.data()[..5],
            );
            let altered = model
                .encode(&[1, 2, 0], Some(&[true, true, false]), kernel)
                .unwrap();
            close(
                altered.next_logits(&[4, 3], None).unwrap().data(),
                last.data(),
            );
            assert_eq!(source.next_logits(&[], None), Err(Error::Shape));
            assert_eq!(source.next_logits(&[5], None), Err(Error::Index));
            assert_eq!(
                source.next_logits(&[4, 3], Some(&[true])),
                Err(Error::Shape)
            );
        }
    }

    #[test]
    fn untied_head_and_bounds() {
        let table = t(&[2, 2], &[1., 2., 3., 4.]);
        let weight = t(&[2, 2], &[2., 0., 0., -1.]);
        let hidden = t(&[1, 2], &[2., 3.]);
        let bias = t(&[2], &[1., 1.]);
        let head = OutputHead {
            weight: Some(&weight),
            bias: Some(&bias),
        };
        assert_eq!(
            head.project(&hidden, &table, true).unwrap().data(),
            &[5., -2.]
        );
        let invalid = OutputHead {
            weight: Some(&hidden),
            bias: None,
        };
        assert_eq!(invalid.project(&hidden, &table, false), Err(Error::Shape));
        let huge = t(&[1, 2], &[f32::MAX, f32::MAX]);
        assert_eq!(head.project(&huge, &table, true), Err(Error::NonFinite));
    }
}
