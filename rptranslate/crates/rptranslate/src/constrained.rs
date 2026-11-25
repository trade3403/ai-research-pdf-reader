//! Exact, bounded target-vocabulary encoding and ordered surface verification.
use crate::{offline, protection};
use rpmodel::marian::Vocabulary;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Input(offline::Error),
    Plan(protection::Error),
    Unrepresentable(usize),
    Missing(usize),
    Limit,
    Truncated,
}

#[derive(Debug, PartialEq)]
pub struct ConstrainedTranslation {
    pub translation: offline::Translation,
    pub protected_spans: usize,
    pub term_spans: usize,
}

/// Minimum number of exact network pieces over UTF-8 boundaries.
/// No NFKC, UNK, special tokens, or unverified substitution.
fn encode_exact(vocab: &Vocabulary, text: &str, blocked: &[u32]) -> Option<Vec<u32>> {
    if text.is_empty() || text.len() > 256 {
        return None;
    }
    let normalized = text.replace(' ', "▁");
    for surface in [normalized.clone(), format!("▁{normalized}")] {
        let mut best = vec![None::<Vec<u32>>; surface.len() + 1];
        best[surface.len()] = Some(Vec::new());
        let mut boundaries: Vec<_> = surface.char_indices().map(|(i, _)| i).collect();
        boundaries.push(surface.len());
        for &start in boundaries[..boundaries.len() - 1].iter().rev() {
            for &end in boundaries.iter().filter(|&&end| end > start) {
                let id = vocab.id(&surface[start..end]);
                if [vocab.unk, vocab.eos, vocab.pad].contains(&id) || blocked.contains(&id) {
                    continue;
                }
                let Some(tail) = &best[end] else {
                    continue;
                };
                if best[start]
                    .as_ref()
                    .is_none_or(|old| old.len() > tail.len() + 1)
                {
                    let mut candidate = vec![id];
                    candidate.extend_from_slice(tail);
                    best[start] = Some(candidate);
                }
            }
        }
        if let Some(ids) = best[0].take() {
            if vocab.decode_skipping_special(&ids).ok().as_deref() == Some(text) {
                return Some(ids);
            }
        }
    }
    None
}

pub(crate) fn encode_phrases(
    vocab: &Vocabulary,
    plan: &protection::Plan,
    blocked: &[u32],
) -> Result<Vec<Vec<u32>>, Error> {
    if plan.spans().len() > 32 {
        return Err(Error::Limit);
    }
    plan.spans()
        .iter()
        .enumerate()
        .map(|(i, span)| {
            let ids =
                encode_exact(vocab, &span.replacement, blocked).ok_or(Error::Unrepresentable(i))?;
            if ids.len() > 64 {
                return Err(Error::Limit);
            }
            Ok(ids)
        })
        .collect()
}

/// A later occurrence cannot satisfy two separate source spans.
pub(crate) fn verify(plan: &protection::Plan, output: &str) -> Result<(), Error> {
    let mut rest = output;
    for (index, span) in plan.spans().iter().enumerate() {
        let at = rest.find(&span.replacement).ok_or(Error::Missing(index))?;
        rest = &rest[at + span.replacement.len()..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protection::Plan;

    #[test]
    fn exact_vocabulary_path_and_space() {
        let vocab = Vocabulary::parse(
            r#"{"</s>":0,"<unk>":1,"<pad>":2,"应":3,"变":4,"应变":5,"0":6,".":7,"1":8,"▁g":9}"#,
        )
        .unwrap();
        assert_eq!(encode_exact(&vocab, "应变", &[]), Some(vec![5]));
        assert_eq!(encode_exact(&vocab, "应变", &[5]), Some(vec![3, 4]));
        assert_eq!(encode_exact(&vocab, "0.1 g", &[]), Some(vec![6, 7, 8, 9]));
        assert_eq!(encode_exact(&vocab, "应力", &[]), None);
        assert_eq!(encode_exact(&vocab, "应变", &[3, 5]), None);
    }

    #[test]
    fn ordered_repeated_literals_and_reject_missing() {
        let p = Plan::prepare("1 Hz, 2 Hz, 1 Hz.", &[]).unwrap();
        assert_eq!(verify(&p, "1 Hz 与 2 Hz，再 1 Hz"), Ok(()));
        assert_eq!(verify(&p, "1 Hz 与 2 Hz"), Err(Error::Missing(2)));
        assert_eq!(verify(&p, "2 Hz 与 1 Hz"), Err(Error::Missing(1)));
    }

    #[test]
    fn unrepresentable_span_and_budget() {
        let vocab = Vocabulary::parse(r#"{"</s>":0,"<unk>":1,"<pad>":2,"1":3}"#).unwrap();
        let p = Plan::prepare("1 Hz", &[]).unwrap();
        assert_eq!(
            encode_phrases(&vocab, &p, &[]),
            Err(Error::Unrepresentable(0))
        );
        let p = Plan::prepare(&"1 ".repeat(33), &[]).unwrap();
        assert_eq!(encode_phrases(&vocab, &p, &[]), Err(Error::Limit));
    }
}
