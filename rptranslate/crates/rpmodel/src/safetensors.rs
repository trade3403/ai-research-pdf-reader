//! Bounded, dependency-free reader for the F32/F16/BF16 Safetensors subset.
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

const MAX_HEADER: u64 = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const MAX_RANK: usize = 16;
const MAX_READ_ELEMENTS: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
    Header,
    Limit,
    Duplicate,
    UnsupportedDtype,
    Shape,
    Offset,
    MissingTensor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dtype {
    F32,
    F16,
    BF16,
}

impl Dtype {
    fn width(self) -> u64 {
        match self {
            Self::F32 => 4,
            Self::F16 | Self::BF16 => 2,
        }
    }

    fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "F32" => Ok(Self::F32),
            "F16" => Ok(Self::F16),
            "BF16" => Ok(Self::BF16),
            _ => Err(Error::UnsupportedDtype),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorInfo {
    pub dtype: Dtype,
    pub shape: Vec<u64>,
    pub offsets: [u64; 2],
    pub elements: u64,
}

/// A restricted JSON grammar: objects, strings and unsigned integer arrays.
/// No recursion into arbitrary JSON; duplicate keys are rejected.
pub(crate) struct Parser<'a> {
    pub(crate) rest: &'a str,
}

impl<'a> Parser<'a> {
    pub(crate) fn ws(&mut self) {
        self.rest = self.rest.trim_start_matches([' ', '\n', '\r', '\t']);
    }

    fn take(&mut self, ch: char) -> bool {
        self.ws();
        if let Some(rest) = self.rest.strip_prefix(ch) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), Error> {
        if self.take(ch) {
            Ok(())
        } else {
            Err(Error::Header)
        }
    }

    fn char(&mut self) -> Result<char, Error> {
        let ch = self.rest.chars().next().ok_or(Error::Header)?;
        self.rest = &self.rest[ch.len_utf8()..];
        Ok(ch)
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        let mut n = 0;
        for _ in 0..4 {
            n = n * 16 + self.char()?.to_digit(16).ok_or(Error::Header)?;
        }
        Ok(n)
    }

    pub(crate) fn string(&mut self) -> Result<String, Error> {
        self.expect('"')?;
        let mut s = String::new();
        loop {
            match self.char()? {
                '"' => return Ok(s),
                '\\' => {
                    let ch = match self.char()? {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'u' => {
                            let first = self.hex4()?;
                            let cp = if (0xd800..=0xdbff).contains(&first) {
                                if self.char()? != '\\' || self.char()? != 'u' {
                                    return Err(Error::Header);
                                }
                                let low = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(Error::Header);
                                }
                                0x10000 + ((first - 0xd800) << 10) + low - 0xdc00
                            } else {
                                first
                            };
                            char::from_u32(cp).ok_or(Error::Header)?
                        }
                        _ => return Err(Error::Header),
                    };
                    s.push(ch);
                }
                ch if ch < '\u{20}' => return Err(Error::Header),
                ch => s.push(ch),
            }
        }
    }

    pub(crate) fn number(&mut self) -> Result<u64, Error> {
        self.ws();
        let len = self.rest.bytes().take_while(u8::is_ascii_digit).count();
        let digits = &self.rest[..len];
        if digits.is_empty() || (len > 1 && digits.starts_with('0')) {
            return Err(Error::Header);
        }
        self.rest = &self.rest[len..];
        digits.parse().map_err(|_| Error::Limit)
    }

    fn numbers(&mut self, limit: usize) -> Result<Vec<u64>, Error> {
        self.expect('[')?;
        let mut v = Vec::new();
        if self.take(']') {
            return Ok(v);
        }
        loop {
            if v.len() == limit {
                return Err(Error::Limit);
            }
            v.push(self.number()?);
            if self.take(']') {
                return Ok(v);
            }
            self.expect(',')?;
        }
    }

    pub(crate) fn object<T>(
        &mut self,
        limit: usize,
        mut value: impl FnMut(&mut Self, &str) -> Result<T, Error>,
    ) -> Result<BTreeMap<String, T>, Error> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        if self.take('}') {
            return Ok(map);
        }
        loop {
            if map.len() == limit {
                return Err(Error::Limit);
            }
            let key = self.string()?;
            if map.contains_key(&key) {
                return Err(Error::Duplicate);
            }
            self.expect(':')?;
            let item = value(self, &key)?;
            map.insert(key, item);
            if self.take('}') {
                return Ok(map);
            }
            self.expect(',')?;
        }
    }
}

/// Internal bounded parser for a flat JSON string-to-integer vocabulary.
pub(crate) fn parse_vocab_json(input: &str) -> Result<BTreeMap<String, u64>, Error> {
    if input.len() > 16 * 1024 * 1024 {
        return Err(Error::Limit);
    }
    let mut p = Parser { rest: input };
    let result = p.object(200_000, |p, _| p.number())?;
    p.ws();
    if !p.rest.is_empty() {
        return Err(Error::Header);
    }
    Ok(result)
}

enum Field {
    Dtype(Dtype),
    Numbers(Vec<u64>),
}

enum Entry {
    Metadata(BTreeMap<String, String>),
    Tensor(TensorInfo),
}

fn tensor(p: &mut Parser<'_>) -> Result<TensorInfo, Error> {
    let mut fields = p.object(3, |p, key| match key {
        "dtype" => Ok(Field::Dtype(Dtype::parse(&p.string()?)?)),
        "shape" => Ok(Field::Numbers(p.numbers(MAX_RANK)?)),
        "data_offsets" => Ok(Field::Numbers(p.numbers(2)?)),
        _ => Err(Error::Header),
    })?;
    let Some(Field::Dtype(dtype)) = fields.remove("dtype") else {
        return Err(Error::Header);
    };
    let Some(Field::Numbers(shape)) = fields.remove("shape") else {
        return Err(Error::Header);
    };
    let Some(Field::Numbers(offsets)) = fields.remove("data_offsets") else {
        return Err(Error::Header);
    };
    let offsets: [u64; 2] = offsets.try_into().map_err(|_| Error::Offset)?;
    let elements = if shape.contains(&0) {
        0
    } else {
        shape
            .iter()
            .try_fold(1_u64, |n, &dim| n.checked_mul(dim).ok_or(Error::Shape))?
    };
    let size = elements.checked_mul(dtype.width()).ok_or(Error::Shape)?;
    if offsets[1].checked_sub(offsets[0]) != Some(size) {
        return Err(Error::Offset);
    }
    Ok(TensorInfo {
        dtype,
        shape,
        offsets,
        elements,
    })
}

/// Index stays in memory; payload is sought and decoded only on request.
/// Reader must refer to a stable, unmodified file while in use.
pub struct SafeTensorReader<R> {
    reader: R,
    data_start: u64,
    tensors: BTreeMap<String, TensorInfo>,
    metadata: BTreeMap<String, String>,
}

impl<R: Read + Seek> SafeTensorReader<R> {
    pub fn open(mut reader: R) -> Result<Self, Error> {
        let file_len = reader.seek(SeekFrom::End(0)).map_err(|_| Error::Io)?;
        reader.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
        let mut length = [0_u8; 8];
        reader.read_exact(&mut length).map_err(|_| Error::Header)?;
        let length = u64::from_le_bytes(length);
        if length == 0 || length > MAX_HEADER {
            return Err(Error::Limit);
        }
        let data_start = 8 + length;
        if data_start > file_len {
            return Err(Error::Header);
        }
        let mut bytes = vec![0_u8; length as usize];
        reader.read_exact(&mut bytes).map_err(|_| Error::Io)?;
        if bytes.first() != Some(&b'{') {
            return Err(Error::Header);
        }
        let mut p = Parser {
            rest: std::str::from_utf8(&bytes).map_err(|_| Error::Header)?,
        };
        let entries = p.object(MAX_ENTRIES, |p, key| {
            if key == "__metadata__" {
                Ok(Entry::Metadata(p.object(MAX_ENTRIES, |p, _| p.string())?))
            } else {
                Ok(Entry::Tensor(tensor(p)?))
            }
        })?;
        if !p.rest.bytes().all(|b| b == b' ') {
            return Err(Error::Header);
        }
        let mut tensors = BTreeMap::new();
        let mut metadata = BTreeMap::new();
        for (name, entry) in entries {
            match entry {
                Entry::Tensor(info) => {
                    tensors.insert(name, info);
                }
                Entry::Metadata(map) => metadata = map,
            }
        }
        let mut ranges: Vec<_> = tensors.values().map(|t| t.offsets).collect();
        ranges.sort_unstable();
        let mut cursor = 0;
        for [start, end] in ranges {
            if start != cursor || end > file_len - data_start {
                return Err(Error::Offset);
            }
            cursor = end;
        }
        if cursor != file_len - data_start {
            return Err(Error::Offset);
        }
        Ok(Self {
            reader,
            data_start,
            tensors,
            metadata,
        })
    }

    pub fn tensors(&self) -> &BTreeMap<String, TensorInfo> {
        &self.tensors
    }

    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    /// Decode at most 1M contiguous elements to f32, preserving IEEE specials.
    /// Consumers must separately check finite values before model computation.
    pub fn read_f32(&mut self, name: &str, start: u64, count: usize) -> Result<Vec<f32>, Error> {
        if count > MAX_READ_ELEMENTS {
            return Err(Error::Limit);
        }
        let t = self.tensors.get(name).ok_or(Error::MissingTensor)?;
        if start > t.elements || count as u64 > t.elements - start {
            return Err(Error::Offset);
        }
        let width = t.dtype.width();
        let offset = self.data_start + t.offsets[0] + start * width;
        self.reader
            .seek(SeekFrom::Start(offset))
            .map_err(|_| Error::Io)?;
        let mut raw = vec![0; count * width as usize];
        self.reader.read_exact(&mut raw).map_err(|_| Error::Io)?;
        let values = match t.dtype {
            Dtype::F32 => raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect(),
            Dtype::F16 => raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| half_to_f32(u16::from_le_bytes(*b)))
                .collect(),
            Dtype::BF16 => raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| f32::from_bits(u32::from(u16::from_le_bytes(*b)) << 16))
                .collect(),
        };
        Ok(values)
    }
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exp = u32::from((bits >> 10) & 31);
    let fraction = u32::from(bits & 1023);
    let magnitude = match exp {
        0 if fraction == 0 => 0,
        0 => {
            let shift = fraction.leading_zeros() - 21;
            ((113 - shift) << 23) | (((fraction << shift) & 1023) << 13)
        }
        31 => 0x7f80_0000 | (fraction << 13),
        _ => ((exp + 112) << 23) | (fraction << 13),
    };
    f32::from_bits(sign | magnitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn file(header: &str, payload: &[u8]) -> Vec<u8> {
        let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn rejected(header: &str, payload: &[u8], error: Error) {
        assert_eq!(
            SafeTensorReader::open(Cursor::new(file(header, payload))).err(),
            Some(error)
        );
    }

    #[test]
    fn official_reference_fixture() {
        // Produced by safetensors Python 0.7.0 serialize, not by this reader.
        let bytes = include_str!("../tests/reference.hex").trim();
        let bytes: Vec<u8> = (0..bytes.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&bytes[i..i + 2], 16).unwrap())
            .collect();
        let mut reader = SafeTensorReader::open(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.metadata()["source"], "reference");
        assert_eq!(reader.tensors()["a"].shape, vec![2]);
        assert_eq!(reader.tensors()["c"].shape, Vec::<u64>::new());
        assert_eq!(reader.read_f32("a", 0, 2), Ok(vec![1.25, -2.5]));
        assert_eq!(reader.read_f32("a", 1, 1), Ok(vec![-2.5]));
        assert_eq!(reader.read_f32("b", 0, 2), Ok(vec![1.0, 2_f32.powi(-24)]));
        assert_eq!(reader.read_f32("c", 0, 1), Ok(vec![-1.0]));
        assert_eq!(reader.read_f32("a", 2, 1), Err(Error::Offset));
        assert_eq!(reader.read_f32("missing", 0, 0), Err(Error::MissingTensor));
        assert_eq!(
            reader.read_f32("a", 0, MAX_READ_ELEMENTS + 1),
            Err(Error::Limit)
        );
    }

    #[test]
    fn malformed_ranges() {
        rejected(
            r#"{"a":{"dtype":"F32","shape":[1],"data_offsets":[1,5]}}"#,
            &[0; 5],
            Error::Offset,
        );
        rejected(
            r#"{"a":{"dtype":"F32","shape":[1],"data_offsets":[0,4]},"b":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#,
            &[0; 4],
            Error::Offset,
        );
        rejected(
            r#"{"a":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#,
            &[0; 3],
            Error::Offset,
        );
        rejected("{}", &[0], Error::Offset);
        rejected(
            r#"{"a":{"dtype":"F32","shape":[18446744073709551615,2],"data_offsets":[0,0]}}"#,
            &[],
            Error::Shape,
        );
    }

    #[test]
    fn malformed_header() {
        rejected(
            r#"{"a":{"dtype":"I8","shape":[1],"data_offsets":[0,1]}}"#,
            &[0],
            Error::UnsupportedDtype,
        );
        rejected(
            r#"{"__metadata__":{"a":"x","a":"y"}}"#,
            &[],
            Error::Duplicate,
        );
        rejected(r#"{"__metadata__":{"a":1}}"#, &[], Error::Header);
        rejected(r#"{"__metadata__":{"a":"\ud800"}}"#, &[], Error::Header);
        rejected("{}x", &[], Error::Header);
        assert_eq!(
            SafeTensorReader::open(Cursor::new(vec![0xff; 8])).err(),
            Some(Error::Limit)
        );
        assert_eq!(
            SafeTensorReader::open(Cursor::new(vec![4, 0, 0, 0, 0, 0, 0, 0])).err(),
            Some(Error::Header)
        );
    }

    #[test]
    fn empty_tensor_and_unicode() {
        let header = r#"{"\u5b66\ud83d\ude00":{"dtype":"F16","shape":[0,3],"data_offsets":[0,0]}}"#;
        let mut reader = SafeTensorReader::open(Cursor::new(file(header, &[]))).unwrap();
        assert_eq!(reader.read_f32("学😀", 0, 0), Ok(vec![]));
    }

    #[test]
    fn half_edge_cases() {
        assert_eq!(half_to_f32(0x8000).to_bits(), (-0.0_f32).to_bits());
        assert_eq!(half_to_f32(0x0001), 2_f32.powi(-24));
        assert_eq!(half_to_f32(0x0400), 2_f32.powi(-14));
        assert_eq!(half_to_f32(0x7bff), 65504.0);
        assert_eq!(half_to_f32(0xfc00), f32::NEG_INFINITY);
        assert!(half_to_f32(0x7e00).is_nan());
    }
}
