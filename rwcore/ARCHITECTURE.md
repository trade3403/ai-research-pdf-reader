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

C4.3a (pending CI): font_outline.rs shares validated head/maxp parsers, validates the complete short/long loca index and borrows glyf slices. Lazy simple glyph decoding checks ordered contour endpoints, repeated flags, signed coordinate deltas and resource budgets. Exact half-design-unit paths reconstruct implied on-curve midpoints, including all-off-curve contours. Hint instructions inert; declared bounds advisory, point bounds not true curve extrema. Composite detected only.
