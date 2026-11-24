//! Bounded SentencePiece ModelProto reader. Inspect any model; only the
//! identity and default nmt_nfkc Unigram profiles are supported.
use crate::{normalization::CharsMap, tokenizer::Unigram};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_PIECES: usize = 200_000;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Wire,
    InvalidModel,
    Unsupported,
    MissingId,
    Normalize,
}

enum Value<'a> {
    Var(u64),
    Bytes(&'a [u8]),
    Fixed32(u32),
    Fixed64,
}

impl<'a> Value<'a> {
    fn var(self) -> Result<u64, Error> {
        match self {
            Self::Var(n) => Ok(n),
            _ => Err(Error::Wire),
        }
    }

    fn bytes(self) -> Result<&'a [u8], Error> {
        match self {
            Self::Bytes(b) => Ok(b),
            _ => Err(Error::Wire),
        }
    }

    fn string(self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.bytes()?).map_err(|_| Error::InvalidModel)
    }
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    let (a, b) = input.split_at_checked(n).ok_or(Error::Wire)?;
    *input = b;
    Ok(a)
}

fn varint(input: &mut &[u8]) -> Result<u64, Error> {
    let mut n = 0;
    for shift in (0..70).step_by(7) {
        let b = take(input, 1)?[0];
        if shift == 63 && b > 1 {
            return Err(Error::Wire);
        }
        n |= u64::from(b & 127) << shift;
        if b < 128 {
            return Ok(n);
        }
    }
    Err(Error::Wire)
}

fn field<'a>(input: &mut &'a [u8]) -> Result<(u32, Value<'a>), Error> {
    let key = varint(input)?;
    let number = u32::try_from(key >> 3).map_err(|_| Error::Wire)?;
    if number == 0 || number > 0x1fff_ffff {
        return Err(Error::Wire);
    }
    let value = match key & 7 {
        0 => Value::Var(varint(input)?),
        1 => {
            take(input, 8)?;
            Value::Fixed64
        }
        2 => {
            let len = usize::try_from(varint(input)?).map_err(|_| Error::Limit)?;
            Value::Bytes(take(input, len)?)
        }
        5 => Value::Fixed32(u32::from_le_bytes(
            take(input, 4)?.try_into().map_err(|_| Error::Wire)?,
        )),
        _ => return Err(Error::Wire),
    };
    Ok((number, value))
}

#[derive(Debug)]
pub struct Piece<'a> {
    pub text: &'a str,
    pub score: f32,
    pub kind: u64,
}

fn piece(mut data: &[u8]) -> Result<Piece<'_>, Error> {
    let mut p = Piece {
        text: "",
        score: 0.0,
        kind: 1,
    };
    while !data.is_empty() {
        let (n, v) = field(&mut data)?;
        match n {
            1 => p.text = v.string()?,
            2 => match v {
                Value::Fixed32(x) => p.score = f32::from_bits(x),
                _ => return Err(Error::Wire),
            },
            3 => p.kind = v.var()?,
            _ => {}
        }
    }
    if p.text.is_empty() || !p.score.is_finite() || !(1..=6).contains(&p.kind) {
        return Err(Error::InvalidModel);
    }
    Ok(p)
}

#[derive(Debug)]
pub struct Model<'a> {
    pub pieces: Vec<Piece<'a>>,
    pub model_type: u64,
    pub normalizer: &'a str,
    pub charsmap: &'a [u8],
    pub unk_id: i32,
    pub bos_id: i32,
    pub eos_id: i32,
    pub pad_id: i32,
    pub byte_fallback: bool,
    identity_flags: bool,
    unsupported_options: bool,
}

impl<'a> Model<'a> {
    pub fn parse(mut data: &'a [u8]) -> Result<Self, Error> {
        if data.len() > MAX_FILE {
            return Err(Error::Limit);
        }
        let mut m = Self {
            pieces: Vec::new(),
            model_type: 1,
            normalizer: "",
            charsmap: &[],
            unk_id: 0,
            bos_id: 1,
            eos_id: 2,
            pad_id: -1,
            byte_fallback: false,
            identity_flags: true,
            unsupported_options: false,
        };
        let mut seen_trainer = false;
        let mut seen_norm = false;
        let mut names = BTreeSet::new();
        while !data.is_empty() {
            let (n, v) = field(&mut data)?;
            match n {
                1 => {
                    if m.pieces.len() == MAX_PIECES {
                        return Err(Error::Limit);
                    }
                    let p = piece(v.bytes()?)?;
                    if !names.insert(p.text) {
                        return Err(Error::InvalidModel);
                    }
                    m.pieces.push(p);
                }
                2 if !seen_trainer => {
                    seen_trainer = true;
                    let mut spec = v.bytes()?;
                    while !spec.is_empty() {
                        let (n, v) = field(&mut spec)?;
                        match n {
                            3 => m.model_type = v.var()?,
                            24 => m.unsupported_options |= v.var()? != 0,
                            35 => m.byte_fallback = v.var()? != 0,
                            40 => m.unk_id = v.var()? as i32,
                            41 => m.bos_id = v.var()? as i32,
                            42 => m.eos_id = v.var()? as i32,
                            43 => m.pad_id = v.var()? as i32,
                            44 => m.unsupported_options |= v.string()? != " ⁇ ",
                            _ => {}
                        }
                    }
                }
                3 if !seen_norm => {
                    seen_norm = true;
                    let mut spec = v.bytes()?;
                    while !spec.is_empty() {
                        let (n, v) = field(&mut spec)?;
                        match n {
                            1 => m.normalizer = v.string()?,
                            2 => m.charsmap = v.bytes()?,
                            3..=5 => m.identity_flags &= v.var()? == 1,
                            _ => {}
                        }
                    }
                }
                2 | 3 => return Err(Error::InvalidModel),
                5 => m.unsupported_options |= !v.bytes()?.is_empty(),
                _ => {}
            }
        }
        if m.pieces.is_empty() || !(1..=4).contains(&m.model_type) {
            return Err(Error::InvalidModel);
        }
        let check = |id: i32, kind: u64| {
            usize::try_from(id)
                .ok()
                .and_then(|i| m.pieces.get(i))
                .is_some_and(|p| p.kind == kind)
        };
        if !check(m.unk_id, 2)
            || m.pieces.iter().filter(|p| p.kind == 2).count() != 1
            || [m.bos_id, m.eos_id, m.pad_id]
                .iter()
                .any(|&id| id != -1 && !check(id, 3))
        {
            return Err(Error::InvalidModel);
        }
        Ok(m)
    }

    /// Refuse models requiring normalization or piece types not implemented.
    pub fn identity_unigram(&self) -> Result<Unigram, Error> {
        if self.model_type != 1
            || self.normalizer != "identity"
            || !self.charsmap.is_empty()
            || !self.identity_flags
            || self.byte_fallback
            || self.unsupported_options
            || self.pieces.iter().any(|p| p.kind > 3)
        {
            return Err(Error::Unsupported);
        }
        self.unigram_pieces()
    }

    /// Construct a tokenizer for the verified default nmt_nfkc profile.
    pub fn nmt_unigram(&self) -> Result<NmtUnigram<'a>, Error> {
        if self.model_type != 1
            || self.normalizer != "nmt_nfkc"
            || self.charsmap.is_empty()
            || !self.identity_flags
            || self.byte_fallback
            || self.unsupported_options
            || self.pieces.iter().any(|p| p.kind > 3)
        {
            return Err(Error::Unsupported);
        }
        Ok(NmtUnigram {
            normalizer: CharsMap::parse(self.charsmap).map_err(|_| Error::Normalize)?,
            tokenizer: self.unigram_pieces()?,
        })
    }

    fn unigram_pieces(&self) -> Result<Unigram, Error> {
        let mut tsv = String::new();
        for (i, p) in self.pieces.iter().enumerate() {
            if p.text.chars().any(char::is_control) {
                return Err(Error::InvalidModel);
            }
            writeln!(&mut tsv, "{i}\t{}\t{}\t{}", p.kind, p.score, p.text)
                .map_err(|_| Error::InvalidModel)?;
            if tsv.len() > MAX_FILE {
                return Err(Error::Limit);
            }
        }
        Unigram::from_tsv(&tsv).map_err(|_| Error::InvalidModel)
    }

    /// Map SentencePiece IDs to independently supplied Marian network IDs.
    /// Unknown pieces map to the explicitly supplied Marian UNK. The caller
    /// must handle language prefixes and EOS separately.
    pub fn marian_ids(
        &self,
        ids: &[usize],
        vocabulary: &BTreeMap<String, u32>,
        unknown_id: u32,
    ) -> Result<Vec<u32>, Error> {
        if ids.len() > 65536 {
            return Err(Error::Limit);
        }
        ids.iter()
            .map(|&id| {
                let p = self.pieces.get(id).ok_or(Error::MissingId)?;
                Ok(vocabulary.get(p.text).copied().unwrap_or(unknown_id))
            })
            .collect()
    }
}

/// End-to-end SentencePiece ID codec; Marian vocabulary IDs are separate.
pub struct NmtUnigram<'a> {
    normalizer: CharsMap<'a>,
    tokenizer: Unigram,
}

impl NmtUnigram<'_> {
    pub fn encode(&self, input: &str) -> Result<Vec<usize>, Error> {
        let normalized = self
            .normalizer
            .normalize(input)
            .map_err(|_| Error::Normalize)?;
        self.tokenizer
            .encode_normalized(&normalized)
            .map_err(|_| Error::Limit)
    }

    pub fn decode(&self, ids: &[usize]) -> Result<String, Error> {
        self.tokenizer.decode(ids).map_err(|_| Error::MissingId)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let hex = include_str!("../tests/spm_identity.hex").trim();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn official_protobuf_fixture() {
        let bytes = fixture();
        let m = Model::parse(&bytes).unwrap();
        assert_eq!(m.pieces.len(), 8);
        assert_eq!(m.normalizer, "identity");
        assert_eq!((m.unk_id, m.bos_id, m.eos_id, m.pad_id), (0, 1, 2, 3));
        let tok = m.identity_unigram().unwrap();
        let ids = tok.encode("学 a").unwrap();
        assert_eq!(ids, vec![7, 4, 6]);
        assert_eq!(tok.decode(&ids).unwrap(), "学 a");
        let vocab = BTreeMap::from([("▁学".into(), 91), ("a".into(), 42)]);
        assert_eq!(m.marian_ids(&ids, &vocab, 3), Ok(vec![91, 3, 42]));
        assert_eq!(m.marian_ids(&[8], &vocab, 3), Err(Error::MissingId));
    }

    #[test]
    fn rejects_bad_wire() {
        for bad in [
            vec![],
            vec![0],
            vec![10, 127, 1],
            vec![0xff; 11],
            vec![13, 0, 0],
        ] {
            assert!(Model::parse(&bad).is_err());
        }
        let mut bytes = fixture();
        bytes.pop();
        assert!(Model::parse(&bytes).is_err());
    }

    #[test]
    fn refuses_unimplemented_normalizer() {
        let bytes = fixture();
        let mut m = Model::parse(&bytes).unwrap();
        m.charsmap = &[1];
        assert_eq!(m.identity_unigram().err(), Some(Error::Unsupported));
    }
}
