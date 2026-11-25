//! Conservative, explainable domain selection for one translation segment.
//! Distinct cues only; ambiguous words are never their own evidence.
use crate::{
    glossary::{self, Domain, Glossary},
    protection::{self, Plan},
};
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub geo_score: u16,
    pub bio_score: u16,
    pub geo_cues: u16,
    pub bio_cues: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Local,
    Document,
    Mixed,
    Insufficient,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub domain: Domain,
    pub basis: Basis,
    pub local: Evidence,
    pub document: Evidence,
}

/// Immutable context for one document; callers must not reuse it across papers.
pub struct DocumentContext(Evidence);

pub struct ContextEngine {
    general: Glossary,
    geo: Glossary,
    bio: Glossary,
}

fn cue(word: &str) -> Option<(Domain, u16, &'static str)> {
    use Domain::{Biology, Geotechnical};
    Some(match word {
        "earthquake" | "earthquakes" => (Geotechnical, 3, "earthquake"),
        "seismic" | "seismicity" => (Geotechnical, 3, "seismic"),
        "geotechnical" => (Geotechnical, 3, "geotechnical"),
        "soil" | "soils" => (Geotechnical, 2, "soil"),
        "shear" => (Geotechnical, 2, "shear"),
        "bedrock" => (Geotechnical, 3, "bedrock"),
        "sediment" | "sediments" | "sedimentary" => (Geotechnical, 2, "sediment"),
        "basin" | "basins" => (Geotechnical, 1, "basin"),
        "liquefaction" => (Geotechnical, 3, "liquefaction"),
        "bacterial" | "bacteria" | "bacterium" => (Biology, 3, "bacteria"),
        "microbial" | "microorganism" | "microorganisms" => (Biology, 3, "microbe"),
        "gene" | "genes" | "genetic" => (Biology, 2, "gene"),
        "cell" | "cells" | "cellular" => (Biology, 2, "cell"),
        "protein" | "proteins" => (Biology, 2, "protein"),
        "genome" | "genomes" | "genomic" => (Biology, 3, "genome"),
        "pathogen" | "pathogens" => (Biology, 3, "pathogen"),
        "antibiotic" | "antibiotics" => (Biology, 3, "antibiotic"),
        "enzyme" | "enzymes" => (Biology, 2, "enzyme"),
        _ => return None,
    })
}

impl Evidence {
    /// Ignore protected formulas, symbols, quantities and chemical literals.
    /// Repeated words or inflected aliases cannot inflate a cue's score.
    pub fn analyze(text: &str) -> Result<Self, protection::Error> {
        let plan = Plan::prepare(text, &[])?;
        let mut seen = BTreeSet::new();
        let mut result = Self::default();
        for gap in plan.gaps() {
            for token in gap.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if let Some((domain, weight, group)) = cue(&token.to_ascii_lowercase()) {
                    if !seen.insert(group) {
                        continue;
                    }
                    match domain {
                        Domain::Geotechnical => {
                            result.geo_score += weight;
                            result.geo_cues += 1;
                        }
                        Domain::Biology => {
                            result.bio_score += weight;
                            result.bio_cues += 1;
                        }
                        Domain::General => {}
                    }
                }
            }
        }
        Ok(result)
    }

    fn mixed(self) -> bool {
        self.geo_score > 0 && self.bio_score > 0
    }

    fn empty(self) -> bool {
        self.geo_score == 0 && self.bio_score == 0
    }

    fn strong(self) -> Option<Domain> {
        if self.mixed() {
            None
        } else if self.geo_score >= 4 && self.geo_cues >= 2 {
            Some(Domain::Geotechnical)
        } else if self.bio_score >= 4 && self.bio_cues >= 2 {
            Some(Domain::Biology)
        } else {
            None
        }
    }
}

impl DocumentContext {
    pub fn from_text(text: &str) -> Result<Self, protection::Error> {
        Evidence::analyze(text).map(Self)
    }

    pub fn evidence(&self) -> Evidence {
        self.0
    }
}

impl ContextEngine {
    pub fn builtin() -> Result<Self, glossary::Error> {
        Ok(Self {
            general: Glossary::builtin(Domain::General)?,
            geo: Glossary::builtin(Domain::Geotechnical)?,
            bio: Glossary::builtin(Domain::Biology)?,
        })
    }

    pub fn decide(
        &self,
        text: &str,
        document: Option<&DocumentContext>,
    ) -> Result<Decision, protection::Error> {
        let local = Evidence::analyze(text)?;
        let document = document.map_or(Evidence::default(), DocumentContext::evidence);
        let (domain, basis) = if local.mixed() {
            (Domain::General, Basis::Mixed)
        } else if let Some(domain) = local.strong() {
            (domain, Basis::Local)
        } else if !local.empty() {
            (Domain::General, Basis::Insufficient)
        } else if let Some(domain) = document.strong() {
            (domain, Basis::Document)
        } else if document.mixed() {
            (Domain::General, Basis::Mixed)
        } else if !document.empty() {
            (Domain::General, Basis::Insufficient)
        } else {
            (Domain::General, Basis::None)
        };
        Ok(Decision {
            domain,
            basis,
            local,
            document,
        })
    }

    pub(crate) fn glossary(&self, domain: Domain) -> &Glossary {
        match domain {
            Domain::General => &self.general,
            Domain::Geotechnical => &self.geo,
            Domain::Biology => &self.bio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protection::Kind;

    #[test]
    fn explicit_local_domains_override_document() {
        let e = ContextEngine::builtin().unwrap();
        let doc = DocumentContext::from_text("Soil shear during an earthquake.").unwrap();
        let geo = e.decide("Soil shear strain increased.", None).unwrap();
        assert_eq!((geo.domain, geo.basis), (Domain::Geotechnical, Basis::Local));
        let bio = e
            .decide("Bacterial cells contain this strain.", Some(&doc))
            .unwrap();
        assert_eq!((bio.domain, bio.basis), (Domain::Biology, Basis::Local));
        for (source, decision, expected) in [
            ("strain", geo, "应变"),
            ("strain", bio, "菌株"),
            ("stress", bio, "胁迫"),
        ] {
            let plan = Plan::prepare_indexed(source, e.glossary(decision.domain)).unwrap();
            assert_eq!(plan.spans()[0].replacement, expected);
            assert_eq!(plan.spans()[0].kind, Kind::Term);
        }
    }

    #[test]
    fn unknown_and_mixed_do_not_force_homonyms() {
        let e = ContextEngine::builtin().unwrap();
        let doc = DocumentContext::from_text("Soil shear during an earthquake.").unwrap();
        for (text, prior, basis) in [
            ("strain stress matrix culture", None, Basis::None),
            ("soil bacteria strain", Some(&doc), Basis::Mixed),
            ("protein strain", Some(&doc), Basis::Insufficient),
        ] {
            let d = e.decide(text, prior).unwrap();
            assert_eq!((d.domain, d.basis), (Domain::General, basis));
            let plan = Plan::prepare_indexed(text, e.glossary(d.domain)).unwrap();
            assert!(plan.spans().iter().all(|s| s.kind != Kind::Term));
        }
    }

    #[test]
    fn immutable_document_prior_for_neutral_segments() {
        let e = ContextEngine::builtin().unwrap();
        let geo = DocumentContext::from_text("Seismic waves in a soil basin.").unwrap();
        let bio = DocumentContext::from_text("Bacterial cells and genes.").unwrap();
        let text = "The strain increased.";
        assert_eq!(e.decide(text, Some(&geo)).unwrap().basis, Basis::Document);
        assert_eq!(e.decide(text, Some(&bio)).unwrap().domain, Domain::Biology);
        assert_eq!(e.decide(text, None).unwrap().domain, Domain::General);
        assert_eq!(
            e.decide(text, Some(&geo)).unwrap().domain,
            Domain::Geotechnical
        );
    }

    #[test]
    fn deduplicate_cues_and_ignore_formula() {
        let a = Evidence::analyze("SOIL soils soil $bacteria+gene$").unwrap();
        assert_eq!((a.geo_score, a.geo_cues, a.bio_score), (2, 1, 0));
        let b = Evidence::analyze("earthquake earthquakes seismic").unwrap();
        assert_eq!((b.geo_score, b.geo_cues), (6, 2));
        let c = Evidence::analyze("soil2 subsoil soil_1 ésoil").unwrap();
        assert_eq!(c, Evidence::default());
    }

    #[test]
    fn limits_and_malformed_science_are_errors() {
        assert_eq!(
            Evidence::analyze("broken $x"),
            Err(protection::Error::UnclosedMath)
        );
        assert_eq!(
            Evidence::analyze(&"x".repeat(65537)),
            Err(protection::Error::Limit)
        );
    }
}
