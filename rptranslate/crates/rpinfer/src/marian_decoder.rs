//! Owned actual Marian decoder weights, full-prefix forward and tied logits.
use crate::{
    decoder::{decode_hidden, DecoderLayer, DecoderMask},
    marian_encoder::{OwnedAttention, OwnedFfn, OwnedNorm},
    marian_weights::{MarianEmbeddings, Side},
    matmul::Kernel,
    tensor::{Error, Tensor},
    vocabulary::LoadError,
};
use rpmodel::{config::MarianConfig, safetensors::SafeTensorReader};
use std::io::{Read, Seek};

pub const DECODER_PAYLOAD_BYTES: usize = 100_896_768;
const LAYERS: usize = 6;
const HIDDEN: usize = 512;
const MAX_POSITIONS: usize = 512;

struct OwnedLayer {
    self_attention: OwnedAttention,
    self_norm: OwnedNorm,
    cross_attention: OwnedAttention,
    cross_norm: OwnedNorm,
    ffn: OwnedFfn,
    final_norm: OwnedNorm,
}

impl OwnedLayer {
    fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        index: usize,
    ) -> Result<Self, LoadError> {
        let name = format!("model.decoder.layers.{index}");
        Ok(Self {
            self_attention: OwnedAttention::load(reader, &format!("{name}.self_attn"))?,
            self_norm: OwnedNorm::load(reader, &format!("{name}.self_attn_layer_norm"))?,
            cross_attention: OwnedAttention::load(reader, &format!("{name}.encoder_attn"))?,
            cross_norm: OwnedNorm::load(reader, &format!("{name}.encoder_attn_layer_norm"))?,
            ffn: OwnedFfn::load(reader, &name)?,
            final_norm: OwnedNorm::load(reader, &format!("{name}.final_layer_norm"))?,
        })
    }

    fn view(&self) -> DecoderLayer<'_> {
        DecoderLayer {
            self_attention: self.self_attention.view(),
            self_norm: self.self_norm.view(),
            cross_attention: self.cross_attention.view(),
            cross_norm: self.cross_norm.view(),
            ffn: self.ffn.view(),
            final_norm: self.final_norm.view(),
        }
    }
}

/// Owns the 156 decoder-layer tensors; the caller supplies shared embeddings.
pub struct MarianDecoder {
    layers: Vec<OwnedLayer>,
}

impl MarianDecoder {
    pub fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        config: &MarianConfig,
        max_bytes: usize,
    ) -> Result<Self, LoadError> {
        if (
            config.hidden,
            config.decoder_layers,
            config.decoder_heads,
            config.decoder_ffn,
            config.max_positions,
        ) != (HIDDEN, LAYERS, 8, 2048, MAX_POSITIONS)
        {
            return Err(Error::Shape.into());
        }
        if max_bytes < DECODER_PAYLOAD_BYTES {
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

    pub fn layers(&self) -> Vec<DecoderLayer<'_>> {
        self.layers.iter().map(OwnedLayer::view).collect()
    }

    pub fn payload_bytes(&self) -> usize {
        DECODER_PAYLOAD_BYTES
    }

    /// Prefix recomputation with checkpoint position weights. Explicit target
    /// keep flags must retain the first key (PAD may be the start token).
    pub fn forward(
        &self,
        embeddings: &MarianEmbeddings,
        ids: &[u32],
        memory: &Tensor,
        masks: DecoderMask<'_>,
        kernel: Kernel,
    ) -> Result<Tensor, Error> {
        let &[source, width] = memory.shape() else {
            return Err(Error::Shape);
        };
        if ids.is_empty()
            || source == 0
            || width != HIDDEN
            || masks
                .target_keep
                .is_some_and(|mask| mask.len() != ids.len() || !mask[0])
            || masks
                .source_keep
                .is_some_and(|mask| mask.len() != source || !mask.iter().any(|&v| v))
        {
            return Err(Error::Shape);
        }
        if source > MAX_POSITIONS || ids.len() > MAX_POSITIONS {
            return Err(Error::Limit);
        }
        let input = embeddings.input(Side::Decoder, ids, 0)?;
        decode_hidden(&input, memory, &self.layers(), masks, kernel)
    }

    /// Project the full prefix or only its last row using the tied table.
    pub fn logits(
        &self,
        embeddings: &MarianEmbeddings,
        ids: &[u32],
        memory: &Tensor,
        masks: DecoderMask<'_>,
        kernel: Kernel,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        let hidden = self.forward(embeddings, ids, memory, masks, kernel)?;
        embeddings.project(&hidden, last_only)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marian_encoder::ENCODER_PAYLOAD_BYTES;

    #[test]
    fn decoder_and_total_budget_match_inventory() {
        let shapes = rpmodel::package::expected_shapes();
        let layers: Vec<_> = shapes
            .iter()
            .filter(|(name, _)| name.starts_with("model.decoder.layers."))
            .collect();
        assert_eq!(layers.len(), 156);
        assert_eq!(
            layers
                .iter()
                .map(|(_, shape)| shape.iter().product::<u64>() * 4)
                .sum::<u64>(),
            DECODER_PAYLOAD_BYTES as u64
        );
        assert_eq!(
            (DECODER_PAYLOAD_BYTES + ENCODER_PAYLOAD_BYTES + 135_479_204) as u64,
            rpmodel::package::PAYLOAD_BYTES
        );
    }
}
