//! In-memory bilingual corpus validation. No network or file collection.
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SentencePair {
    pub source: String,
    pub target: String,
    pub origin: String,
    pub license: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataError {
    EmptyField,
    TooLong,
}

impl SentencePair {
    pub fn validate(&self) -> Result<(), DataError> {
        let fields = [&self.source, &self.target, &self.origin, &self.license];
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(DataError::EmptyField);
        }
        if fields.iter().any(|field| field.len() > 64 * 1024) {
            return Err(DataError::TooLong);
        }
        Ok(())
    }
}

/// Deduplicate exact bilingual text after trimming, retaining first provenance.
/// License strings are metadata, not a legal determination.
pub fn deduplicate(pairs: Vec<SentencePair>) -> Result<Vec<SentencePair>, DataError> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for mut pair in pairs {
        pair.validate()?;
        pair.source = pair.source.trim().to_owned();
        pair.target = pair.target.trim().to_owned();
        if seen.insert((pair.source.clone(), pair.target.clone())) {
            output.push(pair);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> SentencePair {
        SentencePair {
            source: " The result is stable. ".into(),
            target: "结果稳定。".into(),
            origin: "synthetic test fixture".into(),
            license: "project-owned fixture".into(),
        }
    }

    #[test]
    fn validates_and_deduplicates() {
        let p = pair();
        let mut q = p.clone();
        q.source = q.source.trim().into();
        assert_eq!(deduplicate(vec![p, q]).unwrap().len(), 1);
    }

    #[test]
    fn rejects_missing_provenance() {
        let mut p = pair();
        p.license.clear();
        assert_eq!(p.validate(), Err(DataError::EmptyField));
    }

    #[test]
    fn rejects_oversize() {
        let mut p = pair();
        p.source = "a".repeat(65537);
        assert_eq!(p.validate(), Err(DataError::TooLong));
    }
}
