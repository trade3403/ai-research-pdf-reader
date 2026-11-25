//! Owned actual Marian encoder weights, borrowing shared embeddings for inference.
use crate::{
    encoder::{encode_hidden, EncoderLayer, Norm},
    layers::{Linear, MultiHead},
    marian_weights::{read_dense, MarianEmbeddings, Side},
    matmul::Kernel,
    tensor::{Error, Tensor},
    transformer::{Activation, FeedForward},
    vocabulary::LoadError,
};
use rpmodel::{config::MarianConfig, safetensors::SafeTensorReader};
use std::io::{Read, Seek};

pub const ENCODER_PAYLOAD_BYTES: usize = 75_657_216;
const HIDDEN: usize = 512;
const INNER: usize = 2048;
const HEADS: usize = 8;
const LAYERS: usize = 6;

pub(crate) struct OwnedLinear {
    weight: Tensor,
    bias: Tensor,
}

impl OwnedLinear {
    pub(crate) fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
        output: usize,
        input: usize,
    ) -> Result<Self, LoadError> {
        Ok(Self {
            weight: read_dense(reader, &format!("{name}.weight"), &[output, input])?,
            bias: read_dense(reader, &format!("{name}.bias"), &[output])?,
        })
    }

    pub(crate) fn view(&self) -> Linear<'_> {
        Linear {
            weight: &self.weight,
            bias: Some(&self.bias),
        }
    }
}

pub(crate) struct OwnedNorm {
    weight: Tensor,
    bias: Tensor,
}

impl OwnedNorm {
    pub(crate) fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
    ) -> Result<Self, LoadError> {
        Ok(Self {
            weight: read_dense(reader, &format!("{name}.weight"), &[HIDDEN])?,
            bias: read_dense(reader, &format!("{name}.bias"), &[HIDDEN])?,
        })
    }

    pub(crate) fn view(&self) -> Norm<'_> {
        Norm {
            weight: &self.weight,
            bias: &self.bias,
            epsilon: 1e-5,
        }
    }
}

pub(crate) struct OwnedAttention {
    q: OwnedLinear,
    k: OwnedLinear,
    v: OwnedLinear,
    output: OwnedLinear,
}

impl OwnedAttention {
    pub(crate) fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
    ) -> Result<Self, LoadError> {
        Ok(Self {
            q: OwnedLinear::load(reader, &format!("{name}.q_proj"), HIDDEN, HIDDEN)?,
            k: OwnedLinear::load(reader, &format!("{name}.k_proj"), HIDDEN, HIDDEN)?,
            v: OwnedLinear::load(reader, &format!("{name}.v_proj"), HIDDEN, HIDDEN)?,
            output: OwnedLinear::load(reader, &format!("{name}.out_proj"), HIDDEN, HIDDEN)?,
        })
    }

    pub(crate) fn view(&self) -> MultiHead<'_> {
        MultiHead {
            q: self.q.view(),
            k: self.k.view(),
            v: self.v.view(),
            output: self.output.view(),
            heads: HEADS,
        }
    }
}

pub(crate) struct OwnedFfn {
    first: OwnedLinear,
    second: OwnedLinear,
}

impl OwnedFfn {
    pub(crate) fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
    ) -> Result<Self, LoadError> {
        Ok(Self {
            first: OwnedLinear::load(reader, &format!("{name}.fc1"), INNER, HIDDEN)?,
            second: OwnedLinear::load(reader, &format!("{name}.fc2"), HIDDEN, INNER)?,
        })
    }

    pub(crate) fn view(&self) -> FeedForward<'_> {
        FeedForward {
            first: self.first.view(),
            second: self.second.view(),
            activation: Activation::Silu,
        }
    }
}

struct OwnedLayer {
    attention: OwnedAttention,
    attention_norm: OwnedNorm,
    ffn: OwnedFfn,
    final_norm: OwnedNorm,
}

impl OwnedLayer {
    fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        index: usize,
    ) -> Result<Self, LoadError> {
        let name = format!("model.encoder.layers.{index}");
        Ok(Self {
            attention: OwnedAttention::load(reader, &format!("{name}.self_attn"))?,
            attention_norm: OwnedNorm::load(reader, &format!("{name}.self_attn_layer_norm"))?,
            ffn: OwnedFfn::load(reader, &name)?,
            final_norm: OwnedNorm::load(reader, &format!("{name}.final_layer_norm"))?,
        })
    }

    fn view(&self) -> EncoderLayer<'_> {
        EncoderLayer {
            attention: self.attention.view(),
            attention_norm: self.attention_norm.view(),
            ffn: self.ffn.view(),
            final_norm: self.final_norm.view(),
        }
    }
}

/// Owns exactly the 96 fixed-profile encoder layer tensors. Shared embeddings,
/// positions and output bias are owned separately by MarianEmbeddings.
pub struct MarianEncoder {
    layers: Vec<OwnedLayer>,
}

impl MarianEncoder {
    pub fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        config: &MarianConfig,
        max_bytes: usize,
    ) -> Result<Self, LoadError> {
        if (
            config.hidden,
            config.encoder_layers,
            config.encoder_heads,
            config.encoder_ffn,
        ) != (HIDDEN, LAYERS, HEADS, INNER)
        {
            return Err(Error::Shape.into());
        }
        if max_bytes < ENCODER_PAYLOAD_BYTES {
            return Err(Error::Limit.into());
        }
        let mut layers = Vec::new();
        layers
            .try_reserve_exact(LAYERS)
            .map_err(|_| Error::Allocation)?;
        for index in 0..LAYERS {
            layers.push(OwnedLayer::load(reader, index)?);
        }
        Ok(Self { layers })
    }

    pub fn layers(&self) -> Vec<EncoderLayer<'_>> {
        self.layers.iter().map(OwnedLayer::view).collect()
    }

    pub fn payload_bytes(&self) -> usize {
        ENCODER_PAYLOAD_BYTES
    }

    /// Use actual stored source positions and the previously loaded shared table.
    pub fn forward(
        &self,
        embeddings: &MarianEmbeddings,
        ids: &[u32],
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        if ids.is_empty()
            || source_keep.is_some_and(|mask| mask.len() != ids.len() || !mask.iter().any(|&v| v))
        {
            return Err(Error::Shape);
        }
        let input = embeddings.input(Side::Encoder, ids, 0)?;
        encode_hidden(&input, &self.layers(), source_keep, kernel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_budget_matches_audited_inventory() {
        let shapes = rpmodel::package::expected_shapes();
        let layers: Vec<_> = shapes
            .iter()
            .filter(|(name, _)| name.starts_with("model.encoder.layers."))
            .collect();
        assert_eq!(layers.len(), 96);
        assert_eq!(
            layers
                .iter()
                .map(|(_, shape)| shape.iter().product::<u64>() * 4)
                .sum::<u64>(),
            ENCODER_PAYLOAD_BYTES as u64
        );
    }
}
