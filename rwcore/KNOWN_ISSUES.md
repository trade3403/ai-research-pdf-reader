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
