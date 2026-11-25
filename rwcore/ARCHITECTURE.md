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
