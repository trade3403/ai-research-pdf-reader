//! Immutable per-prefix KV snapshots; cross projections shared across branches.
use crate::{
    attention::Mask,
    decoder::Decoder,
    layers::MultiHead,
    matmul::Kernel,
    tensor::{buffer, layout, Binary, Error, Tensor},
    transformer::{embedding, input_embeddings},
    vocabulary::RowTable,
};
use std::rc::Rc;

const MAX_CACHE_BYTES: usize = 256 * 1024 * 1024;

struct Kv {
    k: Tensor,
    v: Tensor,
}

impl Kv {
    fn project(att: &MultiHead<'_>, x: &Tensor, kernel: Kernel) -> Result<Self, Error> {
        Ok(Self {
            k: att.k.forward(x, kernel)?,
            v: att.v.forward(x, kernel)?,
        })
    }

    fn append(&self, row: &Self) -> Result<Self, Error> {
        Ok(Self {
            k: append_row(&self.k, &row.k)?,
            v: append_row(&self.v, &row.v)?,
        })
    }
}

fn append_row(old: &Tensor, row: &Tensor) -> Result<Tensor, Error> {
    let &[rows, width] = old.shape() else {
        return Err(Error::Shape);
    };
    if row.shape() != [1, width] {
        return Err(Error::Shape);
    }
    let count = rows.checked_add(1).ok_or(Error::Limit)?;
    let (_, size) = layout(&[count, width])?;
    let mut data = buffer(size)?;
    data.extend_from_slice(old.data());
    data.extend_from_slice(row.data());
    Tensor::new(&[count, width], data)
}

pub(crate) struct Cache<'m, 'a> {
    decoder: &'m Decoder<'a>,
    positions: Option<&'a Tensor>,
    cross: Rc<Vec<Kv>>,
    own: Vec<Kv>,
    source_keep: Option<&'m [bool]>,
    target_keep: Vec<bool>,
    limit: usize,
    kernel: Kernel,
}

impl<'m, 'a> Cache<'m, 'a> {
    pub(crate) fn new(
        decoder: &'m Decoder<'a>,
        memory: &Tensor,
        source_keep: Option<&'m [bool]>,
        limit: usize,
        branches: usize,
        kernel: Kernel,
    ) -> Result<Self, Error> {
        Self::new_with_positions(decoder, memory, source_keep, limit, branches, kernel, None)
    }

    pub(crate) fn new_with_positions(
        decoder: &'m Decoder<'a>,
        memory: &Tensor,
        source_keep: Option<&'m [bool]>,
        limit: usize,
        branches: usize,
        kernel: Kernel,
        positions: Option<&'a Tensor>,
    ) -> Result<Self, Error> {
        let &[source, hidden] = memory.shape() else {
            return Err(Error::Shape);
        };
        let &[_, width] = decoder.embeddings.shape() else {
            return Err(Error::Shape);
        };
        if source == 0
            || hidden == 0
            || width != hidden
            || decoder.layers.is_empty()
            || positions.is_some_and(|p| p.shape() != [decoder.max_positions, hidden])
            || source_keep.is_some_and(|m| m.len() != source)
        {
            return Err(Error::Shape);
        }
        if limit == 0
            || limit > 512
            || limit > decoder.max_positions
            || !(1..=16).contains(&branches)
            || decoder.layers.len() > 64
        {
            return Err(Error::Limit);
        }
        // Two generations of beam snapshots and one shared cross cache.
        // Bounds KV payload only, not total model or temporary workspace.
        let bytes = limit
            .checked_mul(2 * branches + 1)
            .and_then(|v| v.checked_add(source))
            .and_then(|v| v.checked_mul(hidden))
            .and_then(|v| v.checked_mul(decoder.layers.len()))
            .and_then(|v| v.checked_mul(8))
            .ok_or(Error::Limit)?;
        if bytes > MAX_CACHE_BYTES {
            return Err(Error::Limit);
        }
        layout(&[limit, hidden])?;
        let mut cross = Vec::new();
        for layer in decoder.layers {
            cross.push(Kv::project(&layer.cross_attention, memory, kernel)?);
        }
        Ok(Self {
            decoder,
            positions,
            cross: Rc::new(cross),
            own: Vec::new(),
            source_keep,
            target_keep: Vec::new(),
            limit,
            kernel,
        })
    }

    /// Compute exactly one new hidden row; the parent snapshot is unchanged.
    /// Target PAD/start tokens are only hidden when the caller says so.
    pub(crate) fn step(&self, token: u32, keep: bool) -> Result<(Self, Tensor), Error> {
        let offset = self.target_keep.len();
        if offset >= self.limit {
            return Err(Error::Limit);
        }
        let mut target_keep = self.target_keep.clone();
        target_keep.push(keep);
        let mut x = if let Some(positions) = self.positions {
            let tokens = embedding(
                self.decoder.embeddings,
                &[token],
                self.decoder.embedding_scale,
            )?;
            let row = Tensor::new(&[1, tokens.shape()[1]], positions.row(offset)?.to_vec())?;
            tokens.binary(&row, Binary::Add)?
        } else {
            input_embeddings(
                self.decoder.embeddings,
                &[token],
                self.decoder.embedding_scale,
                offset,
                self.decoder.max_positions,
            )?
        };
        let mut own = Vec::new();
        for (i, layer) in self.decoder.layers.iter().enumerate() {
            let row = Kv::project(&layer.self_attention, &x, self.kernel)?;
            let kv = if offset == 0 {
                row
            } else {
                self.own[i].append(&row)?
            };
            // There are no future keys in this snapshot.
            let mask = Mask {
                key_keep: Some(&target_keep),
                ..Mask::default()
            };
            let branch =
                layer
                    .self_attention
                    .forward_projected(&x, &kv.k, &kv.v, mask, self.kernel)?;
            x = layer.self_norm.residual(&x, &branch)?;
            let cross = &self.cross[i];
            let mask = Mask {
                key_keep: self.source_keep,
                ..Mask::default()
            };
            let branch = layer.cross_attention.forward_projected(
                &x,
                &cross.k,
                &cross.v,
                mask,
                self.kernel,
            )?;
            x = layer.cross_norm.residual(&x, &branch)?;
            let branch = layer.ffn.forward(&x, self.kernel)?;
            x = layer.final_norm.residual(&x, &branch)?;
            own.push(kv);
        }
        Ok((
            Self {
                decoder: self.decoder,
                positions: self.positions,
                cross: Rc::clone(&self.cross),
                own,
                source_keep: self.source_keep,
                target_keep,
                limit: self.limit,
                kernel: self.kernel,
            },
            x,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_is_immutable_and_checks_width() {
        let a = Tensor::new(&[2, 2], vec![1., 2., 3., 4.]).unwrap();
        let b = Tensor::new(&[1, 2], vec![5., 6.]).unwrap();
        let c = append_row(&a, &b).unwrap();
        assert_eq!(c.shape(), &[3, 2]);
        assert_eq!(c.data(), &[1., 2., 3., 4., 5., 6.]);
        assert_eq!(a.data(), &[1., 2., 3., 4.]);
        assert_eq!(append_row(&a, &a), Err(Error::Shape));
    }
}
