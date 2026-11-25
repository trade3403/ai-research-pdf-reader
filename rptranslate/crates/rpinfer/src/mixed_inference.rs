//! Real mixed-precision Marian. Packed matrices, dynamic INT8 linear layers,
//! FP32 attention/KV/residuals/positions, and the existing bounded beam policy.
use crate::{
    attention::{scaled_dot_product, Mask},
    cache::append_row,
    encoder::{padding_mask, Norm},
    layers::head_slice,
    marian_model::{search_options, GenerationMode, GenerationOptions},
    matmul::Kernel,
    mixed_model::MixedModel,
    quantized_linear::{Int8Kernel, QuantizedLinear},
    search::{search, Hypothesis},
    tensor::{buffer, layout, Binary, Error, Tensor},
    transformer::{activate, Activation},
};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Copy)]
pub struct MixedKernels {
    pub integer: Int8Kernel,
    pub float: Kernel,
}

impl Default for MixedKernels {
    fn default() -> Self {
        Self {
            integer: Int8Kernel::Auto,
            float: Kernel::Auto,
        }
    }
}

struct Kv {
    k: Tensor,
    v: Tensor,
}

impl Kv {
    fn append(&self, row: &Self) -> Result<Self, Error> {
        Ok(Self {
            k: append_row(&self.k, &row.k)?,
            v: append_row(&self.v, &row.v)?,
        })
    }
}

struct Attention<'a> {
    q: QuantizedLinear<'a>,
    k: QuantizedLinear<'a>,
    v: QuantizedLinear<'a>,
    output: QuantizedLinear<'a>,
}

impl Attention<'_> {
    fn project(&self, x: &Tensor, kernels: MixedKernels) -> Result<Kv, Error> {
        Ok(Kv {
            k: self.k.forward_with(x, false, kernels.integer)?,
            v: self.v.forward_with(x, false, kernels.integer)?,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        kv: &Kv,
        mask: Mask<'_>,
        kernels: MixedKernels,
    ) -> Result<Tensor, Error> {
        let q = self.q.forward_with(x, false, kernels.integer)?;
        let rows = q.shape()[0];
        let mut joined = buffer(rows * 512)?;
        joined.resize(rows * 512, 0.);
        for head in 0..8 {
            let start = head * 64;
            let qh = head_slice(&q, start, 64)?;
            let kh = head_slice(&kv.k, start, 64)?;
            let vh = head_slice(&kv.v, start, 64)?;
            let context = scaled_dot_product(&qh, &kh, &vh, mask, kernels.float)?.context;
            for (row, part) in joined
                .chunks_exact_mut(512)
                .zip(context.data().chunks_exact(64))
            {
                row[start..start + 64].copy_from_slice(part);
            }
        }
        self.output
            .forward_with(&Tensor::new(&[rows, 512], joined)?, false, kernels.integer)
    }
}

struct Layer<'a> {
    own: Attention<'a>,
    own_norm: Norm<'a>,
    cross: Option<(Attention<'a>, Norm<'a>)>,
    first: QuantizedLinear<'a>,
    second: QuantizedLinear<'a>,
    final_norm: Norm<'a>,
}

impl Layer<'_> {
    fn finish(&self, x: &Tensor, kernels: MixedKernels) -> Result<Tensor, Error> {
        let first = self.first.forward_with(x, false, kernels.integer)?;
        let ff = self.second.forward_with(
            &activate(&first, Activation::Silu)?,
            false,
            kernels.integer,
        )?;
        self.final_norm.residual(x, &ff)
    }
}

impl MixedModel {
    fn linear(&self, name: &str) -> Result<QuantizedLinear<'_>, Error> {
        Ok(QuantizedLinear {
            weight: self.matrix(&format!("{name}.weight")).ok_or(Error::Shape)?,
            bias: Some(
                self.auxiliary(&format!("{name}.bias"))
                    .ok_or(Error::Shape)?,
            ),
        })
    }

    fn norm(&self, name: &str) -> Result<Norm<'_>, Error> {
        Ok(Norm {
            weight: self
                .auxiliary(&format!("{name}.weight"))
                .ok_or(Error::Shape)?,
            bias: self
                .auxiliary(&format!("{name}.bias"))
                .ok_or(Error::Shape)?,
            epsilon: 1e-5,
        })
    }

    fn attention(&self, name: &str) -> Result<Attention<'_>, Error> {
        Ok(Attention {
            q: self.linear(&format!("{name}.q_proj"))?,
            k: self.linear(&format!("{name}.k_proj"))?,
            v: self.linear(&format!("{name}.v_proj"))?,
            output: self.linear(&format!("{name}.out_proj"))?,
        })
    }

    fn layers(&self, decoder: bool) -> Result<Vec<Layer<'_>>, Error> {
        let side = if decoder { "decoder" } else { "encoder" };
        (0..6)
            .map(|i| {
                let p = format!("model.{side}.layers.{i}");
                Ok(Layer {
                    own: self.attention(&format!("{p}.self_attn"))?,
                    own_norm: self.norm(&format!("{p}.self_attn_layer_norm"))?,
                    cross: if decoder {
                        Some((
                            self.attention(&format!("{p}.encoder_attn"))?,
                            self.norm(&format!("{p}.encoder_attn_layer_norm"))?,
                        ))
                    } else {
                        None
                    },
                    first: self.linear(&format!("{p}.fc1"))?,
                    second: self.linear(&format!("{p}.fc2"))?,
                    final_norm: self.norm(&format!("{p}.final_layer_norm"))?,
                })
            })
            .collect()
    }

    fn input(&self, ids: &[u32], decoder: bool, offset: usize) -> Result<Tensor, Error> {
        if ids.is_empty() {
            return Err(Error::Shape);
        }
        let end = offset.checked_add(ids.len()).ok_or(Error::Limit)?;
        if end > self.config().max_positions {
            return Err(Error::Limit);
        }
        let side = if decoder { "decoder" } else { "encoder" };
        let pos = self
            .auxiliary(&format!("model.{side}.embed_positions.weight"))
            .ok_or(Error::Shape)?;
        let shared = self.matrix("model.shared.weight").ok_or(Error::Shape)?;
        let tokens = shared.embedding(ids, self.config().embedding_scale)?;
        let positions = Tensor::new(
            &[ids.len(), 512],
            pos.data()[offset * 512..end * 512].to_vec(),
        )?;
        tokens.binary(&positions, Binary::Add)
    }

    pub fn encode_mixed(
        &self,
        ids: &[u32],
        source_keep: Option<&[bool]>,
        kernels: MixedKernels,
    ) -> Result<MixedSession<'_>, Error> {
        let mut x = self.input(ids, false, 0)?;
        let keep = match source_keep {
            Some(mask) if mask.len() == ids.len() => mask.to_vec(),
            Some(_) => return Err(Error::Shape),
            None => padding_mask(ids, self.config().generation.pad)?,
        };
        if !keep.iter().any(|&v| v) {
            return Err(Error::Shape);
        }
        for layer in self.layers(false)? {
            let kv = layer.own.project(&x, kernels)?;
            let branch = layer.own.forward(
                &x,
                &kv,
                Mask {
                    key_keep: Some(&keep),
                    ..Mask::default()
                },
                kernels,
            )?;
            x = layer.finish(&layer.own_norm.residual(&x, &branch)?, kernels)?;
        }
        let bias = self.auxiliary("final_logits_bias").ok_or(Error::Shape)?;
        Ok(MixedSession {
            model: self,
            layers: self.layers(true)?,
            memory: x,
            keep,
            bias: Tensor::new(&[self.config().vocab], bias.data().to_vec())?,
            kernels,
        })
    }
}

/// Source memory and masks are request-owned; weights are borrowed. KV
/// snapshots are FP32, immutable and local to one generation request.
pub struct MixedSession<'a> {
    model: &'a MixedModel,
    layers: Vec<Layer<'a>>,
    memory: Tensor,
    keep: Vec<bool>,
    bias: Tensor,
    kernels: MixedKernels,
}

struct Snapshot {
    cross: Rc<Vec<Kv>>,
    own: Vec<Kv>,
    keep: Vec<bool>,
    limit: usize,
}

impl MixedSession<'_> {
    pub fn memory(&self) -> &Tensor {
        &self.memory
    }

    fn head(&self, hidden: &Tensor, last_only: bool) -> Result<Tensor, Error> {
        QuantizedLinear {
            weight: self
                .model
                .matrix("model.shared.weight")
                .ok_or(Error::Shape)?,
            bias: Some(&self.bias),
        }
        .forward_with(hidden, last_only, self.kernels.integer)
    }

    fn cross(&self) -> Result<Vec<Kv>, Error> {
        self.layers
            .iter()
            .map(|layer| {
                layer
                    .cross
                    .as_ref()
                    .ok_or(Error::Shape)?
                    .0
                    .project(&self.memory, self.kernels)
            })
            .collect()
    }

    fn validate_prefix(&self, ids: &[u32], keep: Option<&[bool]>) -> Result<(), Error> {
        if ids.is_empty() || keep.is_some_and(|m| m.len() != ids.len() || !m[0]) {
            return Err(Error::Shape);
        }
        if ids.len() > self.model.config().max_positions {
            return Err(Error::Limit);
        }
        if ids
            .iter()
            .any(|&id| id as usize >= self.model.config().vocab)
        {
            return Err(Error::Index);
        }
        Ok(())
    }

    pub fn logits(
        &self,
        ids: &[u32],
        keep: Option<&[bool]>,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        self.validate_prefix(ids, keep)?;
        let mut x = self.model.input(ids, true, 0)?;
        let cross = self.cross()?;
        for (layer, cross_kv) in self.layers.iter().zip(&cross) {
            let kv = layer.own.project(&x, self.kernels)?;
            let branch = layer.own.forward(
                &x,
                &kv,
                Mask {
                    key_keep: keep,
                    causal_offset: Some(0),
                    ..Mask::default()
                },
                self.kernels,
            )?;
            x = layer.own_norm.residual(&x, &branch)?;
            let (att, norm) = layer.cross.as_ref().ok_or(Error::Shape)?;
            let branch = att.forward(
                &x,
                cross_kv,
                Mask {
                    key_keep: Some(&self.keep),
                    ..Mask::default()
                },
                self.kernels,
            )?;
            x = layer.finish(&norm.residual(&x, &branch)?, self.kernels)?;
        }
        self.head(&x, last_only)
    }

    fn root(&self, limit: usize, branches: usize) -> Result<Snapshot, Error> {
        if limit == 0 || limit > 512 || !(1..=16).contains(&branches) {
            return Err(Error::Limit);
        }
        let bytes = limit
            .checked_mul(2 * branches + 1)
            .and_then(|v| v.checked_add(self.keep.len()))
            .and_then(|v| v.checked_mul(512 * 6 * 8))
            .ok_or(Error::Limit)?;
        if bytes > 256 * 1024 * 1024 {
            return Err(Error::Limit);
        }
        Ok(Snapshot {
            cross: Rc::new(self.cross()?),
            own: Vec::new(),
            keep: Vec::new(),
            limit,
        })
    }

    fn step(&self, parent: &Snapshot, token: u32, keep: bool) -> Result<(Snapshot, Tensor), Error> {
        let offset = parent.keep.len();
        if offset >= parent.limit {
            return Err(Error::Limit);
        }
        let mut target_keep = parent.keep.clone();
        target_keep.push(keep);
        let mut x = self.model.input(&[token], true, offset)?;
        let mut own = Vec::new();
        for (i, layer) in self.layers.iter().enumerate() {
            let row = layer.own.project(&x, self.kernels)?;
            let kv = if offset == 0 {
                row
            } else {
                parent.own[i].append(&row)?
            };
            let branch = layer.own.forward(
                &x,
                &kv,
                Mask {
                    key_keep: Some(&target_keep),
                    ..Mask::default()
                },
                self.kernels,
            )?;
            x = layer.own_norm.residual(&x, &branch)?;
            let (att, norm) = layer.cross.as_ref().ok_or(Error::Shape)?;
            let branch = att.forward(
                &x,
                &parent.cross[i],
                Mask {
                    key_keep: Some(&self.keep),
                    ..Mask::default()
                },
                self.kernels,
            )?;
            x = layer.finish(&norm.residual(&x, &branch)?, self.kernels)?;
            own.push(kv);
        }
        Ok((
            Snapshot {
                cross: Rc::clone(&parent.cross),
                own,
                keep: target_keep,
                limit: parent.limit,
            },
            self.head(&x, true)?,
        ))
    }

    pub fn cached_logits(&self, ids: &[u32], keep: Option<&[bool]>) -> Result<Tensor, Error> {
        self.validate_prefix(ids, keep)?;
        let vocab = self.model.config().vocab;
        let (_, size) = layout(&[ids.len(), vocab])?;
        let mut data = buffer(size)?;
        let mut snapshot = self.root(ids.len(), 1)?;
        for (i, &token) in ids.iter().enumerate() {
            let (next, logits) = self.step(&snapshot, token, keep.is_none_or(|m| m[i]))?;
            data.extend_from_slice(logits.data());
            snapshot = next;
        }
        Tensor::new(&[ids.len(), vocab], data)
    }

    pub fn generate(&self, options: GenerationOptions) -> Result<Vec<Hypothesis>, Error> {
        let config = search_options(self.model.config(), options)?;
        let vocab = self.model.config().vocab;
        if options.mode == GenerationMode::Full {
            return search(vocab, config, |ids| self.logits(ids, None, true));
        }
        let root = self.root(config.max_new, config.beam_width)?;
        let mut previous = BTreeMap::new();
        let mut current = BTreeMap::new();
        let mut depth = 1;
        search(vocab, config, |prefix| {
            if prefix.len() == depth + 1 {
                previous = std::mem::take(&mut current);
                depth += 1;
            }
            if prefix.len() != depth {
                return Err(Error::Shape);
            }
            let (&token, parent) = prefix.split_last().ok_or(Error::Shape)?;
            let snapshot = if parent.is_empty() {
                &root
            } else {
                previous.get(parent).ok_or(Error::Shape)?
            };
            let (next, logits) = self.step(snapshot, token, true)?;
            current.insert(prefix.to_vec(), next);
            Ok(logits)
        })
    }
}
