# RWCore architecture / algorithm index

Source paths below are relative to rwcore/core/src. Read actual interfaces at HEAD before modifications. Detailed phase history is in README.md.

| Module | Responsibility |
|---|---|
| zip_guard.rs / package.rs / opc.rs | Bounded archive preflight, OPC relationship graph, XML and DOCX readers |
| model.rs / formatting.rs | Native blocks, paragraphs, runs, direct/effective styles, ordered UTF-8 image anchors |
| numbering.rs | List definition and counter engine |
| section.rs / geometry.rs | Section ranges and checked twip page geometry |
| stories.rs / fields.rs / page_numbers.rs / page_structure.rs | Header/footer stories, fields, page numbering and supplied page-count geometry |
| table.rs / table_grid.rs | Native recursive table parsing and sparse merge topology |
| table_properties.rs / table_effective.rs / table_borders.rs | Appearance, margin inheritance, border and three-line recognition |
| table_geometry.rs / table_rows.rs / table_columns.rs / table_layout.rs | Integer rectangles, measured row constraints, bounded column allocation, recursive layout orchestration |
| media.rs / image_header.rs | Offline scoped media links, unique raw binary storage and six raster header dimensions |
| drawing_inline.rs | Inline picture metadata, EMU conversion and scoped media binding |

Run.text is plain text; Run.pictures holds sorted InlinePictureAt anchors at UTF-8 byte boundaries; Run::inline_content() yields borrowed text/picture items. Text edits must maintain offsets. Body, nested body tables and top-level stories share picture parser. MediaCatalog is opt-in, bind using actual owning OPC part. Floating metadata should reuse picture metadata logic without interpreting a floating node as inline.

C2.4d-1 (verified CI #55): `drawing_anchor.rs` adds independent floating picture metadata parsing, typed relative reference and position choice, anchor/wrap distances, layer/overlap/locking flags and simple position. Shared picture payload parser does not accept anchors through the inline API. Floating metadata is not inserted into Run or rendered; polygon/effect geometry and placement were outside that batch.

C2.4d-2 (verified CI #56): `drawing_layout.rs` provides signed half-open EMU frames, exact twip conversion, explicit relative references, alignment and simple-position override, per-edge exclusion distances, and single-anchor rectangular line-band subtraction. Tight/through requires explicit approximation opt-in. No implicit cell/page placement, multi-anchor collision or painting.

C2.4e (verified CI #57): `picture_geometry.rs` parses standalone pic:pic srcRect and xfrm, validates coordinate bounds, computes outward source pixel windows and reversible normalized-source-to-EMU affine mapping (local flips before center rotation), exact cardinal directions and four-corner bounds. Independent of inline/floating metadata and layout; caller must reconcile wp:extent, shape frames and actual source dimensions.

C2.4f (verified CI #58): `drawing_integration.rs` attaches a source-order drawing index to Run, reusing inline payload storage and owning separate floating nodes. Geometry parsed on existing XML nodes with Absent/Parsed/Unavailable states. read_docx_with_media loads referenced stories and catalog in two passes; pictures(max) traverses nested body tables and loaded stories with actual source-part scope. Occurrences bind, inspect embedded headers/crop, and expose explicit placement and restricted geometry mapping (matching extents and zero shape offset only). Old inline_content view excludes floating pictures. C3 must preserve both inline and combined indices across text edits.

C3.1 (verified CI #59): `edit.rs` supplies typed body/nested table run paths, UTF-8/scalar/UTF-16 boundary conversion, bounded sequential text replacements with expected-text checks, touched-run staging and atomic commit. Inline and mixed floating indices are updated together. Insertion affinity and interior anchor handling are explicit, no implicit image deletion; structure unchanged. No history, cross-run ranges or story editing yet.

C3.2 (verified CI #60): `selection.rs` supplies borrowed paragraph prefix counts for UTF-8/scalar/UTF-16, run-local/global conversion with explicit shared-boundary affinity, directional normalized selections, selected text and cross-run replacement compiled to C3.1 atomic edits. Replacement inherits the start run, retains other run structure; strict interior drawing checks include empty/shared-boundary runs, preservation is local to each run. No cross-paragraph or structural edit yet.

C3.3a (verified CI #61): `history.rs` owns the document in a read-only-facing DocumentEditor, shares staged C3.1 transaction and C3.2 selection planning, stores only changed-run before/after text and two offset arrays, checks revision and current snapshot before restore. Retention is bounded by entries, logical text bytes, offset counts and run-change counts. Undo/redo transfer entries, new successful changes discard redo and evict oldest undo; failures and net noops retain history. No persistent or structural history.

C3.3b (verified CI #62): `structure.rs` stages one contiguous paragraph-only sibling range, splits runs at validated Unicode boundaries with explicit equal-offset drawing side and right-side inline index rebasing, joins adjacent paragraphs with explicit formatting policy without merging runs, checks bounded logical budgets and section coverage, shifts enclosing and subsequent section ranges on body splice. No cross-section/cell joins. Standalone transaction; structural paths must be refreshed. C3.3c integrates structural undo history.

C3.3c (verified CI #63): shared stage/commit functions in structure.rs; DocumentEditor::structure records bounded before/after paragraph slice and changed section snapshots in the existing unified LIFO history. Text history remains offsets/text only. Exact structural slice/section comparison before restore; revision-checked run_at/paragraph_at and mutation calls prevent old revision path use. No stable IDs or automatic remapping. History run_changes budget includes structural paragraph/run/section slots; no full-document copy.

C4.1 (verified CI #64): font.rs parses raw big-endian SFNT and TTC v1/v2 directories with checked range/count budgets. Borrowed face/table slices and independently sorted tags; advisory search-field diagnostic; exact shared table region allowed with matching tag/checksum, other overlaps rejected. Optional per-table checksum treats head adjustment as zero, TTC DSIG is raw only. No glyph parsing or rendering.

C4.2a (verified CI #65): font_metrics.rs parses head/maxp/hhea/hmtx on a borrowed FontFace, with strict lengths, flavor/version checks, signed metric fields and glyph count bounds. hmtx is zero-copy, trailing bearings use the final long advance. FontScale converts signed design units directly to integer twips or 26.6 pixels using checked i128 rational arithmetic and ties-away rounding. Header extrema/maxp claims are not independently verified against outlines. Default horizontal instance only.

C4.2b (verified CI #66): font_cmap.rs selects a supported Unicode cmap 12 over 4, Windows then Unicode platform records. Shares strict maxp parser; validates all selected format 4 segment glyphs with bounded work, relative idRangeOffset and modulo delta arithmetic, format 12 intervals with checked final glyph ID, binary-search lookup with missing glyph 0 as None. No fallback across subtables. Reference: https://learn.microsoft.com/en-us/typography/opentype/spec/cmap .

C4.2c (verified CI #67): font_measure.rs combines cmap and horizontal metrics from the same FontFace. Nominal scalar mapping retains UTF-8 byte ranges, glyph metrics and missing flags, validates byte/scalar budgets, rejects controls and line separators, and accumulates design units before rounding cumulative endpoints to twips or 26.6 pixels. Explicit Reject/Notdef policy, signed raw hhea line metrics. This is logical-order nominal measurement, not a shaping or line-layout engine.

C4.3a (verified CI #68): font_outline.rs shares validated head/maxp parsers, validates the complete short/long loca index and borrows glyf slices. Lazy simple glyph decoding checks ordered contour endpoints, repeated flags, signed coordinate deltas and resource budgets. Exact half-design-unit paths reconstruct implied on-curve midpoints, including all-off-curve contours. Hint instructions inert; declared bounds advisory, point bounds not true curve extrema. Composite detected only.

C4.3b (verified CI #69): font_composite.rs decodes component flags, signed XY or unsigned real-point attachment, F2Dot14 matrix and optional inert instructions. Recursively assembles real points and contour endpoints, fixed 16.16 per-transform rounding, unscaled default offset, separate visit/component/leaf-geometry budgets and hard depth cap. Metric-source ID is metadata, not transformed metric calculation. Reference: https://learn.microsoft.com/en-us/typography/opentype/spec/glyf .

C4.3c (verified CI #70): font_path.rs validates assembled point range and complete contour coverage, emits unified closed quadratic paths with 17 fractional bits (exact midpoints from 16.16 points), computes independent axis extrema by i128 rational arithmetic, outward rounding by less than one path unit, and separate control bounds. Empty/degenerate contours explicit; metric-source ID and overlap retained. No fill, hinting or rasterization.

C4.3d (verified CI #71): font_flatten.rs chooses minimal sufficient dyadic subdivision count per quadratic based on componentwise second-difference bound, including half-unit rounding allowance for intermediate vertices. Evaluates vertices using exact i128 polynomial numerators and ties-away rounding, preserves implicit closed contour edges, signed polygon area/orientation and vertex bounds. Depth capped at 24, total vertices and contours bounded; failure if tolerance cannot be met. No device transform, fill or rasterization.

C4.3e (verified CI #72): font_raster.rs transforms flattened PATH_ONE vertices to signed 16.16 y-down coordinates using caller ppem and baseline in 26.6 units. Per subsample scanline, collect half-open nonhorizontal edge intersections, sort exact rational x positions with i128 cross-products, apply equal crossings before sample classification, accumulate nonzero/even-odd coverage at regular centers, and round to row-major 8-bit alpha. Caller viewport clips output without dropping crossings to its left. Input edges, dimensions, pixels, samples and edge scans are bounded. Reference rasterizer, not an optimized active-edge engine or analytic area calculation.

C4.3f (verified CI #73): font_render.rs builds a single borrowed TrueTypeRenderer from one face, combines cmap/metrics/outline assembly/path/flatten/raster, exposes scalar and glyph ID entrypoints, Reject/Notdef, outward rational pixel viewport from original path bounds and 26.6 baseline placement. Pixel-space componentwise tolerance includes half-device-unit vertex rounding and floors to positive path-grid tolerance; impossible sub-grid budgets fail. Original hmtx glyph advance and bearing are scaled directly; composite metric-source metadata retained separately. Empty glyphs retain advance with a zero-size bitmap.

C4.3g (verified CI #74): font_text.rs renders logical Unicode scalars through the same TrueTypeRenderer, retains UTF-8 ranges, glyph metrics and 26.6 placements, scales cumulative original hmtx units before rounding, unions nonempty bitmap viewports and composites straight alpha in source order. Empty glyphs retain advance. Bounded bytes/scalars, aggregate retained glyph pixels and sample counts, output dimensions and pixels, per-glyph stage limits. No exact outline-union coverage or shaping.

C4.3h-1 (verified CI #75): font_compare.rs compares caller-supplied straight-alpha bitmaps in absolute coordinates, zero outside each viewport. Bounds input lengths, union rectangle area and integer accumulators; reports absolute/squared/max error and ink sums. No reference generator or real font corpus yet.

C4.3h-2a verified CI #76: real_font.rs exercises unmodified ABeeZee Regular OFL font with separately extracted cmap4/hmtx values, directory checksums, simple/composite smoke and translation invariance. No independent reference raster yet.

C4.3h-2b verified CI #77: real_font_reference.rs uses fixed 128 subdivisions per quadratic and independent per-sample winding via cross products, 8x8 sampling and expanded viewport. Test-only oracle shares parsed paths, not production flattening or scanline code. Explicit conservative alpha error gates.

C4.3h-2c pending CI: independent offline JS raw glyf point/endpoint and nominal metric fixtures for ABeeZee Italic; production Rust comparison tests and smoke rendering.

C4.3h-2d1 pending CI: independent JS raw SFNT/loca/glyf decoder recursively expands translation-only components and produces full ordered point/endpoint reference fixtures for Regular glyph IDs 6 and 81. Rust tests compare exact 16.16 coordinates.

C4.3h-2d2 pending CI: Abel Regular second-family fixture, independently extracted nominal metrics, parameterized test-only reference raster units per em, four new grayscale comparisons. Original ABeeZee six cases retained.

C4.3h-2e1 pending CI: independent JS raw SFNT decoder derives exact ordered 16.16 point and endpoint reference for four Aref Ruqaa transformed composite glyphs. Uniform and anisotropic scales, reflections and signed offsets; no runtime dependency.

C4.3h-2e2a pending CI: four independent hand-calculated synthetic composite boundary regressions, general off-diagonal matrix with scaled offset and point attachment, nested half-LSB rounding, byte-index attachment after reflection. No runtime changes.

C4.4a1 pending CI: font_coverage.rs bounded Coverage format 1 glyph list and format 2 ranges, strict ordering and contiguous index validation, binary search, exclusive glyph count and expansion bounds. No substitution execution.

C4.4a2a pending CI: font_gsub.rs bounded GSUB 1.0/1.1 top-level directories, absolute child offsets, cumulative record and byte budgets; no child body, variation execution or substitution validation.

C4.4a2b pending CI: GSUB child headers, default and tagged LangSys, required feature and lookup index validation, lookup flags and optional mark filtering set metadata, bounded cumulative records. Subtable bodies opaque.

C4.4a3a pending CI: standalone type 1 format 1 modulo-65536 delta and format 2 explicit replacements linked to validated Coverage; all outputs checked against face glyph count, bounded input and expanded coverage. No lookup scheduling.

C4.4a3b pending CI: independent offline JS raw SFNT GSUB lookup directory and Coverage decoding produced all direct type 1 mappings for ABeeZee Regular/Italic and Aref Ruqaa Regular. Test integration traverses production GSUB layout and validates each subtable mapping over entire face glyph domain, including misses. Abel fixture has no GSUB. No execution scheduling.

C4.4a4a pending CI: MultipleSubst format 1 coverage-indexed nonempty sequences, validated output IDs, cumulative expanded glyph budget including shared offsets; single-position atomic bounded expansion. No scheduling or recursive substitution.

C4.4a4b pending CI: independently extracted direct Type 2 reference from Aref Ruqaa original SFNT bytes, all six subtables, full face glyph-domain comparison and atomic expansion budget tests. ABeeZee direct Type 2 absence verified in tests. No scheduling.

C4.4a5a pending CI: AlternateSubst format 1 coverage-indexed nonempty ordered candidate sets, explicit zero-based selection, validated output IDs, cumulative candidate budget including shared offsets, atomic in-place replacement. No feature-value selection policy or lookup scheduling.

C4.4a5b pending CI: original OFL Alegreya Sans Regular fixture; independently decoded direct GSUB Type 3 mapping, candidate-order and atomic explicit selection across all referenced glyphs. No scheduling or shaping execution.

C4.4a6a pending CI: LigatureSubst format 1 coverage-indexed ordered sets, nested relative offsets, bounded rule/component budgets, first matching contiguous rule in font order, atomic contraction. No GDEF filtering or cluster merging.

C4.4a6b pending CI: independent raw SFNT GSUB Type 4 reference across ABeeZee Regular/Italic, Aref Ruqaa Regular, Alegreya Sans Regular (18 direct subtables, 142 ordered rules); full face coverage, exact rule order and prefix reference, contiguous contraction. No external shaping engine or lookup scheduling.

C4.4a7a pending CI: GSUB Type 5 format 1 glyph-based context parser, nullable sets, ordered contiguous prefix matches, including empty action list; action sequence/lookup indices validated, cumulative rule/input/action budgets including repeated offsets. No nested execution or GDEF filtering.

C4.4a7b pending CI: shared bounded ClassDef formats 1/2, implicit zero within face domain; class context type 5 format 2 uses first-glyph Coverage gate, class-indexed rule sets and shared bounded parser with format 1. No nested execution.

C4.4a7c pending CI: GSUB Type 5 format 3 per-position Coverage matching with ordered lookup actions; validates header/offsets, action indices, cumulative records and expanded coverage glyphs, charging shared offsets per reference. All three Type 5 formats have standalone matching source; no nested execution.

C4.4a7d pending CI: original OFL Padauk and Noto Sans Devanagari fixtures; independent raw SFNT extraction of all Padauk direct Type 5 subtables and Noto Format 1. Exact ordered actions/rules, glyph-domain coverage gates, representative class sequence and coverage position matching. No nested execution.


C4.4a7e1: standalone GSUB Type 6 Format 1 glyph-based chaining. First covered input, nearest-first backtrack, forward lookahead, font-order first match including empty actions; input-relative checked action indices; cumulative rule/glyph/action limits charge shared offsets. Ten new synthetic tests (653/platform, 1959 expected); CI pending. No nested execution, GDEF filtering, class/coverage chaining, shaping or real Type 6 font acceptance. Module: font_chain_context_subst.rs. ContextLimits.max_input is the full three-segment span, max_total_input counts implicit first input as well.

C4.4a7e2 pending CI: font_chain_context_subst.rs adds Type 6 Format 2 independent backtrack/input/lookahead ClassDefs and class-indexed nullable sets, Coverage gate, class zero within face domain, contiguous ordered match. Shared bounded rule parser retains Format 1 API. Input class IDs bounded by set count; other class IDs are uint16 and absent classes never match. Ten synthetic tests pending CI.

C4.4a7e3 pending CI: font_chain_context_subst.rs adds CoverageChainContextSubstitution and standalone Type 6 Format 3 parser. Nearest-first backtrack, forward input/lookahead, ordered input-relative actions; all offsets relative to subtable and after complete header. Position, cumulative coverage record and expanded glyph budgets including aliases. Formats 1/2 APIs retained. Ten synthetic tests pending CI.

C4.4a7e4 pending CI: real_chain_context_subst.rs independently extracts SFNT table offsets, lookup/subtable offsets, expanded Coverage, dense ClassDef and ordered rules/actions for all 321 direct Type 6 subtables in existing Padauk and Noto Sans Devanagari fixtures. Tests compare exact rule data, reachable representative matches, truncation and context mutations; no production dependencies or core API changes.

C4.4a8a pending CI: font_extension_subst.rs parses GSUB Type 7 Format 1 with checked unsigned 32-bit relative offsets, rejects recursive/invalid target kinds, resolves borrowed target suffixes and same-kind wrappers in lookup order. ExtensionTarget dispatches existing Type 1-6 parsers with individual limits. Type 8 metadata supported but decoding not implemented. Twelve synthetic tests pending CI; no external dependency.

C4.4a8b1 pending CI: real_extension_targets.rs independently extracts original direct targets and wraps each selected target at offsets 8 and 70000; exact typed parser output equals direct parser output. Census of seven existing original fonts found no native Type 7; the wrappers are synthetic and original native Type 7 acceptance is not claimed. Three tests, no production changes.

C4.4a9a pending CI: font_reverse_subst.rs implements Type 8 Format 1 input-coverage-indexed substitutions with nearest-first backtrack and forward lookahead, cumulative position/record/expanded-glyph/replace budgets and alias charging. replace_reverse prevalidates run length, glyph domain and coverage probe upper bound, then visits each position once right-to-left on the updated stream. Type 7 dispatch supports Reverse variant and separate limits. Twelve synthetic tests pending CI; no external dependency.

C4.4a8b2a pending CI: unmodified OFL Noto Nastaliq Urdu fixture, independent SFNT and GSUB lookup offset census; all 1503 original Type 7 wrapper offsets/types checked including max 207128, eight representative target parses compared with standalone parsers. Two tests, no production changes. Full target-content reference acceptance deferred.

C4.4a8b2b pending CI: all native Nastaliq extension target decodes (97 simple, 1406 contextual) compared to direct parser and independent raw-byte substitution maps, ordered rule sets, expanded coverage membership/index and actions. Representative class-chain and coverage-context matches checked; no production dependency or API change.

C4.4b1 pending CI: font_gdef.rs validates GDEF 1.0/1.2/1.3 directory, nullable relative offsets and required entry sizes, then decodes two independent ClassDefs using shared parser, with cumulative record and explicit glyph budgets. Typed glyph class and separate mark-attachment query; missing tables default to class zero, out-of-face returns None. Other child bodies remain opaque; twelve synthetic tests, no external dependency.

C4.4b2 pending CI: font_gdef_mark_sets.rs decodes Format 1 with 32-bit child offsets and shared Coverage parser. MarkGlyphSets provides ordered coverage and domain-checked membership queries. GDEF parsing now decodes present mark sets; parse_gdef_with_mark_set_limits allows separate explicit budgets without changing existing GdefLimits struct. Alias references charge cumulative budgets. Twelve new tests, no external dependency.

C4.4b2 repair 1: empty Coverage format 2 (zero range records) remains legal, consistent with shared Coverage format 1 empty-table behavior. Test now explicitly checks empty format 2 membership and rejects unsupported format 3. No production code changes.

C4.4b3 pending CI: real_gdef.rs independently decodes SFNT directory, GDEF offsets, dense ClassDef assignments and expanded coverage lists for seven existing real fonts, validates all 7006 glyphs and 30 mark sets, checks Abel absence, and pins counts/offsets. All real ClassDefs are format 2; coverage formats 1/2. Eight new tests, no production change or dependency.

C4.4b4 pending CI: font_lookup_filter.rs provides shared borrowed LookupFilter with typed eligibility reasons, mark set index and glyph domain validation, flag precedence and bounded original-index projection. No mutation, no inferred glyph classes, no scan-order interpretation of RightToLeft. Gdef exposes glyph_count; GsubLookup integration validates type and flags. Thirteen new tests, no external dependency.

C4.4b5 pending CI: real_gdef.rs extends the independent raw SFNT reference to all 669 native GSUB lookups in seven original GDEF fonts, including 131 nonzero flags and 19 mark set references. Dense class/membership arrays determine expected eligibility for all 829056 lookup/glyph pairs; full-domain original-index projection on ascending and reverse/duplicate runs with exact limits. Six tests, no production changes or dependencies.

C4.4c1 pending CI: font_gsub_plan.rs provides explicit base feature plan using exact or DFLT script and exact or default language, mandatory feature and caller-selected optional tags. Unique LookupList indices sorted ascending with contributing feature indices, bounded raw lookup refs and unique outputs. Missing script/language returns None, no arbitrary fallback. Base-only FeatureVariations marker; no script-specific stages, feature values or execution. Thirteen tests including parsed ABeeZee integration, no external dependency.

C4.4c2 pending CI: real_gsub_plan.rs independently reads raw SFNT/GSUB scripts, language systems and features for seven original OFL fonts. Dense feature masks and lookup-indexed provenance compare base plans for all 103 language systems, selection variations, duplicate tags and fallback; pinned raw counts, Abel no-GSUB case. Eight tests, no production changes or dependencies.

C4.4c3a pending CI: font_gsub_compile.rs compiles typed owned direct and extension targets, preserving wrapper/target offsets and borrowed GDEF filter. Direct and extension dispatch share parse_direct_gsub_target. Internally parses GSUB; validates explicit index order or base plan language/feature membership and lookup provenance. Cumulative subtable count and conservative suffix-byte work charge, per-target limits and plan-ref budgets. No recursive compilation or execution. Twelve new tests, no external dependency.

C4.4c3b pending CI: real_gsub_compile.rs pins independently read raw inventory for 669 lookups and 3382 subtables (1503 native extension wrappers) across seven original OFL fonts. Compare typed payloads with standalone parsers, compile each default language's selected plan, test exact lookup/subtable/suffix-byte limits and one-less failures. Seven new tests; no production changes. Suffix-byte work accounting is not measured memory.

C4.4c4a pending CI: font_gsub_dependencies.rs enumerates ordered direct contextual actions from all six contextual formats and extension targets, preserving stored set/rule/action and source/target offsets. Shared read-only rule-set accessors expose all parsed sets including unreachable/null slots. Cumulative scan/action budgets, source order and target index checks. Cyclic references retained, no execution or closure. Ten tests, no dependency added.

C4.4c4b pending CI: real_gsub_dependencies.rs independently traverses raw GSUB contextual action records for all six formats and extension wrappers across seven existing OFL fonts. Compares 21624 ordered references from 2881 contextual subtables and 13539 rules, pins metadata counts, exact scan/action limits and subset original-index preservation. Seven new tests; no production changes.

C4.4c4c pending CI: font_gsub_closure.rs builds bounded sorted adjacency from explicit covered sources and ordered action references, deduplicating only for graph analysis. Iterative three-color DFS from sorted roots produces reachable vertices, missing sources, distinct known reachable edges and first closed back-edge witness. Raw input and traversal budgets. Thirteen tests including exhaustive three-node graphs and 20000-node chain. No runtime execution or cycle rejection.

C4.4c4d pending CI: real_gsub_dependencies.rs extends raw action reference using an independent Warshall Boolean matrix oracle. Compare static closure for all 669 individual roots, 103 real base language plans, full and partial source selections. Raw census pins 541 direct, 778 positive-length reachable pairs, 237 indirect-only, max inclusive closure 44; no cycles observed in these fonts. Seven new tests, no production changes.

C4.4c5a pending CI: compile_gsub_program and compile_gsub_plan_program use one parsed layout, shared budgets and deduplicated pending indices; preserve top-level roots separately from sorted compiled dependency targets. Complete closure with diagnostic cycles. Seven tests.

C4.4c5b pending CI: seven existing real-font fixtures validate program integration against independently extracted raw action sites and matrix reachability; raw lookup offsets establish aggregate compile budgets including native extension wrappers. Compare compiled payloads against existing standalone compiler, keep roots distinct from dependencies, check exact budgets and small-fixture one-less failures. No production algorithm change or actual substitution execution.

C4.4d1a pending CI: CompiledGsubProgram retains validated glyph domain; execute_gsub_single executes only effective Type 1 scheduled roots, rejects other roots before work, visits logical positions once per root and evaluates current-glyph eligibility. First matching subtable wins even on identity. Immutable input, bounded owned output, cumulative visits/probes/matches and explicit root/glyph caps. Type 7 dispatch uses compiled targets. No nested actions or mixed-type execution yet.

C4.4d1b pending CI: independent test-only raw SFNT/GSUB/GDEF oracle expands coverage and class/mark sets, constructs ordered Type 1 replacement maps and runs out-of-place lookup passes; compares all-glyph single-root and combined-root results, visit/probe/match/change counters and exact limits on seven existing fonts. No production algorithm change.

C4.4d2a pending CI: execute_gsub_mixed uses two reusable glyph buffers per ordered root pass; direct or wrapped Type 1/2, first applicable subtable, current input glyph eligibility, no same-root rescan of emitted sequences. Limits bound each intermediate run plus cumulative visits, subtable probes, matches and all emitted glyphs (including copies). Input remains immutable and errors return no partial output. Legacy Type 1 API retained.

C4.4d2b pending CI: test-only raw Type 2 sequence extraction extends the verified Type 1/GDEF oracle. Dense per-glyph decision tables independently select first eligible subtable and compute probes; separate counting and output construction checks all single roots, Type 2-only and combined passes, legacy Type 1 equivalence and exact resource limits across seven fixtures. No production algorithm change.

C4.4d3a pending CI: execute_gsub_with_alternates shares bounded mixed pass execution and accepts an exact ascending list of explicit zero-based choices, one per scheduled effective Type 3 lookup. Missing, extra, wrong-root and out-of-order choices fail preflight even for empty input. Eligible covered glyphs use the first matching subtable; invalid candidate fails without fallback. Direct and Type 7-wrapped alternates supported, legacy mixed API still rejects Type 3. No implicit feature-value selection.

C4.4d3b pending CI: test-only independent AlternateSet decoding preserves raw candidate order, uses existing dense raw-reference eligibility and first-match decisions, checks every valid candidate against the executor and per-glyph one-past-end failure, combined and alternate-only passes with first and highest universally valid choices, exact counters and bounds. Seven fixtures (two native Type 3 and five controls). No production algorithm change.

C4.4d4a pending CI: execute_gsub_with_ligatures extends the bounded pass engine with Type 4 (including Type 7 wrapper). Each ligature pass precomputes eligible original indices, charges a visit per physical input glyph, tries subtables and rules in stored order, compares only eligible components with separate rule/component caps, emits the ligature and intervening ignored gaps in original order, and advances past the final consumed component. Counts ligatures and removed trailing components. Later roots recompute eligibility; legacy APIs reject Type 4. No cluster or attachment metadata.

C4.4d4b pending CI: independent test-only raw LigatureSet decoder and physical queue reference; constructs input from every stored rule, decorates with ineligible glyphs where available, compares results and all counters. Type 4-only and combined Type 1/2/3/4 passes compare exact peak intermediate length and one-less budgets. No production algorithm change.

C4.4d5a pending CI: separate execute_gsub_context supports reachable effective Type 1/5 only, including wrapped targets. Formats 1/2/3 capture matched eligible physical positions; actions execute in stored order with target lookup's own current-glyph eligibility; top-level match advances past input span, nested apply is position-specific. Active (lookup,position) cycle rejection, hard depth 32, cumulative call/probe/filter/rule/symbol/action/match budgets and immutable input. Existing Type 1-4 APIs unchanged.

C4.4d5b pending CI: execute_gsub_context_with_alternates accepts exact ascending choices for all reachable effective Type 3 indices, including dependencies; same choice is used at root and nested calls. Binary-search selection, target GDEF eligibility, strict invalid index, atomic failure, existing depth/work limits. Legacy Type 1/5 API still rejects Type 3. Eight synthetic and two native AlternateSet target tests under synthetic Type 5 parents.

C4.4d5c pending CI: test-only native Type 5 raw decoder and independent supported closure traversal; reference execution uses physical iterators, raw class/coverage arrays and raw GDEF eligibility, applies ordered actions at captured positions. Each supported root is checked with bounded native witnesses, full glyph-domain mixed input, exact counters and one-less limits. Unsupported reachable types are explicitly rejected. No production changes.

C4.4d5d pending CI: execute_gsub_context_with_expansion supports effective Type 1/2/3/5 (including Type 7). Active frames own captured physical positions and exclusive end; every growth rebases anchors strictly after the replacement position and ends strictly after it. First output retains original anchor; nested edits outside an ancestor span do not extend that span. Root Type 2 skips all emitted glyphs. Separate cumulative inserted and edit-work budgets (writes, suffix movement, frame and anchor inspections), peak length, existing work/depth/cycle limits. Old APIs reject Type 2.

C4.4d5e pending CI: independent test-only identity-token oracle uses original successor tokens as exclusive boundaries and resolves action anchors by token search, without physical rebasing. Native Type 2 roots, supported native Type 5 closures with Type 2 dependencies, and native raw Type 2 target samples under constructed two-action parents compare output, counters and exact/one-less selected resource limits. No production changes.

C4.4d5f pending CI: execute_gsub_context_with_contraction extends the expansion API with effective Type 4 direct/wrapped targets. Match using current target eligibility and ordered subtables/rules; keep first physical glyph, preserve ignored gaps, compact suffix once. Consumed trailing anchors become tombstones and subsequent actions on them count but skip target calls. Surviving anchors and exclusive boundaries subtract removed positions before them. Tombstones survive later expansions. Shared edit-work budget charges scanned suffix and conservative anchor/removal comparisons; separate max_removed. Legacy APIs unchanged.

C4.4d5g pending CI: test-only identity ledger retains deleted tokens as boundary landmarks, resolves live action tokens without rebasing, removes matched glyphs in reverse order, tracks intermediate peak and independent counters. Native Type 4 root and combined Type 1/2/3/4 checks, supported native Type 5 closures with Type 4, constructed mixed growth/shrink parents using native Type 4 bytes. No production changes.

C4.4d6a pending CI: execute_gsub_chained_context opts into effective Type 1/3/5/6 direct or wrapped. Backtrack scans nearest-first, then forward input tail and lookahead; all use target lookup's GDEF eligibility, current glyphs and charged physical/symbol scans. Format 2 uses independent class domains; Format 3 charges first coverage on each probe. Only matched input positions are captured, root advances past input, not lookahead. Existing alternate selection and cycle/depth/work limits; legacy APIs reject Type 6.

C4.4d6b pending CI: execute_gsub_chained_context_with_edits opts into effective Type 1–6 with direct/wrapped targets and reuses the verified shared frame expansion/contraction logic, current-glyph matching, tombstones and work budgets. Input-only anchors, live backtrack/lookahead at each new invocation, ancestor exclusive input boundary policy. Legacy APIs unchanged. Tests cover mixed Type 5/6 nesting, filters, dynamic contexts, peak caps and 4096 finite-domain sequences.

C4.4d6c pending CI: test-only native Type 6 decoder and directional physical iterator matcher, independent three-class-domain and coverage decoding, raw Type 5/6 closure traversal. Extend stable identity ledger reference to Type 6; check supported native closures, bounded witnesses and full-domain runs, mixed ordered root schedules, exact output/counters/peak and selected one-less caps. No production changes.

C4.4d8a pending CI: execute_gsub_program opts into effective Type 1–6 and 8, including Type 7 wrappers. Type 8 uses indexed replacements, current eligible input and directional filtered backtrack/lookahead; root traversal is descending physical index, other roots forward, first matching subtable wins including identity. Nested Type 8 acts once at captured position; root direction is not inherited by nested dispatch. Existing context and edit budgets, alternate selection and legacy rejection preserved.

C4.4d8b pending CI: test-only raw reverse decoder, directional physical reference and stable identity ledger extended to reverse roots and nested point invocations. Seven native fonts verify zero Type 8 inventory and mixed all-present-kind dispatch parity; synthetic GSUB tables cover actual Type 8, all effective kinds and mixed root order, GDEF and Coverage Format 2. No production changes.

C4.4d8c pending CI: native Noto Sans Mongolian fixture (unchanged upstream font and OFL 1.1, pinned provenance) includes two Type 8 subtables. Independent raw reference on positive witnesses, ignored marks, incomplete segments and full domain. All 127 nonempty wrapper combinations across seven legal effective kinds tested against direct program plus root/nested reverse parity. No production changes.

C4.4d9 pending CI: stable source-compatible GsubProgramExecLimits/GsubProgramExecResult aliases for unified execution, eight native default-LangSys selected feature schedule regressions linking base planner, provenance validation, program compilation and raw execution reference. See GSUB_EXECUTION.md for scheduling, edit and budget contract. No full shaping claim.
