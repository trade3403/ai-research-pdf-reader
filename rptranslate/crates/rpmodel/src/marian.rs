//! Shared Marian network vocabulary, independent of SentencePiece IDs.
use crate::{safetensors::parse_vocab_json, spm::NmtUnigram};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Vocabulary,
    MissingSpecial,
    DuplicateId,
    InvalidId,
    InvalidLanguage,
    Limit,
    Tokenizer,
}

pub struct Vocabulary {
    by_piece: BTreeMap<String, u32>,
    by_id: BTreeMap<u32, String>,
    pub unk: u32,
    pub eos: u32,
    pub pad: u32,
}

impl Vocabulary {
    pub fn parse(json: &str) -> Result<Self, Error> {
        let raw = parse_vocab_json(json).map_err(|_| Error::Vocabulary)?;
        let mut by_piece = BTreeMap::new();
        let mut by_id = BTreeMap::new();
        for (piece, number) in raw {
            if piece.is_empty() || piece.len() > 1024 || number >= 200_000 {
                return Err(Error::Vocabulary);
            }
            let id = number as u32;
            if by_id.insert(id, piece.clone()).is_some() {
                return Err(Error::DuplicateId);
            }
            by_piece.insert(piece, id);
        }
        let special = |key: &str| by_piece.get(key).copied().ok_or(Error::MissingSpecial);
        Ok(Self {
            unk: special("<unk>")?,
            eos: special("</s>")?,
            pad: special("<pad>")?,
            by_piece,
            by_id,
        })
    }

    pub fn len(&self) -> usize {
        self.by_piece.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_piece.is_empty()
    }

    pub fn id(&self, piece: &str) -> u32 {
        self.by_piece.get(piece).copied().unwrap_or(self.unk)
    }

    pub fn piece(&self, id: u32) -> Result<&str, Error> {
        self.by_id
            .get(&id)
            .map(String::as_str)
            .ok_or(Error::InvalidId)
    }

    pub fn language_codes(&self) -> impl Iterator<Item = &str> {
        self.by_piece
            .keys()
            .map(String::as_str)
            .filter(|s| s.starts_with(">>") && s.ends_with("<<"))
    }

    /// Explicit leading language code, if present, is kept as one network ID.
    /// Unknown codes are rejected instead of silently selecting a language.
    /// No implicit padding, truncation, added-token splitting, or Moses pass.
    pub fn encode(&self, tokenizer: &NmtUnigram<'_>, text: &str) -> Result<Vec<u32>, Error> {
        if text.len() > 64 * 1024 {
            return Err(Error::Limit);
        }
        let mut ids = Vec::new();
        let mut rest = text;
        if text.starts_with(">>") {
            let end = text.find("<<").ok_or(Error::InvalidLanguage)? + 2;
            let code = &text[..end];
            let id = self.by_piece.get(code).ok_or(Error::InvalidLanguage)?;
            ids.push(*id);
            rest = &text[end..];
        }
        ids.extend(
            tokenizer
                .encode_pieces(rest)
                .map_err(|_| Error::Tokenizer)?
                .iter()
                .map(|p| self.id(p)),
        );
        ids.push(self.eos);
        Ok(ids)
    }

    /// Decode normal network pieces, optionally dropping EOS/PAD/UNK.
    /// Non-skipped specials are outside this initial decoding profile.
    pub fn decode_skipping_special(&self, ids: &[u32]) -> Result<String, Error> {
        if ids.len() > 65536 {
            return Err(Error::Limit);
        }
        let mut output = String::new();
        for &id in ids {
            if [self.eos, self.pad, self.unk].contains(&id) {
                continue;
            }
            let p = self.piece(id)?;
            if output.len() + p.len() > 1024 * 1024 {
                return Err(Error::Limit);
            }
            output.push_str(p);
        }
        Ok(output.replace('▁', " ").trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_map() {
        let v = Vocabulary::parse(
            r#"{"</s>":0,"<unk>":7,"<pad>":9,"\u5b66":3,"\ud83d\ude00":4,">>cmn<<":6}"#,
        )
        .unwrap();
        assert_eq!(v.len(), 6);
        assert_eq!((v.eos, v.unk, v.pad), (0, 7, 9));
        assert_eq!(v.id("学"), 3);
        assert_eq!(v.id("😀"), 4);
        assert_eq!(v.id("missing"), 7);
        assert_eq!(v.piece(3), Ok("学"));
        assert_eq!(v.piece(1), Err(Error::InvalidId));
        assert_eq!(v.language_codes().collect::<Vec<_>>(), vec![">>cmn<<"]);
        assert_eq!(
            v.decode_skipping_special(&[9, 3, 7, 4, 0]),
            Ok("学😀".into())
        );
    }

    #[test]
    fn invalid_vocabularies() {
        for input in [
            r#"{"<unk>":1,"</s>":0,"<pad>":2,"a":3,"a":4}"#,
            r#"{"<unk>":1,"</s>":0,"<pad>":2,"a":3,"b":3}"#,
            r#"{"<unk>":1,"</s>":0,"<pad>":2,"a":-1}"#,
            r#"{"<unk>":1,"</s>":0,"<pad>":2,"a":1.5}"#,
            r#"{"<unk>":1,"</s>":0,"<pad>":2,"a":200000}"#,
            r#"{"<unk>":1,"</s>":0}"#,
            r#"{"<unk>":1,"</s>":0,"<pad>":2}x"#,
        ] {
            assert!(Vocabulary::parse(input).is_err(), "{input}");
        }
    }
}
