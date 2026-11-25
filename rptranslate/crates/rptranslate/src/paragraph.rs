//! Bounded paragraph translation with auditable candidate rejection.
//! This is a lexical guard, not a bilingual semantic quality estimator.
use crate::{
    context::{Basis, ContextEngine, DocumentContext},
    glossary::{Domain, Glossary},
    offline, protection,
    protection::{Kind, Plan},
};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

pub const MAX_SOURCE_TOKENS: usize = 192;
const MAX_SEGMENTS: usize = 64;
const MAX_BOUNDARIES: usize = 256;
const MAX_TEXT: usize = 64 * 1024;
const MAX_OUTPUT: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Whole,
    Constrained,
    ProtectedGaps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityIssue {
    Invalid,
    NumericMismatch,
    PercentageMismatch,
    MissingSurface(String),
    ExtraSurface(String),
    Conflict(String),
    RepeatedClause,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Generation(String),
    Quality(QualityIssue),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attempt {
    pub strategy: Strategy,
    pub failure: Failure,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Input(offline::Error),
    Plan(protection::Error),
    Limit,
    LongClause(usize),
    Rejected {
        index: usize,
        attempts: Vec<Attempt>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Segment {
    pub source: Range<usize>,
    pub domain: Domain,
    pub basis: Option<Basis>,
    pub strategy: Strategy,
    pub rejected: Vec<Attempt>,
    pub protected_spans: usize,
    pub text: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParagraphTranslation {
    pub text: String,
    pub segments: Vec<Segment>,
}

fn push_trimmed(
    text: &str,
    out: &mut Vec<Range<usize>>,
    start: usize,
    end: usize,
) -> Result<(), Error> {
    let raw = &text[start..end];
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
        if out.len() >= MAX_BOUNDARIES {
            return Err(Error::Limit);
        }
        let left = start + raw.len() - raw.trim_start().len();
        out.push(left..left + trimmed.len());
    }
    Ok(())
}

fn abbreviation(before: &str) -> bool {
    let word = before.split_whitespace().next_back().unwrap_or("");
    let lower = word.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "fig"
            | "figs"
            | "eq"
            | "eqs"
            | "ref"
            | "refs"
            | "dr"
            | "prof"
            | "mr"
            | "mrs"
            | "al"
            | "etc"
            | "vs"
            | "e.g"
            | "i.e"
            | "u.s"
            | "no"
    ) || (word.len() == 1 && word.as_bytes()[0].is_ascii_uppercase())
}

/// Boundary scanning never cuts a recognized literal or bracketed expression.
/// Sentence boundaries are conservative; clause boundaries are used only for
/// over-budget sentences. All returned ranges are original UTF-8 byte ranges.
fn ranges(text: &str, clauses: bool) -> Result<Vec<Range<usize>>, Error> {
    let plan = Plan::prepare(text, &[]).map_err(Error::Plan)?;
    let mut protected = plan.spans().iter().peekable();
    let mut depth = 0_usize;
    let mut start = 0;
    let mut out = Vec::new();
    for (at, c) in text.char_indices() {
        while protected.peek().is_some_and(|s| s.range.end <= at) {
            protected.next();
        }
        if protected.peek().is_some_and(|s| s.range.contains(&at)) {
            continue;
        }
        match c {
            '(' | '[' | '{' | '（' | '【' => depth += 1,
            ')' | ']' | '}' | '）' | '】' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth != 0 {
            continue;
        }
        let end = at + c.len_utf8();
        let space_after = text[end..].chars().next().is_none_or(char::is_whitespace);
        let boundary = if clauses {
            matches!(c, '；' | '，' | '：') || (matches!(c, ';' | ',' | ':') && space_after)
        } else {
            matches!(c, '。' | '！' | '？')
                || (matches!(c, '!' | '?') && space_after)
                || (c == '.' && space_after && !abbreviation(&text[..at]))
        };
        if boundary {
            push_trimmed(text, &mut out, start, end)?;
            start = end;
        }
    }
    push_trimmed(text, &mut out, start, text.len())?;
    Ok(out)
}

fn chunks(
    text: &str,
    mut fits: impl FnMut(&str) -> Result<bool, offline::Error>,
) -> Result<Vec<Range<usize>>, Error> {
    let mut out = Vec::new();
    for (index, sentence) in ranges(text, false)?.into_iter().enumerate() {
        if fits(&text[sentence.clone()]).map_err(Error::Input)? {
            out.push(sentence);
        } else {
            let local = &text[sentence.clone()];
            let mut pending: Option<Range<usize>> = None;
            for clause in ranges(local, true)? {
                let clause = sentence.start + clause.start..sentence.start + clause.end;
                if !fits(&text[clause.clone()]).map_err(Error::Input)? {
                    return Err(Error::LongClause(index));
                }
                if let Some(previous) = pending.take() {
                    let combined = previous.start..clause.end;
                    if fits(&text[combined.clone()]).map_err(Error::Input)? {
                        pending = Some(combined);
                    } else {
                        out.push(previous);
                        pending = Some(clause);
                    }
                } else {
                    pending = Some(clause);
                }
                if out.len() >= MAX_SEGMENTS {
                    return Err(Error::Limit);
                }
            }
            if let Some(last) = pending {
                out.push(last);
            }
        }
        if out.len() > MAX_SEGMENTS {
            return Err(Error::Limit);
        }
    }
    Ok(out)
}

/// Count ASCII digit sequences including internal decimal dots, even inside
/// formulas and identifiers. Exact strings, not numeric equivalence or units.
fn numbers(text: &str) -> BTreeMap<&str, usize> {
    let bytes = text.as_bytes();
    let mut out = BTreeMap::new();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit() {
            at += 1;
            continue;
        }
        let start = at;
        at += 1;
        while at < bytes.len()
            && (bytes[at].is_ascii_digit()
                || (bytes[at] == b'.' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit)))
        {
            at += 1;
        }
        *out.entry(&text[start..at]).or_insert(0) += 1;
    }
    out
}

/// Count numeric strings immediately preceding percent signs. Only horizontal
/// spaces are ignored; detached signs remain separate, auditable entries.
fn percentages(text: &str) -> BTreeMap<Option<&str>, usize> {
    let mut out = BTreeMap::new();
    for (at, c) in text.char_indices() {
        if !matches!(c, '%' | '％') {
            continue;
        }
        let before = text[..at].trim_end_matches([' ', '\t', '\u{a0}', '\u{202f}']);
        let bytes = before.as_bytes();
        let mut start = bytes.len();
        while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
            start -= 1;
        }
        let value = &before[start..];
        let bound = value.as_bytes().last().is_some_and(u8::is_ascii_digit);
        *out.entry(bound.then_some(value)).or_insert(0) += 1;
    }
    out
}

/// Detect a narrow observed echo pattern: a consecutive clause repeats a
/// long suffix, optionally preceded by an ASCII label such as "2D".
/// This is deliberately lexical and may reject legitimate rhetorical echoes.
fn repeated_suffix(previous: &str, current: &str) -> bool {
    let suffix = current.trim_start_matches(|c: char| c.is_ascii_alphanumeric());
    suffix.chars().count() >= 7
        && suffix.chars().any(|c| !c.is_ascii())
        && previous.ends_with(suffix)
}

fn repeated_clause(text: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut previous = String::new();
    for clause in text.split(['。', '！', '？', '；', ';', '!', '?', '.']) {
        let normalized: String = clause.chars().filter(|c| !c.is_whitespace()).collect();
        if repeated_suffix(&previous, &normalized) {
            return true;
        }
        if normalized.chars().count() >= 6 && !seen.insert(normalized.clone()) {
            return true;
        }
        if !normalized.is_empty() {
            previous = normalized;
        }
    }
    false
}

/// Reject exact detected defects rather than deleting text or guessing fixes.
/// Longest surfaces consume non-overlapping occurrences; reordering is allowed.
/// Numeric equality is a multiset, not a claim of correct number placement.
pub fn audit(source: &str, plan: &Plan, output: &str) -> Result<(), QualityIssue> {
    if output.trim().is_empty() || output.len() > MAX_OUTPUT || output.contains("[[RPX:") {
        return Err(QualityIssue::Invalid);
    }
    if numbers(source) != numbers(output) {
        return Err(QualityIssue::NumericMismatch);
    }
    if percentages(source) != percentages(output) {
        return Err(QualityIssue::PercentageMismatch);
    }
    let mut expected = BTreeMap::new();
    for span in plan.spans() {
        *expected.entry(span.replacement.as_str()).or_insert(0_usize) += 1;
    }
    let mut targets: Vec<_> = expected.into_iter().collect();
    targets.sort_by_key(|(target, _)| Reverse(target.len()));
    let mut observed = vec![0; targets.len()];
    let mut at = 0;
    while at < output.len() {
        if let Some(i) = targets
            .iter()
            .position(|(target, _)| output[at..].starts_with(target))
        {
            observed[i] += 1;
            at += targets[i].0.len();
        } else if let Some(c) = output[at..].chars().next() {
            at += c.len_utf8();
        }
    }
    for ((target, count), seen) in targets.iter().zip(observed) {
        if seen < *count {
            return Err(QualityIssue::MissingSurface((*target).into()));
        }
        if seen > *count {
            return Err(QualityIssue::ExtraSurface((*target).into()));
        }
    }
    for span in plan.spans().iter().filter(|s| s.kind == Kind::Term) {
        // Explicit observed failure patterns, scoped to the expected term.
        let conflicts: &[&str] = match span.replacement.as_str() {
            "最大应变" => &[
                "最大的压力",
                "最大菌株",
                "最大强度的菌株",
                "最大应变量",
                "最大应变强度",
            ],
            "场地响应" => &["现场反应"],
            "入射角" => &["发生角度", "发生率角"],
            "输入地震动" => &["输入动作", "输入动议"],
            "谱幅值" => &["光谱振幅"],
            _ => &[],
        };
        if let Some(bad) = conflicts.iter().find(|&&bad| output.contains(bad)) {
            return Err(QualityIssue::Conflict((*bad).into()));
        }
    }
    if repeated_clause(output) {
        return Err(QualityIssue::RepeatedClause);
    }
    Ok(())
}

struct Selection {
    text: String,
    strategy: Strategy,
    rejected: Vec<Attempt>,
}

fn select(
    source: &str,
    plan: &Plan,
    mut generate: impl FnMut(Strategy) -> Result<String, String>,
) -> Result<Selection, Vec<Attempt>> {
    let mut rejected = Vec::new();
    for strategy in [
        Strategy::Whole,
        Strategy::Constrained,
        Strategy::ProtectedGaps,
    ] {
        let failure = match generate(strategy) {
            Ok(text) => match audit(source, plan, &text) {
                Ok(()) => {
                    return Ok(Selection {
                        text,
                        strategy,
                        rejected,
                    });
                }
                Err(issue) => Failure::Quality(issue),
            },
            Err(error) => Failure::Generation(error),
        };
        rejected.push(Attempt { strategy, failure });
    }
    Err(rejected)
}

pub(crate) fn translate_with(
    source: &str,
    engine: &ContextEngine,
    explicit: Option<Domain>,
    fits: impl FnMut(&str) -> Result<bool, offline::Error>,
    mut generate: impl FnMut(&str, &Glossary, Strategy) -> Result<String, String>,
) -> Result<ParagraphTranslation, Error> {
    if source.trim().is_empty() {
        return Err(Error::Input(offline::Error::Empty));
    }
    if source.trim_start().starts_with(">>") {
        return Err(Error::Input(offline::Error::LanguagePrefix));
    }
    if source.len() > MAX_TEXT {
        return Err(Error::Limit);
    }
    let prior = DocumentContext::from_text(source).map_err(Error::Plan)?;
    let chunks = chunks(source, fits)?;
    let mut segments = Vec::new();
    let mut text = String::new();
    for (index, range) in chunks.into_iter().enumerate() {
        let input = &source[range.clone()];
        let (domain, basis) = if let Some(domain) = explicit {
            (domain, None)
        } else {
            let d = engine.decide(input, Some(&prior)).map_err(Error::Plan)?;
            (d.domain, Some(d.basis))
        };
        let glossary = engine.glossary(domain);
        let plan = Plan::prepare_indexed(input, glossary).map_err(Error::Plan)?;
        let result = select(input, &plan, |strategy| generate(input, glossary, strategy))
            .map_err(|attempts| Error::Rejected { index, attempts })?;
        if result.text.len() + 1 > MAX_OUTPUT.saturating_sub(text.len()) {
            return Err(Error::Limit);
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&result.text);
        segments.push(Segment {
            source: range,
            domain,
            basis,
            strategy: result.strategy,
            rejected: result.rejected,
            protected_spans: plan.spans().len(),
            text: result.text,
        });
    }
    Ok(ParagraphTranslation { text, segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_boundaries_preserve_literals_and_abbreviations() {
        let s = "Fig. 1 shows 0.1 g and $x. y$. We measured strain. (A. B.) Results.";
        let parts: Vec<_> = ranges(s, false)
            .unwrap()
            .into_iter()
            .map(|r| &s[r])
            .collect();
        assert_eq!(
            parts,
            [
                "Fig. 1 shows 0.1 g and $x. y$.",
                "We measured strain.",
                "(A. B.) Results."
            ]
        );
    }

    #[test]
    fn long_clauses_only_split_at_safe_boundaries() {
        let s = "We saw soil; we saw shear; we saw strain.";
        let out = chunks(s, |t| Ok(t.split_whitespace().count() <= 3)).unwrap();
        let parts: Vec<_> = out.into_iter().map(|r| &s[r]).collect();
        assert_eq!(parts, ["We saw soil;", "we saw shear;", "we saw strain."]);
        assert_eq!(
            chunks("one two three four", |t| Ok(t.len() < 10)),
            Err(Error::LongClause(0))
        );
    }

    #[test]
    fn audit_rejects_observed_failures_and_duplicate_numbers() {
        let g = Glossary::builtin(Domain::Geotechnical).unwrap();
        let s = "We measured maximum strain at 0.1 g.";
        let p = Plan::prepare_indexed(s, &g).unwrap();
        assert_eq!(audit(s, &p, "我们在0.1 g时测量了最大应变。"), Ok(()));
        assert_eq!(
            audit(s, &p, "我们测量了最大应变，0.1克。0.1 g。"),
            Err(QualityIssue::NumericMismatch)
        );
        assert_eq!(
            audit(s, &p, "0.1 g最大应变强度"),
            Err(QualityIssue::Conflict("最大应变强度".into()))
        );
        assert_eq!(
            audit(s, &p, "0.1 g最大应变最大应变"),
            Err(QualityIssue::ExtraSurface("最大应变".into()))
        );
        assert_eq!(
            audit(s, &p, "0.1 g最大菌株"),
            Err(QualityIssue::MissingSurface("最大应变".into()))
        );
    }

    #[test]
    fn overlapping_surfaces_and_repeated_source_counts() {
        let g = Glossary::builtin(Domain::Geotechnical).unwrap();
        let s = "Maximum strain and strain at 1 Hz and 1 Hz.";
        let p = Plan::prepare_indexed(s, &g).unwrap();
        assert_eq!(audit(s, &p, "最大应变与应变，1 Hz及1 Hz。"), Ok(()));
        assert_eq!(
            audit(s, &p, "最大应变，1 Hz及1 Hz。"),
            Err(QualityIssue::MissingSurface("应变".into()))
        );
        let p = Plan::prepare("Results are stable.", &[]).unwrap();
        assert_eq!(
            audit(
                "Results are stable.",
                &p,
                "研究结果保持稳定。研究结果保持稳定。"
            ),
            Err(QualityIssue::RepeatedClause)
        );
    }

    #[test]
    fn percentage_binding_and_detached_signs() {
        assert_eq!(percentages("50% and 2.5 %"), percentages("2.5％及50 %"));
        assert_ne!(percentages("50% and 2.5"), percentages("50及2.5%"));
        assert_ne!(percentages("50%"), percentages("50 后减少%"));
        assert_ne!(percentages("50%"), percentages("50% %"));
        let source = "The reduction is 50%.";
        let plan = Plan::prepare(source, &[]).unwrap();
        assert_eq!(audit(source, &plan, "减少50%。"), Ok(()));
        assert_eq!(
            audit(source, &plan, "达到50，减少%。"),
            Err(QualityIssue::PercentageMismatch)
        );
    }

    #[test]
    fn repeated_suffix_regression() {
        assert!(repeated_clause("显示我们二维区域中层的几何。2D区域中层的几何。"));
        assert!(repeated_clause("结果表明研究区的地震波传播。研究区的地震波传播。"));
        assert!(!repeated_clause("研究区的地震波传播。研究区的地震波衰减。"));
        assert!(!repeated_clause("结果稳定。仍然稳定。"));
    }

    #[test]
    fn paragraph_prior_and_fallback_are_reported() {
        let engine = ContextEngine::builtin().unwrap();
        let s = "Soil shear strain increased. Maximum strain increased.";
        let out = translate_with(
            s,
            &engine,
            None,
            |_| Ok(true),
            |source, _, strategy| {
                if strategy == Strategy::Whole {
                    Ok("菌株增加。".into())
                } else if strategy == Strategy::Constrained {
                    Ok("错误".into())
                } else if source.starts_with("Soil") {
                    Ok("土壤剪应变增加。".into())
                } else {
                    Ok("最大应变增加。".into())
                }
            },
        )
        .unwrap();
        assert_eq!(out.segments.len(), 2);
        assert_eq!(out.segments[0].basis, Some(Basis::Local));
        assert_eq!(out.segments[1].basis, Some(Basis::Document));
        for segment in out.segments {
            assert_eq!(segment.strategy, Strategy::ProtectedGaps);
            assert_eq!(segment.rejected.len(), 2);
        }
    }

    #[test]
    fn all_bad_candidates_fail_without_partial_output() {
        let engine = ContextEngine::builtin().unwrap();
        let err = translate_with(
            "Results are stable. We measured maximum strain.",
            &engine,
            Some(Domain::Geotechnical),
            |_| Ok(true),
            |source, _, _| {
                if source.starts_with("Results") {
                    Ok("结果稳定。".into())
                } else {
                    Ok("我们测量了最大菌株。".into())
                }
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::Rejected { index: 1, attempts } if attempts.len() == 3));
    }
}
