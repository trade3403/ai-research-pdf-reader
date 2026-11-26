//! Versioned in-process document selection protocol. No PDF/Word dependency.
use crate::{
    context::ContextEngine,
    document,
    glossary::Domain,
    offline::{OfflineTranslator, Precision},
    paragraph::{self, ParagraphTranslation},
    protection::{Kind, Plan},
};
use rpinfer::{marian_model::GenerationOptions, matmul::Kernel};
use rpmodel::catalog::Version;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

pub const VERSION: u32 = 1;
pub const MAX_BLOCK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Word,
}

/// Zero-based positions in the caller's immutable document revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Location {
    Pdf { page: u32, block: u32 },
    Word { paragraph: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguagePair {
    EnglishToSimplifiedChinese,
}

/// Identity/version are supplied by the trusted model owner, not inferred
/// from weights. Precision is also checked against the resident translator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity<'a> {
    pub id: &'a str,
    pub version: Version,
    pub precision: Precision,
    pub language: LanguagePair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Formula,
    Citation,
    Term,
    Quantity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSpan {
    pub range: Range<usize>,
    pub kind: HostKind,
}

pub struct Selection<'a> {
    pub id: u32,
    pub location: Location,
    pub block: &'a str,
    /// Half-open UTF-8 byte offsets, not UTF-16 or glyph indices.
    pub range: Range<usize>,
    /// Structural hints from the document adapter, in block coordinates.
    pub host_spans: &'a [HostSpan],
}

pub struct Request<'a> {
    pub protocol: u32,
    pub document_id: &'a str,
    pub revision: u64,
    pub format: Format,
    pub model: ModelIdentity<'a>,
    pub selections: &'a [Selection<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Protocol,
    Identity,
    InputLimit,
    InvalidOrder,
    InvalidLocation,
    InvalidRange,
    InvalidProtection,
    SplitProtection,
    UnsupportedHostProtection,
    PrecisionMismatch,
    ModelSession,
    InvalidMapping,
    OutputLimit,
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Protocol => "protocol",
            Self::Identity => "identity",
            Self::InputLimit => "input_limit",
            Self::InvalidOrder => "invalid_order",
            Self::InvalidLocation => "invalid_location",
            Self::InvalidRange => "invalid_range",
            Self::InvalidProtection => "invalid_protection",
            Self::SplitProtection => "split_protection",
            Self::UnsupportedHostProtection => "unsupported_host_protection",
            Self::PrecisionMismatch => "precision_mismatch",
            Self::ModelSession => "model_session",
            Self::InvalidMapping => "invalid_mapping",
            Self::OutputLimit => "output_limit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedSpan {
    pub range: Range<usize>,
    pub kind: Kind,
}

#[derive(Debug)]
pub struct PreparedBlock<'a> {
    pub id: u32,
    pub location: Location,
    pub range: Range<usize>,
    pub source: &'a str,
    pub detected: Vec<DetectedSpan>,
    pub host_spans: Vec<HostSpan>,
}

#[derive(Debug)]
pub struct Prepared<'a> {
    document_id: &'a str,
    revision: u64,
    format: Format,
    model: ModelIdentity<'a>,
    blocks: Vec<PreparedBlock<'a>>,
}

impl<'a> Prepared<'a> {
    pub fn blocks(&self) -> &[PreparedBlock<'a>] {
        &self.blocks
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn contained(selection: &Range<usize>, span: &Range<usize>) -> Result<bool, Error> {
    if span.end <= selection.start || span.start >= selection.end {
        return Ok(false);
    }
    if span.start < selection.start || span.end > selection.end {
        return Err(Error::SplitProtection);
    }
    Ok(true)
}

/// Validate the complete request before any inference or text slicing.
pub fn prepare<'a>(request: &Request<'a>) -> Result<Prepared<'a>, Error> {
    if request.protocol != VERSION {
        return Err(Error::Protocol);
    }
    if !valid_id(request.document_id) || !valid_id(request.model.id) {
        return Err(Error::Identity);
    }
    if request.selections.is_empty() || request.selections.len() > document::MAX_BLOCKS {
        return Err(Error::InputLimit);
    }
    let mut total = 0;
    let mut last = None;
    let mut locations = BTreeSet::new();
    let mut blocks = Vec::with_capacity(request.selections.len());
    for selection in request.selections {
        total += selection.block.len().min(document::MAX_INPUT_BYTES + 1);
        if total > document::MAX_INPUT_BYTES || selection.block.len() > MAX_BLOCK_BYTES {
            return Err(Error::InputLimit);
        }
        if last.is_some_and(|id| id >= selection.id) {
            return Err(Error::InvalidOrder);
        }
        last = Some(selection.id);
        if !matches!(
            (request.format, selection.location),
            (Format::Pdf, Location::Pdf { .. }) | (Format::Word, Location::Word { .. })
        ) || !locations.insert(selection.location)
        {
            return Err(Error::InvalidLocation);
        }
        let source = selection
            .block
            .get(selection.range.clone())
            .filter(|s| !s.trim().is_empty())
            .ok_or(Error::InvalidRange)?;
        if selection.host_spans.len() > 1024 {
            return Err(Error::InputLimit);
        }
        let mut host_spans = Vec::new();
        let mut previous = 0;
        for span in selection.host_spans {
            if span.range.start < previous
                || span.range.start >= span.range.end
                || selection.block.get(span.range.clone()).is_none()
            {
                return Err(Error::InvalidProtection);
            }
            previous = span.range.end;
            if contained(&selection.range, &span.range)? {
                host_spans.push(span.clone());
            }
        }
        let plan = Plan::prepare(selection.block, &[]).map_err(|_| Error::InvalidProtection)?;
        let mut detected = Vec::new();
        for span in plan.spans() {
            if contained(&selection.range, &span.range)? {
                detected.push(DetectedSpan {
                    range: span.range.clone(),
                    kind: span.kind,
                });
            }
        }
        blocks.push(PreparedBlock {
            id: selection.id,
            location: selection.location,
            range: selection.range.clone(),
            source,
            detected,
            host_spans,
        });
    }
    Ok(Prepared {
        document_id: request.document_id,
        revision: request.revision,
        format: request.format,
        model: request.model.clone(),
        blocks,
    })
}

#[derive(Debug)]
pub struct BilingualBlock<'a> {
    pub original: PreparedBlock<'a>,
    /// Accepted segment offsets are absolute within the original block.
    /// Rejected outcomes retain their original paragraph diagnostics.
    pub outcome: Result<ParagraphTranslation, paragraph::Error>,
}

#[derive(Debug)]
pub struct BilingualResult<'a> {
    pub protocol: u32,
    pub document_id: &'a str,
    pub revision: u64,
    pub format: Format,
    pub model: ModelIdentity<'a>,
    pub blocks: Vec<BilingualBlock<'a>>,
}

fn map_result<'a>(
    prepared: Prepared<'a>,
    translated: document::DocumentTranslation,
) -> Result<BilingualResult<'a>, Error> {
    if prepared.blocks.len() != translated.blocks.len() {
        return Err(Error::InvalidMapping);
    }
    let mut blocks = Vec::with_capacity(prepared.blocks.len());
    for (original, result) in prepared.blocks.into_iter().zip(translated.blocks) {
        if original.id != result.id {
            return Err(Error::InvalidMapping);
        }
        let mut outcome = result.outcome;
        if let Ok(ref mut paragraph) = outcome {
            for segment in &mut paragraph.segments {
                if original.source.get(segment.source.clone()).is_none() {
                    return Err(Error::InvalidMapping);
                }
                segment.source.start += original.range.start;
                segment.source.end += original.range.start;
            }
        }
        blocks.push(BilingualBlock { original, outcome });
    }
    Ok(BilingualResult {
        protocol: VERSION,
        document_id: prepared.document_id,
        revision: prepared.revision,
        format: prepared.format,
        model: prepared.model,
        blocks,
    })
}

const MAX_HOST_SPANS: usize = 64;
const MAX_SELECTION_SEGMENTS: usize = 64;
const MAX_SELECTION_OUTPUT: usize = 256 * 1024;

fn rejected(failure: paragraph::Failure) -> paragraph::Error {
    paragraph::Error::Rejected {
        index: 0,
        attempts: vec![paragraph::Attempt {
            strategy: paragraph::Strategy::ProtectedGaps,
            failure,
        }],
    }
}

fn host_error(code: &str) -> paragraph::Error {
    rejected(paragraph::Failure::Generation(code.into()))
}

fn append_segment(
    result: &mut ParagraphTranslation,
    segment: paragraph::Segment,
) -> Result<(), paragraph::Error> {
    if result.segments.len() >= MAX_SELECTION_SEGMENTS
        || segment.text.len() + 1 > MAX_SELECTION_OUTPUT.saturating_sub(result.text.len())
    {
        return Err(paragraph::Error::Limit);
    }
    if !result.text.is_empty() {
        result.text.push('\n');
    }
    result.text.push_str(&segment.text);
    result.segments.push(segment);
    Ok(())
}

/// Only the host's explicitly marked literal spans are copied. All nonblank
/// gaps go through the existing paragraph engine; a failed gap rejects the
/// entire selection. A final whole-selection audit catches join artifacts.
fn protected_with(
    block: &PreparedBlock<'_>,
    engine: &ContextEngine,
    explicit: Option<Domain>,
    mut run: impl FnMut(&str, Option<Domain>) -> Result<ParagraphTranslation, paragraph::Error>,
) -> Result<ParagraphTranslation, paragraph::Error> {
    if block.host_spans.is_empty() {
        return run(block.source, explicit);
    }
    if block.host_spans.len() > MAX_HOST_SPANS {
        return Err(paragraph::Error::Limit);
    }
    let domain = match explicit {
        Some(domain) => domain,
        None => engine
            .decide(block.source, None)
            .map_err(paragraph::Error::Plan)?
            .domain,
    };
    let plan = Plan::prepare_indexed(block.source, engine.glossary(domain))
        .map_err(paragraph::Error::Plan)?;
    let mut ranges = Vec::with_capacity(block.host_spans.len());
    for span in &block.host_spans {
        let range = span.range.start - block.range.start..span.range.end - block.range.start;
        if block.source[range.clone()].trim().is_empty() {
            return Err(host_error("empty_host_literal"));
        }
        for detected in plan.spans() {
            if detected.range.start < range.end
                && detected.range.end > range.start
                && (detected.range.start < range.start || detected.range.end > range.end)
            {
                return Err(host_error("host_splits_scientific_span"));
            }
        }
        ranges.push(range);
    }
    let mut result = ParagraphTranslation {
        text: String::new(),
        segments: Vec::new(),
    };
    let mut cursor = 0;
    let mut translated_gap = false;
    let mut expected = BTreeMap::new();
    for (index, gap_end) in ranges
        .iter()
        .map(|range| range.start)
        .chain(std::iter::once(block.source.len()))
        .enumerate()
    {
        let gap = &block.source[cursor..gap_end];
        if !gap.trim().is_empty() {
            let translated = run(gap, Some(domain))?;
            if !document::complete(gap, &translated) {
                return Err(host_error("invalid_gap_mapping"));
            }
            translated_gap = true;
            for mut segment in translated.segments {
                segment.source.start += cursor;
                segment.source.end += cursor;
                append_segment(&mut result, segment)?;
            }
        }
        if let Some(range) = ranges.get(index) {
            let literal = &block.source[range.clone()];
            *expected.entry(literal).or_insert(0_usize) += 1;
            append_segment(
                &mut result,
                paragraph::Segment {
                    source: range.clone(),
                    domain,
                    basis: None,
                    strategy: paragraph::Strategy::ProtectedGaps,
                    rejected: Vec::new(),
                    protected_spans: 1,
                    text: literal.into(),
                },
            )?;
            cursor = range.end;
        }
    }
    if !translated_gap {
        return Err(host_error("host_only_selection"));
    }
    for (literal, count) in expected {
        if result.text.match_indices(literal).count() != count {
            return Err(host_error("host_literal_count"));
        }
    }
    paragraph::audit(block.source, &plan, &result.text)
        .map_err(|issue| rejected(paragraph::Failure::Quality(issue)))?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Accepted,
    Partial,
    Rejected,
}

impl BilingualResult<'_> {
    pub fn status(&self) -> Status {
        let accepted = self.blocks.iter().filter(|b| b.outcome.is_ok()).count();
        if accepted == 0 {
            Status::Rejected
        } else if accepted == self.blocks.len() {
            Status::Accepted
        } else {
            Status::Partial
        }
    }
}

/// Synchronous, local, audited translation using the existing resident model.
/// Host spans are exact literals, never automatically translated terms.
pub fn translate<'a>(
    prepared: Prepared<'a>,
    translator: &OfflineTranslator,
    engine: &ContextEngine,
    explicit: Option<Domain>,
    options: GenerationOptions,
    kernel: Kernel,
) -> Result<BilingualResult<'a>, Error> {
    if prepared.model.precision != translator.precision() {
        return Err(Error::PrecisionMismatch);
    }
    let session = translator.session().map_err(|_| Error::ModelSession)?;
    let blocks: Vec<_> = prepared
        .blocks
        .iter()
        .map(|b| document::Block {
            id: b.id,
            source: b.source,
        })
        .collect();
    let mut originals = prepared.blocks.iter();
    let translated = document::translate_with(&blocks, |source| {
        let original = originals.next().ok_or(paragraph::Error::Limit)?;
        if original.source != source {
            return Err(host_error("invalid_selection_mapping"));
        }
        protected_with(original, engine, explicit, |gap, domain| {
            session.translate_paragraph(gap, engine, domain, options, kernel)
        })
    })
    .map_err(|e| match e {
        document::Error::InputLimit => Error::InputLimit,
        document::Error::InvalidOrder => Error::InvalidOrder,
        document::Error::InvalidCoverage(_) => Error::InvalidMapping,
        document::Error::OutputLimit => Error::OutputLimit,
    })?;
    map_result(prepared, translated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(selections: &'a [Selection<'a>], format: Format) -> Request<'a> {
        Request {
            protocol: VERSION,
            document_id: "paper-1",
            revision: 7,
            format,
            model: ModelIdentity {
                id: "opus-en-zh",
                version: Version(1, 0, 0),
                precision: Precision::Int8,
                language: LanguagePair::EnglishToSimplifiedChinese,
            },
            selections,
        }
    }

    fn pdf(block: &str, range: Range<usize>) -> Selection<'_> {
        Selection {
            id: 1,
            location: Location::Pdf { page: 0, block: 2 },
            block,
            range,
            host_spans: &[],
        }
    }

    #[test]
    fn pdf_utf8_and_detected_protection() {
        let text = "α 12 MPa";
        let selections = [pdf(text, 3..text.len())];
        let prepared = prepare(&request(&selections, Format::Pdf)).unwrap();
        let block = &prepared.blocks()[0];
        assert_eq!(block.source, "12 MPa");
        assert_eq!(block.range, 3..text.len());
        assert!(!block.detected.is_empty());
        assert_eq!(block.detected[0].range.start, 3);
    }

    #[test]
    fn invalid_ranges_and_partial_formula() {
        for range in [1..2, 2..2, 0..99, Range { start: 4, end: 3 }] {
            let selections = [pdf("α text", range)];
            assert_eq!(
                prepare(&request(&selections, Format::Pdf)).unwrap_err(),
                Error::InvalidRange
            );
        }
        let selections = [pdf("a $x+y$ b", 3..6)];
        assert_eq!(
            prepare(&request(&selections, Format::Pdf)).unwrap_err(),
            Error::SplitProtection
        );
    }

    #[test]
    fn rejects_protocol_identity_and_format() {
        let selections = [pdf("Text", 0..4)];
        let mut input = request(&selections, Format::Word);
        assert_eq!(prepare(&input).unwrap_err(), Error::InvalidLocation);
        input.protocol = 2;
        assert_eq!(prepare(&input).unwrap_err(), Error::Protocol);
        input.protocol = VERSION;
        input.document_id = "../paper";
        assert_eq!(prepare(&input).unwrap_err(), Error::Identity);
    }

    #[test]
    fn rejects_duplicate_ids_and_locations() {
        let mut selections = [pdf("Text", 0..4), pdf("More", 0..4)];
        assert_eq!(
            prepare(&request(&selections, Format::Pdf)).unwrap_err(),
            Error::InvalidOrder
        );
        selections[1].id = 2;
        assert_eq!(
            prepare(&request(&selections, Format::Pdf)).unwrap_err(),
            Error::InvalidLocation
        );
    }

    #[test]
    fn rejects_input_budget() {
        let large = "a".repeat(MAX_BLOCK_BYTES + 1);
        let selections = [pdf(&large, 0..1)];
        assert_eq!(
            prepare(&request(&selections, Format::Pdf)).unwrap_err(),
            Error::InputLimit
        );
        assert_eq!(
            prepare(&request(&[], Format::Pdf)).unwrap_err(),
            Error::InputLimit
        );
    }

    #[test]
    fn word_host_hints_and_partial_rejection() {
        let spans = [HostSpan {
            range: 2..5,
            kind: HostKind::Citation,
        }];
        let mut selection = pdf("a [x] b", 0..7);
        selection.location = Location::Word { paragraph: 3 };
        selection.host_spans = &spans;
        let mut selections = [selection];
        let prepared = prepare(&request(&selections, Format::Word)).unwrap();
        assert_eq!(prepared.blocks()[0].host_spans, spans);
        selections[0].range = 3..7;
        assert_eq!(
            prepare(&request(&selections, Format::Word)).unwrap_err(),
            Error::SplitProtection
        );
    }

    #[test]
    fn preserves_rejected_source_and_identity() {
        let selections = [pdf("Text", 0..4)];
        let prepared = prepare(&request(&selections, Format::Pdf)).unwrap();
        let result = map_result(
            prepared,
            document::DocumentTranslation {
                blocks: vec![document::BlockResult {
                    id: 1,
                    outcome: Err(paragraph::Error::Limit),
                }],
            },
        )
        .unwrap();
        assert_eq!(result.document_id, "paper-1");
        assert_eq!(result.revision, 7);
        assert_eq!(result.blocks[0].original.source, "Text");
        assert_eq!(result.blocks[0].outcome, Err(paragraph::Error::Limit));
    }

    #[test]
    fn maps_accepted_segment_and_rejects_wrong_id() {
        let selections = [pdf("a Text", 2..6)];
        let input = request(&selections, Format::Pdf);
        let translated = document::DocumentTranslation {
            blocks: vec![document::BlockResult {
                id: 1,
                outcome: Ok(ParagraphTranslation {
                    text: "文本".into(),
                    segments: vec![paragraph::Segment {
                        source: 0..4,
                        domain: Domain::General,
                        basis: None,
                        strategy: paragraph::Strategy::Whole,
                        rejected: vec![],
                        protected_spans: 0,
                        text: "文本".into(),
                    }],
                }),
            }],
        };
        let result = map_result(prepare(&input).unwrap(), translated).unwrap();
        assert_eq!(
            result.blocks[0].outcome.as_ref().unwrap().segments[0].source,
            2..6
        );
        let wrong = document::DocumentTranslation {
            blocks: vec![document::BlockResult {
                id: 9,
                outcome: Err(paragraph::Error::Limit),
            }],
        };
        assert_eq!(
            map_result(prepare(&input).unwrap(), wrong).unwrap_err(),
            Error::InvalidMapping
        );
    }

    fn synthetic_gap(source: &str, target: &str) -> ParagraphTranslation {
        ParagraphTranslation {
            text: target.into(),
            segments: vec![paragraph::Segment {
                source: 0..source.len(),
                domain: Domain::General,
                basis: None,
                strategy: paragraph::Strategy::Whole,
                rejected: vec![],
                protected_spans: 0,
                text: target.into(),
            }],
        }
    }

    fn host_block<'a>(source: &'a str, spans: Vec<HostSpan>) -> PreparedBlock<'a> {
        PreparedBlock {
            id: 1,
            location: Location::Pdf { page: 0, block: 0 },
            range: 5..5 + source.len(),
            source,
            detected: vec![],
            host_spans: spans,
        }
    }

    #[test]
    fn exact_host_formula_and_citation() {
        let source = "Results $x+y$ [Ref]";
        let block = host_block(
            source,
            vec![
                HostSpan {
                    range: 13..18,
                    kind: HostKind::Formula,
                },
                HostSpan {
                    range: 19..24,
                    kind: HostKind::Citation,
                },
            ],
        );
        let engine = ContextEngine::builtin().unwrap();
        let mut calls = 0;
        let result = protected_with(&block, &engine, Some(Domain::General), |gap, _| {
            calls += 1;
            Ok(synthetic_gap(gap, "结果"))
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.text, "结果\n$x+y$\n[Ref]");
        assert!(document::complete(source, &result));
        assert_eq!(result.segments[1].source, 8..13);
    }

    #[test]
    fn rejects_duplicate_host_literal_and_failed_gap() {
        let block = host_block(
            "Results [Ref]",
            vec![HostSpan {
                range: 13..18,
                kind: HostKind::Citation,
            }],
        );
        let engine = ContextEngine::builtin().unwrap();
        assert!(protected_with(&block, &engine, Some(Domain::General), |gap, _| {
            Ok(synthetic_gap(gap, "结果[Ref]"))
        })
        .is_err());
        assert_eq!(
            protected_with(&block, &engine, None, |_, _| Err(paragraph::Error::Limit)),
            Err(paragraph::Error::Limit)
        );
    }

    #[test]
    fn rejects_host_only_and_scientific_span_split() {
        let engine = ContextEngine::builtin().unwrap();
        let only = host_block(
            "[Ref]",
            vec![HostSpan {
                range: 5..10,
                kind: HostKind::Citation,
            }],
        );
        assert!(protected_with(&only, &engine, None, |_, _| panic!("no gap")).is_err());
        let split = host_block(
            "maximum strain",
            vec![HostSpan {
                range: 13..19,
                kind: HostKind::Term,
            }],
        );
        assert!(protected_with(&split, &engine, Some(Domain::Geotechnical), |_, _| {
            panic!("split terminology")
        })
        .is_err());
    }

    #[test]
    fn rejects_numeric_join_regression() {
        let block = host_block(
            "Results 12 MPa",
            vec![HostSpan {
                range: 13..19,
                kind: HostKind::Quantity,
            }],
        );
        let engine = ContextEngine::builtin().unwrap();
        assert!(protected_with(&block, &engine, Some(Domain::General), |gap, _| {
            Ok(synthetic_gap(gap, "结果 13"))
        })
        .is_err());
    }

    #[test]
    fn status_partial_preserves_original_order() {
        let mut selections = [pdf("Text", 0..4), pdf("More", 0..4)];
        selections[1].id = 2;
        selections[1].location = Location::Pdf { page: 1, block: 0 };
        let input = request(&selections, Format::Pdf);
        let result = map_result(
            prepare(&input).unwrap(),
            document::DocumentTranslation {
                blocks: vec![
                    document::BlockResult {
                        id: 1,
                        outcome: Ok(synthetic_gap("Text", "文本")),
                    },
                    document::BlockResult {
                        id: 2,
                        outcome: Err(paragraph::Error::Limit),
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!(result.status(), Status::Partial);
        assert_eq!(result.blocks[1].original.source, "More");
    }
}
