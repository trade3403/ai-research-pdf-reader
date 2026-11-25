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
    let max_retries = if options.beam_width == 1 { MAX_RETRIES } else { 0 };
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
