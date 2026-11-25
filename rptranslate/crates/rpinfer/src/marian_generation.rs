//! Real Marian generation on an immutable encoded source, with stored positions.
use crate::{
    cache::Cache,
    decoder::{Decoder, DecoderMask},
    marian_decoder::MarianDecoder,
    marian_weights::{MarianEmbeddings, Side},
    matmul::Kernel,
    search::{search_constrained, validate, Hypothesis, SearchConfig},
    tensor::{buffer, layout, Error, Tensor},
    vocabulary::RowTable,
};
use std::collections::BTreeMap;

/// Borrows weights and source memory. All cache snapshots are request-local.
pub struct MarianEncoded<'a> {
    decoder: &'a MarianDecoder,
    embeddings: &'a MarianEmbeddings,
    memory: &'a Tensor,
    source_keep: Option<&'a [bool]>,
    kernel: Kernel,
}

impl<'a> MarianEncoded<'a> {
    pub fn new(
        decoder: &'a MarianDecoder,
        embeddings: &'a MarianEmbeddings,
        memory: &'a Tensor,
        source_keep: Option<&'a [bool]>,
        kernel: Kernel,
    ) -> Result<Self, Error> {
        let &[source, hidden] = memory.shape() else {
            return Err(Error::Shape);
        };
        if source == 0
            || hidden != embeddings.shared().shape()[1]
            || source_keep.is_some_and(|m| m.len() != source || !m.iter().any(|&v| v))
        {
            return Err(Error::Shape);
        }
        if source > embeddings.positions(Side::Encoder).shape()[0] {
            return Err(Error::Limit);
        }
        Ok(Self {
            decoder,
            embeddings,
            memory,
            source_keep,
            kernel,
        })
    }

    fn vocab(&self) -> usize {
        self.embeddings.shared().shape()[0]
    }

    pub fn logits(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        self.decoder.logits(
            self.embeddings,
            prefix,
            self.memory,
            DecoderMask {
                source_keep: self.source_keep,
                target_keep,
            },
            self.kernel,
            last_only,
        )
    }

    pub fn cached_logits(
        &self,
        prefix: &[u32],
        target_keep: Option<&[bool]>,
    ) -> Result<Tensor, Error> {
        if prefix.is_empty() || target_keep.is_some_and(|m| m.len() != prefix.len() || !m[0]) {
            return Err(Error::Shape);
        }
        let (_, size) = layout(&[prefix.len(), self.vocab()])?;
        let layers = self.decoder.layers();
        let positions = self.embeddings.positions(Side::Decoder);
        let decoder = Decoder {
            embeddings: self.embeddings.shared(),
            embedding_scale: self.embeddings.embedding_scale(),
            max_positions: positions.shape()[0],
            layers: &layers,
        };
        let mut cache = Cache::new_with_positions(
            &decoder,
            self.memory,
            self.source_keep,
            prefix.len(),
            1,
            self.kernel,
            Some(positions),
        )?;
        let mut data = buffer(size)?;
        for (i, &token) in prefix.iter().enumerate() {
            let (next, hidden) = cache.step(token, target_keep.is_none_or(|m| m[i]))?;
            let logits = self.embeddings.project(&hidden, true)?;
            data.extend_from_slice(logits.data());
            cache = next;
        }
        Tensor::new(&[prefix.len(), self.vocab()], data)
    }

    pub fn generate(&self, config: SearchConfig<'_>) -> Result<Vec<Hypothesis>, Error> {
        self.generate_constrained(config, &[])
    }

    pub fn generate_constrained(
        &self,
        config: SearchConfig<'_>,
        phrases: &[Vec<u32>],
    ) -> Result<Vec<Hypothesis>, Error> {
        validate(config, self.vocab())?;
        if config.max_new > self.embeddings.positions(Side::Decoder).shape()[0] {
            return Err(Error::Limit);
        }
        search_constrained(self.vocab(), config, phrases, |prefix| {
            self.logits(prefix, None, true)
        })
    }

    pub fn generate_cached(&self, config: SearchConfig<'_>) -> Result<Vec<Hypothesis>, Error> {
        self.generate_cached_constrained(config, &[])
    }

    pub fn generate_cached_constrained(
        &self,
        config: SearchConfig<'_>,
        phrases: &[Vec<u32>],
    ) -> Result<Vec<Hypothesis>, Error> {
        validate(config, self.vocab())?;
        let layers = self.decoder.layers();
        let positions = self.embeddings.positions(Side::Decoder);
        let decoder = Decoder {
            embeddings: self.embeddings.shared(),
            embedding_scale: self.embeddings.embedding_scale(),
            max_positions: positions.shape()[0],
            layers: &layers,
        };
        let root = Cache::new_with_positions(
            &decoder,
            self.memory,
            self.source_keep,
            config.max_new,
            config.beam_width,
            self.kernel,
            Some(positions),
        )?;
        let mut previous = BTreeMap::new();
        let mut current = BTreeMap::new();
        let mut depth = 1;
        search_constrained(self.vocab(), config, phrases, |prefix| {
            if prefix.len() == depth + 1 {
                previous = std::mem::take(&mut current);
                depth += 1;
            }
            if prefix.len() != depth {
                return Err(Error::Shape);
            }
            let (&token, parent) = prefix.split_last().ok_or(Error::Shape)?;
            let cache = if parent.is_empty() {
                &root
            } else {
                previous.get(parent).ok_or(Error::Shape)?
            };
            let (next, hidden) = cache.step(token, true)?;
            let logits = self.embeddings.project(&hidden, true)?;
            current.insert(prefix.to_vec(), next);
            Ok(logits)
        })
    }
}
