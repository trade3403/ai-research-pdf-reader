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

C2.4e (pending CI): `picture_geometry.rs` parses standalone pic:pic srcRect and xfrm, validates coordinate bounds, computes outward source pixel windows and reversible normalized-source-to-EMU affine mapping (local flips before center rotation), exact cardinal directions and four-corner bounds. Independent of inline/floating metadata and layout; caller must reconcile wp:extent, shape frames and actual source dimensions.
