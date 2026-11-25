//! Local translation API and historical D1.0 smoke test.
pub mod constrained;
pub mod context;
pub mod glossary;
pub mod offline;
pub mod protection;
pub mod scientific;
use rpdata::{deduplicate, SentencePair};
use rpmodel::ModelSpec;

#[derive(Debug, PartialEq, Eq)]
pub enum TranslationError {
    ModelNotReady,
}

/// Historical model-less entry point: explicit failure rather than a fake translation.
pub fn translate(_source: &str) -> Result<String, TranslationError> {
    Err(TranslationError::ModelNotReady)
}

/// Synthetic test fixture only. Does not claim model translation.
pub fn self_check() -> Result<u64, &'static str> {
    let pair = SentencePair {
        source: "The result is stable.".into(),
        target: "结果稳定。".into(),
        origin: "synthetic fixture".into(),
        license: "project-owned fixture".into(),
    };
    let pairs = deduplicate(vec![pair.clone(), pair]).map_err(|_| "corpus validation")?;
    if pairs.len() != 1 {
        return Err("corpus deduplication");
    }
    if rpinfer::dot(&[1.0, 2.0], &[3.0, 4.0]).map_err(|_| "numeric check")? != 11.0 {
        return Err("numeric result");
    }
    ModelSpec {
        id: "rpmodel-target-not-trained".into(),
        parameters: 100_000_000,
        bits_per_weight: 8,
    }
    .estimated_weight_bytes()
    .map_err(|_| "model budget")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_smoke_test() {
        assert_eq!(self_check(), Ok(100_000_000));
    }

    #[test]
    fn no_fake_translation() {
        assert_eq!(translate("A paper."), Err(TranslationError::ModelNotReady));
    }
}
