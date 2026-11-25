//! Bounded row-block storage for large FP32 vocabulary weights.
use crate::tensor::{layout, Error, Tensor};
use rpmodel::safetensors::{Error as ReaderError, SafeTensorReader};
use std::io::{Read, Seek};

const MAX_ROWS: usize = 200_000;
const MAX_WIDTH: usize = 8192;
const MAX_BLOCK_ELEMENTS: usize = 1024 * 1024;
const MAX_TABLE_BYTES: usize = 512 * 1024 * 1024;

mod private {
    pub trait Sealed {}
}

/// Immutable row-major vocabulary view, implemented only by validated storage.
pub trait RowTable: private::Sealed {
    fn shape(&self) -> &[usize];
    fn row(&self, index: usize) -> Result<&[f32], Error>;
}

impl private::Sealed for Tensor {}
impl RowTable for Tensor {
    fn shape(&self) -> &[usize] {
        self.shape()
    }

    fn row(&self, index: usize) -> Result<&[f32], Error> {
        let &[rows, width] = self.shape() else {
            return Err(Error::Shape);
        };
        if width == 0 || index >= rows {
            return Err(Error::Index);
        }
        Ok(&self.data()[index * width..(index + 1) * width])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    Tensor(Error),
    Reader(ReaderError),
}

impl From<Error> for LoadError {
    fn from(value: Error) -> Self {
        Self::Tensor(value)
    }
}

impl From<ReaderError> for LoadError {
    fn from(value: ReaderError) -> Self {
        Self::Reader(value)
    }
}

/// Each block is a normal bounded Tensor; the logical table may exceed the
/// single Tensor limit. The table owns one copy of every vocabulary row.
#[derive(Debug)]
pub struct ChunkedTable {
    shape: [usize; 2],
    rows_per_block: usize,
    blocks: Vec<Tensor>,
}

impl ChunkedTable {
    /// Load sequential blocks, never requesting more than 1M elements.
    /// max_bytes is a payload budget, excluding temporary reader allocations.
    pub fn from_rows(
        rows: usize,
        width: usize,
        rows_per_block: usize,
        max_bytes: usize,
        mut read: impl FnMut(usize, usize) -> Result<Vec<f32>, LoadError>,
    ) -> Result<Self, LoadError> {
        if rows == 0 || width == 0 || rows_per_block == 0 {
            return Err(Error::Shape.into());
        }
        if rows > MAX_ROWS
            || width > MAX_WIDTH
            || rows_per_block > rows
            || rows_per_block.checked_mul(width).ok_or(Error::Limit)? > MAX_BLOCK_ELEMENTS
        {
            return Err(Error::Limit.into());
        }
        let bytes = rows
            .checked_mul(width)
            .and_then(|n| n.checked_mul(4))
            .ok_or(Error::Limit)?;
        if bytes > max_bytes || bytes > MAX_TABLE_BYTES {
            return Err(Error::Limit.into());
        }
        let mut blocks = Vec::new();
        blocks
            .try_reserve_exact(rows.div_ceil(rows_per_block))
            .map_err(|_| Error::Allocation)?;
        for start in (0..rows).step_by(rows_per_block) {
            let count = rows_per_block.min(rows - start);
            layout(&[count, width])?;
            blocks.push(Tensor::new(&[count, width], read(start, count)?)?);
        }
        Ok(Self {
            shape: [rows, width],
            rows_per_block,
            blocks,
        })
    }

    /// Reuse the autonomous Safetensors reader's checked contiguous slices.
    pub fn load<R: Read + Seek>(
        reader: &mut SafeTensorReader<R>,
        name: &str,
        rows_per_block: usize,
        max_bytes: usize,
    ) -> Result<Self, LoadError> {
        let info = reader
            .tensors()
            .get(name)
            .ok_or(ReaderError::MissingTensor)?;
        let &[rows, width] = info.shape.as_slice() else {
            return Err(Error::Shape.into());
        };
        let rows = usize::try_from(rows).map_err(|_| Error::Limit)?;
        let width = usize::try_from(width).map_err(|_| Error::Limit)?;
        Self::from_rows(rows, width, rows_per_block, max_bytes, |start, count| {
            Ok(reader.read_f32(name, (start * width) as u64, count * width)?)
        })
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn payload_bytes(&self) -> usize {
        self.shape[0] * self.shape[1] * 4
    }
}

impl private::Sealed for ChunkedTable {}
impl RowTable for ChunkedTable {
    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn row(&self, index: usize) -> Result<&[f32], Error> {
        if index >= self.shape[0] {
            return Err(Error::Index);
        }
        self.blocks[index / self.rows_per_block].row(index % self.rows_per_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{seq2seq::OutputHead, transformer::embedding};
    use std::io::Cursor;

    #[test]
    fn actual_safetensors_slice_loading_and_shared_projection() {
        // Valid F32 Safetensors, five rows with a partial final block.
        let header = r#"{"table":{"dtype":"F32","shape":[5,2],"data_offsets":[0,40]}}"#;
        let values = [1_f32, 2., 3., 4., 5., 6., 7., 8., 9., 10.];
        let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(header.as_bytes());
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let mut reader = SafeTensorReader::open(Cursor::new(bytes)).unwrap();
        let table = ChunkedTable::load(&mut reader, "table", 2, 40).unwrap();
        assert_eq!(table.shape(), &[5, 2]);
        assert_eq!(table.block_count(), 3);
        assert_eq!(table.payload_bytes(), 40);
        assert_eq!(table.row(4), Ok(&[9., 10.][..]));
        assert_eq!(table.row(5), Err(Error::Index));
        assert_eq!(
            embedding(&table, &[4, 1, 2, 0], 2.).unwrap().data(),
            &[18., 20., 6., 8., 10., 12., 2., 4.]
        );
        let dense = Tensor::new(&[5, 2], values.to_vec()).unwrap();
        let hidden = Tensor::new(&[2, 2], vec![2., -1., -1., 2.]).unwrap();
        let bias = Tensor::new(&[5], vec![0., 1., 2., 3., 4.]).unwrap();
        let head = OutputHead {
            weight: None,
            bias: Some(&bias),
        };
        let all = head.project(&hidden, &table, false).unwrap();
        assert_eq!(all.data(), &[0., 3., 6., 9., 12., 3., 6., 9., 12., 15.]);
        assert_eq!(all, head.project(&hidden, &dense, false).unwrap());

        assert_eq!(
            ChunkedTable::load(&mut reader, "table", 2, 39).unwrap_err(),
            LoadError::Tensor(Error::Limit)
        );
    }

    #[test]
    fn limits_and_invalid_reader_results() {
        let fail = |_, _| panic!("must reject before reading");
        assert_eq!(
            ChunkedTable::from_rows(0, 2, 1, 100, fail).unwrap_err(),
            LoadError::Tensor(Error::Shape)
        );
        assert_eq!(
            ChunkedTable::from_rows(200_001, 2, 1, 100, fail).unwrap_err(),
            LoadError::Tensor(Error::Limit)
        );
        assert_eq!(
            ChunkedTable::from_rows(5000, 512, 3000, usize::MAX, fail).unwrap_err(),
            LoadError::Tensor(Error::Limit)
        );
        assert_eq!(
            ChunkedTable::from_rows(5, 2, 2, 39, fail).unwrap_err(),
            LoadError::Tensor(Error::Limit)
        );
        assert_eq!(
            ChunkedTable::from_rows(5, 2, 2, 40, |_, _| Ok(vec![1.])).unwrap_err(),
            LoadError::Tensor(Error::Shape)
        );
        assert_eq!(
            ChunkedTable::from_rows(5, 2, 2, 40, |_, n| Ok(vec![f32::NAN; n * 2])).unwrap_err(),
            LoadError::Tensor(Error::NonFinite)
        );
    }

    #[test]
    fn opus_shape_without_relaxing_tensor_limit() {
        const ROWS: usize = 65_001;
        const WIDTH: usize = 512;
        const BYTES: usize = ROWS * WIDTH * 4;
        assert_eq!(Tensor::zeros(&[ROWS, WIDTH]), Err(Error::Limit));
        let table = ChunkedTable::from_rows(ROWS, WIDTH, 2048, BYTES, |start, count| {
            let mut values = vec![0.; count * WIDTH];
            for i in 0..count {
                let id = (start + i) as f32;
                values[i * WIDTH] = id;
                values[(i + 1) * WIDTH - 1] = -id;
            }
            Ok(values)
        })
        .unwrap();
        assert_eq!(table.block_count(), 32);
        assert_eq!(table.payload_bytes(), 133_122_048);
        for id in [0, 2047, 2048, 4096, 65_000] {
            assert_eq!(table.row(id).unwrap()[0], id as f32);
        }
        let mut values = vec![0.; WIDTH];
        values[0] = 2.;
        values[WIDTH - 1] = 1.;
        let hidden = Tensor::new(&[1, WIDTH], values).unwrap();
        let head = OutputHead {
            weight: None,
            bias: None,
        };
        let logits = head.project(&hidden, &table, true).unwrap();
        assert_eq!(logits.shape(), &[1, ROWS]);
        for (id, &value) in logits.data().iter().enumerate() {
            assert_eq!(value, id as f32);
        }
    }
}
