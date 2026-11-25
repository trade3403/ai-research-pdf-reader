//! Owning fixed-profile model and request-local token inference sessions.
use crate::{
    encoder::padding_mask,
    marian_decoder::{MarianDecoder, DECODER_PAYLOAD_BYTES},
    marian_encoder::{MarianEncoder, ENCODER_PAYLOAD_BYTES},
    marian_generation::MarianEncoded,
    marian_weights::MarianEmbeddings,
    matmul::Kernel,
    search::{Hypothesis, MarianGeneration, SearchConfig},
    tensor::{Error, Tensor},
    vocabulary::LoadError,
};
use rpmodel::{config::MarianConfig, package};
use std::path::Path;

pub const MODEL_PAYLOAD_BYTES: usize = 312_033_188;
const EMBEDDING_PAYLOAD_BYTES: usize = 135_479_204;

#[derive(Debug, PartialEq, Eq)]
pub enum ModelError {
    Budget,
    Package(package::Error),
    Weights(LoadError),
    Inference(Error),
}

impl From<Error> for ModelError {
    fn from(error: Error) -> Self {
        Self::Inference(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    Full,
    Cached,
}

/// Limits include only newly generated tokens, not the initial start token.
/// Special IDs, suppression, forced EOS and penalty come from verified metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationOptions {
    pub beam_width: usize,
    pub max_new: usize,
    pub mode: GenerationMode,
}

fn search_options(
    config: &MarianConfig,
    options: GenerationOptions,
) -> Result<SearchConfig<'_>, Error> {
    let max_new = config
        .generation
        .max_length
        .checked_sub(1)
        .ok_or(Error::Shape)?;
    if options.max_new == 0
        || options.max_new > max_new
        || options.beam_width == 0
        || options.beam_width > 16
    {
        return Err(Error::Limit);
    }
    let mut generation = MarianGeneration::from_profile(&config.generation);
    generation.num_beams = options.beam_width;
    generation.max_length = options.max_new.checked_add(1).ok_or(Error::Limit)?;
    generation.search_config(config.vocab)
}

/// Exactly one owner of each canonical tensor. No open weight file is retained.
/// The weight budget excludes metadata, scratch buffers and request-local KV.
pub struct MarianModel {
    config: MarianConfig,
    embeddings: MarianEmbeddings,
    encoder: MarianEncoder,
    decoder: MarianDecoder,
}

impl MarianModel {
    pub fn load(root: &Path, max_weight_bytes: usize) -> Result<Self, ModelError> {
        if max_weight_bytes < MODEL_PAYLOAD_BYTES {
            return Err(ModelError::Budget);
        }
        let (config, mut reader) = package::open_profile(root).map_err(ModelError::Package)?;
        let embeddings = MarianEmbeddings::load(&mut reader, &config, EMBEDDING_PAYLOAD_BYTES)
            .map_err(ModelError::Weights)?;
        let encoder = MarianEncoder::load(&mut reader, &config, ENCODER_PAYLOAD_BYTES)
            .map_err(ModelError::Weights)?;
        let decoder = MarianDecoder::load(&mut reader, &config, DECODER_PAYLOAD_BYTES)
            .map_err(ModelError::Weights)?;
        drop(reader);
        Ok(Self {
            config,
            embeddings,
            encoder,
            decoder,
        })
    }

    pub fn config(&self) -> &MarianConfig {
        &self.config
    }

    pub fn payload_bytes(&self) -> usize {
        self.embeddings.payload_bytes()
            + self.encoder.payload_bytes()
            + self.decoder.payload_bytes()
    }

    pub fn default_options(&self) -> GenerationOptions {
        GenerationOptions {
            beam_width: self.config.generation.num_beams,
            max_new: self.config.generation.max_length - 1,
            mode: GenerationMode::Cached,
        }
    }

    /// Source mask is copied. If absent, derive it from source PAD IDs.
    /// No automatic target mask: target start may have the same ID as PAD.
    pub fn encode(
        &self,
        source: &[u32],
        source_keep: Option<&[bool]>,
        kernel: Kernel,
    ) -> Result<TokenSession<'_>, Error> {
        if source.is_empty() {
            return Err(Error::Shape);
        }
        if source.len() > self.config.max_positions {
            return Err(Error::Limit);
        }
        if source.iter().any(|&id| id as usize >= self.config.vocab) {
            return Err(Error::Index);
        }
        let mask = if let Some(mask) = source_keep {
            if mask.len() != source.len() {
                return Err(Error::Shape);
            }
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(mask.len())
                .map_err(|_| Error::Allocation)?;
            owned.extend_from_slice(mask);
            owned
        } else {
            padding_mask(source, self.config.generation.pad)?
        };
        let memory = self
            .encoder
            .forward(&self.embeddings, source, Some(&mask), kernel)?;
        Ok(TokenSession {
            model: self,
            memory,
            source_keep: mask,
            kernel,
        })
    }

    /// Convenience entry point: encode once and generate ranked hypotheses.
    pub fn generate_tokens(
        &self,
        source: &[u32],
        source_keep: Option<&[bool]>,
        options: GenerationOptions,
        kernel: Kernel,
    ) -> Result<Vec<Hypothesis>, Error> {
        search_options(&self.config, options)?;
        self.encode(source, source_keep, kernel)?.generate(options)
    }
}

/// Owns source memory and mask but borrows immutable model weights.
/// Independent sessions do not share mutable request state or KV snapshots.
pub struct TokenSession<'a> {
    model: &'a MarianModel,
    memory: Tensor,
    source_keep: Vec<bool>,
    kernel: Kernel,
}

impl TokenSession<'_> {
    fn encoded(&self) -> Result<MarianEncoded<'_>, Error> {
        MarianEncoded::new(
            &self.model.decoder,
            &self.model.embeddings,
            &self.memory,
            Some(&self.source_keep),
            self.kernel,
        )
    }

    pub fn memory(&self) -> &Tensor {
        &self.memory
    }

    pub fn source_keep(&self) -> &[bool] {
        &self.source_keep
    }

    pub fn logits(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        self.encoded()?.logits(prefix, target_keep, last_only)
    }

    pub fn cached_logits(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
    ) -> Result<Tensor, Error> {
        self.encoded()?.cached_logits(prefix, target_keep)
    }

    pub fn generate(&self, options: GenerationOptions) -> Result<Vec<Hypothesis>, Error> {
        let search = search_options(&self.model.config, options)?;
        let encoded = self.encoded()?;
        match options.mode {
            GenerationMode::Full => encoded.generate(search),
            GenerationMode::Cached => encoded.generate_cached(search),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_payload_and_early_budget_rejection() {
        assert_eq!(
            MODEL_PAYLOAD_BYTES,
            EMBEDDING_PAYLOAD_BYTES + ENCODER_PAYLOAD_BYTES + DECODER_PAYLOAD_BYTES
        );
        assert_eq!(MODEL_PAYLOAD_BYTES as u64, package::PAYLOAD_BYTES);
        assert_eq!(
            MarianModel::load(Path::new("nonexistent-model"), MODEL_PAYLOAD_BYTES - 1).err(),
            Some(ModelError::Budget)
        );
    }
}
