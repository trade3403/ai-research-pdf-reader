//! Bounded document orchestration over audited paragraph translation.
//! Every input block has exactly one accepted or rejected outcome.
use crate::{
    context::ContextEngine,
    glossary::Domain,
    offline::TextSession,
    paragraph::{self, ParagraphTranslation},
};
use rpinfer::{marian_model::GenerationOptions, matmul::Kernel};

pub const MAX_BLOCKS: usize = 1024;
pub const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_BLOCK_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

pub struct Block<'a> {
    pub id: u32,
    pub source: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InputLimit,
    InvalidOrder,
    InvalidCoverage(u32),
    OutputLimit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockResult {
    pub id: u32,
    pub outcome: Result<ParagraphTranslation, paragraph::Error>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DocumentTranslation {
    pub blocks: Vec<BlockResult>,
}

fn complete(source: &str, translated: &ParagraphTranslation) -> bool {
    if translated.segments.is_empty() {
        return false;
    }
    let mut end = 0;
    let mut combined = String::new();
    for segment in &translated.segments {
        let range = &segment.source;
        if range.start < end || range.start >= range.end {
            return false;
        }
        if !source
            .get(end..range.start)
            .is_some_and(|gap| gap.trim().is_empty())
            || !source
                .get(range.clone())
                .is_some_and(|part| !part.trim().is_empty())
            || segment.text.trim().is_empty()
        {
            return false;
        }
        end = range.end;
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&segment.text);
    }
    source.get(end..).is_some_and(|tail| tail.trim().is_empty()) && combined == translated.text
}

fn translate_with(
    blocks: &[Block<'_>],
    mut run: impl FnMut(&str) -> Result<ParagraphTranslation, paragraph::Error>,
) -> Result<DocumentTranslation, Error> {
    if blocks.is_empty() || blocks.len() > MAX_BLOCKS {
        return Err(Error::InputLimit);
    }
    let mut total = 0;
    let mut last = None;
    for block in blocks {
        if block.source.trim().is_empty() || block.source.len() > MAX_BLOCK_BYTES {
            return Err(Error::InputLimit);
        }
        total += block.source.len();
        if total > MAX_INPUT_BYTES {
            return Err(Error::InputLimit);
        }
        if last.is_some_and(|id| id >= block.id) {
            return Err(Error::InvalidOrder);
        }
        last = Some(block.id);
    }
    let mut results = Vec::with_capacity(blocks.len());
    let mut output = 0;
    for block in blocks {
        let outcome = run(block.source);
        if let Ok(ref translated) = outcome {
            if !complete(block.source, translated) {
                return Err(Error::InvalidCoverage(block.id));
            }
            output += translated.text.len();
            if output > MAX_OUTPUT_BYTES {
                return Err(Error::OutputLimit);
            }
        }
        results.push(BlockResult {
            id: block.id,
            outcome,
        });
    }
    Ok(DocumentTranslation { blocks: results })
}

/// Reuse a resident session; failures are explicit, never copied source text.
/// IDs are strictly increasing (gaps allowed for sampled documents). Segment
/// offsets remain UTF-8 byte offsets relative to the corresponding input block.
/// Document success describes orchestration, not translation quality.
pub fn translate(
    session: &TextSession<'_>,
    blocks: &[Block<'_>],
    engine: &ContextEngine,
    explicit: Option<Domain>,
    options: GenerationOptions,
    kernel: Kernel,
) -> Result<DocumentTranslation, Error> {
    translate_with(blocks, |source| {
        session.translate_paragraph(source, engine, explicit, options, kernel)
    })
}

/// First-pass failure is retained even when a retry succeeds.
#[derive(Debug, PartialEq, Eq)]
pub struct RetryRecord {
    pub id: u32,
    pub initial: paragraph::Error,
    pub recovered: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecoveryReport {
    pub document: DocumentTranslation,
    pub retries: Vec<RetryRecord>,
}

pub const MAX_RETRIES: usize = 64;

fn recover_with(
    blocks: &[Block<'_>],
    mut document: DocumentTranslation,
    max_retries: usize,
    mut run: impl FnMut(&str) -> Result<ParagraphTranslation, paragraph::Error>,
) -> Result<RecoveryReport, Error> {
    let mut output: usize = document
        .blocks
        .iter()
        .filter_map(|block| block.outcome.as_ref().ok())
        .map(|result| result.text.len())
        .sum();
    let mut retries = Vec::new();
    for (block, result) in blocks.iter().zip(&mut document.blocks) {
        if retries.len() >= max_retries.min(MAX_RETRIES) {
            break;
        }
        let eligible = matches!(
            &result.outcome,
            Err(paragraph::Error::Rejected { attempts, .. })
                if attempts.iter().any(|a| matches!(a.failure, paragraph::Failure::Quality(_)))
        );
        if !eligible {
            continue;
        }
        let next = run(block.source);
        if let Ok(ref translated) = next {
            if !complete(block.source, translated) {
                return Err(Error::InvalidCoverage(block.id));
            }
            output += translated.text.len();
            if output > MAX_OUTPUT_BYTES {
                return Err(Error::OutputLimit);
            }
        }
        let recovered = next.is_ok();
        let initial = std::mem::replace(&mut result.outcome, next)
            .expect_err("only rejected outcomes are retried");
        retries.push(RetryRecord {
            id: block.id,
            initial,
            recovered,
        });
    }
    Ok(RecoveryReport { document, retries })
}

/// Opt-in bounded retry of lexical failures only. The first pass is exactly
/// the ordinary document path. Retry uses a wider beam, unchanged audit,
/// domain, maximum output length and source ranges; no accepted block is
/// regenerated. No retry for invalid input, plan errors or generation-only
/// failures. With beam width other than one the retry is disabled.
pub fn translate_with_retry(
    session: &TextSession<'_>,
    blocks: &[Block<'_>],
    engine: &ContextEngine,
    explicit: Option<Domain>,
    options: GenerationOptions,
    kernel: Kernel,
) -> Result<RecoveryReport, Error> {
    let document = translate(session, blocks, engine, explicit, options, kernel)?;
    let max_retries = if options.beam_width == 1 {
        MAX_RETRIES
    } else {
        0
    };
    recover_with(blocks, document, max_retries, |source| {
        session.translate_paragraph(
            source,
            engine,
            explicit,
            GenerationOptions {
                beam_width: 2,
                ..options
            },
            kernel,
        )
    })
}

pub const MAX_NUMERIC_RETRIES: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub struct NumericRetryRecord {
    pub id: u32,
    /// Present when a new accepted translation replaced the prior rejection.
    pub previous: Option<paragraph::Error>,
    /// Present on failed numeric recovery; the prior rejection stays intact.
    pub failure: Option<paragraph::Error>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NumericRecoveryReport {
    pub previous: RecoveryReport,
    pub numeric: Vec<NumericRetryRecord>,
}

fn recover_numeric_with(
    blocks: &[Block<'_>],
    mut previous: RecoveryReport,
    budget: usize,
    mut run: impl FnMut(&str) -> Result<ParagraphTranslation, paragraph::Error>,
) -> Result<NumericRecoveryReport, Error> {
    let mut output: usize = previous
        .document
        .blocks
        .iter()
        .filter_map(|b| b.outcome.as_ref().ok())
        .map(|result| result.text.len())
        .sum();
    let mut numeric = Vec::new();
    for (block, result) in blocks.iter().zip(&mut previous.document.blocks) {
        if numeric.len() >= budget.min(MAX_NUMERIC_RETRIES) {
            break;
        }
        let eligible = matches!(
            &result.outcome,
            Err(paragraph::Error::Rejected { attempts, .. })
                if attempts.iter().any(|a| matches!(
                    a.failure,
                    paragraph::Failure::Quality(
                        paragraph::QualityIssue::NumericMismatch
                        | paragraph::QualityIssue::PercentageMismatch
                    )
                ))
        );
        if !eligible {
            continue;
        }
        match run(block.source) {
            Ok(translated) => {
                if !complete(block.source, &translated) {
                    return Err(Error::InvalidCoverage(block.id));
                }
                output += translated.text.len();
                if output > MAX_OUTPUT_BYTES {
                    return Err(Error::OutputLimit);
                }
                let old = std::mem::replace(&mut result.outcome, Ok(translated))
                    .expect_err("only rejected outcomes are retried");
                numeric.push(NumericRetryRecord {
                    id: block.id,
                    previous: Some(old),
                    failure: None,
                });
            }
            Err(error) => numeric.push(NumericRetryRecord {
                id: block.id,
                previous: None,
                failure: Some(error),
            }),
        }
    }
    Ok(NumericRecoveryReport { previous, numeric })
}

/// Opt-in numeric recovery after the unchanged initial and beam retry passes.
/// Only remaining numeric/percentage rejections are eligible. Existing
/// accepted blocks are untouched, and all original audits still apply.
pub fn translate_with_numeric_retry(
    session: &TextSession<'_>,
    blocks: &[Block<'_>],
    engine: &ContextEngine,
    explicit: Option<Domain>,
    options: GenerationOptions,
    kernel: Kernel,
) -> Result<NumericRecoveryReport, Error> {
    let previous = translate_with_retry(session, blocks, engine, explicit, options, kernel)?;
    let budget = if options.beam_width == 1 {
        MAX_NUMERIC_RETRIES
    } else {
        0
    };
    recover_numeric_with(blocks, previous, budget, |source| {
        session.translate_numeric_paragraph(
            source,
            engine,
            explicit,
            GenerationOptions {
                beam_width: 2,
                ..options
            },
            kernel,
        )
    })
}

/// Structural and lexical evidence only; never a semantic quality score.
#[derive(Debug, PartialEq, Eq)]
pub struct QualitySummary {
    pub blocks: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub source_words: usize,
    pub accepted_words: usize,
    pub source_numbers: usize,
    pub accepted_numbers: usize,
    pub segments: usize,
    pub beam_recovered: usize,
    pub numeric_recovered: usize,
    pub first_pass_accepted: usize,
}

impl QualitySummary {
    /// Human review is mandatory even when all blocks pass lexical audit.
    pub fn needs_semantic_review(&self) -> bool {
        true
    }

    pub fn has_rejections(&self) -> bool {
        self.rejected != 0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QualityError {
    Input,
    Mapping,
    Provenance(u32),
    Coverage(u32),
    Lexical(u32, paragraph::QualityIssue),
    Plan(u32, crate::protection::Error),
}

fn eligible(error: &paragraph::Error, numeric: bool) -> bool {
    matches!(
        error,
        paragraph::Error::Rejected { attempts, .. }
            if attempts.iter().any(|a| match &a.failure {
                paragraph::Failure::Quality(issue) => {
                    !numeric || matches!(
                        issue,
                        paragraph::QualityIssue::NumericMismatch
                            | paragraph::QualityIssue::PercentageMismatch
                    )
                }
                paragraph::Failure::Generation(_) => false,
            })
    )
}

/// Verify the source mapping, every accepted segment and recovery provenance
/// without any inference. This is a lexical report, not a release approval.
/// The caller must supply the original blocks and the same glossary engine.
pub fn assess(
    blocks: &[Block<'_>],
    report: &NumericRecoveryReport,
    engine: &ContextEngine,
) -> Result<QualitySummary, QualityError> {
    let results = &report.previous.document.blocks;
    if blocks.is_empty() || blocks.len() > MAX_BLOCKS {
        return Err(QualityError::Input);
    }
    if results.len() != blocks.len()
        || blocks.windows(2).any(|w| w[0].id >= w[1].id)
        || blocks.iter().zip(results).any(|(b, r)| b.id != r.id)
    {
        return Err(QualityError::Mapping);
    }
    let retries = &report.previous.retries;
    let numeric = &report.numeric;
    if retries.len() > MAX_RETRIES
        || numeric.len() > MAX_NUMERIC_RETRIES
        || retries.windows(2).any(|w| w[0].id >= w[1].id)
        || numeric.windows(2).any(|w| w[0].id >= w[1].id)
    {
        return Err(QualityError::Mapping);
    }
    let mut summary = QualitySummary {
        blocks: blocks.len(),
        accepted: 0,
        rejected: 0,
        source_words: 0,
        accepted_words: 0,
        source_numbers: 0,
        accepted_numbers: 0,
        segments: 0,
        beam_recovered: 0,
        numeric_recovered: 0,
        first_pass_accepted: 0,
    };
    let mut input = 0;
    let mut output = 0;
    for (block, result) in blocks.iter().zip(results) {
        input += block.source.len();
        if input > MAX_INPUT_BYTES
            || block.source.len() > MAX_BLOCK_BYTES
            || block.source.trim().is_empty()
        {
            return Err(QualityError::Input);
        }
        let words = block.source.split_whitespace().count();
        let numbers = paragraph::numeric_string_count(block.source);
        summary.source_words += words;
        summary.source_numbers += numbers;
        let beam = retries
            .binary_search_by_key(&block.id, |r| r.id)
            .ok()
            .map(|i| &retries[i]);
        let num = numeric
            .binary_search_by_key(&block.id, |r| r.id)
            .ok()
            .map(|i| &numeric[i]);
        let accepted = result.outcome.is_ok();
        if let Some(record) = beam {
            if !eligible(&record.initial, false)
                || (record.recovered && (!accepted || num.is_some()))
                || (!record.recovered && accepted && num.is_none())
            {
                return Err(QualityError::Provenance(block.id));
            }
            summary.beam_recovered += usize::from(record.recovered);
        }
        if let Some(record) = num {
            match (&record.previous, &record.failure, &result.outcome) {
                (Some(old), None, Ok(_)) if eligible(old, true) => {
                    summary.numeric_recovered += 1;
                }
                (None, Some(_), Err(old)) if eligible(old, true) => {}
                _ => return Err(QualityError::Provenance(block.id)),
            }
        }
        if let Ok(translated) = &result.outcome {
            if !complete(block.source, translated) {
                return Err(QualityError::Coverage(block.id));
            }
            output += translated.text.len();
            if output > MAX_OUTPUT_BYTES || translated.segments.len() > 64 {
                return Err(QualityError::Coverage(block.id));
            }
            for segment in &translated.segments {
                let source = &block.source[segment.source.clone()];
                let glossary = engine.glossary(segment.domain);
                let plan = crate::protection::Plan::prepare_indexed(source, glossary)
                    .map_err(|e| QualityError::Plan(block.id, e))?;
                paragraph::audit(source, &plan, &segment.text)
                    .map_err(|e| QualityError::Lexical(block.id, e))?;
                if num.is_some() {
                    let extended =
                        crate::protection::Plan::prepare_indexed_numeric(source, glossary)
                            .map_err(|e| QualityError::Plan(block.id, e))?;
                    paragraph::audit(source, &extended, &segment.text)
                        .map_err(|e| QualityError::Lexical(block.id, e))?;
                }
            }
            summary.accepted += 1;
            summary.accepted_words += words;
            summary.accepted_numbers += numbers;
            summary.segments += translated.segments.len();
        } else {
            summary.rejected += 1;
        }
    }
    for id in retries
        .iter()
        .map(|r| r.id)
        .chain(numeric.iter().map(|r| r.id))
    {
        if blocks.binary_search_by_key(&id, |b| b.id).is_err() {
            return Err(QualityError::Provenance(id));
        }
    }
    summary.first_pass_accepted =
        summary.accepted - summary.beam_recovered - summary.numeric_recovered;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paragraph::{Segment, Strategy};

    fn accepted(source: &str) -> ParagraphTranslation {
        ParagraphTranslation {
            text: "译文".into(),
            segments: vec![Segment {
                source: 0..source.len(),
                domain: Domain::Geotechnical,
                basis: None,
                strategy: Strategy::Whole,
                rejected: vec![],
                protected_spans: 0,
                text: "译文".into(),
            }],
        }
    }

    #[test]
    fn preserves_order_and_explicit_rejection() {
        let blocks = [
            Block {
                id: 3,
                source: "A.",
            },
            Block {
                id: 8,
                source: "B.",
            },
            Block {
                id: 9,
                source: "C.",
            },
        ];
        let result = translate_with(&blocks, |source| {
            if source == "B." {
                Err(paragraph::Error::Limit)
            } else {
                Ok(accepted(source))
            }
        })
        .unwrap();
        assert_eq!(
            result.blocks.iter().map(|b| b.id).collect::<Vec<_>>(),
            [3, 8, 9]
        );
        assert!(result.blocks[0].outcome.is_ok());
        assert_eq!(result.blocks[1].outcome, Err(paragraph::Error::Limit));
        assert!(result.blocks[2].outcome.is_ok());
    }

    #[test]
    fn validates_all_inputs_before_inference() {
        let blocks = [Block { id: 1, source: "A" }, Block { id: 1, source: "B" }];
        assert_eq!(
            translate_with(&blocks, |_| panic!("must not run")),
            Err(Error::InvalidOrder)
        );
        let long = "a".repeat(MAX_BLOCK_BYTES + 1);
        let blocks = [Block {
            id: 1,
            source: &long,
        }];
        assert_eq!(
            translate_with(&blocks, |_| panic!("must not run")),
            Err(Error::InputLimit)
        );
        let large = "x".repeat(MAX_BLOCK_BYTES);
        let blocks: Vec<_> = (0..33).map(|id| Block { id, source: &large }).collect();
        assert_eq!(
            translate_with(&blocks, |_| panic!("must not run")),
            Err(Error::InputLimit)
        );
    }

    #[test]
    fn verifies_utf8_coverage_and_joined_text() {
        let source = "学 A";
        let mut result = accepted(source);
        assert!(complete(source, &result));
        result.segments[0].source = 1..source.len();
        assert!(!complete(source, &result));
        result.segments[0].source = 4..source.len();
        assert!(!complete(source, &result));
        result.segments[0].source = 0..3;
        assert!(!complete(source, &result));
        result.segments[0].source = 0..source.len();
        result.text.push('!');
        assert!(!complete(source, &result));
    }

    fn rejection(quality: bool) -> paragraph::Error {
        paragraph::Error::Rejected {
            index: 0,
            attempts: vec![paragraph::Attempt {
                strategy: Strategy::Whole,
                failure: if quality {
                    paragraph::Failure::Quality(paragraph::QualityIssue::NumericMismatch)
                } else {
                    paragraph::Failure::Generation("failure".into())
                },
            }],
        }
    }

    #[test]
    fn retry_only_lexical_rejections_and_keep_original_failure() {
        let blocks = [
            Block { id: 1, source: "A" },
            Block { id: 2, source: "B" },
            Block { id: 3, source: "C" },
            Block { id: 4, source: "D" },
        ];
        let first = translate_with(&blocks, |source| match source {
            "A" => Ok(accepted(source)),
            "B" | "D" => Err(rejection(true)),
            _ => Err(rejection(false)),
        })
        .unwrap();
        let result = recover_with(&blocks, first, 64, |source| match source {
            "B" => Ok(accepted(source)),
            "D" => Err(rejection(false)),
            _ => panic!("ineligible retry"),
        })
        .unwrap();
        assert_eq!(result.retries.len(), 2);
        assert_eq!(result.retries[0].id, 2);
        assert!(result.retries[0].recovered);
        assert_eq!(result.retries[0].initial, rejection(true));
        assert_eq!(result.document.blocks[0].outcome, Ok(accepted("A")));
        assert_eq!(result.document.blocks[1].outcome, Ok(accepted("B")));
        assert_eq!(result.document.blocks[2].outcome, Err(rejection(false)));
        assert_eq!(result.document.blocks[3].outcome, Err(rejection(false)));
        assert!(!result.retries[1].recovered);
    }

    #[test]
    fn retry_budget_and_invalid_coverage() {
        let blocks = [Block { id: 1, source: "A" }, Block { id: 2, source: "B" }];
        let first = translate_with(&blocks, |_| Err(rejection(true))).unwrap();
        let result = recover_with(&blocks, first, 1, |source| {
            assert_eq!(source, "A");
            Ok(accepted(source))
        })
        .unwrap();
        assert_eq!(result.retries.len(), 1);
        assert_eq!(result.document.blocks[1].outcome, Err(rejection(true)));
        let first = translate_with(&blocks, |_| Err(rejection(true))).unwrap();
        assert_eq!(
            recover_with(&blocks, first, 1, |source| {
                let mut result = accepted(source);
                result.segments.clear();
                Ok(result)
            }),
            Err(Error::InvalidCoverage(1))
        );
    }

    #[test]
    fn numeric_recovery_preserves_success_and_failed_prior_diagnosis() {
        let blocks = [
            Block { id: 1, source: "A" },
            Block { id: 2, source: "B" },
            Block { id: 3, source: "C" },
            Block { id: 4, source: "D" },
        ];
        let first = translate_with(&blocks, |source| match source {
            "A" => Ok(accepted(source)),
            "B" | "C" => Err(rejection(true)),
            _ => Err(rejection(false)),
        })
        .unwrap();
        let previous = recover_with(&blocks, first, 0, |_| panic!("no retry")).unwrap();
        let result = recover_numeric_with(&blocks, previous, 32, |source| match source {
            "B" => Ok(accepted(source)),
            "C" => Err(rejection(false)),
            _ => panic!("ineligible numeric retry"),
        })
        .unwrap();
        assert_eq!(result.numeric.len(), 2);
        assert_eq!(result.numeric[0].previous, Some(rejection(true)));
        assert_eq!(result.numeric[1].failure, Some(rejection(false)));
        assert_eq!(
            result.previous.document.blocks[0].outcome,
            Ok(accepted("A"))
        );
        assert_eq!(
            result.previous.document.blocks[1].outcome,
            Ok(accepted("B"))
        );
        assert_eq!(
            result.previous.document.blocks[2].outcome,
            Err(rejection(true))
        );
        assert_eq!(
            result.previous.document.blocks[3].outcome,
            Err(rejection(false))
        );
    }

    #[test]
    fn quality_summary_reports_rejections_and_needs_review() {
        let blocks = [Block { id: 1, source: "A" }, Block { id: 2, source: "B" }];
        let first = translate_with(&blocks, |source| {
            if source == "A" {
                Ok(accepted(source))
            } else {
                Err(rejection(true))
            }
        })
        .unwrap();
        let previous = recover_with(&blocks, first, 0, |_| panic!("no retry")).unwrap();
        let report = recover_numeric_with(&blocks, previous, 0, |_| panic!("no retry")).unwrap();
        let engine = ContextEngine::builtin().unwrap();
        let summary = assess(&blocks, &report, &engine).unwrap();
        assert_eq!(summary.accepted, 1);
        assert_eq!(summary.rejected, 1);
        assert_eq!(summary.source_words, 2);
        assert_eq!(summary.accepted_words, 1);
        assert!(summary.has_rejections());
        assert!(summary.needs_semantic_review());
    }

    #[test]
    fn quality_summary_rejects_forged_numeric_output() {
        let blocks = [Block { id: 1, source: "2" }];
        let document = translate_with(&blocks, |s| Ok(accepted(s))).unwrap();
        let report = NumericRecoveryReport {
            previous: RecoveryReport {
                document,
                retries: vec![],
            },
            numeric: vec![],
        };
        let engine = ContextEngine::builtin().unwrap();
        assert_eq!(
            assess(&blocks, &report, &engine),
            Err(QualityError::Lexical(
                1,
                paragraph::QualityIssue::NumericMismatch
            ))
        );
    }

    #[test]
    fn quality_summary_rejects_unknown_and_contradictory_provenance() {
        let blocks = [Block { id: 1, source: "A" }];
        let document = translate_with(&blocks, |s| Ok(accepted(s))).unwrap();
        let mut report = NumericRecoveryReport {
            previous: RecoveryReport {
                document,
                retries: vec![RetryRecord {
                    id: 99,
                    initial: rejection(true),
                    recovered: true,
                }],
            },
            numeric: vec![],
        };
        let engine = ContextEngine::builtin().unwrap();
        assert_eq!(
            assess(&blocks, &report, &engine),
            Err(QualityError::Provenance(99))
        );
        report.previous.retries[0].id = 1;
        report.previous.retries[0].recovered = false;
        assert_eq!(
            assess(&blocks, &report, &engine),
            Err(QualityError::Provenance(1))
        );
        report.previous.retries[0].recovered = true;
        let summary = assess(&blocks, &report, &engine).unwrap();
        assert_eq!(summary.first_pass_accepted, 0);
        assert_eq!(summary.beam_recovered, 1);
        assert!(!summary.has_rejections());
        assert!(summary.needs_semantic_review());
    }

    #[test]
    fn invalid_success_never_becomes_document_success() {
        let blocks = [Block {
            id: 42,
            source: "A.",
        }];
        assert_eq!(
            translate_with(&blocks, |source| {
                let mut result = accepted(source);
                result.segments.clear();
                Ok(result)
            }),
            Err(Error::InvalidCoverage(42))
        );
    }
}
