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


CI #194: a9ba8c115d6e710eb8388fe6cce26926464c96e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45977073609 : 1262/platform, 3786 total, zero failed/ignored; exact SHA and Release verified in all three job logs. Native Type 7 format 3 fixture remains outstanding.

C4.5j1: GPOS Type 8 format 1 glyph chained context positioning, shared bounded sequence grammar, reverse-stream backtrack, forward input/lookahead, first-match priority, empty-action semantics, input-only sequence indices and owning GPOS lookup bounds. Shared format 1/2 allocation handling now fallible. Twelve new regressions; 1274/platform expected, CI pending. No nested positioning execution, GDEF filtering, cycle evaluation or native Type 8 reference. Next C4.5j2 class chained context positioning format 2.


CI #195: cbab07ea5e34d562ddcf91eaf354b5e172636d3f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45981814345 : 1274/platform, 3822 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5j2: GPOS Type 8 format 2 class chained context positioning with independent backtrack/input/lookahead ClassDefs, initial coverage gate, implicit class zero, ordered input-only lookup references and bounded matching. GPOS compatibility permits assigned trailing-only input classes without initial rule sets; rejects out-of-range covered initial classes and undefined trailing-only references. GSUB strict path unchanged. Twelve regressions; 1286/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 8 reference. Next C4.5j3 coverage chained context positioning format 3.


CI #196: 478271b3394c9c30ca44f90391db8c5a0208ffb7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45984213080 : 1286/platform, 3858 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5j3: GPOS Type 8 format 3 coverage chained context positioning, independent per-position Coverage in all three segments, nearest-first backtrack, forward input/lookahead, input-relative ordered action records, owning GPOS lookup domain, empty context/coverage semantics, cumulative record and expanded glyph budgets across segments charged per reference. Shared format 3 allocations now fallible. Twelve new regressions; 1298/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 8 reference. Next C4.5j4 independent real-font reference with per-format evidence.


CI #197: 86d37f0d408e3f37bb9cbe82b67f4dfb83cbe1f2, run 45988945486: 1298/platform, 3894 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5j4: independent raw-byte Type 8 reference test across nine retained fonts, Type 9 targets, rule/action equality, representative class/glyph matching with truncations and exhaustive one-position coverage domain checks. Positive native gate; per-format native evidence not yet known. Separate synthetic positive reference all three formats. Two tests, 1300/platform expected. Next C4.5k extension positioning after CI. No local compile.


CI #198: 40556894fffc8385931b8158f4c0468d92954163, run 45991303281: 1300/platform, 3900 total, zero failed/ignored; SHA and Release verified. Native chained context positive gate passed; per-format counts not exposed.

C4.5k: GPOS Type 9 extension checked 32-bit offsets, borrowed target suffix, homogeneous lookup resolution, all direct types 1..8 dispatch with per-child limits, no recursive wrappers. Six new tests; 1306/platform expected. No local compile; native extension differential and exhaustive dispatch fixtures still outstanding. Next unified GPOS lookup compilation/scheduling after CI.


CI #199: f80c3a531e62f31ff68dc8cdffe6eafa22bc98c0, run 45993639612: 1306/platform, 3918 total, zero failed/ignored; exact SHA and Release verified.

C4.5l1: explicit selected GPOS LookupList compilation, original indices and source/target offsets, homogeneous Type 9 targets, shared GDEF filter construction, cumulative subtable and conservative suffix-byte budgets, fallible vector allocation. Eight new regressions; 1314/platform expected, CI pending. No execution, dependency closure or feature plan yet. Next C4.5l2 GPOS feature selection and plan provenance, then dependency closure and scheduling. Native extension differential remains outstanding.


CI #200 failed: b5126b6116edd58705d25d53f4cfc873c8e1f239, run 45995876561: 635 passed, 2 failed, 0 ignored, 677 not run per platform; exact SHA verified. direct_and_extension_order and cumulative_suffix_and_subtable_budgets both rejected GPOS type 9 with Invalid(lookup kind).

C4.5l1 repair: introduce LookupFilter::from_gpos_lookup with separate 1..=9 kind domain, sharing existing flag/GDEF validation; compile_gpos uses it. GSUB 1..=8 validation unchanged. Preserve all failing tests; add domain/reserved flag/missing mark set regression. Expected 1315/platform, 3945 total; CI pending, no local compile. Next C4.5l2 only after green.


CI #201: 6e17f0c0fde4713b1c85de451168ca5f5278f7d5, run 45998230655: 1315/platform, 3945 total, zero failed/ignored; exact SHA and Release verified.

C4.5l2: distinct GposPlan, shared base feature selection, required features, language/script fallback, ordered unique lookup provenance, compilation revalidation against original GPOS bytes with bounded references. Fourteen new tests (13 plan + 1 real compilation), 1329/platform expected. Base-only variation diagnostic; variable feature selection, dependency closure, execution and native extension differential remain outstanding. Next C4.5l3 dependency discovery/closure after green. No local compile.


CI #202: d346d71fbbdbee13c44a12dbfb85f87b22c3d899, run 46000598254: 1329/platform, 3987 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5l3: GPOS dependency enumeration for all six Type 7/8 contextual formats including Type 9 wrappers; ordered action provenance, duplicate and unreachable rule retention, bounded scans, iterative graph closure shared with GSUB, missing sources and deterministic cycle witnesses. 24 new regressions (11 discovery + 13 closure), 1353/platform expected. Static analysis only; no nested execution. Next C4.5l4 reachable program compilation and root scheduling after green. No local compile.


CI #203: cc7916c87a85ae767ab568dd1f7d0235776e90d6, run 46001862519: 1353/platform, 4059 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5l4: compile reachable GPOS programs from explicit roots or validated base plan, compile each dependency once, original lookup indices and ordered actions retained, root-only top-level schedule, cumulative lookup/subtable/byte/action/scan budgets, diagnostic cycle witnesses and complete closure verification. Eight new tests, 1361/platform expected. No runtime execution yet; next C4.5m1 bounded single/pair positioning execution and original-index filtering. Native extension differential and variable feature selection remain open. No local compile.


CI #204: d49e69210a051486894eea9ae678de6118c20acf, run 46002959737: 1361/platform, 4083 total, zero failed/ignored; exact SHA and Release verified.

C4.5m1: bounded GPOS Single/Pair/ClassPair root execution including extension targets, first matching subtable, pair ValueRecord2 consumption, GDEF filtered original indices, separate design/device pixel adjustments, atomic owned result, work budgets. Nine new regressions; 1370/platform expected. Unsupported contextual/attachment roots rejected even for empty runs. No local compile. Next C4.5m2 nested contextual execution, depth/work limits and cycle protection; attachment and RTL cursive execution later.


CI #205: 239261babef1776c8ac1f72f66de3d273ec56dcd, run 46005447958: 1370/platform, 4110 total, zero failed/ignored; exact SHA and Release verified.

C4.5m2: bounded GPOS context/chained context execution for all six formats, nested single/pair and extension dispatch, parent-filtered original-index mapping with child eligibility, ordered actions, first match and input consumption, active lookup/index cycle guard, hard depth cap 64 and cumulative call/action/context/filter/subtable budgets. Eight new regressions; 1378/platform expected. Existing m1 API retained. No local compile. Attachment kinds rejected, no runtime attachment graph or variation evaluation; next C4.5m3 cursive/mark attachment execution design after green.


CI #206: 330c79d40b6a7c1172bd256df4647345ea0f800c, run 46007916470: 1378/platform, 4134 total, zero failed/ignored; exact SHA and Release verified.

C4.5m3a: explicit attachment forest foundation, anchor delta extraction, iterative parent-chain propagation of final pen origins and local residual placement, independent design/pixel spaces, cycle/depth/edge limits, atomic overflow rejection. Nine new tests; 1387/platform expected. Caller supplies edges, final pen origins and residuals; automatic MarkToBase/MarkToMark/MarkToLigature parent/component selection and Cursive RTL advance redistribution not yet integrated. Next C4.5m3b attachment selection/execution integration, then cursive and ligature provenance. No local compile.


CI #207: 937553db9680af31158e74cb13f825bd5d16248c, run 46010328089: 1387/platform, 4161 total, zero failed/ignored; SHA and Release verified. C4.5m3b: independent mark execution result with residuals and attachment edges, Type 4/6 roots and nested dispatch, GDEF eligible parent search, mark barriers, replacement semantics, bounded parent visits/updates, final origin resolver. Five new tests; 1392/platform expected. No local compile. Mark ligature provenance, exact ligature-id compatibility and cursive/RTL advance redistribution remain open. Next C4.5m3c after green.


CI #208: c1416bee96bb6c2c4b63c961692b6004d6a72a2d run 46012819780, 1392/platform 4176 total zero failed/ignored, exact SHA and Release verified. C4.5m3c: Type5 execution using explicit occurrence-specific ligature ID/component source metadata; validates unique IDs, source references and component counts; last-component fallback for unmatched identities; null anchors not matched; conservative MarkToMark same-source compatibility in provenance API. Five new tests, 1397/platform expected. No local compile. GSUB automatic provenance generation and full MarkToMark compatibility (unmapped/ligature edge cases), RTL/cursive advance redistribution remain open. Next C4.5m3d cursive execution design after green.


CI #209: 87d0901cc8c865c9cb77c902ec724548951544b7 run 46015378965, 1397/platform 4191 total zero failed/ignored, exact SHA and Release verified. C4.5m3d1: standalone horizontal cursive root execution, adjacent eligible glyph pairs and extension wrappers, signed nominal advance interval solver, RTL flag fixed-end attachment, preserved skipped advances, final origin/attachment resolution, resource bounds. Seven new tests, 1404/platform expected. No local compile. This is a bounded horizontal constraint solver, not complete OpenType RTL advance redistribution or mixed nested mark/cursive execution. Next C4.5m3d2 mixed execution and direction-aware redistribution/differential validation.


CI #210: 43732dd7f69449390ab224c13cdd29346ea603b6, run https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46017959056, exact SHA and raw job logs verified: 1404 passed/platform, 4212 total, zero failed/ignored. Release Windows 26.65s, Ubuntu 12.33s, macOS 19.25s.

C4.5m3d2a: shared bounded horizontal mixed GPOS scheduler, effective types 1–8 including extensions, same root/nested index/filter/call/action budgets and residual/attachment state; reuses horizontal interval/origin helpers and explicit ligature source validation. Cursive charges shared attachment updates and separate connection/span budgets. Rejects repeated cursive child, replacement of its parent, and existing mark edge on cursive endpoint; final forest validation. DesignOnly preserves caller pixel values and returns zero pixel origins. Five new synthetic regressions; 1409/platform expected, CI pending. No local Rust toolchain or network checkout; GitHub CI is required. Still limited signed logical-order horizontal convention; no full direction-aware redistribution, chain reversal, automatic provenance or real-font differential evidence. Next C4.5m3d2b after green: explicit flow direction and advance redistribution; then mixed chain interactions and differential validation.


CI #211: 2d07759d696c16e0d27cf757a16ecff6e57d1908, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46020802845, verified all raw jobs and exact Checkout SHA: 1409/platform, 4227 total, zero failed/ignored. Release Windows 27.76s, Ubuntu 15.68s, macOS 18.04s.

C4.5m3d2b: explicit GposHorizontalFlow and execute_gpos_mixed_directional, independent from Lookup RightToLeft flag. Logical order retained; physical signed widths and LTR prefix / RTL right-edge post-width origins. Cursive interval [a,b) in LTR and (a,b] in RTL, solve the corresponding endpoint width after resetting moving-child placement, preserve intervening widths and separate device corrections. Checked terminal pen, interval and coordinate arithmetic. Legacy API retains old behavior. Seven new synthetic tests including independent width-search grid (324 cases), both lookup flags/directions, skipped mark, devices, nested extension, mark chain, terminal overflow; 1416/platform expected, CI pending. Not full OpenType direction-aware endpoint redistribution (two-end corrections and overlapping chain reversal), no bidi reordering, no external reference differential evidence; the explicit physical coordinate contract is bounded and caller supplied. No local compilation performed. Next C4.5m3d2c after green: mixed chain interaction, stale/replacement edge handling and overlapping constraints, then real-font differential.


CI #212: a6340184ab18429801fb4d590febce90c771690b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46024889008, verified exact SHA and raw logs: 1416/platform, 4248 total, zero failed/ignored. Release Windows 24.83s, Ubuntu 9.43s, macOS 8.50s.

C4.5m3d2c-1: directional mixed attachment contribution ledger, per-child reverse index and ordered non-overlapping active intervals; checked removal of only old cursive design/pixel advance contributions, preserving intervening value adjustments. Mark and cursive can replace moving-child edges, exact-pair fixed-end reversal clears stale edge; final forest rejects cycles. Adjacent intervals accepted; other overlapping live intervals explicitly rejected in logarithmic lookup, not silently overwritten. Removal charges shared attachment-update budget; legacy signed API retains its old policy. Eight new synthetic regressions; 1424/platform expected, CI pending. No local compilation. Residual placement follows local contract after detach, not full world-position preservation. General chain reversal, simultaneous overlapping interval solver and full OpenType dual-end redistribution remain incomplete. Next C4.5m3d2c-2 after green: bounded overlapping interval reconciliation and chain semantics, then C4.5m3d2d real-font differential.


CI #213: 95126dcf2aca943277619ba21beee72b302395de, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46028084906, exact SHA and all raw job logs verified: 1424/platform, 4272 total, zero failed/ignored. Release Windows 26.56s, Ubuntu 15.02s, macOS 26.31s.

C4.5m3d2c-2a: standalone exact integer overlapping interval solver (font_gpos_intervals.rs), prefix potential equalities, iterative weighted union-find with union by size, contradiction detection, deterministic lexicographic soft nominal-width retention (LTR reverse, RTL forward), physical anchor-to-interval conversion, checked potential/width/index arithmetic and cumulative work/allocation/domain limits. Seven new tests including independent exhaustive small width reference (50 parameter cases), overlapping/nested/crossing spans, redundant and inconsistent cycles, order independence, overflow and 20,000-glyph chain. 1431/platform expected, CI pending. No local compilation. Signed widths permitted; no optimization of total absolute deviation or positivity guarantee. Solver is standalone; mixed runtime retains its existing overlap rejection and tests until integration. Next C4.5m3d2c-2b after green: integrate active constraints with mixed ledger, preserve lookup-order semantics and budget, then real-font differential. General OpenType chain reversal and two-end redistribution remain incomplete.


CI #214: 823c24a8432715ef129d506fe62e18f9af53d254, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46031232864, exact SHA and raw jobs verified: 1431/platform, 4293 total, zero failed/ignored. Release Windows 15.00s, Ubuntu 12.17s, macOS 9.32s.

C4.5m3d2c-2b: opt-in execute_gpos_mixed_joint integrates active pair constraints with existing root/nested scheduler, GDEF filters, mark/cursive moving-child replacement, exact-pair reversal and final forest. Captures hard interval equations at attachment time; defers solving until final pen calculation, noncursive advances remain soft preferences, local placement residuals remain ordered; shared design/pixel union-find work budget and active constraint/glyph bounds. Legacy immediate directional and signed APIs unchanged, including rejection regression. Six synthetic regressions: nested intervals both flows, crossing filters, replacement, nested extension/soft value order, reversal/cycle, device/budget. 1437/platform expected, CI pending. No local compilation. Explicit deferred hard-constraint policy, not full OpenType immediate advance semantics; later ValueRecords in constrained widths may be redistributed, captured equations are not recomputed after later placement changes. General chain reversal and actual external font differential remain incomplete. Next C4.5m3d2d after green: real-font differential and document coverage; refine semantics where evidence differs.


CI #215: 5026ede64f8b8d53608868870916d36fbfbf5722, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46034356840, exact SHA and all raw jobs verified: 1437/platform, 4311 total, zero failed/ignored. Release Windows 25.95s, Ubuntu 10.87s, macOS 22.99s.

C4.5m3d2d-1: real_gpos_mixed_run.rs adds independent test-only raw SFNT/GPOS/GDEF/hmtx oracle over five already-vendored OFL fonts (Aref Ruqaa, Noto Nastaliq Urdu, Noto Sans Mongolian, Padauk, ABeeZee). Selects actual native Cursive and MarkBase root lookups, up to 24 matched pairs per lookup, both physical flows, compares nominal widths, final design origins, all adjustments and attachment edges for joint and immediate directional engines. Requires positive nonzero evidence, excludes mark filtering sets/attachment-class flags, design only, explicit glyph IDs, not external HarfBuzz differential or full script shaping. Two independent test functions, 1439/platform expected, CI pending; no local compilation or real case counts confirmed yet. Next C4.5m3d2d-2 after green: native multi-lookup chains, device and external reference comparison with explicit unsupported scope, then combined GSUB/GPOS. No new font downloads or production dependencies.
