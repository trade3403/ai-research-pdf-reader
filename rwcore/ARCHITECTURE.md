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

C3.2 (pending CI): `selection.rs` supplies borrowed paragraph prefix counts for UTF-8/scalar/UTF-16, run-local/global conversion with explicit shared-boundary affinity, directional normalized selections, selected text and cross-run replacement compiled to C3.1 atomic edits. Replacement inherits the start run, retains other run structure; strict interior drawing checks include empty/shared-boundary runs, preservation is local to each run. No cross-paragraph or structural edit yet.
