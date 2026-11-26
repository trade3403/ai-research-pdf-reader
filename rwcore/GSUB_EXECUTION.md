# GSUB bounded execution contract — C4.4d9

Status: bounded GSUB execution accepted at CI #146 (1036/platform, 3108 total; revision 5c06c9b0638380a8d093d1adc6f9c20155ce3951). This is not complete OpenType shaping.

## Stable API
- parse_gsub_layout / build_gsub_base_plan: base script/language selection, required features, explicit optional tags, sorted unique roots and feature provenance.
- compile_gsub_plan_program / compile_gsub_program: validate provenance or explicit roots, compile reachable targets with static closure and cycle diagnostics.
- execute_gsub_program: unified effective types 1–6 and 8, including Type 7 wrappers. GsubProgramExecLimits and GsubProgramExecResult are source-compatible aliases, not new budget schemes. Legacy subset entry points retain their original rejection behavior.
- Explicit GsubAlternateSelection choices for every reachable Type 3 lookup, sorted by original index. Candidate index is zero-based and checked on eligible covered input. No implicit choice or feature-value inference.
- Output is a glyph-ID buffer and counters. Inputs remain immutable; errors return no partial output.

## Scheduling and edit policy
Roots in ascending LookupList order, one pass each; effective Type 8 scans descending physical indices, other roots advance forward. First matching subtable/rule wins. Nested actions run only at captured input anchors in stored order. Backtrack nearest-first, lookahead forward, current lookup GDEF filtering across all segments. Expansion keeps first output anchor; contraction retains first component, preserves ignored gaps and tombstones consumed anchors. Descendants can inspect/edit beyond ancestor input; exclusive input boundary is rebased. This policy is not a guarantee of external shaper equivalence.

## Budget contract
Input/program sizes, peak glyph length, depth (hard ceiling 32), calls, probes, filter checks, rules, symbols, actions and matches; inserted/removed glyphs and shared edit work. Bounds on logical glyph counts/work, not measured allocator RSS or wall-clock guarantees. Static cycles permitted when not recurrent at runtime; active same lookup/position cycle fails closed.

## Example
```rust
use rwcore::*;

fn run_base_pass(
    gsub: &[u8], glyph_count: u32, gdef: Option<&Gdef>,
    script: [u8;4], language: Option<[u8;4]>, enabled: &[[u8;4]],
    glyphs: &[u16], choices: &[GsubAlternateSelection],
) -> Result<Option<GsubProgramExecResult>, Error> {
    let layout = parse_gsub_layout(gsub, Default::default())?;
    let Some(plan) = build_gsub_base_plan(&layout, script, language,
        enabled, Default::default())? else { return Ok(None); };
    let program = compile_gsub_plan_program(gsub, &plan, glyph_count,
        gdef, Default::default())?;
    execute_gsub_program(&program, glyphs, choices,
        GsubProgramExecLimits::default()).map(Some)
}
```

## Acceptance boundary
CI #144 verified 1028 tests/platform on the exact preceding revision, including positive native Mongolian Type 8 and 127 synthetic wrapper masks. C4.4d9 adds eight native base-plan-to-execution regressions using bounded selected default-language schedules and independent raw root/output/counter references; not exhaustive rules or complete document layout. CI pending.
Remaining: FeatureVariations condition/substitution evaluation, script-specific shaping stages, feature values/masks, Unicode normalization and cluster/ligature component propagation, GPOS, real point attachments, external shaper parity, performance/security and DOCX corpus.

## CI #145 repair
The generic fixture selection cap of 32 roots excluded the native Mongolian reverse-containing feature. The pinned font's default mong LangSys rclt has 74 distinct roots, including native Type 8 lookup 93. The repaired regression checks the complete real feature plan, with unchanged engine budgets and independent reference comparisons. Other seven fixtures retain their original selection cap. Final acceptance still pending CI.

## Acceptance result
CI #146 passed all three platforms, including the complete 74-root native mong/rclt regression. The bounded GSUB execution contract is accepted. C4.4e1 starts a separate FeatureVariations parser/selector; no change to the accepted execution semantics or base planner yet.


## Variable GSUB API contract (C4.4e7, CI pending)
CI #152, 3baf3be5ff3f891f5dab192d3a873dc9ba321d3b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45802058303 : 1091/platform, 3273 total, zero failed/ignored; exact HEAD and three Release builds verified from actual job logs.

C4.4e7: five additional integration regressions for raw named-instance coordinates across Archivo, Devanagari and Nastaliq, ordered/tagged/default coordinate equivalence, repeat execution and plan provenance. Existing independent raw design-space and glyph ledger reference tests retained unchanged. 1096/platform expected; CI pending. No production substitution changes or external shaper equivalence claim.

Use parse_variable_gsub_face on a parsed FontFace. Supply signed 16.16 user coordinates in fvar order or tagged coordinates (missing tags default); named instances expose ordered coordinates. normalize returns F2DOT14 after avar 1.0. build_plan selects explicit features; compile_plan rebuilds the request and rejects inconsistent provenance; execute returns an atomic Option result. None means no matching script/language. Alternate choices remain explicit. Equivalent plans are accepted, not cryptographically bound. No implicit rvrn, cluster tracking, script staging, positioning or variation outlines.


CI #153, 1cfd3f4d22737f26d82e04243739c1e9137b7f9e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45808724109 : 1096/platform, 3288 total, zero failed/ignored, exact SHA and Release builds verified from three job logs. C4.4e7 bounded variable GSUB scheduling accepted.
