//! RPQ8 v1: bounded little-endian indexed symmetric INT8 container.
//! SHA256 detects corruption, not publisher authenticity. Stable input required.
use crate::sha256::digest_reader;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Seek, SeekFrom, Write},
};

const MAGIC: &[u8; 8] = b"RPQ8BIN\0";
const HEADER: u64 = 32;
const TRAILER: u64 = 64;
const MAX_FILE: u64 = 512 * 1024 * 1024;
const MAX_INDEX: u64 = 1024 * 1024;
const MAX_TENSORS: usize = 4096;
const MAX_ELEMENTS: u64 = 128 * 1024 * 1024;
const MAX_BLOCK: u32 = 4096;
const BATCH_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
    Format,
    Limit,
    Integrity,
    Index,
    Block,
    Allocation,
}

fn read<const N: usize>(r: &mut impl Read) -> Result<[u8; N], Error> {
    let mut out = [0; N];
    r.read_exact(&mut out).map_err(|_| Error::Io)?;
    Ok(out)
}

fn u32le(r: &mut impl Read) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(read(r)?))
}

fn u64le(r: &mut impl Read) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(read(r)?))
}

fn write_bytes(w: &mut impl Write, bytes: &[u8]) -> Result<(), Error> {
    w.write_all(bytes).map_err(|_| Error::Io)
}

fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
}

fn inventory(shape: &[u64], block: u32) -> Result<(u64, u64, u64), Error> {
    if shape.is_empty() || shape.len() > 8 || shape.contains(&0) || block == 0 || block > MAX_BLOCK
    {
        return Err(Error::Format);
    }
    let mut elements = 1_u64;
    for &dim in shape {
        elements = elements
            .checked_mul(dim)
            .filter(|&v| v <= MAX_ELEMENTS)
            .ok_or(Error::Limit)?;
    }
    let blocks = elements.div_ceil(u64::from(block));
    let bytes = elements
        .checked_add(blocks.checked_mul(4).ok_or(Error::Limit)?)
        .ok_or(Error::Limit)?;
    Ok((elements, blocks, bytes))
}

fn validate_block(scale: f32, codes: &[i8]) -> Result<(), Error> {
    if !scale.is_finite() || scale <= 0.0 || codes.contains(&i8::MIN) {
        return Err(Error::Block);
    }
    let peak = codes.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    if f64::from(scale) * f64::from(peak) > f64::from(f32::MAX) {
        return Err(Error::Block);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorInfo {
    pub shape: Vec<u64>,
    pub block_size: u32,
    pub elements: u64,
    pub blocks: u64,
    pub offset: u64,
    pub bytes: u64,
}

#[derive(Debug, PartialEq)]
pub struct Block {
    pub scale: f32,
    pub codes: Vec<i8>,
}

/// A verified index. Block semantics are checked before each block is returned.
/// This is an INT8-only container, not yet a full mixed-precision model package.
pub struct Reader<R> {
    inner: R,
    tensors: BTreeMap<String, TensorInfo>,
    payload_start: u64,
}

impl<R: Read + Seek> Reader<R> {
    pub fn open(mut inner: R, max_bytes: u64) -> Result<Self, Error> {
        let len = inner.seek(SeekFrom::End(0)).map_err(|_| Error::Io)?;
        if len > max_bytes.min(MAX_FILE) {
            return Err(Error::Limit);
        }
        if len < HEADER + TRAILER {
            return Err(Error::Format);
        }
        inner.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
        if &read::<8>(&mut inner)? != MAGIC || u32le(&mut inner)? != 1 {
            return Err(Error::Format);
        }
        let count = u32le(&mut inner)? as usize;
        let index_bytes = u64le(&mut inner)?;
        let payload_bytes = u64le(&mut inner)?;
        if count == 0 || count > MAX_TENSORS || index_bytes > MAX_INDEX {
            return Err(Error::Limit);
        }
        let payload_start = HEADER.checked_add(index_bytes).ok_or(Error::Limit)?;
        let prefix = payload_start
            .checked_add(payload_bytes)
            .ok_or(Error::Limit)?;
        if prefix.checked_add(TRAILER) != Some(len) {
            return Err(Error::Format);
        }
        inner.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
        let (hashed, digest) =
            digest_reader((&mut inner).take(prefix), prefix).map_err(|_| Error::Io)?;
        let expected = read::<64>(&mut inner)?;
        if hashed != prefix || expected.as_slice() != digest.as_bytes() {
            return Err(Error::Integrity);
        }
        inner.seek(SeekFrom::Start(HEADER)).map_err(|_| Error::Io)?;
        let mut index = Vec::new();
        index
            .try_reserve_exact(index_bytes as usize)
            .map_err(|_| Error::Allocation)?;
        index.resize(index_bytes as usize, 0);
        inner.read_exact(&mut index).map_err(|_| Error::Io)?;
        let mut cursor = Cursor::new(index);
        let mut tensors = BTreeMap::new();
        let mut next = 0_u64;
        let mut previous = String::new();
        for _ in 0..count {
            let name_len = u16::from_le_bytes(read(&mut cursor)?) as usize;
            let [rank, reserved] = read(&mut cursor)?;
            let block_size = u32le(&mut cursor)?;
            let offset = u64le(&mut cursor)?;
            let bytes = u64le(&mut cursor)?;
            if name_len == 0 || name_len > 255 || rank == 0 || rank > 8 || reserved != 0 {
                return Err(Error::Format);
            }
            let mut shape = Vec::with_capacity(rank as usize);
            for _ in 0..rank {
                shape.push(u64le(&mut cursor)?);
            }
            let mut name = vec![0; name_len];
            cursor.read_exact(&mut name).map_err(|_| Error::Io)?;
            let name = String::from_utf8(name).map_err(|_| Error::Format)?;
            let (elements, blocks, required) = inventory(&shape, block_size)?;
            if !name_ok(&name) || name <= previous || offset != next || bytes != required {
                return Err(Error::Index);
            }
            next = next.checked_add(bytes).ok_or(Error::Limit)?;
            if next > payload_bytes {
                return Err(Error::Index);
            }
            previous = name.clone();
            tensors.insert(
                name,
                TensorInfo {
                    shape,
                    block_size,
                    elements,
                    blocks,
                    offset,
                    bytes,
                },
            );
        }
        if cursor.position() != index_bytes || next != payload_bytes {
            return Err(Error::Index);
        }
        Ok(Self {
            inner,
            tensors,
            payload_start,
        })
    }

    pub fn tensors(&self) -> &BTreeMap<String, TensorInfo> {
        &self.tensors
    }

    /// One block at a time; no full tensor allocation.
    pub fn read_block(&mut self, name: &str, block: u64, budget: usize) -> Result<Block, Error> {
        let info = self.tensors.get(name).ok_or(Error::Index)?;
        if block >= info.blocks {
            return Err(Error::Index);
        }
        let size = u64::from(info.block_size);
        let start = block.checked_mul(size).ok_or(Error::Limit)?;
        let count = (info.elements - start).min(size) as usize;
        if budget < count + 4 {
            return Err(Error::Limit);
        }
        let offset = self
            .payload_start
            .checked_add(info.offset)
            .and_then(|v| v.checked_add(block.checked_mul(size + 4)?))
            .ok_or(Error::Limit)?;
        self.inner
            .seek(SeekFrom::Start(offset))
            .map_err(|_| Error::Io)?;
        let scale = f32::from_le_bytes(read(&mut self.inner)?);
        let mut raw = [0_u8; MAX_BLOCK as usize];
        self.inner
            .read_exact(&mut raw[..count])
            .map_err(|_| Error::Io)?;
        let mut codes = Vec::new();
        codes
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        codes.extend(raw[..count].iter().map(|&v| v as i8));
        validate_block(scale, &codes)?;
        Ok(Block { scale, codes })
    }

    /// Read one complete tensor with a single seek and bounded batch scratch.
    /// Caller supplies exact-sized output arrays. On error discard both arrays:
    /// a prefix may have been written. Every block uses the original validation.
    pub fn read_tensor_into(
        &mut self,
        name: &str,
        codes: &mut [i8],
        scales: &mut [f32],
    ) -> Result<(), Error> {
        let info = self.tensors.get(name).ok_or(Error::Index)?;
        if codes.len() as u64 != info.elements || scales.len() as u64 != info.blocks {
            return Err(Error::Limit);
        }
        let size = info.block_size as usize;
        let record_size = size + 4;
        let offset = self
            .payload_start
            .checked_add(info.offset)
            .ok_or(Error::Limit)?;
        self.inner
            .seek(SeekFrom::Start(offset))
            .map_err(|_| Error::Io)?;
        let mut raw = [0_u8; BATCH_BYTES];
        let mut block = 0;
        let mut at = 0;
        while block < scales.len() {
            let records = (raw.len() / record_size).min(scales.len() - block);
            let count = (records * size).min(codes.len() - at);
            let bytes = count + records * 4;
            self.inner
                .read_exact(&mut raw[..bytes])
                .map_err(|_| Error::Io)?;
            for (record, scale) in raw[..bytes]
                .chunks(record_size)
                .zip(&mut scales[block..block + records])
            {
                let (header, data) = record.split_at(4);
                let value = f32::from_le_bytes(header.try_into().map_err(|_| Error::Format)?);
                let dest = &mut codes[at..at + data.len()];
                for (code, &byte) in dest.iter_mut().zip(data) {
                    *code = byte as i8;
                }
                validate_block(value, dest)?;
                *scale = value;
                at += data.len();
            }
            block += records;
        }
        Ok(())
    }
}

impl Reader<Cursor<&[u8]>> {
    /// Decode from a verified immutable slice into final arrays. Discard both
    /// arrays on error. No intermediate payload buffer is required.
    pub fn read_tensor_slice_into(
        &self,
        name: &str,
        codes: &mut [i8],
        scales: &mut [f32],
    ) -> Result<(), Error> {
        let info = self.tensors.get(name).ok_or(Error::Index)?;
        if codes.len() as u64 != info.elements || scales.len() as u64 != info.blocks {
            return Err(Error::Limit);
        }
        let start = self
            .payload_start
            .checked_add(info.offset)
            .ok_or(Error::Limit)?;
        let end = start.checked_add(info.bytes).ok_or(Error::Limit)?;
        let start = usize::try_from(start).map_err(|_| Error::Limit)?;
        let end = usize::try_from(end).map_err(|_| Error::Limit)?;
        let raw = self.inner.get_ref().get(start..end).ok_or(Error::Io)?;
        let size = info.block_size as usize;
        for ((record, dest), scale) in raw
            .chunks(size + 4)
            .zip(codes.chunks_mut(size))
            .zip(scales)
        {
            let (header, data) = record.split_at(4);
            let value = f32::from_le_bytes(header.try_into().map_err(|_| Error::Format)?);
            for (code, &byte) in dest.iter_mut().zip(data) {
                *code = byte as i8;
            }
            validate_block(value, dest)?;
            *scale = value;
        }
        Ok(())
    }
}

/// Borrowed already-quantized data. Writer accepts sorted canonical names only.
pub struct TensorInput<'a> {
    pub name: &'a str,
    pub shape: &'a [u64],
    pub block_size: u32,
    pub codes: &'a [i8],
    pub scales: &'a [f32],
}

/// Write to an empty seekable read/write destination. Validate all input
/// before writing. On IO error caller must discard the partial destination.
pub fn write_container<W: Read + Write + Seek>(
    out: &mut W,
    tensors: &[TensorInput<'_>],
    max_bytes: u64,
) -> Result<u64, Error> {
    if tensors.is_empty() || tensors.len() > MAX_TENSORS {
        return Err(Error::Limit);
    }
    if out.seek(SeekFrom::End(0)).map_err(|_| Error::Io)? != 0 {
        return Err(Error::Format);
    }
    let mut index = Vec::new();
    let mut payload = 0_u64;
    let mut previous = "";
    for t in tensors {
        let (elements, blocks, bytes) = inventory(t.shape, t.block_size)?;
        if !name_ok(t.name)
            || t.name <= previous
            || t.codes.len() as u64 != elements
            || t.scales.len() as u64 != blocks
        {
            return Err(Error::Index);
        }
        for (&scale, codes) in t.scales.iter().zip(t.codes.chunks(t.block_size as usize)) {
            validate_block(scale, codes)?;
        }
        let extra = 24 + 8 * t.shape.len() + t.name.len();
        if index.len() + extra > MAX_INDEX as usize {
            return Err(Error::Limit);
        }
        index.try_reserve(extra).map_err(|_| Error::Allocation)?;
        index.extend_from_slice(&(t.name.len() as u16).to_le_bytes());
        index.extend_from_slice(&[t.shape.len() as u8, 0]);
        index.extend_from_slice(&t.block_size.to_le_bytes());
        index.extend_from_slice(&payload.to_le_bytes());
        index.extend_from_slice(&bytes.to_le_bytes());
        for &dim in t.shape {
            index.extend_from_slice(&dim.to_le_bytes());
        }
        index.extend_from_slice(t.name.as_bytes());
        payload = payload.checked_add(bytes).ok_or(Error::Limit)?;
        previous = t.name;
    }
    let prefix = HEADER
        .checked_add(index.len() as u64)
        .and_then(|v| v.checked_add(payload))
        .ok_or(Error::Limit)?;
    let total = prefix.checked_add(TRAILER).ok_or(Error::Limit)?;
    if total > max_bytes.min(MAX_FILE) {
        return Err(Error::Limit);
    }
    write_bytes(out, MAGIC)?;
    write_bytes(out, &1_u32.to_le_bytes())?;
    write_bytes(out, &(tensors.len() as u32).to_le_bytes())?;
    write_bytes(out, &(index.len() as u64).to_le_bytes())?;
    write_bytes(out, &payload.to_le_bytes())?;
    write_bytes(out, &index)?;
    let mut raw = [0_u8; MAX_BLOCK as usize];
    for t in tensors {
        for (&scale, codes) in t.scales.iter().zip(t.codes.chunks(t.block_size as usize)) {
            write_bytes(out, &scale.to_le_bytes())?;
            for (dst, &src) in raw.iter_mut().zip(codes) {
                *dst = src as u8;
            }
            write_bytes(out, &raw[..codes.len()])?;
        }
    }
    out.flush().map_err(|_| Error::Io)?;
    out.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
    let (hashed, digest) =
        digest_reader((&mut *out).take(prefix), prefix).map_err(|_| Error::Io)?;
    if hashed != prefix {
        return Err(Error::Io);
    }
    write_bytes(out, digest.as_bytes())?;
    out.flush().map_err(|_| Error::Io)?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        write_container(
            &mut out,
            &[
                TensorInput {
                    name: "a.weight",
                    shape: &[2, 3],
                    block_size: 4,
                    codes: &[127, -127, 1, 0, 2, -3],
                    scales: &[0.5, 0.25],
                },
                TensorInput {
                    name: "b.weight",
                    shape: &[1],
                    block_size: 1,
                    codes: &[0],
                    scales: &[1.],
                },
            ],
            1024,
        )
        .unwrap();
        out.into_inner()
    }

    #[test]
    fn roundtrip_and_tail() {
        let data = fixture();
        let mut reader = Reader::open(Cursor::new(&data), 1024).unwrap();
        assert_eq!(reader.tensors()["a.weight"].elements, 6);
        assert_eq!(reader.tensors()["a.weight"].blocks, 2);
        let block = reader.read_block("a.weight", 1, 6).unwrap();
        assert_eq!(block.scale, 0.25);
        assert_eq!(block.codes, [2, -3]);
        assert_eq!(reader.read_block("a.weight", 1, 5), Err(Error::Limit));
        assert_eq!(reader.read_block("a.weight", 2, 100), Err(Error::Index));
        assert_eq!(reader.read_block("missing", 0, 100), Err(Error::Index));
    }

    #[test]
    fn batch_matches_reference_across_chunks_and_tail() {
        let codes = vec![-127_i8; 70_003];
        let scales = vec![0.25_f32; codes.len().div_ceil(64)];
        let mut file = Cursor::new(Vec::new());
        write_container(
            &mut file,
            &[TensorInput {
                name: "large",
                shape: &[codes.len() as u64],
                block_size: 64,
                codes: &codes,
                scales: &scales,
            }],
            200_000,
        )
        .unwrap();
        let mut reader = Reader::open(file, 200_000).unwrap();
        let mut actual = vec![0; codes.len()];
        let mut factors = vec![0.; scales.len()];
        reader
            .read_tensor_into("large", &mut actual, &mut factors)
            .unwrap();
        assert_eq!(actual, codes);
        assert_eq!(factors, scales);
        for (index, expected) in actual.chunks(64).enumerate() {
            let block = reader.read_block("large", index as u64, 68).unwrap();
            assert_eq!(block.codes, expected);
            assert_eq!(block.scale.to_bits(), factors[index].to_bits());
        }
    }

    #[test]
    fn batch_rejects_lengths_names_and_invalid_blocks() {
        let data = fixture();
        let mut reader = Reader::open(Cursor::new(data.clone()), 1024).unwrap();
        assert_eq!(
            reader.read_tensor_into("a.weight", &mut [0; 5], &mut [0.; 2]),
            Err(Error::Limit)
        );
        assert_eq!(
            reader.read_tensor_into("a.weight", &mut [0; 6], &mut [0.; 1]),
            Err(Error::Limit)
        );
        assert_eq!(
            reader.read_tensor_into("missing", &mut [], &mut []),
            Err(Error::Index)
        );
        let start = reader.payload_start as usize;
        for (offset, bytes) in [
            (start, 0_f32.to_le_bytes().to_vec()),
            (start + 4, vec![128]),
            (start + 8, f32::MAX.to_le_bytes().to_vec()),
        ] {
            let mut bad = data.clone();
            bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
            let prefix = bad.len() - TRAILER as usize;
            let (_, hash) = digest_reader(&bad[..prefix], prefix as u64).unwrap();
            bad[prefix..].copy_from_slice(hash.as_bytes());
            let mut reader = Reader::open(Cursor::new(bad), 1024).unwrap();
            assert_eq!(
                reader.read_tensor_into("a.weight", &mut [0; 6], &mut [0.; 2]),
                Err(Error::Block)
            );
        }
    }

    #[test]
    fn direct_slice_decode_and_tail() {
        let data = fixture();
        let reader = Reader::open(Cursor::new(data.as_slice()), 1024).unwrap();
        let mut codes = [0; 6];
        let mut scales = [0.; 2];
        reader
            .read_tensor_slice_into("a.weight", &mut codes, &mut scales)
            .unwrap();
        assert_eq!(codes, [127, -127, 1, 0, 2, -3]);
        assert_eq!(scales, [0.5, 0.25]);
        assert_eq!(
            reader.read_tensor_slice_into("a.weight", &mut codes, &mut [0.]),
            Err(Error::Limit)
        );
    }

    #[test]
    fn malformed_file_and_budget() {
        let data = fixture();
        assert_eq!(
            Reader::open(Cursor::new(&data), data.len() as u64 - 1).err(),
            Some(Error::Limit)
        );
        for end in [0, 31, 96, data.len() - 1] {
            assert!(Reader::open(Cursor::new(&data[..end]), 1024).is_err());
        }
        let mut bad = data.clone();
        bad[40] ^= 1;
        assert_eq!(
            Reader::open(Cursor::new(bad), 1024).err(),
            Some(Error::Integrity)
        );
        let mut bad = data;
        bad.push(0);
        assert!(Reader::open(Cursor::new(bad), 1024).is_err());
    }

    #[test]
    fn rejects_invalid_writer_input_before_writing() {
        let mut out = Cursor::new(Vec::new());
        for (name, code, scale) in [
            ("../x", 1, 1.),
            ("x", -128, 1.),
            ("x", 1, 0.),
            ("x", 127, f32::MAX),
        ] {
            assert!(write_container(
                &mut out,
                &[TensorInput {
                    name,
                    shape: &[1],
                    block_size: 1,
                    codes: &[code],
                    scales: &[scale],
                }],
                1024,
            )
            .is_err());
            assert!(out.get_ref().is_empty());
        }
    }
}
