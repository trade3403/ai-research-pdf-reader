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
