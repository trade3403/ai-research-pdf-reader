# Outstanding work and limits

- CI confirms synthetic regressions and compilation, not full Microsoft Word fidelity or real-world performance. Legal real DOCX paper corpus, Word reference comparisons, dependency security/license audit and exact lock artifact provenance are outstanding.
- No GUI, installation package, actual renderer or visual acceptance yet; no claim otherwise.
- Font metrics/shaping and intrinsic nested content measurement still external. Row/column allocation is documented deterministic subset; no complete Word pagination, table style cascade, border painting, distributed vertical placement or RTL reordering.
- Header/footer nested tables and unsupported wrappers remain opaque. Inline images in supported runs preserve order, but invalid/unsupported drawing nodes are diagnosed and omitted, not round-trip preserved. AlternateContent, full floating layout, image-aware line breaking and actual painting still missing.
- Raw media signature and header parsing do not validate complete files, decode pixels, process ICC/DPI/orientation or support SVG/EMF/WMF. External links inert.
- Run public text mutation can invalidate image offsets; inline_content checks boundaries/order but later C3 editing transactions must maintain semantic anchor positions.
- DOCX writing, full round-trip fidelity, actual page counts, formulas, cross references, Electron integration not implemented by current C phase.
- RDHS cannot read precise remaining account/context quota. Fixed handoff checkpoints are preventive, not system limit detection. Branch CAS protects writes but does not replace coordination of old/new windows and existing automation.

- C2.4d-1 verified CI #55: standalone floating metadata only; wrap polygons, effect extents, unknown optional extensions, full schema validation, placement (in that batch), collision handling, painting and document-tree integration are not implemented. Simple position absent defaults to false; numeric values use a conservative integer subset. No real Word fidelity claim.

- C2.4d-2 verified CI #56: signed EMU static placement and single-anchor rectangular avoidance only. Inside edges, measured reference spans and cell context are caller responsibilities. Largest-side ties choose physical left; center uses floor rounding; no clipping. Tight/through polygons are not parsed; fallback must be explicitly enabled. No full line breaking, multi-object collision/flow, actual cell constraint, effect geometry, painter, pagination or Word-reference comparison.

- C2.4e verified CI #57: standalone geometry subset, rejects negative/percent-string crop, tiles and nonzero fillRect; no group transforms, arbitrary shape clipping, effect extent, EXIF orientation, pixel resampling, rendering, or automatic wp:extent reconciliation. Noncardinal geometry uses f64 trig and is not bit-exact across platforms; inverse accuracy depends on scale and coordinate magnitude. XML geometry parser was not wired into document tree in C2.4e; real Word corpus reference comparison remains outstanding.

- C2.4f verified CI #58: mixed drawing index and scoped media integration in supported body/table/story runs. Geometry errors are retained on drawing state; unsupported drawing nodes still omitted with diagnostics. Header/footer nested tables and wrappers opaque; media loading performs two archive passes and strict catalog may reject unsupported unrelated image types. Mutable public Run indices require maintenance by C3 transactions; legacy inline-only view excludes floating. Geometry reconciliation only zero shape offset/equal extents, caller supplies page origins; rotated wrap contours, full layout and drawing not implemented. No real DOCX corpus or GUI acceptance.

- C3.1 verified CI #59: atomic text transactions restricted to existing body and nested body-table runs. No cross-run/paragraph edits, story field consistency, grapheme boundaries, undo/redo, optimistic revision ID or serialization. Stale-text checks are not full concurrency/version checks; text budgets exclude metadata heap allocation. Public mutable model can bypass validation. Real DOCX corpus, reference compatibility, dependency audit and GUI gates outstanding.

- C3.2 verified CI #60: same-paragraph text-only selection, including existing nested body-table paragraphs. Images are zero-width in text indices; explicit run endpoints distinguish shared boundaries but not individual same-offset drawings. Preservation leaves anchors in original runs, not necessarily global selection start. Empty runs retained; runless paragraphs cannot insert yet. No paragraph joins/splits, cross-cell/story edits, field integrity, grapheme segmentation or GUI selection. Prefix lookup is linear in run count; no performance benchmark. Text budgets exclude metadata.

- C3.3a verified CI #61: in-memory owned editor, text-only history, revision local to each editor, no crash recovery or persisted journal, no structural or cross-session identity, no grouping/coalescing, no selection/caret restoration, no GUI or DOCX writer. Retention budgets measure logical text/offset/run counts, not exact heap bytes; C3.1 staging still temporarily clones touched-run metadata. Out-of-memory abort not recoverable by logical transaction guards. Real corpus/performance and lock audit remain outstanding.

- C3.3b verified CI #62: only paragraph siblings within one body section or nested body table cell, no cross-section/cell joins, table insertion/deletion, section-property changes, stories or opaque node edits. Split copies paragraph formatting and keeps runs even when empty; join requires explicit property policy and does not merge runs. Section ranges update; actual list numbering, pagination and layout need recomputation by consumers. Standalone structural operations invalidate old paths; not exposed through text-only DocumentEditor yet. Logical budgets not exact heap cap; no structural undo or persisted history, GUI, DOCX writer or real corpus acceptance.

- C3.3c verified CI #63: unified in-memory text and structural history restricted to C3.3b supported ranges. Revision checks are session-local and require callers to pass the original revision of their path; no automatic remapping or stable IDs. Structural snapshots clone metadata for affected paragraphs and changed section metadata, not media bytes; logical budgets exclude metadata heap size and transient staging. Standalone mutable APIs remain outside editor history. No persistence, crash recovery, actual font layout/pagination, GUI, or DOCX serialization. Real corpus and dependency audit gates outstanding.

- C4.1 verified CI #64: bounded directory-level subset only. Supports raw 0x00010000/OTTO SFNT and TTC v1/v2, not legacy true/typ1, WOFF/WOFF2 or resource forks. No semantic required-table checks, padding-content checks, whole-font checksum, digital signature verification, font license/discovery, cmap, metrics, outline execution, glyph shaping or rasterization. Strict overlap rules may reject unusual fonts; real licensed font corpus and differential verification pending. Table checksum opt-in and per face, no guarantee of validity from parse success.

- C4.2a verified CI #65: only four metric tables, strict lengths and flavor/version constraints may reject unusual fonts. No outline-derived validation of head bounds/hhea extrema/maxp claims, no automatic checksum checks, OS/2 metric selection, variable axes/HVAR/MVAR, vertical metrics, cmap, shaping or rasterization. Scaling rounds each requested value independently; callers should accumulate unrounded design units where possible to avoid drift. Real licensed font corpus and reference-engine differential acceptance pending.

- C4.2b verified CI #66: only selected Unicode cmap 4/12 semantically validated; other records have bounded format-word offsets but are not fully parsed. No per-character fallback, formats 0/2/6/8/10/13/14, symbol encodings, variation selectors, normalization, shaping or font fallback. Strict range and encoding policy may reject unusual fonts; real licensed font corpus and reference comparison outstanding.

- C4.2c verified CI #67: single-face default-instance nominal measurement only. No shaping, kerning, variation selectors, grapheme clusters, normalization, bidi, font fallback, control/tab layout, line breaking, actual ink bounds, Word-specific line spacing or rendering. Combining characters are measured independently; glyph 0 substitution is explicit. Limits cover input text and scalar count, not exact allocator usage. Real licensed font corpus and differential acceptance remain outstanding.

- C4.3a verified CI #68: simple default-instance TrueType outlines only; no composite decoding, CFF/CFF2, variations, hint execution, curve extrema, rasterization or paint. Loca requires first offset zero, exact length and nondecreasing bounded offsets; simple glyph padding must be zero. Maxp claims not enforced beyond glyph count; explicit caller limits bound decoding. Real licensed font corpus, reference geometry comparison and fuzz/security audit outstanding.

- C4.3b verified CI #69: unhinted default-instance composite point assembly only. Phantom-point attachments rejected, ROUND_XY_TO_GRID not applied, no variation deltas, transformed advances or hinting. Fixed 16.16 rounding per component may differ from other engines; strict reserved flags and multiple metric-source rejection. Limits count cumulative leaf geometry and visits, not exact heap size. Composite path conversion and curve bounds follow; no CFF, rasterization or real licensed corpus differential acceptance.

- C4.3c verified CI #70: exact rational extrema of the previously rounded default-instance TrueType outline, conservatively rounded to the path grid; not exact original pre-transform composite geometry, hinted ink bounds, stroke bounds or fill coverage. Degenerate contours contribute point geometry; no flattening, fill, rasterizer, CFF, variations, or real font corpus differential acceptance.

- C4.3d verified CI #71: geometric flattening tolerance is in PATH_ONE design grid units and L-infinity norm, not a device pixel or Euclidean error guarantee. Per-curve uniform dyadic intervals, variable count across curves; includes rounding bound. Degenerate and duplicate vertices retained, signed area can cancel for self-intersections and is not filled area. No fill rule, clipping, device scaling, coverage, hinting or actual font corpus/differential acceptance.

- C4.3e verified CI #72: regular-grid supersampling estimates polygon coverage, not analytic curve/ink coverage. Boundary classification half-open, device vertices rounded to 16.16, input flattening tolerance is still design-space and chosen by caller. Explicit viewport and 26.6 ppem/baseline, no automatic bitmap bounds or advance placement. Reference scanline implementation scans all edges per subrow and sorts intersections; performance unbenchmarked. No gamma, LCD, hinting, CFF, font fallback, real licensed corpus or differential visual acceptance.

- C4.3f verified CI #73: same-face unhinted TrueType grayscale single-glyph rendering only. Device tolerance bounds flattened geometry relative to rounded assembled paths, not coverage error or original pre-transform outlines. Auto viewport uses outward curve bounds and may include transparent pixels. No hinting, phantom-point metrics, transformed USE_MY_METRICS advance, CFF, shaping, text-run composition, font fallback, visual/reference corpus validation or performance acceptance. Stage-specific limits are not a global heap cap.

- C4.3g verified CI #74: nominal single-face logical-order text only, no shaping, kerning, bidi, grapheme clusters, font fallback, tabs/line layout or DOCX painting. Source-over rounded 8-bit alpha is layer composition, not analytic union of overlapping contours. No bitmap cache or benchmark. Aggregate pixel/sample budgets and per-glyph limits bound the reference implementation, not exact heap or cumulative CPU work across every stage. Real licensed font/reference geometry and visual comparisons pending.

- C4.3h-1 pending CI: comparison rectangle includes transparent gaps; no automatic threshold or reference generation. Real font licenses, fixture provenance, reference geometry and visual acceptance remain outstanding.

- C4.3h-2a pending CI: one Latin font only; no independent grayscale reference, multilingual corpus or Word compatibility acceptance. License and source blob provenance retained.

- C4.3h-2b pending CI: test oracle uses shared parsed paths, f64 dense polygon approximation, six cases from one font; not independent outline extraction or analytic coverage. Error thresholds are provisional engineering gates, not Word fidelity guarantees.

- C4.3h-2c pending CI: two styles of one family only; independent simple points but no independent composite assembly or italic reference raster acceptance yet.

- C4.3h-2d1: translation-only real composites; transformed and point-attached real reference fixtures, additional font families and broader corpus validation outstanding.

- C4.3h-2d2: two Latin families only; no CJK, independent Abel outlines, transformed real composite reference, or Word fidelity acceptance. Gray reference shares parsed paths.

- C4.3h-2e1: real scaled/reflected composite fixture only; general off-diagonal matrix, point attachment, Arabic shaping, CJK, and broad font fidelity remain outstanding.

- C4.3h-2e2a: new exact synthetic regressions do not establish real-font off-diagonal matrix or point attachment compatibility; those remain outstanding alongside Arabic shaping/CJK.

- C4.3h-2e2b real off-diagonal and point attachment fixtures outstanding, retained while independent shaping development proceeds. C4.4a1 Coverage parsing only; GSUB/GPOS execution and shaping not implemented.

- C4.4a2a: directory-only GSUB parser; child contents, cross-index checks, overlapping child structures and variation semantics not validated. No real GSUB fixture acceptance yet.

- C4.4a2b: child metadata only; substitution payload, FeatureParams, FeatureVariations, GDEF mark filtering index and overlapping structures are not fully validated; no real GSUB acceptance yet.

- C4.4a3a standalone single substitution only, bounded 16-bit domain scan for format 1; real-font reference, lookup scheduling, GDEF filtering, other substitution types and full shaping outstanding.

- C4.4a3b pending CI: real direct type 1 subtable fixtures only; no extension lookup execution, script/language scheduling, GDEF filtering, Arabic shaping or full shaping acceptance. Reference generated independently from raw font bytes, not an external shaping engine.

- C4.4a4a standalone multiple substitution; zero-length sequences rejected per spec, no real type 2 acceptance yet, no GDEF, cluster propagation or lookup scheduling; overlapping payload validation not comprehensive.

- C4.4a4b: real direct Type 2 reference uses one font family, no extension lookup, GDEF filtering, cluster propagation or Arabic shaping acceptance; independent reference from raw bytes, not external shaping engine.

- C4.4a5a standalone alternate substitution; candidate order preserved, zero-length sets rejected, no real type 3 acceptance yet, no GDEF, feature-value policy, cluster propagation or lookup scheduling; overlapping payload validation not comprehensive.

- C4.4a5b: real direct Type 3 reference uses one font family; no extension lookup, feature policy, GDEF filtering, cluster propagation or full shaping. New font not independently verified for rendering.

- C4.4a6a standalone contiguous ligature substitution requires >=2 input components; no real type 4 acceptance yet, no noncontiguous GDEF matching, cluster merging or lookup scheduling; overlapping payload validation not comprehensive.

- C4.4a6b: direct Type 4 contiguous matching only, no GDEF ignored-glyph handling, cluster merge, extension lookup or full shaping acceptance. Reference from independent raw byte extraction, not an external shaping engine.

- C4.4a7a only Type 5 format 1 matching, no real font reference yet, no class/coverage formats, chained matching, nested lookup execution/cycle checks, GDEF filtering or full shaping. Overlap validation not comprehensive.

- C4.4a7b: ClassDef and class context synthetic tests only; real font references, coverage format 3, chaining, GDEF and nested execution remain outstanding. Overlap validation not comprehensive.

- C4.4a7c: synthetic Type 5 format 3 tests only; real context references, chaining, nested execution, GDEF filtering, cluster propagation and full shaping remain outstanding. Payload overlap validation not comprehensive.

- C4.4a7d: selected real direct Type 5 tests cover all three formats; Noto Format 2/3 not yet checked, variable-font interpolation/rendering not tested; no external shaping comparison, nested execution, GDEF filtering or full shaping acceptance.


- C4.4a7e1: standalone GSUB Type 6 Format 1 glyph-based chaining. First covered input, nearest-first backtrack, forward lookahead, font-order first match including empty actions; input-relative checked action indices; cumulative rule/glyph/action limits charge shared offsets. Ten new synthetic tests (653/platform, 1959 expected); CI pending. No nested execution, GDEF filtering, class/coverage chaining, shaping or real Type 6 font acceptance. Caller supplies contiguous logical glyph stream and start index. No lookup flag interpretation or glyph skipping; parser permits trailing parent-table bytes and charges aliased rules. Real DOCX corpus, security/license review and reference compatibility still outstanding.

- C4.4a7e2: synthetic class chaining only; ClassDef record budgets are per table, rule budgets cumulative, no comprehensive payload overlap checks. No nested lookup execution, GDEF filtering, real Type 6 font reference, shaping, or GUI acceptance. Real DOCX corpus and security/license review remain outstanding.

- C4.4a7e3: standalone matching only, all three Type 6 formats now implemented in source; real direct font references pending. Empty coverage is unmatchable; empty action list can match. Payload overlap checks not comprehensive; no GDEF filtering, nested execution/cycle checks, cluster propagation, full shaping, or Word corpus acceptance.

- C4.4a7e4: two selected OFL fonts, direct Type 6 only, independent raw-byte reference rather than external shaping engine. Representative class matches are not exhaustive over all combinations; no Type 7 wrapper, nested execution, GDEF filtering, variable font interpolation, cluster merging or full Word fidelity. Real DOCX corpus and security/license audit still outstanding.

- C4.4a8a: extension target slices are suffixes, not independently validated extents; per-target budgets, not aggregate decoded multi-target memory budget. No real wrapper acceptance yet, no Type 8 decoder, lookup execution, GDEF filtering, nested context cycles or full shaping. Real DOCX corpus and dependency audit remain outstanding.

- C4.4a8b1: synthetic wrapper integration with real payloads only. Original native Type 7 fixture needed (C4.4a8b2). Larger Noto Sans/Serif retrieval incomplete; absence of native wrappers there was not established. No Type 8 parser or lookup execution, GDEF filtering, cycle protection, full shaping or Word fidelity. Corpus and dependency audit still outstanding.

- C4.4a9a: single contiguous Type 8 subtable pass only, no full lookup scheduling or GDEF filtering. Probe budget counts coverage queries, not individual binary-search comparisons. No native Type 8 font reference yet; native Type 7 reference also outstanding. Payload overlap validation not comprehensive; nested context execution, cluster propagation, Word fidelity and corpus/security/license gates outstanding.

- C4.4a8b2a: native Type 7 fixture found; all-wrapper metadata and eight representative dispatch tests pending CI. Full target decoding and independent content reference still outstanding; no native Type 8 samples in tested fonts. No claim of native Type 8 acceptance, full GSUB execution or Word fidelity. Corpus/security/license gates remain outstanding.

- C4.4a8b2b: full observed Nastaliq native extension target inventory tests pending CI, not exhaustive Cartesian context matching or external shaper equivalence. Native extension target types/formats absent from this font remain covered only by prior synthetic/direct target tests. Type 8 original font reference still needed. Full lookup scheduling, nested execution, GDEF filtering, cluster handling and Word fidelity outstanding.

- C4.4b1: directory and two ClassDefs only; MarkGlyphSetsDef, AttachList, LigCaretList and ItemVariationStore bodies remain undecoded. Child offsets are not validated extents; no lookup flag application or mark filtering yet. Native Type 8 sample, nested lookup execution, cluster handling, GPOS and full Word fidelity remain outstanding.

- C4.4b2: mark-set membership does not imply GDEF Mark classification; lookup flag precedence and actual filtering not implemented. AttachList, LigCaretList and ItemVariationStore remain opaque. Real GDEF reference not yet checked, native Type 8 fixture still needed; full GSUB execution, GPOS, cluster handling and Word fidelity outstanding.

- C4.4b2 CI #109 failed due to a legal empty Coverage format 2 mutation incorrectly classified as malformed. Repair 1 fixes the expectation, preserves all regression cases including a true unsupported-format rejection, and requires fresh three-platform acceptance. Other limitations unchanged.

- C4.4b3: selected real GDEF references cover three versions and ClassDef format 2 only (format 1 remains synthetically tested), no real Component class in these fixtures. ItemVariationStore offsets checked, bodies not decoded; no lookup flags, full shaping or external shaper equivalence yet. Native Type 8 and point-attachment fixtures remain outstanding.

- C4.4b4: eligibility/index projection only; not yet integrated into actual GSUB/GPOS match or edit operations. Missing GDEF classes stay unclassified; no Unicode-based inference. Invalid declared mark set always rejected, including when IgnoreMarks wins. No real flag reference tests yet. Nested lookups, scheduling, clusters and full shaping remain outstanding.

- C4.4b5: real lookup flag reference covers observed flags in seven fonts; absent combinations (including RightToLeft and simultaneous IgnoreMarks/UseMarkFilteringSet) retain synthetic checks. Reference is independent raw-data eligibility only, not external shaper equivalence or filtered substitution. Full scheduling, nested execution, clusters, GPOS and Word fidelity outstanding.

- C4.4c1: base plan only, no FeatureVariations evaluation, automatic script/language identification, script-specific staging or feature values/masks. Cross-index validation covers selected LangSys and selected feature lookup refs; not an entire public-layout validator. Lookup execution, filtering during matching, cycles, clusters, GPOS and Word fidelity outstanding. Real plan reference acceptance pending next stage.

- C4.4c2: original fixtures lack RequiredFeature and nonzero FeatureVariations (synthetic checks retained). Independent reference validates base plan only, not script-specific shaping stages, feature values/masks, external shaper equivalence or execution. Native Type 8 and real point-attachment samples still outstanding.

- C4.4c3a: suffix-byte budget is conservative work accounting, not exact allocated memory or certified subtable extent. GDEF must belong to same font (domain checked, identity not proven). Base plan provenance checked but not cryptographic source identity. Only selected lookup targets compiled, nested references not recursively resolved, no execution/cycle protection, no feature variation evaluation. Real compilation acceptance pending next stage.

- C4.4c3b: raw metadata and work charge independently checked; typed payload equality shares standalone production parsers, not an external semantic oracle. Full compilation CI pending; no native Type 8 in fixtures. Nested dependency closure, cycle protection and actual filtered lookup execution remain outstanding, as do complete shaping and Word fidelity.

- C4.4c4a: direct reference inventory is conservative (includes unmatchable rules); not a proof of runtime reachability. Caller supplies full LookupList count; no font identity proof or closure/cycle rejection. Real raw action census and runtime guards remain outstanding. No actual substitution, complete shaping or Word fidelity claim.

- C4.4c4b: reference covers stored actions, not dynamic reachability. No self references observed in selected fixtures, not a proof that mutual cycles are absent. Closure, static cycle diagnostics and runtime execution guards remain separate. No actual substitution or full shaping acceptance.

- C4.4c4c: static closure depends on caller-supplied complete direct inventory for each declared source, from the same font; missing reachable sources prevent proven-acyclic status. Witness is first deterministic DFS cycle, not all cycles or runtime reachability. DFS stack limit is not longest-path limit. Nested execution still requires independent dynamic depth/action/work guards. Real closure reference pending next stage.

- C4.4c4d: matrix reference checks static reachability only; seven fixtures have no static cycles (synthetic cycles retained). Partial source coverage remains explicitly incomplete. No runtime cycle guard or actual filtered execution yet. Native Type 8 and real point attachment fixtures remain outstanding.

- C4.4c5a: static program retains cycles, does not execute. Conservative suffix byte accounting, not memory measurement. Closure stack depth is not runtime recursion bound. Real program reference pending.

- C4.4c5b: reference tests use seven existing fixtures, representative root selections and at most one default and explicit language plan per font. Typed payload equality is not independent shaping semantics. Exact budgets tested for all seven; one-less aggregate limits checked on small fixtures. Static reachability is not runtime execution safety; native Type 8, real point attachment, actual filtered substitution, GPOS, corpus and security/license gates remain outstanding.

- C4.4d1a: single substitution only; input is a caller-provided logical glyph stream, no Unicode mapping, clusters, script-specific staging, masks, mixed-type roots or nested execution. Probe counts are subtable queries, not individual coverage comparisons; no independent real-font execution reference yet. No full shaping or Word compatibility claim.

- C4.4d1b: reference is independent raw-byte Type 1 semantics on seven selected fixtures, not external shaper equivalence. Explicit Type 1 root selection is not a full script/language shaping schedule. Real extension wrapper coverage depends on observed targets; synthetic wrapper regression retained. No Type 2 execution, nested actions, clusters, GPOS, corpus or security/license acceptance yet.

- C4.4d2a: no deletion (existing Type 2 parser rejects empty sequences), cluster propagation, contextual execution, feature masks, other root types or full shaping. Emission budget counts logical glyphs rather than allocator capacity or bytes; buffers may retain capacity between passes. Probe count is not individual coverage comparisons. Real Type 2 execution reference pending.

- C4.4d2b: four fixtures contain Type 2 targets and three serve as Type 1-only controls. Reference checks explicitly selected Type 1/2 roots, not full feature scheduling or external shaping equivalence. No deletion, cluster propagation, nested execution, Type 3 execution, GPOS or corpus/security acceptance yet.

- C4.4d3a: explicit uniform per-root zero-based candidate only; no per-glyph alternate choice, feature-value/mask inference, fallback to first candidate, script-specific scheduling, clusters or nested execution. An index is checked only on eligible covered glyphs; invalid selection errors return no output. Real Type 3 execution reference pending. Corpus and security/license gates outstanding.

- C4.4d3b: two existing real fixtures contain Type 3; five are controls. Tests cover explicit selected roots, not feature-value mapping or complete external shaper equivalence; direct/wrapped synthetic coverage retained. No ligature execution, cluster propagation, nested context, script staging, GPOS or corpus/security acceptance yet.

- C4.4d4a: Type 4 glyph-ID-only execution retains ignored glyphs but does not propagate ligature component associations, clusters or mark attachments, so no full shaping claim. Static GDEF eligibility per root, no feature masks or nested execution. Type 4 visits count physical input positions including consumed glyphs; rule attempts and component comparisons have separate caps. Real-font Type 4 reference pending. Corpus and security/license acceptance outstanding.

- C4.4d4b: seven fixture fonts, explicit selected root execution only. A stored rule may be shadowed by an earlier rule or blocked by filtering; rule-derived inputs do not prove every stored rule independently fires. No cluster or mark attachment propagation, nested context, script staging, GPOS, external shaping equivalence or corpus/security acceptance.

- C4.4d5a: deliberately length-preserving Type 1/5 subset; reachable Type 2/3/4/6/8 rejected even if never dynamically invoked. Nested context may inspect beyond parent match; parent action indices remain physical and length-stable. No masks, cluster propagation, variable-length nested edits, feature staging, full shaping or real-font contextual execution reference. Static cycles permitted, active repeated (lookup,position) fails before eligibility. Hard stack cap 32.

- C4.4d5b: native AlternateSet bytes exercised under constructed Type 5 parents without native GDEF flags; not independent native Type 5 schedule acceptance. Native context closure references remain next. Type 2/4/6/8 reachable targets rejected, no variable-length nested edits, clusters, feature-value masks or full shaping. Uniform per-lookup explicit choices, not per-glyph feature selection.

- C4.4d5c: native Type 5 inventory 6/85/1341 subtables in Padauk/Devanagari/Nastaliq; four negative inventory controls. Only closures entirely within Type 1/3/5 execute. Witness sampling is bounded to eight per subtable (one for Format 3), not exhaustive rule coverage; supported/action counts not yet measured. No external shaper equivalence, variable-length edits, cluster propagation, masks, full feature scheduling or corpus/security acceptance.

- C4.4d5d: nonempty Type 2 growth only; stable first-output anchor policy, no deletion or contraction. Frame tracking and suffix movement are bounded but Vec splicing is not a linear-time full-run shaping algorithm. No cluster/feature mask propagation, native variable-length reference, Type 4 nested contraction, Type 6/8 execution, external shaper equivalence or corpus/security acceptance.

- C4.4d5e: 87 native Type 2 subtables across four fonts, three negative controls; 1432 native Type 5 subtable inventory. Native Type 5 execution only for supported closures containing Type 2, bounded witness sampling; counts of actually executing native nested expansion not yet measured. Constructed parents use native target bytes but not native parent rules/flags. No contraction, clusters, full shaping, external engine equivalence, performance benchmark or corpus/security acceptance.

- C4.4d5f: explicit consumed-anchor skip policy, not a claim of full external shaper equivalence. Ancestor exclusive boundary tracks surviving original span, not necessarily all descendants' inspected spans. No ligature component or cluster metadata, masks, Type 6/8 execution or native contraction reference yet. Vec compaction and anchor scans bounded, no performance benchmark.

- C4.4d5g: 80 native Type 4 subtables across seven fonts. Constructed parents and Type 2 targets are synthetic, native Type 4 target bytes unchanged; native Type 5 execution limited to supported closures and bounded witnesses. Actual dynamic native nested contraction counts not measured yet. No external shaping equivalence or ligature cluster/component metadata; Type 6/8, feature masks, performance and corpus/security acceptance outstanding.

- C4.4d6a: length-preserving subset only, rejects reachable Type 2/4/8 even on empty input. Native Type 6 execution reference, variable-length integration, script staging, masks, clusters and full shaping equivalence outstanding. No external shaper comparison or performance benchmark.

- C4.4d6b: explicit opt-in variable-length Type 6 execution using existing consumed-anchor skip policy and ancestor input boundaries. Descendants can inspect/edit outside ancestor input, so this is not a claim of external shaper equivalence. Native Type 6 execution reference, Type 8, cluster/mark-component metadata, feature masks, full script stages, performance and corpus/security acceptance outstanding.

- C4.4d6c: native Type 6 inventory 9/9/35/52/103/218/1023 subtables (1449 total). Only closures within Type 1–6 execute; first candidate per Type 3, at most 64 individual witnesses per root, 16 concatenated witnesses, eight ordered roots. Static inventory is not actual matched-rule count; diagnostic execution counts not yet measured. No external shaper equivalence, Type 8 execution, clusters/masks or full shaping acceptance.

- C4.4d8a: reverse root and nested point invocation policy verified only by pending synthetic tests; native Type 8 reference and external shaper equivalence outstanding. Full lookup-type dispatch is not full shaping: no clusters, masks, script stages, feature variations or GPOS integration. No benchmark/corpus/security acceptance.

- C4.4d8b: zero native Type 8 in current seven-font corpus; positive Type 8 tests are synthetic, native positive fixture still required. Native mixed dispatch uses selected roots, not language shaping or exhaustive native rules. No external engine equivalence, cluster/mark metadata, masks, FeatureVariations, GPOS, benchmark or full corpus/security acceptance.

- C4.4d8c: previous native Type 8 absence addressed by original Noto Sans Mongolian fixture (2 native subtables, 3715 glyphs), pending positive execution CI. Only selected reverse lookup tested in that new font, not complete Mongolian shaping. All wrapper combinations are constructed test data, not native font schedules. No external equivalence, clusters, masks, FeatureVariations, GPOS, benchmark or corpus/security acceptance.

- C4.4d9: bounded explicit/base-plan GSUB execution candidate only. Native plan integration uses selected default-LangSys feature schedules, not full language shaping or all rules. No FeatureVariations evaluation, cluster metadata, masks, GPOS, external shaper parity or measured memory/runtime/security acceptance.

- #145 failure: test selection cap, not demonstrated substitution failure. Full Mongolian rclt plan repair pending CI; no gate waiver. Existing native Type 8 positive regression retained; full language shaping not claimed.

- C4.4e1: only Format 1 condition interpretation, strict malformed-known-table rejection; unsupported condition/version and out-of-domain axis skip per selection policy. Coordinates must already include normalization/avar remapping; fvar axis count is caller-supplied. Alternate feature parameters opaque. No variation-aware compile integration or native positive FeatureVariations fixture yet; no script stages, masks or cluster propagation.

- C4.4e2: normalized F2DOT14 coordinates and fvar axis count remain caller-supplied, no cryptographic font identity guarantee; selected Feature membership and effective lookup provenance revalidated. No native positive variation reference, fvar/avar normalization, feature parameter interpretation, full shaping, masks, clusters or GPOS yet.

- C4.4e3: native positive Archivo covers one Format 1 condition, one alternate Feature and seven Type 1 mappings, not all real variation table structures. Inputs are normalized F2DOT14; fvar/avar conversion and outline interpolation not implemented by this milestone. Full shaping, GPOS, clusters/masks, external parity and security/performance acceptance remain open.

- C4.4e4: strict fvar 1.0 subset, zero-axis rejected as unsupported, reserved fields and malformed ranges rejected; no full tag registry, name/STAT cross-validation or named-instance semantic deduplication. Input already Fixed 16.16. Pre-avar normalization only; normalize_f2dot14 must not be used as final coordinate when avar applies. No avar, gvar/HVAR/MVAR, outline interpolation, script staging or full shaping yet.

- C4.4e5: strict avar 1.0 parsing rejects malformed order/range/reserved/trailing bytes instead of optional record recovery. Missing essential identity anchors leave axis unchanged. Version 2 and cross-axis remapping unsupported. Equal axis counts do not prove same-face fvar/avar provenance; caller must supply associated tables. Intermediate rounding policy documented, not external engine equivalence. User-coordinate GSUB integration, gvar/HVAR/MVAR, full shaping and external parity outstanding.

- C4.4e6: same-face association is structural, not cryptographic identity or checksum certification. Compatible equivalent plans accepted, stale/forged selection rejected via full rebuild. fvar required; missing GSUB is an error in this specialized API. No automatic rvrn enabling, script stages, masks, clusters, avar 2.0, gvar/HVAR/MVAR, GPOS or external shaper equivalence. Native tests cover selected features, not complete language shaping.


CI #152, 3baf3be5ff3f891f5dab192d3a873dc9ba321d3b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45802058303 : 1091/platform, 3273 total, zero failed/ignored; exact HEAD and three Release builds verified from actual job logs.

C4.4e7: five additional integration regressions for raw named-instance coordinates across Archivo, Devanagari and Nastaliq, ordered/tagged/default coordinate equivalence, repeat execution and plan provenance. Existing independent raw design-space and glyph ledger reference tests retained unchanged. 1096/platform expected; CI pending. No production substitution changes or external shaper equivalence claim.


CI #153, 1cfd3f4d22737f26d82e04243739c1e9137b7f9e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45808724109 : 1096/platform, 3288 total, zero failed/ignored, exact SHA and Release builds verified from three job logs. C4.4e7 bounded variable GSUB scheduling accepted.

C4.4f1: font_source_run adds immutable logical scalar and nominal glyph provenance, UTF-8 byte lookup, checked scalar ranges, explicit missing glyph policy and bounded allocation. Six regressions; 1102/platform expected, CI pending. Scalar seeds are not extended grapheme clusters; no GSUB edit propagation, normalization or bidi yet. Next C4.4f2 bounded source-set union and expansion/contraction ledger.


CI #154, b17e2423e7679c8fe196c5ccace84b8655a5628f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45812104603 : 1102/platform, 3306 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f2: font_provenance adds a bounded explicit glyph edit ledger, sorted unique scalar source sets, expansion duplication, noncontiguous contraction preserving gaps, checked glyph domain, cumulative logical work and transactional limit failures. Nine regressions including independent BTreeSet subset reference; 1111/platform expected, CI pending. No deletion, grapheme segmentation or live GSUB integration. Next C4.4f3 integrate provenance with GSUB edit execution (begin with single/multiple substitution).


CI #155, 0cc58a89276964ca9e35056c6e51be825c66e92e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45815344242 : 1111/platform, 3333 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f3: optional provenance ledger inside existing GSUB scheduler, public execute_gsub_with_provenance for effective Type 1/2 and Type 7 wrappers only, preflight rejection of other reachable kinds, atomic failure and glyph output consistency check. Five regressions with native cmap seeds and synthetic lookup tables; 1116/platform expected, CI pending. No external engine parity or full cluster shaping. Next C4.4f4 integrate Type 3/4 provenance including noncontiguous ligature components.


CI #156, 6edc88d741591fcca892316154419873096afbb6, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45818748339 : 1116/platform, 3348 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f4: execute_gsub_with_direct_provenance adds explicit Type 3 choices and Type 4 exact component-set union in the existing scheduler, including Type 7 wrappers and ignored gaps. Original Type 1/2 API retains rejection behavior. Five new synthetic lookup regressions with native cmap seeds; 1121/platform expected, CI pending. No contextual/reverse provenance or full cluster semantics. Next C4.4f5 integrate nested Type 5/6 and reverse Type 8 provenance.


CI #157, cd0a0a9b188cdf64c8563663e92ed4201e1864be, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45822148460 : all three jobs failed at direct_alternate_explicit_choice with Invalid("gsub provenance divergence"); each log records 652 passed, 1 failed, 0 ignored before abort. Exact SHA verified. No successful full Release gate.

C4.4f4 repair: Type 3 branch omitted provenance update. Centralize Type 1/3 replacement through State::replace_one, preserving counters and atomic output semantics. Retain failed regression unchanged; add Type 3 -> Type 4 source/counter parity and exact provenance work limit regressions. 1123/platform expected, CI pending. Do not advance C4.4f5 until full verification.


CI #158, 9547f8e2d047df9a7c115d6f6dd521d59ad497a1, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45825597415 : 1123/platform, 3369 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4f4 repaired and accepted.

C4.4f5: full-program tracked API, Type 8 uses shared replace_one, nested Type 5/6 reuse live edit anchors and exact source propagation. Six regressions for growth, consumed anchors, chain conditions, reverse cascade and nested reverse, budgets; 1129/platform expected, CI pending. Restricted APIs retain preflight. No full grapheme clusters, script staging or native all-kind provenance acceptance. Next C4.4f6 comprehensive native provenance acceptance and API contract.


CI #159, fcbf2bf908ac894069ff7fc17c94cdc1167f47a5, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45829163360 : 1129/platform, 3387 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f6: VariableGsubFace::execute_with_provenance integrates existing dynamic scheduling with full tracked execution. Five native-font regressions for Archivo, Devanagari, Nastaliq default selection, Archivo rvrn scalar source oracle and source budget. Existing synthetic all-kind independent expected source sets retained. 1134/platform expected; CI pending. Native tests do not establish positive native coverage of every lookup kind or external shaper parity. Next C4.4g1 explicit per-glyph feature-mask foundation.


CI #160, de26d8cd614985ce3828f9a92521a047ee399d7b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45832722223 : 1134/platform, 3402 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4f6 accepted.

C4.4g1: bounded dense binary scalar feature masks (64 selected IDs), ordered range overrides, required-feature protection, lookup contribution bitsets, conservative intersection projection through exact provenance, resource caps. Six regressions including exhaustive small reference; 1140/platform expected, CI pending. No scheduler gating or per-feature alternate values yet. Next C4.4g2 integrate validated root lookup gating and live masks.


CI #161, fb271073de1f4b8c03a0c8405b8c91ef09877346, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45836441328 : 1140/platform, 3420 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4g2: retain validated static/dynamic plan in compiled program; masked root-anchor gating uses OR of contributing feature bits and live intersection of exact scalar provenance. Separate cumulative mask work including final projection, legacy execution unchanged, unvalidated explicit-root programs rejected. Three native regression tests; 1143/platform expected, CI pending. Root anchor only: contextual descendants and additional ligature components not independently gated; next C4.4g3 component and contextual mask policy, synthetic positive gating/edits. No claim of full shaping.


CI #162, c13b1f4c1bea9bfed4522d39792878472d1c8ea9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45840611187 : 1143/platform, 3429 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4g3: separate strict mask API, common feature intersection across matched input/ligature components, inherited feature bits and live action-anchor checks, mask-disabled components are barriers not ignored gaps; backtrack/lookahead remain conditions. Legacy root-only API preserved. Three synthetic positive/negative regressions with native cmap seeds, 1146/platform expected, CI pending. Explicit conservative policy, not external shaper parity. Next C4.4g4 strict native and independent reference acceptance.


CI #163, 7dc069e500b0c2294290db99c7c50db81ab928c0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45843654359 : all three jobs failed compilation E0061 at font_gsub_context_execute.rs:551, forward apply_at missing masking and bits; exact SHA verified, no tests executed.

C4.4g3 repair: pass mask state and root bits to forward scheduler call, audit all apply_at callers, ensure all six context candidate formats reject zero common bits before selecting a match (including coverage formats), preserve existing regressions unchanged. 1146/platform expected; CI pending. Do not advance C4.4g4 until full gate.


CI #164, d022359b89d5b05127f4814fe621438c988e29e3, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45847003383 : 1146/platform, 3438 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4g3 repaired and accepted.

C4.4g4: same-face variable strict mask wrapper, independent exhaustive 256 two-feature ligature cases and 16 reverse mask cases, native Archivo variable-coordinate parity and disabled-feature acceptance, exact runtime budget. Four regressions; 1150/platform expected, CI pending. Conservative policy only, not external shaper equivalence. Next C4.4h1 explicit bounded script-stage scheduling foundation.


CI #165, e13eff3fa51552abe7b865eacc2f187e8bbcd28d, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45850640589 : 1150/platform, 3450 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4g4 accepted.

C4.4h1: bounded explicit stage schedule from validated compiled plan, complete unique feature partition including required feature, caller-ordered stages, ascending roots per stage, shared roots deduplicated within stage and repeated across stages with exact contributor sets, separate cumulative caps. Five regressions with synthetic validated plan and independent reference; 1155/platform expected, CI pending. No execution, automatic script ordering or reordering yet. Next C4.4h2 staged execution on persistent provenance and masks.


CI #166, e0aa39366eedfea8884d5360efbdc645b39524dc, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45854217323 : 1155/platform, 3465 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h2: execute_staged_gsub builds validated partition and executes stage roots in order in one existing scheduler state, persistent glyph/source buffers, strict stage contributor masks, cumulative execution/source/mask budgets, one explicit alternate per reachable lookup. Three regressions for order, growth/contraction, shared masks, single-stage parity and cumulative limits; 1158/platform expected, CI pending. No automatic script ordering, reordering or per-stage callbacks. Next C4.4h3 native staged acceptance and explicit script feature ordering policy.


CI #167, c5912c746eb503d692b7354878322edbab70b875, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45857691944 : 1158/platform, 3474 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h3: explicit ordered feature-tag groups, duplicate rejection, absent tag omission, unlisted selected feature tail, same-face variable tag-stage wrapper. Four regressions for mapping/order/bounds and native Archivo parity/disabled multi-group execution; 1162/platform expected, CI pending. Tag mapping requires same FeatureList; variable wrapper enforces it. No automatic script-specific rules yet; native disabled tests do not prove positive multi-stage substitutions. Next C4.4h4 explicit script policy profiles and positive native stage acceptance.


CI #168, 7ef98a449d337c1a1819e6435e23f8896e496984, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45861096884 : 1162/platform, 3486 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h4: explicit experimental Latin, Arabic and IndicV2 feature ordering profiles, unsupported script rejection, no implicit enabling; tests for profile selection, unique tags, partition and positive native Archivo scalar substitution under multi-stage scheduling. Three regressions, 1165/platform expected, CI pending. Native test requires actual substitution and compares existing single-pass engine; does not prove two stages both mutate or independent external shaper parity. Arabic/Indic profiles require caller masks and preprocessing, not complete script shapers. Next C4.4h5 profile integration and positive multi-stage evidence before seal.


CI #169, 72bd70f901432c11f395e9355cd389abb17aa44a, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45864396745 : 1165/platform, 3495 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h5: traced staged execution with bounded per-root change counts aggregated by stage, same-face script profile wrapper rejecting DFLT fallback, synthetic positive two-stage growth/contraction evidence, no-op/disabled diagnostics and native profile parity. Three regressions, 1168/platform expected, CI pending. Native profile trace does not require two positive native stages; full external shaper parity, joining/syllable/reorder rules remain outstanding. Next C4.4h6 native two-positive-stage evidence and contract gate.


CI #170, 3ddf448753e932444669d301899515c95568a448, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45867643339 : 1168/platform, 3504 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h6: native Archivo maximum-weight BMP cmap acceptance, two nonzero stage change counters required (rvrn then case), exact scalar provenance, deterministic repeat. One regression; 1169/platform expected, CI pending. Native positive two-stage gate may expose unreachable cmap coverage; do not weaken assertion if it fails. Seal only after CI. Next C4.5a bounded GPOS header/directory foundation, complex script preprocessing remains outstanding.


CI #171, 93eb665e7e92059227ba28d0973a69067f6d3d15, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45870727460 : 1169/platform, 3507 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4h6 native two-positive-stage gate passed.

C4.5a: shared bounded OpenType Layout directory and child decoder, separate GPOS entry points with kind domain 1..9, GSUB retains 1..8, script/language/feature references, lookup flags and subtable offsets, GPOS 1.1 variation offset. Five regressions including native raw lookup header comparison, 1174/platform expected, CI pending. Shared structural aliases and historical error prefix retained. No positioning subtable decoding or execution yet. Next C4.5b1 ValueRecord and Device/VariationIndex foundation.


CI #172, 98d74fce846a478c36ea00c315fcebe97d7852e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45873638947 : 1174/platform, 3522 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5b1: ValueRecord 8 defined format bits, signed placement/advance, parent-relative nullable Device offsets with caller-provided inline lower bound, packed 2/4/8-bit signed pixel deltas and opaque VariationIndex, cumulative per-record Device entry budget. Six regressions including all 256 formats, exhaustive signed values, truncation and budgets; 1180/platform expected, CI pending. No ItemVariationStore evaluation or positioning application. Next C4.5b2 Anchor and shared adjustment primitives.


CI #173, 2b58e61999d973b0b730e93a99fefca1ffefc3d6, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45876487806 : 1180/platform, 3540 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5b2: Anchor formats 1–3, signed nominal coordinates, contour-point index, relative nullable Device offsets with shared cumulative entries cap, checked four-field design adjustment addition and separate anisotropic pixel deltas. Five regressions; 1185/platform expected, CI pending. Unresolved contour and variation indices rejected in pixel resolution; no contour evaluation or ItemVariationStore yet. Next C4.5c1 SinglePos formats 1/2 parsing and application.


CI #174, b015a73591e4da545ef931fd0bc0d28896dadff0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45879217950 : 1185/platform, 3555 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5c1: standalone SinglePos formats 1/2, coverage index lookup, shared/individual value records, checked design/pixel adjustment, cumulative device entry cap across records. Five regressions, 1190/platform expected, CI pending. Nominal design-only path explicitly ignores devices; combined path rejects unresolved variation indices. No lookup flags or script scheduling yet. Next C4.5c2 independent native SinglePos reference acceptance.


CI #175, 1dbbedd4caa3a74043a1f66d1f4b16dc295ebd89, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45881899044 : 1190/platform, 3570 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5c2: independent native raw SinglePos oracle over eight retained font fixtures, direct and extension-contained targets, dense coverage reference, signed values, per-bit Device extraction, nominal and anisotropic pixel adjustment, unresolved variation rejection. One regression, 1191/platform expected, CI pending. Require positive native format 1 and 2 and nonzero nominal adjustment; fixture inventory not locally verified. No full lookup execution or external shaper parity. Next C4.5d1 PairPos format 1 glyph-pair decoding and adjustment.


CI #176, 8e3518cf12026185d0d032698da68c7c8ddaadfa, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45884649416 : 1191/platform, 3573 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d1: standalone PairPos format 1, coverage-indexed PairSets, ordered second-glyph binary search, two ValueRecords, cumulative pair/device caps, checked atomic design/pixel adjustments, zero-format matching. Five regressions, 1196/platform expected, CI pending. Device offsets are PairSet-relative; no lookup scheduling or class-based PairPos yet. Next C4.5d2 PairPos format 2 class matrix.


CI #177, 83d81985278920ed4669cdba557bef733868032f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45887358589 : 1196/platform, 3588 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d2: PairPos format 2 dense class matrix, both ClassDef formats, class zero fallback, first glyph coverage and second glyph domain, checked atomic dual adjustment, subtable-relative Device and cumulative cell/device limits including zero-stride matrices. Five regressions, 1201/platform expected, CI pending. Next C4.5d3 independent native PairPos format 1/2 acceptance. No lookup scheduling or variation resolution yet.


CI #178, eaa2a8d23a3789d00e408cef91614ffa4b29cde4, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45890110177 : 1201/platform, 3603 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d3: independent native PairPos oracle over eight retained font fixtures, direct and extension targets, dense explicit-pair reference and class maps, all covered first glyphs and second glyph class mappings, reachable matrix cells, dual design/pixel adjustment, strict positive gates for both formats. One regression, 1202/platform expected, CI pending. No external shaper parity or full lookup execution. Next C4.5e1 CursivePos entry/exit anchor parsing and nominal connection math.


CI #179, 1a82af003aeb9e7c095d7122c8bfc9c0f708923d, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45892747565 : 1202/platform, 3606 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5e1: CursivePos format 1 nullable entry/exit anchors, coverage mapping, cumulative Device budget, checked two-origin connection geometry with explicitly fixed entry or exit, separate nominal design and pixel corrections. Five regressions, 1207/platform expected, CI pending. This is standalone geometry, not full OpenType cursive advance redistribution, RTL lookup behavior or attachment chain execution. Next C4.5e2 native independent cursive reference acceptance.


CI #180, d88e99e2bd5561a2f40a67cc54d915bbe9a79049, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45895332135 : 1207/platform, 3621 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5e2: independent native CursivePos oracle over five retained font fixtures, raw coverage and anchor format decoding, per-bit Device reference, covered-pair cross-product, fixed entry/exit geometry, pixel-space correction or explicit unresolved error. Strict positive native subtable, anchor and unequal-coordinate pair gates. One regression, 1208/platform expected, CI pending. No full cursive advance redistribution, RTL attachment chain or external shaper parity. Next C4.5f1 MarkToBase parsing and nominal attachment geometry.


CI #181, bf3dc8f0cd77ad4123fa9e6acf43f291d4803723, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45897928513 : 1208/platform, 3624 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5f1: MarkToBase format 1, MarkArray required anchors, BaseArray nullable class matrix, independent coverage, class and cell caps, cumulative device budget, checked nominal and separate pixel attachment geometry with base fixed. Five regressions, 1213/platform expected, CI pending. No automatic base selection, attachment chain or outline/variation resolution. Next C4.5f2 native independent MarkToBase reference acceptance.


CI #182, 883969021ba4fc485be6edb8014cecda6a61d8e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45900444622 : 1213/platform, 3639 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5f2: independent raw-byte native MarkToBase oracle across nine retained font fixtures, including extension lookup traversal, coverage formats, mark class and anchor records, nullable base matrix cells, all covered pairs, independent world-coordinate equality and per-bit Device pixel checks or explicit unresolved errors. Strict positive gates for subtables, marks, base anchors, matched pairs, unequal coordinates and resolved pixel checks. One regression, 1214/platform expected, CI pending. No automatic base selection, attachment chain, external shaper parity or outline/variation resolution. Next C4.5g1 MarkToLigature format 1 component attachment foundation.


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
