//! D1.2a: bounded Unigram Viterbi over an exported SentencePiece vocabulary.
//! Supports pre-normalized input; not a full Marian network tokenizer.
use std::collections::BTreeMap;

const MAX_VOCAB_BYTES: usize = 16 * 1024 * 1024;
const MAX_PIECES: usize = 200_000;
const MAX_PIECE_BYTES: usize = 128;
const MAX_INPUT_BYTES: usize = 64 * 1024;
const MARKER: char = '▁';

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    InvalidVocabulary,
    InvalidId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Normal,
    Unknown,
    Control,
}

struct Piece {
    text: String,
    score: f32,
    kind: Kind,
}

pub struct Unigram {
    pieces: Vec<Piece>,
    normal: BTreeMap<String, usize>,
    unknown: usize,
    unknown_score: f64,
    max_piece_bytes: usize,
}

impl Unigram {
    /// TSV: sequential ID, SentencePiece type (1, 2, 3), f32 score, piece.
    /// Other types and non-identity normalization are not supported here.
    pub fn from_tsv(tsv: &str) -> Result<Self, Error> {
        if tsv.len() > MAX_VOCAB_BYTES {
            return Err(Error::Limit);
        }
        let mut pieces = Vec::new();
        let mut normal = BTreeMap::new();
        let mut names = BTreeMap::new();
        let mut unknown = None;
        let mut min_score = f32::INFINITY;
        let mut max_piece_bytes = 0;
        for line in tsv.lines() {
            if pieces.len() == MAX_PIECES {
                return Err(Error::Limit);
            }
            let mut fields = line.splitn(4, '\t');
            let id = fields
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or(Error::InvalidVocabulary)?;
            let kind = match fields.next() {
                Some("1") => Kind::Normal,
                Some("2") => Kind::Unknown,
                Some("3") => Kind::Control,
                _ => return Err(Error::InvalidVocabulary),
            };
            let score = fields
                .next()
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|s| s.is_finite() && s.abs() <= 1e6)
                .ok_or(Error::InvalidVocabulary)?;
            let piece = fields.next().ok_or(Error::InvalidVocabulary)?;
            if id != pieces.len()
                || piece.is_empty()
                || piece.len() > MAX_PIECE_BYTES
                || piece.chars().any(char::is_control)
                || names.insert(piece, id).is_some()
            {
                return Err(Error::InvalidVocabulary);
            }
            match kind {
                Kind::Normal => {
                    normal.insert(piece.to_owned(), id);
                    min_score = min_score.min(score);
                    max_piece_bytes = max_piece_bytes.max(piece.len());
                }
                Kind::Unknown => {
                    if unknown.replace(id).is_some() {
                        return Err(Error::InvalidVocabulary);
                    }
                }
                Kind::Control => {}
            }
            pieces.push(Piece {
                text: piece.to_owned(),
                score,
                kind,
            });
        }
        if normal.is_empty() {
            return Err(Error::InvalidVocabulary);
        }
        Ok(Self {
            pieces,
            normal,
            unknown: unknown.ok_or(Error::InvalidVocabulary)?,
            unknown_score: f64::from(min_score) - 10.0,
            max_piece_bytes,
        })
    }

    /// SentencePiece identity + dummy prefix + remove extra ASCII spaces
    /// + escape spaces. No NFKC or compiled charsmap.
    pub fn normalize(input: &str) -> Result<String, Error> {
        if input.len() > MAX_INPUT_BYTES {
            return Err(Error::Limit);
        }
        let mut output = String::new();
        for word in input.split(' ').filter(|s| !s.is_empty()) {
            output.push(MARKER);
            output.push_str(word);
        }
        Ok(output)
    }

    pub fn encode(&self, input: &str) -> Result<Vec<usize>, Error> {
        let normalized = Self::normalize(input)?;
        self.encode_normalized(&normalized)
    }

    /// Encode text already normalized by the model's configured normalizer.
    /// A separate bound accommodates expansion by compiled character maps.
    pub fn encode_normalized(&self, normalized: &str) -> Result<Vec<usize>, Error> {
        let segments = self.segments(normalized)?;
        Ok(segments.into_iter().map(|(_, _, id)| id).collect())
    }

    /// Preserve the actual surface of unknown spans for shared network vocabularies.
    pub fn encode_normalized_pieces(&self, normalized: &str) -> Result<Vec<String>, Error> {
        let segments = self.segments(normalized)?;
        Ok(segments
            .into_iter()
            .map(|(start, end, id)| {
                if id == self.unknown {
                    normalized[start..end].to_owned()
                } else {
                    self.pieces[id].text.clone()
                }
            })
            .collect())
    }

    fn segments(&self, normalized: &str) -> Result<Vec<(usize, usize, usize)>, Error> {
        if normalized.len() > 1024 * 1024 {
            return Err(Error::Limit);
        }
        let mut positions: Vec<_> = normalized.char_indices().map(|(i, _)| i).collect();
        positions.push(normalized.len());
        let n = positions.len() - 1;
        let mut best = vec![f64::NEG_INFINITY; n + 1];
        let mut previous = vec![(0, self.unknown); n + 1];
        best[0] = 0.0;
        for i in 0..n {
            let mut single = false;
            for j in i + 1..=n {
                if positions[j] - positions[i] > self.max_piece_bytes {
                    break;
                }
                if let Some(&id) = self.normal.get(&normalized[positions[i]..positions[j]]) {
                    single |= j == i + 1;
                    let score = best[i] + f64::from(self.pieces[id].score);
                    if score > best[j] {
                        best[j] = score;
                        previous[j] = (i, id);
                    }
                }
            }
            if !single {
                let score = best[i] + self.unknown_score;
                if score > best[i + 1] {
                    best[i + 1] = score;
                    previous[i + 1] = (i, self.unknown);
                }
            }
        }
        let mut reversed = Vec::new();
        let mut i = n;
        while i != 0 {
            let (start, id) = previous[i];
            reversed.push((positions[start], positions[i], id));
            i = start;
        }
        reversed.reverse();
        let mut merged: Vec<(usize, usize, usize)> = Vec::new();
        for (start, end, id) in reversed {
            if let Some(last) = merged.last_mut() {
                if id == self.unknown && last.2 == self.unknown {
                    last.1 = end;
                    continue;
                }
            }
            merged.push((start, end, id));
        }
        Ok(merged)
    }

    /// Control pieces do not appear in output; unknown uses default surface.
    /// IDs are SentencePiece IDs, not Marian network vocabulary IDs.
    pub fn decode(&self, ids: &[usize]) -> Result<String, Error> {
        if ids.len() > MAX_INPUT_BYTES {
            return Err(Error::Limit);
        }
        let mut output = String::new();
        let mut beginning = true;
        for &id in ids {
            let p = self.pieces.get(id).ok_or(Error::InvalidId)?;
            match p.kind {
                Kind::Control => {}
                Kind::Unknown => {
                    output.push_str(" ⁇ ");
                    beginning = false;
                }
                Kind::Normal => {
                    for ch in p.text.chars() {
                        if beginning && ch == MARKER {
                            continue;
                        }
                        beginning = false;
                        output.push(if ch == MARKER { ' ' } else { ch });
                    }
                }
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOCAB: &str = include_str!("../tests/unigram_reference.tsv");

    #[test]
    fn reference_sentencepiece_021() {
        let model = Unigram::from_tsv(VOCAB).unwrap();
        let cases: &[(&str, &str, &[usize], &str)] = &[
            ("", "", &[], ""),
            ("  ab  学术。  ", "▁ab▁学术。", &[8, 12, 13], "ab 学术。"),
            ("a b", "▁a▁b", &[19, 4, 6], "a b"),
            ("ab12", "▁ab12", &[8, 16], "ab12"),
            ("学术。", "▁学术。", &[12, 13], "学术。"),
            ("é β", "▁é▁β", &[4, 17, 4, 18], "é β"),
            ("a😀🧪b", "▁a😀🧪b", &[19, 0, 6], "a ⁇ b"),
            ("a\tb", "▁a\tb", &[19, 0, 6], "a ⁇ b"),
            ("  ", "", &[], ""),
            ("a\nb", "▁a\nb", &[19, 0, 6], "a ⁇ b"),
            ("▁ab", "▁▁ab", &[4, 8], "ab"),
        ];
        for (input, normalized, ids, decoded) in cases {
            assert_eq!(Unigram::normalize(input).unwrap(), *normalized);
            assert_eq!(model.encode(input).unwrap(), *ids);
            assert_eq!(model.decode(ids).unwrap(), *decoded);
        }
    }

    #[test]
    fn normalized_input_is_not_normalized_twice() {
        let model = Unigram::from_tsv(VOCAB).unwrap();
        assert_eq!(model.encode_normalized("▁ab▁学术。"), Ok(vec![8, 12, 13]));
        assert_eq!(model.encode_normalized(""), Ok(vec![]));
        assert_eq!(
            model.encode_normalized(&"a".repeat(1024 * 1024 + 1)),
            Err(Error::Limit)
        );
    }

    #[test]
    fn unknown_surface_is_preserved() {
        let model = Unigram::from_tsv(VOCAB).unwrap();
        assert_eq!(
            model.encode_normalized_pieces("▁a😀🧪b").unwrap(),
            vec!["▁a", "😀🧪", "b"]
        );
    }

    #[test]
    fn controls_and_invalid_id() {
        let model = Unigram::from_tsv(VOCAB).unwrap();
        assert_eq!(model.decode(&[1, 8, 2, 3]), Ok("ab".into()));
        assert_eq!(model.decode(&[999]), Err(Error::InvalidId));
        assert_eq!(
            model.encode(&"a".repeat(MAX_INPUT_BYTES + 1)),
            Err(Error::Limit)
        );
    }

    #[test]
    fn rejects_invalid_vocabularies() {
        for invalid in [
            "",
            "0\t1\t0\ta",
            "1\t2\t0\t<unk>",
            "0\t2\t0\t<unk>\n1\t1\tNaN\ta",
            "0\t2\t0\t<unk>\n1\t4\t0\ta",
            "0\t2\t0\t<unk>\n1\t1\t0\ta\n2\t1\t0\ta",
            "0\t2\t0\t<unk>\n1\t2\t0\tother\n2\t1\t0\ta",
        ] {
            assert_eq!(
                Unigram::from_tsv(invalid).err(),
                Some(Error::InvalidVocabulary)
            );
        }
    }

    #[test]
    fn viterbi_not_greedy() {
        let model = Unigram::from_tsv(
            "0\t2\t0\t<unk>\n1\t1\t-1\t▁\n2\t1\t-1\ta\n3\t1\t-1\tb\n4\t1\t-10\tab",
        )
        .unwrap();
        assert_eq!(model.encode("ab"), Ok(vec![1, 2, 3]));
    }
}
