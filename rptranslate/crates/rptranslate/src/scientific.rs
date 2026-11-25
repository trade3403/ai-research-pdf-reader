//! Opt-in scientific translation. Recognized literals never enter the model.
use crate::{
    glossary::Glossary,
    offline::{Error as OfflineError, Translation},
    protection::{self, Kind, Plan, Term},
};

const MAX_CALLS: usize = 128;
const MAX_GENERATED: usize = 4096;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Input(OfflineError),
    Plan(protection::Error),
    Gap { index: usize, error: OfflineError },
    Truncated { index: usize },
    Budget,
    EmptyGap { index: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScientificTranslation {
    pub text: String,
    pub protected_spans: usize,
    pub term_spans: usize,
    pub model_calls: usize,
    pub generated_tokens: usize,
    /// Separate gap translation may lose surrounding sentence context.
    pub fragmented_context: bool,
}

pub(crate) fn translate_with(
    source: &str,
    glossary: &[Term],
    translate: impl FnMut(&str) -> Result<Translation, OfflineError>,
) -> Result<ScientificTranslation, Error> {
    validate_source(source)?;
    let plan = Plan::prepare(source, glossary).map_err(Error::Plan)?;
    execute(plan, translate)
}

pub(crate) fn translate_indexed_with(
    source: &str,
    glossary: &Glossary,
    translate: impl FnMut(&str) -> Result<Translation, OfflineError>,
) -> Result<ScientificTranslation, Error> {
    validate_source(source)?;
    let plan = Plan::prepare_indexed(source, glossary).map_err(Error::Plan)?;
    execute(plan, translate)
}

fn validate_source(source: &str) -> Result<(), Error> {
    if source.trim().is_empty() {
        return Err(Error::Input(OfflineError::Empty));
    }
    if source.trim_start().starts_with(">>") {
        return Err(Error::Input(OfflineError::LanguagePrefix));
    }
    Ok(())
}

fn execute(
    plan: Plan,
    mut translate: impl FnMut(&str) -> Result<Translation, OfflineError>,
) -> Result<ScientificTranslation, Error> {
    let gaps = plan.gaps();
    let calls = gaps
        .iter()
        .filter(|gap| gap.chars().any(char::is_alphanumeric))
        .count();
    if calls > MAX_CALLS {
        return Err(Error::Budget);
    }
    let mut generated_tokens = 0_usize;
    let mut translated = Vec::with_capacity(gaps.len());
    for (index, gap) in gaps.iter().enumerate() {
        if !gap.chars().any(char::is_alphanumeric) {
            translated.push((*gap).to_owned());
            continue;
        }
        let core = gap.trim();
        let left = gap.len() - gap.trim_start().len();
        let right = left + core.len();
        let result = translate(core).map_err(|error| Error::Gap { index, error })?;
        if result.reached_limit {
            return Err(Error::Truncated { index });
        }
        if result.text.trim().is_empty() {
            return Err(Error::EmptyGap { index });
        }
        generated_tokens = generated_tokens
            .checked_add(result.target_tokens.len().saturating_sub(1))
            .filter(|&n| n <= MAX_GENERATED)
            .ok_or(Error::Budget)?;
        translated.push(format!(
            "{}{}{}",
            &gap[..left],
            result.text.trim(),
            &gap[right..]
        ));
    }
    let borrowed: Vec<_> = translated.iter().map(String::as_str).collect();
    let text = plan.assemble(&borrowed).map_err(Error::Plan)?;
    Ok(ScientificTranslation {
        text,
        protected_spans: plan.spans().len(),
        term_spans: plan.spans().iter().filter(|s| s.kind == Kind::Term).count(),
        model_calls: calls,
        generated_tokens,
        fragmented_context: !plan.spans().is_empty() && calls > 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(text: &str) -> Translation {
        Translation {
            text: text.into(),
            source_tokens: vec![1],
            target_tokens: vec![0, 1],
            score: 0.0,
            reached_limit: false,
        }
    }

    #[test]
    fn protected_content_never_reaches_model() {
        let mut inputs = Vec::new();
        let output = translate_with("At 0.1 g, $x=2$ and 30°.", &[], |s| {
            inputs.push(s.to_owned());
            Ok(result(if s == "At" { "在" } else { "及" }))
        })
        .unwrap();
        assert_eq!(inputs, ["At", "and"]);
        assert_eq!(output.text, "在 0.1 g, $x=2$ 及 30°.");
        assert_eq!(output.protected_spans, 3);
        assert_eq!(output.model_calls, 2);
        assert!(output.fragmented_context);
    }

    #[test]
    fn all_protected_requires_no_inference() {
        let output = translate_with("0.1 g, 30°, $x=2$.", &[], |_| panic!("model called")).unwrap();
        assert_eq!(output.text, "0.1 g, 30°, $x=2$.");
        assert_eq!(output.model_calls, 0);
        assert!(!output.fragmented_context);
    }

    #[test]
    fn indexed_terms_never_enter_model() {
        let g = Glossary::builtin(crate::glossary::Domain::Geotechnical).unwrap();
        let mut inputs = Vec::new();
        let output = translate_indexed_with("We measured MAXIMUM STRAIN at 0.1 g.", &g, |s| {
            inputs.push(s.to_owned());
            Ok(result("测量"))
        })
        .unwrap();
        assert_eq!(inputs, ["We measured", "at"]);
        assert_eq!(output.term_spans, 1);
        assert_eq!(output.protected_spans, 2);
        assert!(output.text.contains("最大应变"));
        assert!(output.text.contains("0.1 g"));
    }

    #[test]
    fn plain_text_uses_single_call() {
        let output = translate_with("The result is stable.", &[], |s| {
            assert_eq!(s, "The result is stable.");
            Ok(result("结果稳定。"))
        })
        .unwrap();
        assert_eq!(output.text, "结果稳定。");
        assert_eq!(output.model_calls, 1);
        assert_eq!(output.protected_spans, 0);
    }

    #[test]
    fn fails_closed_without_partial_translation() {
        assert_eq!(
            translate_with("At 30°.", &[], |_| Err(OfflineError::NoTranslation)),
            Err(Error::Gap {
                index: 0,
                error: OfflineError::NoTranslation,
            })
        );
        assert_eq!(
            translate_with("At 30°.", &[], |_| {
                let mut r = result("在");
                r.reached_limit = true;
                Ok(r)
            }),
            Err(Error::Truncated { index: 0 })
        );
        assert_eq!(
            translate_with("At 30°.", &[], |_| Ok(result(" "))),
            Err(Error::EmptyGap { index: 0 })
        );
        assert_eq!(
            translate_with("At 30°.", &[], |_| Ok(result("[[RPX:0000]]"))),
            Err(Error::Plan(protection::Error::ReservedMarker))
        );
        assert_eq!(
            translate_with("broken $x", &[], |_| panic!("model called")),
            Err(Error::Plan(protection::Error::UnclosedMath))
        );
    }

    #[test]
    fn budgets_and_reserved_prefix() {
        let input = "At 1 Hz ".repeat(MAX_CALLS + 1);
        assert_eq!(
            translate_with(&input, &[], |_| panic!("model called")),
            Err(Error::Budget)
        );
        assert_eq!(
            translate_with(">>other<< 30°", &[], |_| panic!("model called")),
            Err(Error::Input(OfflineError::LanguagePrefix))
        );
        assert_eq!(
            translate_with("At 30°.", &[], |_| {
                let mut r = result("在");
                r.target_tokens = vec![0; MAX_GENERATED + 2];
                Ok(r)
            }),
            Err(Error::Budget)
        );
    }
}
