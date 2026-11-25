//! Actual fixed-profile shared embeddings, position tables and tied LM head.
use crate::{
    seq2seq::OutputHead,
    tensor::{buffer, layout, Binary, Error, Tensor},
    transformer::embedding,
    vocabulary::{ChunkedTable, LoadError, RowTable},
};
use rpmodel::{
    config::MarianConfig,
    safetensors::{Dtype, Error as ReaderError, SafeTensorReader},
};
use std::io::{Read, Seek};

#[derive(Debug, Clone, Copy)]
pub enum Side {
    Encoder,
    Decoder,
}

pub(crate) fn read_dense<R: Read + Seek>(
    reader: &mut SafeTensorReader<R>,
    name: &str,
    shape: &[usize],
) -> Result<Tensor, LoadError> {
    let info = reader
        .tensors()
        .get(name)
        .ok_or(ReaderError::MissingTensor)?;
    if info.dtype != Dtype::F32
        || info
            .shape
            .iter()
            .copied()
            .ne(shape.iter().map(|&d| d as u64))
    {
        return Err(Error::Shape.into());
    }
    let (_, count) = layout(shape)?;
    let mut data = buffer(count)?;
    for start in (0..count).step_by(1024 * 1024) {
        data.extend(reader.read_f32(name, start as u64, (count - start).min(1024 * 1024))?);
    }
    Ok(Tensor::new(shape, data)?)
}

/// Own one shared word table, two checkpoint position tables and output bias.
/// Budget covers all four payloads, not the process or temporary read buffers.
#[derive(Debug)]
pub struct MarianEmbeddings {
    shared: ChunkedTable,
    encoder_positions: Tensor,
    decoder_positions: Tensor,
    bias: Tensor,
    scale: f32,
    payload_bytes: usize,
}

impl MarianEmbeddings {
    /// Call with the reader and config returned by package::open_profile.
    /// Checks dimensions and budget again before allocating any weights.
    pub fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        config: &MarianConfig,
        max_bytes: usize,
    ) -> Result<Self, LoadError> {
        let (vocab, hidden, positions) = (config.vocab, config.hidden, config.max_positions);
        if (vocab, hidden, positions) != (65001, 512, 512)
            || config.embedding_scale != 512_f32.sqrt()
        {
            return Err(Error::Shape.into());
        }
        let shared_bytes = vocab * hidden * 4;
        let payload_bytes = shared_bytes + 2 * positions * hidden * 4 + vocab * 4;
        if payload_bytes > max_bytes {
            return Err(Error::Limit.into());
        }
        let info = reader
            .tensors()
            .get("model.shared.weight")
            .ok_or(ReaderError::MissingTensor)?;
        if info.dtype != Dtype::F32 || info.shape != [vocab as u64, hidden as u64] {
            return Err(Error::Shape.into());
        }
        let shared = ChunkedTable::load(reader, "model.shared.weight", 2048, shared_bytes)?;
        let encoder_positions = read_dense(
            reader,
            "model.encoder.embed_positions.weight",
            &[positions, hidden],
        )?;
        let decoder_positions = read_dense(
            reader,
            "model.decoder.embed_positions.weight",
            &[positions, hidden],
        )?;
        let mut bias = read_dense(reader, "final_logits_bias", &[1, vocab])?;
        bias.reshape(&[vocab])?;
        Ok(Self {
            shared,
            encoder_positions,
            decoder_positions,
            bias,
            scale: config.embedding_scale,
            payload_bytes,
        })
    }

    pub fn shared(&self) -> &ChunkedTable {
        &self.shared
    }

    pub fn positions(&self, side: Side) -> &Tensor {
        match side {
            Side::Encoder => &self.encoder_positions,
            Side::Decoder => &self.decoder_positions,
        }
    }

    pub fn bias(&self) -> &Tensor {
        &self.bias
    }

    pub fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }

    pub fn tokens(&self, ids: &[u32]) -> Result<Tensor, Error> {
        embedding(&self.shared, ids, self.scale)
    }

    /// Use the actual stored position rows, not reconstructed sinusoids.
    pub fn input(&self, side: Side, ids: &[u32], offset: usize) -> Result<Tensor, Error> {
        let table = self.positions(side);
        let &[positions, width] = table.shape() else {
            return Err(Error::Shape);
        };
        let end = offset.checked_add(ids.len()).ok_or(Error::Limit)?;
        if end > positions {
            return Err(Error::Index);
        }
        let tokens = self.tokens(ids)?;
        let mut data = buffer(ids.len() * width)?;
        for pos in offset..end {
            data.extend_from_slice(table.row(pos)?);
        }
        let positions = Tensor::new(&[ids.len(), width], data)?;
        tokens.binary(&positions, Binary::Add)
    }

    pub fn head(&self) -> OutputHead<'_> {
        OutputHead {
            weight: None,
            bias: Some(&self.bias),
        }
    }

    pub fn project(&self, hidden: &Tensor, last_only: bool) -> Result<Tensor, Error> {
        self.head().project(hidden, &self.shared, last_only)
    }
}
