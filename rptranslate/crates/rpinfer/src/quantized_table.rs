//! Row-aligned packed vocabulary storage. No dense FP32 table is retained.
use crate::{
    quantization::{BlockInt8, MAX_BLOCK},
    tensor::{buffer, layout, Error, Tensor},
};
use rpmodel::{
    quantized::{self, TensorInput},
    safetensors::{self, Dtype, SafeTensorReader},
};
use std::io::{Read, Seek};

const MAX_ROWS: usize = 200_000;
const MAX_WIDTH: usize = 8192;
const MAX_BYTES: usize = 512 * 1024 * 1024;
const MAX_CHUNK: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    Tensor(Error),
    Float(safetensors::Error),
    Quantized(quantized::Error),
}

impl From<Error> for LoadError {
    fn from(error: Error) -> Self {
        Self::Tensor(error)
    }
}

#[derive(Debug)]
pub struct QuantizedTable {
    shape: [usize; 2],
    block_size: usize,
    codes: Vec<i8>,
    scales: Vec<f32>,
}

impl QuantizedTable {
    fn allocate(shape: &[u64], block_size: usize, budget: usize) -> Result<Self, LoadError> {
        let &[rows, width] = shape else {
            return Err(Error::Shape.into());
        };
        let rows = usize::try_from(rows).map_err(|_| Error::Limit)?;
        let width = usize::try_from(width).map_err(|_| Error::Limit)?;
        if rows == 0 || width == 0 || block_size == 0 || !width.is_multiple_of(block_size) {
            return Err(Error::Shape.into());
        }
        if rows > MAX_ROWS || width > MAX_WIDTH || block_size > MAX_BLOCK {
            return Err(Error::Limit.into());
        }
        let count = rows.checked_mul(width).ok_or(Error::Limit)?;
        let blocks = count / block_size;
        let bytes = blocks
            .checked_mul(4)
            .and_then(|v| v.checked_add(count))
            .ok_or(Error::Limit)?;
        if bytes > budget.min(MAX_BYTES) {
            return Err(Error::Limit.into());
        }
        let mut codes = Vec::new();
        codes
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        Ok(Self {
            shape: [rows, width],
            block_size,
            codes,
            scales: buffer(blocks)?,
        })
    }

    /// Convert at most 1 Mi FP32 elements per read, aligned to row boundaries.
    pub fn from_fp32<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
        block_size: usize,
        budget: usize,
    ) -> Result<Self, LoadError> {
        let info = reader.tensors().get(name).ok_or(Error::Index)?;
        if info.dtype != Dtype::F32 {
            return Err(Error::Shape.into());
        }
        let mut table = Self::allocate(&info.shape, block_size, budget)?;
        let [rows, width] = table.shape;
        let rows_per_chunk = MAX_CHUNK / width;
        for start in (0..rows).step_by(rows_per_chunk) {
            let count = rows_per_chunk.min(rows - start) * width;
            let data = reader
                .read_f32(name, (start * width) as u64, count)
                .map_err(LoadError::Float)?;
            let q = BlockInt8::quantize(&data, block_size, count + count / block_size * 4)?;
            table.codes.extend_from_slice(q.codes());
            table.scales.extend_from_slice(q.scales());
        }
        Ok(table)
    }

    /// Read all packed blocks, each semantically checked by the RPQ8 reader.
    pub fn load<R: Read + Seek>(
        reader: &mut quantized::Reader<R>,
        name: &str,
        budget: usize,
    ) -> Result<Self, LoadError> {
        let info = reader.tensors().get(name).ok_or(Error::Index)?;
        let size = info.block_size as usize;
        let mut table = Self::allocate(&info.shape, size, budget)?;
        table.codes.resize(info.elements as usize, 0);
        table.scales.resize(info.blocks as usize, 0.);
        reader
            .read_tensor_into(name, &mut table.codes, &mut table.scales)
            .map_err(LoadError::Quantized)?;
        Ok(table)
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn payload_bytes(&self) -> usize {
        self.codes.len() + 4 * self.scales.len()
    }

    pub fn block_count(&self) -> usize {
        self.scales.len()
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Borrow validated packed row data without dequantization or allocation.
    pub(crate) fn packed_row(&self, id: usize) -> Result<(&[i8], &[f32]), Error> {
        let [rows, width] = self.shape;
        if id >= rows {
            return Err(Error::Index);
        }
        let start = id * width;
        let first = start / self.block_size;
        Ok((
            &self.codes[start..start + width],
            &self.scales[first..first + width / self.block_size],
        ))
    }

    /// Borrow packed weights directly for writing, no full-table copy.
    pub fn as_input<'a>(&'a self, name: &'a str, shape: &'a [u64]) -> TensorInput<'a> {
        TensorInput {
            name,
            shape,
            block_size: self.block_size as u32,
            codes: &self.codes,
            scales: &self.scales,
        }
    }

    /// Caller provides exactly one row of scratch space.
    pub fn row_into(&self, id: usize, out: &mut [f32]) -> Result<(), Error> {
        let [rows, width] = self.shape;
        if id >= rows {
            return Err(Error::Index);
        }
        if out.len() != width {
            return Err(Error::Shape);
        }
        let start = id * width;
        let first = start / self.block_size;
        for (index, (codes, dest)) in self.codes[start..start + width]
            .chunks_exact(self.block_size)
            .zip(out.chunks_exact_mut(self.block_size))
            .enumerate()
        {
            let scale = self.scales[first + index];
            for (&code, dst) in codes.iter().zip(dest) {
                *dst = f32::from(code) * scale;
            }
        }
        Ok(())
    }

    pub fn embedding(&self, ids: &[u32], scale: f32) -> Result<Tensor, Error> {
        if !scale.is_finite() {
            return Err(Error::NonFinite);
        }
        let width = self.shape[1];
        let (_, count) = layout(&[ids.len(), width])?;
        let mut data = buffer(count)?;
        data.resize(count, 0.);
        for (&id, row) in ids.iter().zip(data.chunks_exact_mut(width)) {
            self.row_into(id as usize, row)?;
            for v in row {
                *v *= scale;
            }
        }
        Tensor::new(&[ids.len(), width], data)
    }

    /// Tied FP32 accumulation over one dequantized row at a time.
    /// This is not yet an INT8 integer matrix-multiplication kernel.
    pub fn project(
        &self,
        hidden: &Tensor,
        bias: Option<&Tensor>,
        last_only: bool,
    ) -> Result<Tensor, Error> {
        let &[rows, width] = hidden.shape() else {
            return Err(Error::Shape);
        };
        let [vocab, features] = self.shape;
        if rows == 0 || features != width || bias.is_some_and(|b| b.shape() != [vocab]) {
            return Err(Error::Shape);
        }
        let count = if last_only { 1 } else { rows };
        let (_, size) = layout(&[count, vocab])?;
        if size.checked_mul(width).ok_or(Error::Limit)? > 1 << 30 {
            return Err(Error::Limit);
        }
        let mut data = buffer(size)?;
        let mut scratch = buffer(width)?;
        scratch.resize(width, 0.);
        let start = if last_only { (rows - 1) * width } else { 0 };
        for row in hidden.data()[start..].chunks_exact(width) {
            for id in 0..vocab {
                self.row_into(id, &mut scratch)?;
                let mut sum = 0_f32;
                for (&x, &y) in row.iter().zip(&scratch) {
                    sum += x * y;
                }
                if let Some(bias) = bias {
                    sum += bias.data()[id];
                }
                data.push(sum);
            }
        }
        Tensor::new(&[count, vocab], data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn packed_rows_and_tied_projection() {
        let mut file = Cursor::new(Vec::new());
        quantized::write_container(
            &mut file,
            &[TensorInput {
                name: "shared",
                shape: &[2, 4],
                block_size: 2,
                codes: &[2, -2, 4, 0, 1, 3, 2, -4],
                scales: &[0.5, 0.25, 1., 0.5],
            }],
            1024,
        )
        .unwrap();
        let mut reader = quantized::Reader::open(file, 1024).unwrap();
        assert_eq!(
            QuantizedTable::load(&mut reader, "shared", 23).err(),
            Some(Error::Limit.into())
        );
        let table = QuantizedTable::load(&mut reader, "shared", 24).unwrap();
        assert_eq!(table.payload_bytes(), 24);
        assert_eq!(table.block_count(), 4);
        let embedded = table.embedding(&[1, 0], 2.).unwrap();
        assert_eq!(embedded.data(), [2., 6., 2., -4., 2., -2., 2., 0.]);
        assert_eq!(table.embedding(&[2], 1.).err(), Some(Error::Index));
        let hidden = Tensor::new(&[2, 4], vec![1.; 8]).unwrap();
        let bias = Tensor::new(&[2], vec![0.5, -0.5]).unwrap();
        let full = table.project(&hidden, Some(&bias), false).unwrap();
        assert_eq!(full.data(), [1.5, 2.5, 1.5, 2.5]);
        let last = table.project(&hidden, Some(&bias), true).unwrap();
        assert_eq!(last.data(), &full.data()[2..]);
        assert_eq!(table.row_into(0, &mut [0.; 3]), Err(Error::Shape));
    }

    #[test]
    fn rejects_unaligned_and_over_budget_shapes() {
        for shape in [&[2, 3][..], &[0, 4], &[2, 4, 1]] {
            assert!(QuantizedTable::allocate(shape, 2, 1024).is_err());
        }
        assert!(QuantizedTable::allocate(&[65001, 512], 64, 35_360_543).is_err());
    }
}
