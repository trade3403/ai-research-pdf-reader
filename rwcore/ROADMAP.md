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


CI #216: 99d983b058de27509a716dd6f1e8e775f8f47345, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46037296945, verified exact SHA and raw logs: 1439/platform, 4317 total, zero failed/ignored; both native coordinate differential tests passed. Release Windows 26.45s, Ubuntu 12.38s, macOS 12.78s. Captured successful test output suppresses per-font sample counts; no exact count claimed.

C4.5m3d2d-2a: extend existing real font oracle with independent bitwise Device decoding and anchor offset selection, ppem grid, supported and unresolved contour/VariationIndex behavior, separate design and pixel origins/adjustments, plus native two-root Cursive+MarkBase triple-glyph chain differential with raw filter and direct world-coordinate reference. Requires nonvacuous device-supported and native chain evidence; nonzero native Device deltas counted but not guaranteed. Two new tests, 1441/platform expected, CI pending. No local compiler or external shaping tool validation. Selected native lookup/glyph cases only, not full script shaping or arbitrary chains. Next C4.5m3d2d-2b after green: external reference baseline (HarfBuzz test-only, reproducible provenance), compare supported semantics and report gaps; no production dependency. No CI workflow changes or new font assets.


CI #217: cbf225f4e3dda3c8988299bc1b5c8b1d4d4a90c7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46040306627, verified exact SHA and raw jobs: 1441/platform, 4323 total, zero failed/ignored; native device and two-lookup chain tests passed. Release Windows 16.64s, Ubuntu 15.86s, macOS 17.35s. Per-font counts not printed on successful default Rust test capture.

C4.5m3d2d-2b-1: external HarfBuzz OT Latin kern reference gate: test-only Rust example gpos_reference_probe, Python standard-library ctypes runner, actual ABeeZee regular/italic fixtures, 10 strings per font (20 cases), explicit kern on/off with stable HB glyph IDs, UPEM LTR design-unit deltas for four position components, exact comparison against native RWCore GPOS plan/joint runner. Requires nonzero evidence, records HB version, font SHA-256, glyph IDs, reference/actual and maximum error in JSON artifact; mismatch fails Linux CI. Ubuntu system libharfbuzz only (apt fallback if missing), no Cargo/production dependency; cross-platform normal CI remains. Expected 1441 Rust tests/platform unchanged plus 20 Linux external comparisons, pending actual CI. No local Rust compilation. Reference uses installed HB version rather than pinning binary; records version and font hash for reproduction. Not a Cursive/Mark/RTL external compatibility claim and not GSUB/cmap validation, because HB provides glyph IDs. Next C4.5m3d2d-2b-2 after green: broaden external differential (mark and complex-script supported scope, document divergences), then GSUB+GPOS integration.


CI #218: b648d1dbb8070a9a21ada76e42e0ed88ae7ba664, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46043289386, verified exact SHA and raw logs: 1441/platform, 4323 total, zero failed/ignored; Release Windows 27.32s, Ubuntu 16.35s, macOS 21.44s. Ubuntu HarfBuzz 12.3.2 actual external kern 20/20, 20 nonzero, 0 mismatches, max error 0 design units; artifact ID 15221177482.

C4.5m3d2d-2b-2a: external Latin mark toggle gate over existing Alegreya Sans Regular, 63 deterministic candidate strings; HB on/off identity/scale and zero baseline placements/vertical advances determine admissible two-glyph cases, all exclusions and actual comparisons reported. Rust probe accepts explicit HB mark-disabled x advances to isolate GPOS from external width normalization; selected mark plan, zero prior residual, four component comparison. All admitted rows must match exactly and at least 3 nonzero reference placements required; no selection by RWCore agreement. Separate Ubuntu CI gate and JSON artifact; old 20 kern comparisons preserved. 1441 Rust tests/platform unchanged, actual admitted mark count and error pending CI. No local compilation; direct binary fetch was unavailable, candidate coverage not confirmed before CI. Reference uses HB glyph IDs and widths, not independent cmap/GSUB/normalization; no RTL, mkmk, arbitrary ligature provenance or Cursive external equivalence. Next C4.5m3d2d-2b-2b after green: multi-mark and complex-script external evidence with explicit scope, then joint GSUB/GPOS. No production dependency.


CI #219: 17dbae73f351dc0fca4fe91861eaf1dbe5d31958, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46045978739, exact SHA and raw jobs verified: 1441/platform, 4323 total, zero failed/ignored. Release Windows 25.09s, Ubuntu 12.11s, macOS 19.90s. HarfBuzz 12.3.2 Kern 20/20 all nonzero, Mark 45/45 from 63 candidates (17 nonzero placement, 18 exclusions), both max error 0. Mark artifact 15222007853; Kern artifact 15222815992.

C4.5m3d2d-2b-2b-1: test-only native Latin stacked mark+mkmk external gate. Up to five vendored fonts, independent raw feature presence check, 441 candidate strings per eligible font, stable three-glyph identity and zero off baseline placement required; compare all admitted four-component deltas against native joint executor, plus independently count HB mkmk-on versus mark-only nonzero positioning. Requires at least one positive native mkmk case and zero mismatch. Retains every candidate exclusion and actual delta in separate JSON artifact. Existing Kern and Mark gates unchanged. 1441 Rust tests/platform expected, external admitted/nonzero count pending CI. No local compilation or binary-font access; exact candidate coverage unverified. Test-only HB IDs/widths and LTR default coordinate scope, no full shaping/RTL or arbitrary ligature claim. Next C4.5m3d2d-2b-2b-2 after green: complex-script/RTL external reference and explicit compatibility gaps, then combined GSUB+GPOS.


CI #220: d1972edafd836c8cf72df720970af53f8dd3f707, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46048762243, exact SHA and raw logs verified: 1441/platform, 4323 total, zero failed/ignored. Release Windows 26.40s, Ubuntu 10.27s, macOS 22.67s. HarfBuzz 12.3.2 Kern 20/20, Mark 45/45, stack 1072/1072 from 1764 candidates (1024 nonzero, 852 mkmk effect), all max error 0. Stack artifact 15223770935. Total 1137 exact external rows. Abel lacks eligible feature pair; four fonts admitted.

C4.5m3d2d-2b-2b-2a: Arabic RTL external diagnostic baseline for Aref Ruqaa and Noto Nastaliq Urdu, separate Mark and Cursive toggles, native feature inventory, logical-order conversion of HB RTL output, baseline physical widths, per-glyph four-component and right-edge-relative world-coordinate errors; up to 114 candidates. Reports all exclusions, probe errors and mismatches, requires nonzero external evidence and actual comparisons for both features. Diagnostic gate enforces evidence/accounting, not zero mismatch; compatibility_pass is explicitly separate. Previous 1137 external exact gate unchanged. Expected 1441 Rust tests/platform, RTL counts and error pending CI. No local compilation or font binary inspection. HB supplies glyph IDs and baseline advances, no independent cmap/GSUB/Bidi/full-script claim. Next C4.5m3d2d-2b-2b-2b after green: use actual RTL discrepancy report to repair bounded algorithm/coordinate semantics and establish exact gate for supported cases, then joint GSUB+GPOS. No production dependencies.


CI #221: c8589bd984096b0eef3b4bd28897eb2b2bbb9036, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46051403140, exact SHA and raw jobs verified: 1441/platform, 4323 total, zero failed/ignored. Release Windows 20.44s, Ubuntu 15.48s, macOS 20.47s. HB 12.3.2 previous 1137/1137 exact, RTL 53/53 exact (114 candidates, 61 excluded; 45 Mark, 8 Cursive; all nonzero; no probe errors; both component/world error 0; compatibility_pass=true). RTL artifact 15224208510. Per-font log counters are cumulative: Aref 25 Mark/4 Cursive, Noto incremental 20 Mark/4 Cursive. Total external 1190 exact. No algorithm change justified by these admitted samples.

C4.5m3d2d-2b-2b-2b: promote RTL diagnostic to strict external parity gate without changing production algorithm. Require fixed 114 candidate inventory, per-font/feature verified positive evidence floors, no missing row or unexplained exclusion, exact component and world-coordinate comparisons, zero errors and coherent totals. Eight Python unit tests exercise gate acceptance and failure conditions. CI preserves detailed report on failure; original 1137 exact comparisons unchanged. Expected 1441 Rust tests/platform plus eight Ubuntu Python policy tests and at least 1190 external exact rows. Not a claim of full Arabic shaping; 61 excluded baseline candidates and external HB glyph IDs/widths remain scope limits. No local Rust compilation. Next after green C4.5m4a: native GSUB+GPOS joint glyph-run API and bounded provenance integration, independent of HB; review existing ROADMAP before implementing. Remaining unsupported full shaping, arbitrary ligature provenance, variation/point positioning and production text segmentation retained.


CI #222: 0ad35a46448643aa78c1a4f0dd9a0829ee182986, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46053978408, exact SHA raw jobs verified, 1441/platform, 4323 total, zero failed/ignored; Release Windows 28.38s, Ubuntu 12.37s, macOS 16.39s. HB 12.3.2 1190/1190 exact, RTL gate_pass=true (53 exact, 61 excluded), eight Python gate tests passed. RTL artifact 15226190275.

C4.5m4a: new production execute_joint_shape API executes native compiled GSUB with scalar provenance, bounds expansion using GPOS/output caps, fetches final glyph hmtx metrics, executes native joint GPOS with design coordinates and preserves provenance, origins, advances and attachments. Eight tests: replacement/metric refresh, expansion, contraction RTL, empty roots/text, domain mismatch, output cap and actual ABeeZee native feature pipeline. Expected 1449/platform, 4347 total, plus existing external gates. CI pending, no local compilation. Explicit same-font caller contract, no cryptographic identity; design only, no mark width normalization or stage masks. Reject reachable GPOS Type 5 and Type 6 after actual ligature contraction pending occurrence/component lineage, rather than inventing mapping from scalar sets. Next C4.5m4b after green: same-face source/feature orchestration and native occurrence-level ligature component provenance bridge (bounded steps), then staged/masked execution and external combined shaping tests. No production dependency.


CI #223: 3ff2a47b2005ca20440c73be5cf14ba824ae459b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46056675231, verified exact SHA and raw logs: 1449/platform, 4347 total, zero failed/ignored. Release Windows 26.54s, Ubuntu 16.85s, macOS 25.40s. All eight joint tests passed, HB 12.3.2 1190/1190 exact and eight Python policy tests passed.

C4.5m4b-1: same-face shape_font_face API constructs source cmap mapping, hmtx metrics, GDEF (explicit mark-set limits), base GSUB/GPOS language plans and compiled programs from one immutable FontFace; returns source, both selected plans, UPEM and joint result. Caller selects script/language/tags/alternates/flow, missing glyph and unavailable-feature policies. Early source/tag/output budgets; preserve base-only variation flags; both layout tables and selectable LangSys required in this initial API, no silent missing-table fallback. Six new tests using ABeeZee: native text pipeline, missing features strict/report, missing glyph policy, budgets, empty run and RTL pen. Expected 1455/platform, 4365 total, existing HB 1190 and Python 8 retained. No local compilation, pending CI. No automatic ligature component mapping yet; Type5 and post-contraction Type6 rejection retained, no mark normalization, no implicit stages/bidi. Next C4.5m4b-2: occurrence-level contraction event/lineage recording during GSUB then bounded GPOS component bridge; do not infer exact component lineage from deduplicated scalar sets.


CI #224 failure: d75fe4573e3b7074d971a7040e5990a40a0bb502, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46059185474, exact SHA confirmed in all three raw job logs. All platforms failed before Rust tests ran: joint_shape.rs lines 93–95 E0515 returning borrowed temporary feature-tag array, line 112 E0716 two temporary feature-tag arrays dropped while still borrowed. No new Rust test totals, Release build or external parity result available for #224. Last accepted #223 remains 1449/platform, 4347 total, 1190 HB exact and 8 policy tests.

C4.5m4b-1 repair attempt 1: replace three temporary borrowed arrays with explicit static typed feature-tag arrays in integration tests; preserve all six new tests, all existing assertions, public API, production code and CI workflow unchanged. Expected 1455/platform, 4365 total, external 1190 and Python 8; pending CI, no local compilation. Do not start C4.5m4b-2 until exact repair SHA passes all three platforms and external gates.


CI #225: 163783fcd40077365cb2ec62cd04c90f1cfcd36f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46061656980, exact SHA raw logs verified, 1455/platform, 4365 total, zero failed/ignored; Release Windows 25.31s, Ubuntu 12.15s, macOS 17.48s. HB 1190 exact and Python 8 passed. C4.5m4b-1 repaired and accepted.

C4.5m4b-2a: opt-in execute_gsub_program_with_ligature_trace / ProvenanceLedger::from_run_with_ligature_trace record actual ordered contraction components (including duplicate scalar sources), unique contraction IDs, retain identity across single replacement; nested contraction and expansion of an existing ligature flagged ambiguous. Independent event/component/work bounds, fallible copying before mutation; legacy provenance counters and behavior unchanged. Six tests for distinct occurrences, skipped gap, duplicated sources, replacement/expansion, nested ambiguity and atomic trace budget. Expected 1461/platform, 4383 total, HB 1190 and Python 8 unchanged. No local compilation; pending CI. Does not infer mark ownership yet; no automatic GPOS Type5 integration. Next C4.5m4b-2b after green: derive bounded occurrence/component mapping for simple unambiguous contractions and skipped marks; explicit ambiguity rejection and GPOS integration tests. No production dependency.


CI #226: cd925ceaff080a1fcf186e8a0355952d2d4349d9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46064243810, exact SHA raw jobs verified 1461/platform, 4383 total, zero failed/ignored. Release Windows 26.47s, Ubuntu 12.68s, macOS 9.41s. Six trace tests passed; HB 1190 exact, Python 8 passed.

C4.5m4b-2b-1: native bounded map_gpos_ligature_sources derives occurrence-specific GPOS parent and mark component IDs from opt-in traced GSUB ledger and GDEF. Requires exact scalar partition, singleton ordered component sources, classified ligature parents; logical scan assigns skipped marks to preceding component and adjacent trailing marks to last; rejects shared/missing sources, nonmark gaps, untracked ligatures and ambiguity. Six new tests including direct traced GSUB -> mapper -> existing GPOS joint Type5 anchor geometry in LTR/RTL with synthetic layout and native cmap/hmtx. Expected 1467/platform, 4401 total; existing HB1190 and Python8 unchanged. No local compilation; CI pending. Scope is simple unambiguous contractions only, not full component history or mark-filter ownership; existing joint API Type5 restriction unchanged. Next C4.5m4b-2b-2: opt-in unified execution with same-face GDEF, strict parent matching and component count validation, more real font evidence; nested and split ligatures remain unsupported. No production dependency.


CI #227: c6960ea48992ba719cf05b72cf3e797cf20c6a19, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46066764827, exact SHA raw logs verified 1467/platform, 4401 total, zero failed/ignored. Release Windows 23.98s, Ubuntu 15.27s, macOS 7.90s. Six mapping tests passed including synthetic GPOS Type5 LTR/RTL anchors; HB1190 exact, Python8 passed.

C4.5m4b-2b-2: opt-in execute_joint_shape_with_ligatures and shape_font_face_with_ligatures integrate traced GSUB, same-face GDEF, bounded mapper, post-substitution metrics and strict joint GPOS. New strict runner requires exact occurrence parent ID/component count for covered Type5, disallows fallback and conflicting MarkBase pairs; existing MarkMark component compatibility retained. Legacy APIs unchanged. Five tests: full synthetic LTR/RTL pipeline, wrong/unmapped parent, wrong component count, trace/mapping budget, native ABeeZee same-face kern equivalence. Expected 1472/platform, 4416 total, HB1190 and Python8 unchanged. No local compile, CI pending. New native same-face test assumes ABeeZee contains GDEF glyph classes; actual CI to verify. No general nested/split ligature support, no full Unicode shaper, no native Type5 real-font combined external evidence yet. Next C4.5m4b-2b-3 after green: real font combined ligature/mark evidence and scope verification, then staged/masked GSUB+GPOS integration. No production dependency.


CI #228: 579b53e3f8b53dbc2ee107c26a71b4e791e21416, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46069216817, exact SHA raw logs verified 1472/platform, 4416 total, zero failed/ignored. Release Windows 26.93s, Ubuntu 9.94s, macOS 12.22s; five strict joint tests passed, HB1190 exact, Python8 passed.

C4.5m4b-2b-3a: native joint ligature diagnostic across nine retained fonts, independent raw lookup/coverage/GSUB rules/GPOS anchors/hmtx/GDEF oracle; reverse native BMP cmap for direct input, first-match rule priority, eligible flags only (no mark sets/attachment classes), bounded contraction verification and intersection with native Type5 parent/mark coverage. For admitted trailing mark rows, strict unified execution must match exact native glyphs, metrics, edges and world coordinates both directions. Positive native contractions required; actual direct Unicode Type5 intersection not presumed, report explicitly distinguishes zero evidence, admitted/exact/nonzero counts. Linux logs report; one new Rust test expected 1473/platform 4419 total, previous gates retained. No local compilation or binary font inspection. Scope not full script shaping, no external combined parity claim; if no positive Type5 rows, expand fixture or contextual input coverage before claiming combined native acceptance. Next C4.5m4b-2b-3b after green: evaluate actual report, add positive evidence if needed or tighten native gate; then staged/masked joint execution.


CI #229: 603f72f23928d2998e186e0b4c6d341ea9e2a015, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46071602422, exact SHA raw logs verified 1473/platform, 4419 total, zero failed/ignored; Release Windows 16.33s, Ubuntu 10.02s, macOS 26.75s, HB1190 exact, Python8 passed. Native inventory totals [4390,1256,127,0,0,0,0]; combined_positive_evidence=false for original nine fixtures. Diagnostic accepted, positive native joint Type5 gate not yet accepted.

C4.5m4b-2b-3b: supplemental distribution DejaVu Sans full font on Ubuntu CI, prepare fonts-dejavu-core only if absent, record package version and SHA256, set RWCORE_DEJAVU_REFERENCE. Independent local fontTools inspection of container DejaVu Sans 2.37 SHA256 57f73e11f51999432bf7ab22ce55b6f945d5eca1bf824404cfa9ec2e3718c84e found GSUB16 lam-alef U+FEDF U+FE8E -> U+FEFB, GPOS8 marks U+064B/U+064E with distinct nonzero component anchor differences. Local font is not a claim about CI font bytes. Test independently locates raw rules and anchors, checks skipped and trailing marks for both components, both physical directions, actual glyphs, lineage, raw widths, edges and world origins. CI requires 4 cases/8 direction checks/8 nonzero exact and prints report, no zero-evidence pass. Retain nine-font inventory and previous gates. No new Rust test functions (supplement existing), expected 1473/platform, 4419 total; new positive gate Linux only. No local Rust compile. Explicit presentation forms and selected roots, no joining/normalization/full Arabic shaping or HB combined parity claim; system font not vendored and not hash-pinned, CI records actual hash. Next after green: C4.5m4c staged/masked joint execution, with external combined shaping gate later. No production dependency.


CI #230: 426f503f15640a50f78c3a5db23531f159be6c31, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46073846432, raw logs exact SHA 1473/platform 4419 total zero failed/ignored; Release Windows 27.33s, Ubuntu 12.56s, macOS 26.08s. HB1190 exact, Python8 passed. Native DejaVu package 2.37-8build1 SHA256 b4c632e3cdf9acc7f28758fb5a323c8524d7fc6660d46904d9b6cbe2809c419c, actual 4 cases/8 direction checks/8 nonzero exact, max error 0. Original inventory 127 contractions, zero direct joint rows retained. Bounded native Type5 gate accepted, no full-script or combined HB parity claim.

C4.5m4c-1: execute_staged_joint_shape uses validated GSUB stage schedule, strict scalar masks, cumulative budgets, optional ordered ligature trace, shared joint preflight and post-GSUB metrics/GPOS finalizer. Exposes schedule, stage change counts, projected masks, skipped roots, blocked checks and mask work. Existing APIs preserve behavior. Four new tests: stage order and post metrics, range masks independent of GPOS namespace, budgets and invalid partition, native ABeeZee staged opt-in trace/kern parity. Expected 1477/platform 4431 total, retain HB1190, Python8, DejaVu 8 direction checks. No local compilation; pending CI. GPOS executes once over final glyph stream, not yet stage/mask gated. No automatic joining, reordering, same-face staged convenience or arbitrary nested ligature support. Next C4.5m4c-2 after green: GPOS independent feature-mask scheduling with strict common-input gating and attachment semantics, followed by same-face staged orchestration. No production dependencies added.


CI #231: 2692ac003e03f190d4f4119279b4a80a11302c18, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46076224845, exact SHA all raw logs, 1477/platform 4431 total, zero failed/ignored; Release Windows 15.25s Ubuntu 15.39s macOS 21.59s; four staged joint tests, HB1190 exact, Python8 and DejaVu 8 directions exact, native 127 contractions retained.

C4.5m4c-2a: GposScalarFeatureMasks independent namespace and projection, build_gpos_stage_schedule shared validated partition logic, execute_staged_gpos_joint with cumulative runner, shared attachment and interval state. Common feature bits on anchor, pair/cursive endpoints, mark parent/child, context matched inputs and inherited nested action bits; mask-disabled participants are barriers, backtrack/lookahead conditions only, work charged cumulatively. Legacy APIs retain unmasked behavior. Four tests (range masks, contraction projection, invalid partition/budget, native kern parity). Expected 1481/platform 4443 total, prior gates unchanged. No local compile; pending CI. GPOS mask/stage runner is separate entry, not yet integrated into same-face staged joint API; synthetic pair/context/attachment-specific negative tests still required next. Next C4.5m4c-2b: strict pair/context/attachment and cross-stage state tests, then integrate both namespaces. No production dependency.


CI #232: 445f7b9c1335b9779bbb6c1ed86aff3e2c1e365e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46078453217, exact SHA all raw logs, 1481/platform 4443 total zero failed/ignored; Release Windows 26.38s Ubuntu 15.98s macOS 15.99s; 4 GPOS mask tests passed; HB1190 exact, Python8 and DejaVu 8 directions exact.

C4.5m4c-2b: six additional synthetic strict GPOS stage/mask regression tests in gpos_mask_stages.rs using explicit feature plans and native scalar provenance seed: pair barrier, GDEF ignored gap, nested common-feature inheritance and shared root replay, backtrack/lookahead conditions, mark attachment preservation and reversal, cursive+mark joint geometry and cumulative context budget. No production math changes. Expected 1487/platform, 4461 total; retain prior gates. No local compile; pending CI. Scope synthetic multi-lookup evidence, not full OpenType parity. Next C4.5m4c-3 after green: integrate independent GSUB and GPOS masks/schedules into single joint pipeline and same-face orchestration; preserve separate namespaces and strict ligature path. No production dependency.


CI #233: d0d2aebb7b6f3de4f6ad7c54cba8f4c71616a2cb, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46080550319, exact SHA in all three raw logs; 1487/platform, 4461 total, zero failed/ignored; Release Ubuntu 12.11s, Windows 25.24s, macOS 12.57s. All three jobs successful.

C4.5m4c-3: execute_dual_stage_joint_shape and shape_font_face_dual_stage integrate separately validated GSUB/GPOS stages and masks, independent budgets, final glyph metrics, shared same-face parsing and strict optional ligature mapping. Legacy APIs retain their existing behavior. Five new tests: independent range masks and final metrics, GPOS budget/partition atomicity, schedule ordering, native strict parity and empty input. Expected 1492/platform, 4476 total; CI pending, no local Rust compile. No implicit script preprocessing, joining/syllables, variable metrics or complete OpenType parity. Next after green: dual-stage contraction/expansion and native component-attachment differential acceptance before Unicode script-specific shaping. No production dependencies.


CI #234: 333bc0a4378884ad6990d8b930886ac93c1813b0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46082656854, exact SHA in all three raw logs; 1492/platform, 4476 total, zero failed/ignored; Release Ubuntu 15.59s, Windows 26.59s, macOS 16.06s. Linux reference steps and Python 8 tests passed.

C4.5m4c-4a: five synthetic dual-stage regressions in joint_shape.rs, covering expansion mask inheritance, contraction mask intersection in both physical flows, independent strict GSUB participant masks, exact final GPOS projection budget and output cap, strict two-component Type5 attachment geometry and disabled ligature parent. No production algorithm changes. Expected 1497/platform, 4491 total; CI pending, no local Rust compile. Next after green C4.5m4c-4b native component attachment differential using independent raw font reference; no full complex script parity claim.


CI #235: 25ea27e376255a24f6eadbcf54be74bc0e697904, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46084840184, exact SHA in all three raw logs; 1497/platform, 4491 total, zero failed/ignored; Release Ubuntu 12.78s, Windows 26.48s, macOS 20.17s. Native inventory TOTAL [4390,1256,127,0,0,0,0], DejaVu 4 cases/8 direction checks exact, Python 8 tests passed.

C4.5m4c-4b: native_joint_ligature.rs isolates original native lookup roots under synthetic single-feature DFLT plans, retaining the LookupList suffix byte-for-byte and original indices. Extend existing nine-font contraction gate to dual-stage parity with raw nominal width and source reference; extend supplemental Linux DejaVu GSUB4/GPOS5 gate to dual-stage exact component anchors in both flows and masked-parent suppression without changing GSUB contraction. One additional regression for native lookup byte/root preservation; 1498/platform, 4494 total expected; CI pending, no local Rust compile. Native LangSys selection and full script shaping not tested by the isolated root reference; Linux positive Type5 remains supplementary, no HarfBuzz combined parity claim. Next after green: C4.6 Unicode/script preprocessing foundations, then joining, syllable segmentation, reordering and variable metrics, before C5. No production algorithm or dependency changes.


CI #237: bc01038b2841464baa12c7e9f528af3229fe1624, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46088357541, exact SHA all three raw logs; 1498/platform, 4494 total, zero failed/ignored; Release Ubuntu 16.11s, Windows 21.20s, macOS 8.82s. Dual-stage native contractions 127 exact; DejaVu 8 exact direction checks and 8 masked parent blocks, max error 0; Python 8 tests passed. C4.5 bounded dual-stage gate accepted.

C4.6a: native Unicode Script property reader (unicode_scripts.rs), explicit caller-supplied Scripts.txt snapshot, canonical two-field UCD range syntax, sorted disjoint range index, Unknown default, strict supported @missing directive, bounded allocations, binary lookup, logical raw script spans with exact scalar/UTF-8 offsets. UAX #24 https://www.unicode.org/reports/tr24/ and UAX #44 https://www.unicode.org/reports/tr44/ reference. Eight new tests including exhaustive scalar-domain lookup against a synthetic reference; expected 1506/platform, 4518 total; CI pending, no local Rust compile. No complete pinned Unicode dataset yet, no ScriptExtensions, script resolution, normalization, joining, bidi or Indic shaping. Next C4.6b: pin licensed official UCD snapshot with provenance and full-data validation, then script extensions/resolution and joining. No production dependencies.


CI #238: c4637603b7a73f935866730c5de26cde2cb9e05b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46090195945, exact SHA all three raw logs; 1506/platform, 4518 total, zero failed/ignored; Release Ubuntu 13.15s, Windows 27.32s, macOS 19.10s. Dual-stage native 127 contractions, 8 exact and 8 blocked DejaVu direction checks; Python 8 tests passed.

C4.6b: bundle Unicode 17.0.0 official Scripts.txt and Unicode-3.0 LICENSE, byte-identical Git blobs to Unicode Consortium unicodetools upstream (5574fdd6ae35ddc074397fdc60fbe2e27e795713, d7e7973c2fd6f2586a8999a69dc21e39af26be0f). Provenance README retained. New bundled_unicode_scripts() uses existing native bounded parser; no runtime network or dependencies. 2287 raw records, 174 explicit script values, 159866 explicit code points; four new tests include exhaustive 1112064 valid scalar comparisons against independent dense raw-data reference, version/inventory, mixed text offsets and new/unassigned characters. Expected 1510/platform, 4530 total; CI pending, no local Rust compile. Script property only; no aliases, ScriptExtensions, joining, bidi, normalization or full shaping. Next C4.6c ScriptExtensions and alias resolution after green. No production dependency.


CI #239: fd1b08385408005dc26e9ffd6a7835b9ff2a0edf, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46091919254, exact SHA all three raw logs; 1510/platform, 4530 total, zero failed/ignored; Release Ubuntu 9.82s, Windows 30.45s, macOS 18.24s. Full 1112064 valid scalar Script dense reference passed on all platforms; Linux native and Python gates passed.

C4.6c: native Unicode ScriptExtensions and Script aliases engine, byte-identical official Unicode 17.0.0 ScriptExtensions.txt and PropertyValueAliases.txt under retained Unicode-3.0 license; 206 explicit ranges covering 669 code points, 176 sc alias records. Bounded parsing, loose alias normalization, collision/overlap/duplicate validation, sorted explicit sets with primary Script fallback, allocation-free set queries. Eight new regressions including exhaustive 1112064 valid scalar comparisons with dense explicit-set reference; expected 1518/platform, 4554 total; CI pending, no local Rust compile. No contextual Common/Inherited resolution, joining, bidi, normalization or full shaping. Next C4.6d bounded script-run resolution using primary and extension sets with explicit ambiguity policy. No production dependency.


CI #240: 82ea9b94e96681dc3cf535b96548056a0f33d178, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46093629006, exact SHA all three raw logs; 1518/platform, 4554 total, zero failed/ignored; Release Ubuntu 16.83s, Windows 25.53s, macOS 18.66s. Full 1112064 scalar ScriptExtensions reference passed on all platforms, Linux native and Python gates passed.

C4.6d: native conservative contextual script itemization in unicode_script_resolve.rs. Fixed admitted primary or singleton anchors; two logical directional passes with extension-set compatibility and Unknown/incompatible barriers; explicit Preserve or PreferLeft conflict policy; scalar resolution kind, original UTF-8 offsets, adjacent label spans and bounded bytes/scalars/spans/work. Eight regressions for shared kana, combining marks, conflict, unknown barrier, empty and exact budgets. Expected 1526/platform, 4578 total; CI pending, no local Rust compile. Product policy, not full UAX #9 bidi, grapheme or paired-bracket resolution; no script-specific shaping or reordering. Next C4.6e pinned Unicode joining properties and Arabic joining analysis foundation after green. No production dependencies.


CI #241: 817d30d97b600a2839a29b9a41b16e756035bf90, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46095280418, exact SHA all three raw logs; 1526/platform, 4578 total, zero failed/ignored; Release Ubuntu 15.75s, Windows 22.69s, macOS 21.74s. Contextual script regressions and Linux native/Python gates passed.

C4.6e: native Unicode Joining_Type interval parser and basic logical RTL Arabic joining analysis in unicode_joining.rs. Official Unicode 17.0.0 DerivedJoiningType.txt imported byte-identical (Git blob 9e6aa70d0da1ef6a7a480924cc1ea7b4e8407a33), Unicode-3.0 license retained; 542 ranges / 3004 explicit code points. Transparent marks skipped for link selection while preserving offsets; ZWJ/Tatweel join-causing, ZWNJ and nonjoining barriers; four-form feature tags for D/R/L, no form for C/T/U. Eight regressions including exhaustive 1112064 scalar type reference, all 36 type pairs, forms, controls and budgets. Expected 1534/platform, 4602 total; CI pending, no local Rust compile. Caller-selected logical run only, not full Arabic shaping, Syriac special forms, Mongolian, bidi or normalization. Next after green C4.6f Arabic four-form scalar masks and staged GSUB integration, with explicit feature planning and native-font differential tests. No production dependency.


CI #242: 0bebff9d6138cc50f353f6cd70ff1fef83eee1b9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46097049151, exact SHA all three raw logs; 1534/platform, 4602 total, zero failed/ignored; Release Ubuntu 12.65s, Windows 27.00s, macOS 17.62s. Joining_Type exhaustive scalar reference passed on all three platforms; Linux native and Python gates passed.

C4.6f: native Arabic four-form mask bridge in font_arabic_forms.rs and same-face shape_font_face_arabic_forms in font_joint_shape.rs. Validated GSUB feature IDs, exact source identity, contiguous disabled ranges, shared generation/mask-build work budget, explicit selected features and Arabic stage template, independent GPOS namespace, final glyph metrics. Missing optional forms not synthesized; incompatible required form and Syriac special form features rejected. Six regressions: synthetic actual four-form GSUB substitution, missing forms, source/required checks, special forms, budgets, supplemental Linux DejaVu beh presentation-form cmap oracle. Expected 1540/platform, 4620 total; CI pending, no local Rust compile. Native DejaVu positive test conditional on existing Ubuntu font gate; no claim full Arabic shaping, normalization, bidi, mark width or all fonts. Next after green: C4.6g Arabic rlig/mark/cursive integration and native differential scope. No production dependencies.


CI #243: 5c3a217ff83dbf9f44d4cdea4d2e0bc27d675b8c, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46098932696, exact SHA all three raw logs; 1540/platform, 4620 total, zero failed/ignored; Release Ubuntu 12.82s, Windows 15.97s, macOS 30.43s. Ubuntu supplemental native Arabic beh-form cmap oracle and prior native/Python gates passed. Other platforms conditionally return from supplemental font test.

C4.6g1: add shape_font_face_arabic_forms_staged with independent caller-selected GPOS feature partition and scalar overrides; shared same-face implementation retains legacy single-stage behavior. Two supplemental DejaVu native regressions: logical lam/alef plus two marks at both component positions and physical flows (8 geometry checks), raw cmap/GPOS5 anchor oracle, legacy parity and masked-parent suppression; two-stage kern/mark partition and resource rejection. Expected 1542/platform, 4626 total; CI pending, no local Rust compile. Native tests require existing Linux CI font; other platforms return conditionally. No full HarfBuzz parity, normalization, bidi, mark-width or cursive integration acceptance claim. Next C4.6g2 native cursive/mark stage differential and retained-font coverage after green. No production dependency.


CI #244: e4a9f7a2977812cce2de7b1fd07b88b2b283ce67, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46100612332, exact SHA all three raw logs; 1542/platform, 4626 total, zero failed/ignored; Release Ubuntu 16.40s, Windows 16.25s, macOS 29.43s. Ubuntu supplemental DejaVu logical lam/alef component geometry (8 direction checks), mask suppression, independent stage partition/limits and previous native/Python gates passed. Other platforms conditionally return from supplemental font tests.

C4.6g2: extend real_gpos_mixed_run.rs with two cross-platform retained-native-font differential tests. Preserve raw LookupList bytes/indices under test-only two-feature plans; project scalar masks through an explicit glyph ledger; independently compare cursive/mark-base pair origins, adjustments and edges in both physical flows with either feature disabled and both blocked. For admitted native Type3/Type4 chains, run cursive then mark in distinct stages; require positive native intersection, exact world-anchor equations and preserved mark edge when cursive is disabled. Reuse existing bounded independent raw candidate selection. Expected 1544/platform, 4632 total; CI pending, no local Rust compile. No production code or dependencies changed. Explicit glyph IDs and isolated roots, not full same-face Arabic GSUB-to-cursive acceptance. Next C4.6g3 same-face logical Arabic retained-font cursive/mark integration and scope diagnostic after green; bidi, normalization, variable positioning and full HarfBuzz parity remain separate.


CI #245: 12610bda67f456f681a2e967fcb753ce1eeef504, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46102392578, exact SHA all three raw logs; 1544/platform, 4632 total, zero failed/ignored; Release Ubuntu 12.80s, Windows 27.05s, macOS 21.20s. Native staged pair and positive two-lookup chain tests passed all platforms; Linux native/Python gates passed. Exact diagnostic counts not printed in captured passing-test logs.

C4.6g3: add two native logical Arabic acceptance tests in real_gpos_mixed_run.rs. Retained Aref Ruqaa and Noto Nastaliq Urdu: 48 bounded input/profile attempts with original LangSys selection; explicit missing/unsupported/oracle-exclusion reasons and admitted Type3/4 anchor world-coordinate assertions; diagnostic report target/arabic-native-scope.txt, not a guaranteed positive same-face cursive intersection. Supplemental Linux DejaVu: require positive logical beh pair + mark Type4 oracle, presentation-form cmap IDs and raw anchor coordinates, both physical flows. Expected 1546/platform, 4638 total; CI pending, no local Rust compile. No production code/dependency change. Native logical cursive support remains evidence-dependent; no full Arabic/HarfBuzz parity, bidi, normalization or mark-width claim. Next after green inspect retained scope evidence, complete bounded Arabic acceptance or address supported-profile blockers, then Indic property/syllable foundation. Do not claim full shaping on zero intersection.


CI #246: 817d17b3f9548cccf5efa55ede7fd92d0a2ff084, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46104108976, exact SHA all three raw logs; 1546/platform, 4638 total, zero failed/ignored; Release Ubuntu 10.72s, Windows 25.62s, macOS 19.73s. Retained Arabic scope test and positive Ubuntu DejaVu logical mark-base oracle passed; Linux historical native and Python gates passed. Scope report was neither emitted nor uploaded, so exact retained-font positive/unsupported counts remain unverified.

C4.6h1: native Unicode IndicSyllabicCategory and IndicPositionalCategory typed property index and scalar annotation, bundled byte-identical Unicode 17.0.0 UCD data (967/653 ranges; 4856/1299 explicit code points) with retained Unicode-3.0 license. Shared bounded parser, exact defaults, overlap and malformed input checks, binary lookup and original UTF-8 offsets. Six regressions including two independent exhaustive scalar-domain references. Expected 1552/platform, 4656 total; CI pending, no local Rust compile. RWCore CI adds Ubuntu retained Arabic scope report output and 49-line inventory gate, without changing test or build gates. Arabic full native cursive coverage remains unresolved pending report; no full Arabic/Indic shaping, syllable grammar, reordering or bidi claim. Next after green inspect actual Arabic report, and begin bounded Devanagari syllable grammar and source-preserving cluster analysis. No production dependency.


CI #247: d43b8c44491bf4cd134799a4e35f93b861d4eed3, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46105830676, exact SHA all three raw logs; 1552/platform, 4656 total, zero failed/ignored; Release Ubuntu 12.41s, Windows 16.48s, macOS 19.68s. Both Indic exhaustive 1112064 scalar reference tests passed on all platforms; Linux historical native and Python gates passed. Arabic scope report: 48 attempts, 2 exact Aref Ruqaa logical cursive cases (meem pair and meem pair with fatha), 6 no unique direct anchor chain, 40 outside direct Type3/4 oracle (16 Aref, 24 Nastaliq). Exclusions are not demonstrated production failures; full Arabic complex feature/mark coverage remains open.

C4.6h2: native conservative Devanagari cluster state machine in indic_clusters.rs. Script/ScriptExtensions gating; consonant + optional nukta, virama-linked consonants, ZWJ and terminal ZWNJ policy, single dependent vowel, trailing modifiers, independent vowels and explicit dotted-circle placeholder. Orphan marks/controls preserved as Broken with no fabricated base; foreign/unhandled scalars remain individual Other. Original scalar and UTF-8 spans, last admitted consonant base candidate, terminal virama flags, byte/scalar/cluster/work budgets. Eight regressions for grammar, boundaries, source partition and limits. Expected 1560/platform, 4680 total; CI pending, no local Rust compile. Product grammar only: not full UAX #29, normative Indic syllabification, normalization, visual reordering or GSUB. Next C4.6h3 restricted Devanagari base/reph and pre-base matra planning with source permutation and explicit unsupported cases after green. No production dependency.


CI #248: 637f214a21a54f0e0f38f21f67909229e6a98a8f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46107521926, exact SHA all three raw logs; 1560/platform, 4680 total, zero failed/ignored; Release Ubuntu 16.91s, Windows 26.22s, macOS 8.95s. Devanagari cluster state machine and historical native/Python gates passed.

C4.6h3: native conservative Devanagari pre-shaping reorder planner in indic_reorder.rs. Analyze original logical cluster run; detect RA+virama implicit reph candidate and explicit join-control policy; last admitted base candidate; identify left matra and defer mixed-position matras, conjuncts, terminal virama and join controls. Simple single-base left matra gets provisional cluster-local scalar permutation and inverse, with nukta/tail order retained; all deferred/broken/foreign clusters remain logical. No SourceRun mutation, no actual glyph reordering before GSUB, no reph success or final base claim absent font evidence. Independent byte/scalar/work bounds, eight regressions for planning, inverse and source preservation. Expected 1568/platform, 4704 total; CI pending, no local Rust compile. Next C4.6h4 source-preserving glyph-level permutation and provenance-safe post-GSUB application, with feature evidence and explicit rejection of unsupported clusters; then native dev2 GSUB/GPOS integration. No production dependency.


CI #249: 38455354925e8b755e803f44d1699523a6661707, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46109215806, exact SHA all three raw logs; 1568/platform, 4704 total, zero failed/ignored; Release Ubuntu 13.19s, Windows 35.58s, macOS 22.78s. Devanagari provisional reorder planner, native and Python gates passed.

C4.6h4: native atomic ProvenanceLedger::reorder (physical occurrence permutation, cumulative bounded work, preserves logical source sets and ligature lineage), plus indic_glyph_reorder.rs restricted post-GSUB prebase projection. Validate exact source identity against logical text, ordered cluster ownership, complete source coverage, no cross-cluster or mixed-matra provenance; stable movement of matra-only expansion group, all other glyphs retain relative order, independent work caps. Reject deferred/unsupported/broken clusters atomically. Caller supplies same-run logical GSUB ledger; no feature success inferred, no execution buffer/GPOS integration yet. Eight regressions with retained Noto Devanagari cmap, expansion, contraction lineage, inverse, invalid and resource atomicity. Expected 1576/platform, 4728 total; CI pending, no local Rust compile. Next C4.6h5 integrate restricted simple-base Indic post-GSUB permutation into joint pipeline, reproject masks, rebuild final metrics and verify native dev2 feature evidence. Complex conjunct/reph/normalization/bidi remain open. No production dependency.


CI #250: 4d9c5b2c763a6d81e1f0d665da09ecda69578722, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46110874992, exact SHA all three raw logs; 1576/platform, 4728 total, zero failed/ignored; Release Ubuntu 14.70s, Windows 21.04s, macOS 8.35s. Atomic ledger permutation, Indic glyph expansion/contraction and source safety regressions passed all platforms; Linux historical native and Python gates passed.

C4.6h5: restricted opt-in Indic joint pipeline execute_indic_joint_shape and same-face shape_font_face_indic_simple in font_joint_shape.rs. Explicit logical GSUB stages then bounded prebase glyph permutation, preserving original GSUB output separately; final provenance-aligned glyph IDs and reprojected GSUB masks (charged against remaining work), final metrics, independent staged GPOS masks. Existing APIs unchanged. Five tests: actual synthetic GSUB expansion/contraction, dual physical flows, source mask alignment, resource caps, deferred rejection and retained Noto dev2 nukt LangSys selection with logical baseline comparison. Expected 1581/platform, 4743 total; CI pending, no local Rust compile. This is a deliberately restricted selected-feature pipeline, not complete dev2 ordering, reph, half forms or font-specific matra/mark attachment acceptance. Next after green C4.6h6 font-specific Indic feature evidence and base/reph cluster mask planning; native differential and complex reorder remain open. No production dependency.


CI #251: 648f6d04730e6bd8976c4bc24f78f08de8222289, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46112599338, exact SHA all three raw logs; 1581/platform, 4743 total, zero failed/ignored; Release Ubuntu 12.03s, Windows 20.39s, macOS 15.19s. Restricted Indic GSUB/reorder/GPOS joint tests and retained native dev2 nukt selection passed all platforms; historical Linux native/Python gates passed.

C4.6h6: indic_feature_probe.rs enumerates conservative disjoint rphf and half source ranges with provisional base candidate, skipping join-control/terminal/broken/unsupported clusters. Isolated single-candidate strict masked GSUB probe, actual native dev2/deva same-face feature selection, exact source identity, outside-candidate edit rejection, bounded evidence scan and net output classification Unchanged/Changed/WholeRangeContraction. Selected unrelated features (including required) reject; required-feature mask restrictions retained. Context may inspect outside the candidate. Six regressions including synthetic positive reph contraction, half single change, isolation, limits and retained Noto native dev2 rphf/half probes. Expected 1587/platform, 4761 total; CI pending, no local Rust compile. Native probe records net outcome, does not require positive native contraction nor prove semantic reph/half or final base selection. No full complex shaping, final reorder or implicit complete script feature schedule. Next C4.6h7 inspect native probe evidence and implement bounded feature-specific stage outcome/provenance integration and final reph placement planning; preserve complex unsupported boundaries. No production dependency.


CI #252: 766c3de23675093c31b91c6250d1893d0e8b57ce, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46114211556, exact SHA all three raw logs; 1587/platform, 4761 total, zero failed/ignored; Release Ubuntu 16.61s, Windows 28.12s, macOS 20.86s. Six Indic feature-probe tests and previous native/Python gates passed. Native rphf/half outcome details were captured by Rust test harness and not visible in successful logs.

C4.6h7: isolated bare RA+virama+consonant reph after-base candidate planning in indic_feature_probe.rs. Execute the selected isolated rphf feature, require whole-range contraction, exact two-glyph source partition and unchanged base, return occurrence order and inverse; native same-face wrapper. Explicitly reject noncontraction and complex inputs; proposed after-base policy is not a font-proven final position and is not automatically applied. Three regressions (positive synthetic actual contraction and ledger permutation, rejection, native outcome-dependent acceptance). Existing native test writes two report rows (evidence and glyph IDs); Ubuntu RWCore CI prints and requires both rows without changing existing test/build gates. Expected 1590/platform, 4770 total; CI pending, no local Rust compile. Next C4.6h8 inspect emitted native outcomes, build restricted Reph permutation into joint stage with explicit after-base policy and GPOS mask/metrics validation where evidence permits; defer other matra/conjunct/half cases. No production dependency.
