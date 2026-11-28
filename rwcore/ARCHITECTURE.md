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

C4.4d9 repair: pinned native mong default rclt complete 74-root schedule including reverse lookup 93, independently decoded from fixture bytes. Generic 32-root cap remains for other fixtures. No execution-core change.

C4.4e1 pending CI: font_feature_variations parses standalone FeatureVariations 1.0 or GSUB directory body, checked nested offsets, per-reference cumulative record/ref caps, sorted unique feature substitution indices and opaque parameter offsets. Selection uses caller-supplied normalized F2DOT14 coordinates, first matching supported record; null conditions match all, null substitution valid empty, unsupported condition/invalid axis fail match, unknown substitution version skipped. No base planner mutation yet. Reference: https://learn.microsoft.com/en-us/typography/opentype/otspec190/chapter2 .

C4.4e2 pending CI: GsubVariationPlan carries the evaluated plan, axis count/coordinates, presence/first record and selected replaced feature indices. build_gsub_variation_plan reads original bytes and shares base feature selection/ordered lookup logic; compile_gsub_variation_plan_program reselects from original bytes and caller coordinates, validates metadata, effective lookup provenance and replacement inventory, then compiles existing dependency closure. Legacy base interfaces unchanged. No execution-core change.

C4.4e3 pending CI: unchanged OFL Archivo variable fixture, native FeatureVariations raw reference, exhaustive normalized weight interval at three widths, exact parsing budgets, selected native rvrn and case+rvrn dynamic execution via existing independent glyph ledger reference, two native absent-variation controls. No production changes.

C4.4e4 pending CI: font_fvar parses fvar 1.0 axis and named-instance records with exact version-1 sizes, bounds, flags, unique tags, name ID domain checks and cumulative instance-coordinate cap. Default normalization clamps user Fixed input, divides in widened integer arithmetic (16.16 quotient rounded nearest magnitude, ties away from zero), preserves axis order and missing defaults, returns pre-avar Fixed; final conversion adds 2 and arithmetic shifts by 2. References https://learn.microsoft.com/en-us/typography/opentype/spec/fvar and https://learn.microsoft.com/en-us/typography/opentype/spec/otvaroverview .

C4.4e5 pending CI: font_avar parses avar 1.0 in fvar axis order with cumulative pair caps, strictly increasing source and nondecreasing destination, empty or missing identity anchors cause identity mapping. Fixed 16.16 ratio and product rounded nearest, ties upward, and final F2DOT14 conversion only after mapping; helpers normalize ordered/tagged user coordinates. Native raw reference sweeps for Archivo and Devanagari, no avar for Nastaliq. References https://learn.microsoft.com/en-us/typography/opentype/spec/avar and https://learn.microsoft.com/en-us/typography/opentype/spec/otvaroverview .

C4.4e6 pending CI: VariableGsubFace obtains fvar, optional avar, maxp, optional GDEF and GSUB from one parsed SFNT/TTC face; shares existing budgets and normalization/plan/compiler/executor. Ordered/tagged user Fixed coordinates, explicit feature tags, plan rebuild validation and atomic execution. Native Archivo raw design-space boundary and full glyph-domain reference; two absent-variation controls and malformed association/budget tests. No production substitution algorithm change.


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
