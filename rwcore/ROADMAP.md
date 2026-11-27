# C 线路研发路线

Status below is baseline at CI #96, not full Word compatibility.

| Phase | Scope | Status |
|---|---|---|
| C1 | Rust/OPC/ZIP guard, package graph, locked CI | Basic batches verified |
| C2.0 | Paragraph/run attributes, style inheritance | Basic batches verified |
| C2.1 | Numbering definitions, counters, body traversal | Basic batches verified |
| C2.2 | Sections, page geometry, header/footer, page fields | Basic batches verified |
| C2.3 | Tables, merges, properties, borders, nested parse/layout | Basic batches verified |
| C2.4a | Offline media resource catalog | Verified |
| C2.4b | Six raster format header sizes | Verified |
| C2.4c-1 | Inline DrawingML metadata | Verified |
| C2.4c-2 | Ordered inline pictures in document runs | Verified |
| C2.4d-1 | Floating anchor metadata | Verified |
| C2.4d-2 | Position and text wrap algorithms | Verified |
| C2.4e | Picture crop, rotation, geometry | Verified |
| C2.4f | Media/document integration acceptance | Verified |
| C3.1 | Atomic run edits, positions and anchors | Verified |
| C3.2 | Selection ranges and cross-run edits | Verified |
| C3.3a | Bounded undo/redo transaction history | Verified |
| C3.3b | Structural transactions and validation | Verified |
| C3.3c | Structural history and path-safe editor integration | Verified |
| C4.1 | Font container and SFNT directory parsing | Verified |
| C4.2a | Native font metrics and unit conversions | Verified |
| C4.2b | Unicode cmap glyph mapping | Verified |
| C4.2c | Nominal glyph metrics and text measurement integration | Verified |
| C4.3a | Native TrueType loca/glyf simple outlines | Verified |
| C4.3b | Bounded composite glyph assembly | Verified |
| C4.3c | Unified outline paths and curve bounds | Verified |
| C4.3d | Adaptive quadratic flattening and contour geometry | Verified |
| C4.3e | Winding fill and bounded grayscale glyph rasterization | Verified |
| C4.3f | Integrated TrueType glyph bitmap and automatic device geometry | Verified |
| C4.3g | Nominal text-run bitmap placement and cumulative advances | Verified |
| C4.3h-1 | Bounded absolute-coordinate grayscale comparison | Verified |
| C4.3h-2a | Licensed real-font fixture and nominal metric reference checks | Verified |
| C4.3h-2b | Test-only independent grayscale sampling on shared paths | Verified |
| C4.3h-2c | Independent simple geometry and italic fixture | Verified |
| C4.3h-2d1 | Independent translated composite geometry | Verified |
| C4.3h-2d2 | Additional font family validation | Verified |
| C4.3h-2e1 | Real scaled/reflected composite geometry | Verified |
| C4.3h-2e2a | Exact synthetic general matrix and attachment boundary tests | Verified |
| C4.3h-2e2b | Real point attachment/general matrix reference (outstanding) | Planned |
| C4.4a1 | Shared OpenType Coverage format 1/2 decoder | Verified |
| C4.4a2a | GSUB header and top-level directory | Verified |
| C4.4a2b | GSUB child structures and cross-index validation | Verified |
| C4.4a3a | Standalone single substitution format 1/2 | Verified |
| C4.4a3b | Real-font direct type 1 reference and integration | Verified |
| C4.4a4a | Standalone multiple substitution and bounded expansion | Verified |
| C4.4a4b | Real-font direct type 2 reference | Verified |
| C4.4a5a | Standalone alternate substitution and explicit selection | Verified |
| C4.4a5b | Real-font direct type 3 reference | Verified |
| C4.4a6a | Standalone ligature substitution and contraction | Verified |
| C4.4a6b | Real-font direct type 4 reference | Verified |
| C4.4a7a | Contextual substitution format 1 parsing and matching | Verified |
| C4.4a7b | ClassDef and contextual format 2 | Verified |
| C4.4a7c | Contextual format 3 coverage matching | Verified |
| C4.4a7d | Real-font direct contextual references | Verified |
| C4.4a7e1 | Glyph-based chained contextual format 1 | Verified |
| C4.4a7e2 | Class-based chained contextual format 2 | Verified |
| C4.4a7e3 | Coverage-based chained contextual format 3 | Verified |
| C4.4a7e4 | Real-font direct Type 6 reference acceptance | Verified |
| C4.4a8a | GSUB extension wrapper and target dispatch | Verified |
| C4.4a8b1 | Real direct targets in synthetic extension wrappers | Verified |
| C4.4a8b2a | Native Type 7 wrapper inventory and representative dispatch | Verified |
| C4.4a8b2b | Full native Type 7 target decoding and reference checks | Verified |
| C4.4a9a | Reverse chained single substitution Type 8 | Verified |
| C4.4a9b | Real native Type 8 font reference | Outstanding: fixture needed |
| C4.4b1 | GDEF header/directory and glyph class tables | Verified |
| C4.4b2 | GDEF MarkGlyphSetsDef coverage sets | Verified (#110 after repair) |
| C4.4b3 | Real GDEF class and mark-set references | Verified |
| C4.4b4 | LookupFlag glyph eligibility and mark filtering | Verified |
| C4.4b5 | Real GSUB LookupFlag and GDEF references | Verified |
| C4.4c1 | GSUB script/language/feature selection and ordered lookup plan | Verified |
| C4.4c2 | Real Script/LangSys/feature base plan reference | Verified |
| C4.4c3a | Bounded GSUB lookup target compilation | Verified |
| C4.4c3b | Real-font lookup compilation reference | Verified |
| C4.4c4a | Bounded nested GSUB lookup dependency discovery | Verified |
| C4.4c4b | Real raw contextual dependency reference | Verified |
| C4.4c4c | Bounded static dependency closure and cycle diagnostics | Verified |
| C4.4c4d | Real-font dependency closure references | Verified |
| C4.4c5a | Bounded dependency compilation integration | Verified (#122) |
| C4.4c5b | Real-font dependency program reference | Verified (#123) |
| C4.4d1a | Bounded actual single substitution execution with eligibility and root scheduling | Verified (#124) |
| C4.4d1b | Independent real-font Type 1 execution reference | Verified (#125) |
| C4.4d2a | Bounded Type 2 multiple substitution and mixed Type 1/2 execution | Verified (#126) |
| C4.4d2b | Independent real-font Type 2 and mixed Type 1/2 execution reference | Verified (#127) |
| C4.4d3a | Bounded Type 3 alternate selection and mixed root execution | Verified (#128) |
| C4.4d3b | Independent real-font Type 3 and mixed Type 1/2/3 execution reference | Verified (#129) |
| C4.4d4a | Bounded Type 4 ligature execution with filtered positions and mixed roots | Verified (#130) |
| C4.4d4b | Independent real-font Type 4 and mixed Type 1/2/3/4 execution reference | Verified (#131) |
| C4.4d5a | Bounded contextual Type 5 nested execution foundation | Verified (#132) |
| C4.4d5b | Contextual Type 3 nested selection and reference checks | Verified (#133) |
| C4.4d5c | Independent native Type 5 length-preserving closure execution reference | Verified (#134) |
| C4.4d5d | Bounded nested Type 2 expansion and stable action-position tracking | Verified (#135) |
| C4.4d5e | Independent native contextual expansion and action-position reference | Verified (#136) |
| C4.4d5f | Bounded nested Type 4 contraction and consumed-anchor tracking | Verified (#137) |
| C4.4d5g | Independent native contextual contraction and mixed growth/shrink reference | Verified (#138) |
| C4.4d6a | Bounded Type 6 chained contextual execution foundation | Verified (#139) |
| C4.4d6b | Bounded Type 6 variable-length nested integration | Verified (#140) |
| C4.4d6c | Independent native Type 6 mixed execution reference | Verified (#141) |
| C4.4d8a | Bounded Type 8 reverse chained substitution execution | Verified (#142) |
| C4.4d8b | Independent reverse execution and native mixed program reference (synthetic positive Type 8) | Verified (#143) |
| C4.4d8c | Licensed native Type 8 positive fixture and wrapper acceptance | Verified (#144) |
| C4.4d9 | Bounded full GSUB execution acceptance and API consolidation | Verified (#146) |
| C4.4e1 | FeatureVariations condition-set and feature substitution foundation | Verified (#147) |
| C4.4e2 | Variation-aware feature plan and validated program compilation | Verified (#148) |
| C4.4e3 | Native positive FeatureVariations reference and dynamic plan acceptance | Verified (#149) |
| C4.4e4 | Bounded fvar axis parsing and normalized coordinate calculation | Verified (#150) |
| C4.4e5 | Bounded avar 1.0 segment maps and non-linear coordinate remapping | Verified (#151) |
| C4.4e6 | Native user-coordinate normalization and dynamic GSUB execution integration | Verified (#152) |
| C4.4e7 | Bounded variable GSUB scheduling acceptance and API contract | Verified (#153) |
| C4.4f1 | Unicode cluster and glyph provenance foundation | Verified (#154) |
| C4.4f2 | Bounded source-set union and expansion/contraction ledger | Verified (#155) |
| C4.4f3 | GSUB single/multiple substitution provenance integration | Verified (#156) |
| C4.4f4 | GSUB alternate and ligature provenance integration | Verified (#158) |
| C4.4f5 | Nested contextual and reverse GSUB provenance integration | Verified (#159) |
| C4.4f6 | Comprehensive native GSUB provenance acceptance and API contract | Verified (#160) |
| C4.4g1 | Explicit per-glyph feature-mask foundation | Verified (#161) |
| C4.4g2 | Validated root lookup gating and live masks | Verified (#162) |
| C4.4g3 | Component and contextual mask policy | Verified (#164) |
| C4.4g4 | Strict mask native and independent reference acceptance | Verified (#165) |
| C4.4h1 | Explicit bounded script-stage scheduling foundation | Verified (#166) |
| C4.4h2 | Staged execution on persistent provenance and masks | Verified (#167) |
| C4.4h3 | Native staged acceptance and explicit script feature ordering policy | Verified (#168) |
| C4.4h4 | Explicit script policy profiles and positive native stage acceptance | Verified (#169) |
| C4.4h5 | Profile integration and positive multi-stage evidence | Verified (#170) |
| C4.4h6 | Native two-positive-stage evidence and contract gate | Verified (#171) |
| C4.5a | Bounded GPOS header and directory foundation | Verified (#172) |
| C4.5b1 | GPOS ValueRecord and Device/VariationIndex foundation | Verified (#173) |
| C4.5b2 | GPOS Anchor and shared adjustment primitives | Verified (#174) |
| C4.5c1 | SinglePos formats 1/2 parsing and application | Verified (#175) |
| C4.5c2 | Independent native SinglePos reference acceptance | Verified (#176) |
| C4.5d1 | PairPos format 1 glyph-pair decoding and adjustment | Verified (#177) |
| C4.5d2 | PairPos format 2 class matrix | Verified (#178) |
| C4.5d3 | Independent native PairPos reference acceptance | Verified (#179) |
| C4.5e1 | CursivePos entry/exit anchor parsing and nominal connection math | Verified (#180) |
| C4.5e2 | Independent native CursivePos reference acceptance | Verified (#181) |
| C4.5f1 | MarkToBase parsing and nominal attachment geometry | Verified (#182) |
| C4.5f2 | Independent native MarkToBase reference acceptance | Implemented; CI pending |
| C4.5g1 | MarkToLigature format 1 component attachment foundation | Planned |
| C4 | Fonts and glyph shaping/layout | Planned |
| C5 | Automatic pagination | Planned |
| C6 | Advanced graphics and table layout | Planned |
| C7 | Mathematical formula engine | Planned |
| C8 | Research references and cross references | Planned |
| C9 | DOCX writing and round-trip fidelity | Planned |
| C10 | Legal real-paper corpus, compatibility, security/performance | Planned |
| C11 | Electron integration and desktop acceptance | Planned |

Gate every code batch: actual GitHub-hosted Windows/Linux/macOS tests and locked Release builds at same revision, retained regression tests, documented limitations. Stage C10/C11 require actual corpus/reference/device evidence; unit tests do not replace those gates.


CI #183: a42cb2f64f1d097fa7e5562f8105582e6cdc108c, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45914632546 : 1214/platform, 3642 total, zero failed/ignored; exact SHA, native_mark_base_reference and Release verified in three job logs.

C4.5g1: standalone MarkToLigature format 1 with explicit zero-based component selection, cumulative component/cell/device budgets, nullable anchors, shared MarkArray and anchor/coverage decoding, checked design and separate pixel corrections. Five synthetic regressions; 1219/platform expected, CI pending. No automatic component provenance selection, full attachment chain, real-font reference, contour or variation resolution. Next C4.5g2 independent native reference.


CI #184: 56cb2c7aaa389ea6cf5d06b00f7d44b131d98330, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45956312488 : 1219/platform, 3657 total, zero failed/ignored; exact SHA and Release verified in three job logs.

C4.5g2: independent test-only raw-byte native MarkToLigature oracle across nine retained fonts, Type 5 and extension traversal, coverage, component/class matrix, all covered component/mark pairs, world-coordinate equality, separate per-bit Device corrections and explicit unresolved errors. Positive gates for native subtables, marks, multi-component ligatures, anchors, pairs, unequal coordinates and resolved pixel checks. One new test; 1220/platform expected, CI pending. No external shaper parity, automatic component selection, outline/variation resolution or attachment-chain integration. Next C4.5h1 MarkToMark foundation.


CI #185 failed: 00da1e0dc2e43b8426378cfcf3136b000b6af156, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45958213747 : each platform 919 passed, 1 failed, 0 ignored, 300 not run, Release skipped; exact SHA and native evidence [1,178,18,62,3436,3436,0] verified. Last green #184: 1219/platform.

C4.5g2 repair: retain nine full-font reference checks and original aggregate positive gates; add exact 370-byte DejaVu Sans 2.37 native Type 5 subtable excerpt (source and excerpt SHA-256, license retained), plus independent synthetic differential nonzero Device check. Native excerpt has coordinate anchors, not real nonzero Device deltas; native contour/variation resolution and real nonzero Device fixture remain outstanding. 1221/platform expected, CI pending. Next C4.5h1 only after green.


CI #186: 5bc81053a421b1142ee2643f309b6b7237fa7052, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45960201240 : 1221/platform, 3663 total, zero failed/ignored; exact SHA, native_mark_ligature_reference, independent_nonzero_device_attachment and Release verified in all three job logs. Real nonzero Device fixture and contour/variation resolution remain outstanding.

C4.5h1: GPOS Type 6 MarkToMark format 1 explicit fixed Mark2/moving Mark1 roles. Shared identical Type 4/6 binary matrix decoder, checked design/pixel translation, overlapping coverages permitted, cumulative Device/record/cell bounds and nullable anchors. Six regressions; 1227/platform expected, CI pending. Caller establishes lookup type and occurrence selection; no automatic attachment chain, real native Type 6 reference or contour/variation resolution. Next C4.5h2 native independent reference.


CI #187: b23be41a5a1dbfb009e91d6b76a63990c3bb98c9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45961764512 : 1227/platform, 3681 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5h2: independent raw-byte native Type 6 reference across nine retained fonts, including extension traversal, coverage, all Mark1/Mark2 class cells and covered pairs, independent world-coordinate equality and per-bit Device pixel checks or explicit unresolved errors. Native positive gates for subtables, marks, anchors, pairs and distinct coordinates; resolved native pixel count reported separately (may be zero due to fixture limitations). Separate synthetic nonzero Device differential requires exact positive/negative deltas. Two new regressions, 1229/platform expected, CI pending. Real nonzero Device and contour/variation resolution remain outstanding. Next C4.5i1 GPOS Contextual Positioning format 1, after acceptance.


CI #188: 5b61e89bf145a82eea3fd9eade739f4bc91707e7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45963737757 : 1229/platform, 3687 total, zero failed/ignored; exact SHA, native_mark_mark_reference, mark_mark_independent_nonzero_device and Release verified in all three logs. Native resolved pixel count not separately available in passing captured logs; real nonzero Device fixture remains outstanding.

C4.5i1: GPOS Type 7 format 1 glyph-based context positioning parser and immutable prefix/explicit-position matcher. Reuses identical GSUB sequence-rule grammar with owning GPOS lookup count, ordered action records, null sets, first-match priority, empty-action matches and cumulative budgets. Shared context format 1/2 allocations now fallible. Twelve new regressions; 1241/platform expected, CI pending. No positioning execution, GDEF filtering, recursion/cycle evaluation or native Type 7 reference. Next C4.5i2 class context positioning format 2.


CI #189: 24b3845d46032262c7a3f3f29ac56df4259f9a16, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45965933610 : 1241/platform, 3723 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i2: GPOS Type 7 format 2 class context positioning, shared ClassDef and bounded sequence-rule decoder, initial coverage gate, class zero, immutable prefix and explicit-position matching, ordered lookup references validated in owning GPOS domain. Seven regressions; 1248/platform expected, CI pending. No nested execution, filtering, cycle evaluation or native Type 7 reference. Next C4.5i3 coverage context positioning format 3.


CI #190: bc2aae75747e609f0a9193b2cf50b2a7e5700195, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45967906824 : 1248/platform, 3744 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i3: GPOS Type 7 format 3 coverage context positioning, independent coverage per input position, immutable prefix/explicit-position matching, ordered lookup actions, empty-action and empty-coverage semantics, cumulative coverage records/expanded glyph budgets charged per reference. Shared format 3 allocation handling now fallible. Eleven regressions; 1259/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 7 reference. Next C4.5i4 independent real-font reference, with explicit fixture coverage limitations.


CI #191: 8e44732c2198f3fcd832be398ea768b916996a69, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45970076717 : 1259/platform, 3777 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i4: test-only independent raw-byte Type 7 context oracle for formats 1/2/3 across nine retained fonts, including Type 9 wrappers, full rule/action comparison, representative class/glyph matches and prefix checks, coverage domain checks. Native positive gate requires actual subtable, match and action evidence; per-format native coverage may be absent and is not claimed. Separate synthetic positive checks all three formats. Two tests; 1261/platform expected, CI pending. Missing native formats, nested execution, filtering and cycles remain outstanding. Next C4.5j1 chained context positioning format 1 after acceptance.


CI #192 failed: 8bf52a5260ac0bf79da18c74dcdb2cf644ea43c5, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45972476355 : 954 passed, 1 failed, 0 ignored per platform; 306 not run. All native Type 7 counters zero; independent synthetic oracle passed. Exact SHA verified; last green #191 1259/platform.

C4.5i4 repair: retain nine-font traversal and independent synthetic tests, add byte-exact native Gurmukhi Format 1 and Gujarati Format 2 excerpts from installed Debian Noto fonts, with provenance/hash/license and separate positive gates for both. No native Format 3 fixture yet; no nested execution or filtering claim. 1261/platform expected, CI pending. Next C4.5j1 after green.


CI #193 failure: c653a6268c3573168fa3c3121278ef828f3db21c, run 45974803781: 954 passed, 1 failed, 306 not run per platform; exact SHA verified. Native Gujarati Type 7 format 2 rejected with Invalid(class range or value).

C4.5i4 repair 2: verified local Noto Gujarati raw source, glyph count 798, class set count 2, trailing class 2 assigned in ClassDef. GPOS compatibility path permits trailing-only classes without initial rule sets, validates initial coverage indices and undefined trailing references, keeps GSUB strict behavior. Independent oracle representative domain now follows actual class IDs. Added one regression; 1262/platform expected, CI pending. Native Format 3 remains outstanding. Next C4.5j1 only after green.
