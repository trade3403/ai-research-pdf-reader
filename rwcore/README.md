# RWCore — C2.0 direct run formatting

Independent Rust workspace for research manuscripts, on feature/rwcore.
The existing Tiptap Word editor remains unchanged. No Electron or RPCore integration in this milestone.

## Scope
- Read-only, bounded ZIP/OPC input using general-purpose zip and roxmltree crates.
- Independent ZIP32 structural preflight before decompression: central directory, local headers, CRC/size metadata consistency, data descriptors, record coverage, raw duplicate names and expansion budgets.
- Resolve the main document from the root relationship, validate its declared content type and read the selected part.
- Enumerate package parts, content types and relationship edges; classify style, font, image, numbering, theme and other resources.
- Reject escaping paths, ambiguous URI forms, case-colliding visible ZIP names, duplicate relationship IDs, orphan relationship parts and dangling internal targets. External references are inert metadata only.
- Namespace-aware Word paragraphs/runs, Unicode text, tabs, line breaks, diagnostics for unsupported content.
- Renderer-neutral document tree and integer twip type for future layout.
- Synthetic DOCX tests and inspect CLI.

## Run
```sh
cargo test --manifest-path rwcore/Cargo.toml --workspace
cargo run --manifest-path rwcore/Cargo.toml -p rwcore --example inspect -- sample.docx
```

## Boundaries
Not a full OPC or DOCX implementation. Conservative URI subset (encoded UTF-8 and spaces accepted in canonical percent-encoded ZIP names; escaped ASCII delimiters/unreserved characters, fragments, queries and case-mismatched lookups rejected); UTF-8 XML only, basic paragraphs only. Resource detection is metadata only, not image decoding or style interpretation. Missing MIME declarations on non-main parts are exposed as None.
No style resolution, tables, images, formulas, pagination, editing, export or layout fidelity yet.
No macros executed, no extraction, no network access. Archive/part/entry/node limits and DTD rejection are initial defenses, not a security audit. Raw central-directory duplicate names are checked before the ZIP library can collapse them. ZIP32 only; reject ZIP64, multi-disk, encryption, unknown flags, alternate path extras, unindexed data and compression methods other than stored/deflate. Valid but unusual ZIPs may be rejected. Structural preflight does not verify the payload CRC of unused parts; selected parts are read with the ZIP library and a declared-size cap. Default per-entry limit 8 MiB applies to all entries, aggregate declared expansion 128 MiB, ratio 1000:1 plus 1 MiB grace. These are conservative, not a security guarantee.
Dependencies are provisional; CI now generates Cargo.lock, prints the dependency tree, builds/tests with --locked, and publishes the lockfile artifact. Commit the verified lockfile and remove per-run resolution before release; advisory and license audit remain pending. Never replace or overwrite a source DOCX.
No claim of Word compatibility or GUI testing.

## Roadmap
C1.0 foundations and bounded read-only probe.
C1.1 OPC graph and bounded URI resolution implemented in source; build and real corpus not yet verified.
C1.2 ZIP32 structural preflight, bounded expansion and conservative encoded URI support implemented in source. Real corpus and dependency audit remain gates.
C2 styles, numbering, sections, tables, media and unsupported-node preservation.
C3 transactions, selection and undo; C4 typography; C5 pagination; C6 advanced layout;
C7 math; C8 citations/fields; C9 DOCX writer; C10 reference comparison and safety; C11 Electron integration.
Use ECMA-376 / ISO/IEC 29500 as reference; share renderer-neutral interfaces with RPCore after both APIs stabilize. Do not restructure A or B from this branch.

## C1.1 validation

- 10 additional synthetic regression cases in core/tests/package.rs; 10 C1.0 cases retained.
- Run the existing Linux/Windows/macOS CI matrix. CI #16 and #17 subsequently passed.
- Local corpus command (use lawful local test documents, no upload):

```sh
cargo run --manifest-path rwcore/Cargo.toml -p rwcore --example corpus_probe -- sample1.docx sample2.docx
```

The probe reports package/edge/block/warning counts and nonzero exit on parse errors. Success means the supported subset was read, not that layout or content fidelity was achieved. Full ZIP integrity validation, dependency lockfile and real-world corpus results remain pending.

## C1.2 validation and remaining gates

- 13 additional synthetic ZIP/URI cases in core/tests/zip_guard.rs (33 total source test cases).
- No real DOCX corpus was supplied; run corpus_probe against representative legal local Word files (Word, LibreOffice, SCI templates, images, equations, sections, Chinese/English) and record failures without uploading document contents.
- C1.2 CI #16 and locked CI #17 passed on all three platforms; Cargo.lock is committed. No complete security audit or DOCX compatibility claim.
- C2.0 next after foundation verification: WordprocessingML paragraph/run properties, styles and inheritance, explicit diagnostics for unsupported properties; then tables and sections.

## C2.0 continuation

CI #18 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45435743540), tested revision 01fa37bae8056eb37571002de0178a94ffd75201: 38 passed, zero failed or ignored on each of Linux, Windows and macOS (114 executions). Direct run properties now attached to Run during DOCX parsing without reparsing XML; duplicate containers and invalid properties diagnosed. Two new package-level regressions, 40 source tests pending CI. Direct values only, no style inheritance. Next: paragraph properties and style resolution. Real corpus and dependency audit remain outstanding.

## C2.0 paragraph increment

CI #19 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45453903857), revision e14b31fd2a5a6493c4b266db4adcb33c10735d0b: 40 tests passed per platform, zero failed or ignored (120 executions). Direct paragraph properties now attached to the model: style ID, alignment, spacing, signed left/right indentation, first-line/hanging indentation and pagination flags. Line value retains raw units (auto: 240ths of a line; exact/atLeast: twips); no effective layout is computed. Unsupported attributes and conflicting indentation diagnosed. Two new package tests (42 total pending CI). Next: style definitions and bounded inheritance. Real corpus and dependency audit remain outstanding.

## C2.0 style catalog increment

CI #20 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45468659633), revision a957af1d07216cd27f2f024da465efed9e643ee3: 42 tests passed per platform, zero failed or ignored (126 executions). Standalone styles.xml catalog parses direct properties, default flags and basedOn links. Parent-first traversal rejects cycles, missing and cross-kind links and chains beyond 128 entries. Five new tests (47 pending CI). Not yet integrated with DOCX relationship loading; docDefaults, toggle semantics, effective formatting and unsupported style metadata preservation remain pending. Real corpus and dependency audit remain outstanding.

## C2.0 related styles and defaults increment

CI #21 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45482971727), revision e90222e5f4ea06a4ed0dd9fb60955175ad38bb3c: 47 tests passed per platform, zero failed or ignored (141 executions). Main-document styles relationship now loads the bounded style catalog and docDefaults paragraph/run properties; unrelated parts are ignored, external styles remain inert and multiple styles relationships are rejected. Four package regressions (51 total pending CI). Effective formatting, toggle semantics, MIME validation of styles and unsupported default metadata preservation remain pending. Real corpus and dependency audit remain outstanding.

## C2.0 effective formatting increment

CI #22 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45501625050), revision e425bbf2323c51fb37227cafd7959b420cb98fd9: 51 tests passed per platform, zero failed or ignored (153 executions); release builds passed. New explicit resolve_formatting API computes the supported paragraph/run cascade without changing the source model; parent-first style layers, b/i toggle behavior, direct overrides, mutually exclusive color and indentation, and diagnostics. Six additional tests (57 pending CI). Still a subset, not Word rendering fidelity; table/numbering styles, themes, script variants, style metadata, real corpus and dependency audit remain pending. Next: validate this increment, then integrate effective formatting in document traversal and advance numbering/sections.

## C2.0 document formatting view

CI #23 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45505716722), revision 9a5627510adf07b052befa6437a5f05219a55dd4: 57 passed per platform, zero failed or ignored (171 executions); release builds passed. Document::formatted_blocks exposes lazy borrowed source-order views, resolves paragraph styles once per visited paragraph (including empty ones), preserves unsupported blocks and resolves runs individually with explicit errors. Three new regression tests (60 pending CI). No mutation of direct formatting; no layout or rendering guarantee. Real corpus, dependency audit, theme and numbering resolution remain outstanding. Next: validate then implement numbered paragraph references and numbering catalog.

## C2.1 numbering references increment

CI #24 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45508687740), revision f28c2ce851d95a74bf7293e615adbdb6c753284d: prior acceptance recorded 60 tests per platform, no failures or ignored tests. New typed numPr fields preserve optional numId and ilvl (0..8), explicit numId=0 suppression and malformed/duplicate diagnostics. Raw references propagate through the supported property cascade; this is not full numbering style semantics (style-linked levels and numbering property precedence remain pending). Three additional regressions (63 source tests pending CI). Next: bounded numbering.xml catalog and relationship loading, then level selection and counters. No actual list labels/layout yet. Real corpus and dependency audit remain outstanding.

## C2.1b numbering catalog increment

CI #25 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45511332487), revision 3f5953af282761f303b21dc72f9533a19fdc9d33: 63 passed per platform, zero failed or ignored (189 executions). Numbering part parser and main-document relationship loading now preserve abstract levels, instances, overrides, start values, format, text, restart, suffix, style references and direct formatting under existing XML budgets. Internal numbering MIME is required, external links inert, duplicates and missing abstract references rejected. Seven new tests (70 pending CI). Strict subset only: style links, custom formats, picture bullets, level selection, restart counters and label generation still pending. Real corpus and dependency audit outstanding. Next: validate then resolve effective levels and numbering counters.

## C2.1c numbering counter increment

CI #26 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45512198483), revision 0f1415cc0ba1d8e40fc9d9235dbd2d823d491650: 70 tests passed per platform, zero failed or ignored (210 executions). Effective level resolution and explicit stateful label generator now support full level/start overrides, independent instance counters, an initial ancestor restart policy, decimal/letter/Roman/bullet/none formats, suffixes and bounded label output. Errors leave state unchanged. Six new tests (76 pending CI). This subset requires explicit effective numId/ilvl, uses zero for absent start and uses defined start for unvisited ancestors; restart policy and letter formats still need comparison against Word. Style links, legal numbering, picture bullets, custom formats, numbering layout, document traversal, real corpus and dependency audit remain outstanding. Next: validate this increment, then integrate numbering into source-order document traversal and expand reference tests.

## C2.1d document numbering traversal

CI #27 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45512651040), revision a4f2c19cc33dabda5192d3be3195f31575368021: 76 passed per platform, zero failed or ignored (228 executions); release builds passed. Document::numbered_blocks now resolves the supported effective numbering references in source order, independent counters per traversal, explicit zero cancellation, level-zero fallback for omitted level, separate label errors preserving formatted paragraphs, and unchanged source text. Four additional regressions (80 source tests pending CI), including synthetic DOCX package traversal. Unsupported blocks remain opaque, so nested lists and full Word numbering fidelity are not claimed. Style-linked levels, numbering formatting precedence, legal numbering and Word reference comparisons remain pending. Real corpus and dependency audit still outstanding. Next: validate traversal, then start independent C2.2 section/page property parser while retaining numbering gaps for follow-up.

## C2.2a standalone section parser

CI #28 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45515186265), revision fb93ff9236f3b02fa1e411123c135e6aa53d43f7: 80 tests passed per platform, zero failed or ignored (240 executions); release builds passed. New standalone section parser supports typed page width/height, orientation, signed top/bottom and unsigned remaining margins, five break types, equal/explicit unequal column declarations and bounded diagnostics. Six new regression tests (86 source tests pending CI). Integer twips only; strict physical-unit strings, headers/footers, page numbering, section boundaries in the document tree, effective geometry and pagination remain pending. Conservative 64-column parser limit. Next: validate standalone parser and integrate body/paragraph sectPr positions without losing source blocks. Numbering reference fidelity, real corpus and dependency audit remain outstanding.

## C2.2b section boundary integration

CI #29 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526090309), revision 65730a2b7b7aee8ea0f975854a8b91a667b65294: 86 tests passed per platform, zero failed or ignored (258 executions); release builds passed. Document.sections stores half-open source-order content ranges, ending paragraph belongs to its own section, final body sectPr is metadata rather than a content block, implicit final ranges preserve missing properties and empty sections. Duplicate/misplaced boundaries rejected; section diagnostics propagate. Four new package regressions (90 source tests pending CI). Existing bootstrap regression updated to assert the exact new metadata semantics, retained unsupported table and source text. No effective geometry, defaults, header/footer resolution or actual pagination yet. Nested section content in unsupported blocks remains opaque. Next: validate then implement bounded page/content/column geometry with explicit assumptions. Real corpus and dependency audit remain outstanding.

## C2.2c static page geometry

CI #30 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526352722), revision fe49435b6028508e9cbf891d487d0adf3e0ebde6: 90 tests passed per platform, zero failed or ignored (270 executions); release builds passed. New checked integer-twip geometry API calculates physical page, content rectangle, caller-selected left/right/top gutter placement, equal-column widths with deterministic remainder allocation, explicit unequal column rectangles, separator flag and trailing space. Seven new tests (97 source tests pending CI). Dimensions and four margins must be explicit, negative vertical margins rejected for now, missing columns default to one and missing intercolumn spacing to 720 twips. Orientation does not swap stored sizes. No inherited defaults, mirrored margins, RTL, header/footer collision handling, continuous-section layout or pagination yet. Next: validate then add section header/footer references and page-number properties or independent geometry traversal. Real corpus, dependency/security/license audits remain outstanding.

## C2.2d header/footer and page-number metadata

CI #31 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526557273), revision 910b3b99ae82dfd5a89080813c1c626a2d950083: 97 tests passed per platform, zero failed or ignored (291 executions); release builds passed. Section parser now retains default/first/even header and footer relationship IDs, titlePg, pgNumType start/format/chapter metadata, validates duplicates and supports strict relationship namespace. Explicit resolve_header_footer API checks the main part relationship scope, kind, internal target and declared MIME, without loading or fetching resources. Five new regression tests (102 source tests pending CI); two former unsupported-property warning fixtures now exercise still-unsupported pgBorders/docGrid, preserving warning checks. Missing relationship validation occurs when explicitly resolving, not during read_docx. Actual header/footer content loading, settings evenAndOddHeaders, inheritance, field evaluation and physical page numbering remain pending. Next: validate then load bounded header/footer stories and settings. Real corpus, dependency/security/license audits remain outstanding.

## C2.2d-2 bounded header/footer stories

CI #32 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45527483116), revision 9f1383a35d29b8736d151ff13271e58b7097821b: 102 tests passed per platform, zero failed or ignored (306 executions); release builds passed. Opt-in read_docx_with_stories reads referenced header/footer XML once per part, validates relationship scope/kind/MIME and expected root, retains basic paragraphs/runs/direct properties/text and opaque block names, and parses evenAndOddHeaders settings. Uses existing ZIP/XML limits plus 16 MiB aggregate story/settings input budget. Existing read_docx remains metadata-only to preserve API behavior; errors in referenced resources are explicit in strict opt-in mode. Five new tests (107 source tests pending CI). No field evaluation, nested tables/media, inherited variant selection or pagination; unsupported content diagnosed. Next: validate and implement section variant inheritance/selection and field preservation. Real corpus and dependency/security/license audits still outstanding.

## C2.2d-3 inherited header/footer variants

CI #33 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45527761024), revision a03ac72320daefc914ba45a1e44e5005cc56b33a: 107 tests passed per platform, zero failed or ignored (321 executions); release builds passed. New section_stories API resolves six independent header/footer variant slots in one forward pass with borrowed story bindings and source-section provenance. Explicit empty stories override inheritance; titlePg is local, evenAndOddHeaders global. First-page variant takes precedence; absent selected variant is blank. Caller supplies page context (parity and first-in-section), not inferred from physical layout or displayed numbering. Five new regressions (112 source tests pending CI), including package integration, inactive variant inheritance, empty override and inconsistent catalog. No fields or pagination yet; actual Word parity/continuous-section behavior needs reference comparison. Next: validate and preserve PAGE/NUMPAGES field instructions and cached text. Real corpus, dependency/security/license audits remain outstanding.

## C2.2d-4 inert story field metadata

CI #34 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45528097743), revision b88b7bda417cab626fac8f323d6291c301f47e6f: 112 tests passed per platform, zero failed or ignored (336 executions); release builds passed. Story.fields now captures direct simple fields and same-paragraph complex fields spanning runs, raw instructions, PAGE/NUMPAGES/SECTIONPAGES classification, cached results, encoding and source block order. Unknown field commands remain inert. Explicit limits: 16 KiB instruction, 64 KiB cache and 4096 fields per story. Nested, malformed and cross-paragraph fields fail explicitly in opt-in story loading; unsupported wrappers remain untraversed. Existing paragraph rendering model is unchanged; simple field cache is in metadata, not injected into paragraph text. Five new tests (117 source tests pending CI). Actual field evaluation, switches, mixed inline placement and pagination remain pending. Next: validate then implement checked page-count context evaluation and page-number formatting. Real corpus and dependency/security/license audits remain outstanding.

## C2.2d-5 checked page-number context and field evaluation

CI #35 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45528440928), revision 8d2b31a5fd0f52a5a005b0f139ab79f38e9fd75d: 117 tests passed per platform, zero failed or ignored (351 executions); release builds passed. New PageNumberPlan calculates disjoint nonempty section spans from caller-supplied page counts, continuous/restarted displayed numbers, physical page indices and section/document totals without per-page allocation. evaluate_page_field supports PAGE/NUMPAGES/SECTIONPAGES, decimal/decimalZero/Roman/letter section formats and explicit Arabic/ROMAN/roman/ALPHABETIC/alphabetic switches, recognizes MERGEFORMAT/CHARFORMAT without applying typography, and leaves cache untouched. Reuses bounded numbering formatter. Seven new tests (124 source tests pending CI). No layout-derived counts, shared continuous pages, blank-page assignment, negative starts, chapter prefixes, other switches or inline field replacement; unsupported cases fail or require caller context. Word reference fidelity (including letter sequence) needs real comparison. Next: validate and integrate page geometry, page-number context and story selection for supplied page ranges. Real corpus and dependency/security/license audits remain outstanding.

## C2.2e integrated static page structure

CI #36 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45529564472), revision ba40ad0ec6560478124f2b318b61a6e1d3fa31d4: 124 tests passed per platform, zero failed or ignored (372 executions); release builds passed. New PageStructurePlan combines explicit section geometry, caller-supplied disjoint page counts, physical/display numbering, inherited story selection and lazy per-field evaluation. Ordered section block coverage is checked; each result exposes the section range rather than claiming page-level body placement. Borrowed resources and per-section allocations avoid copying stories or allocating every page. Five new regressions (129 source tests pending CI). Requires explicit geometry, gutter placement, loaded story catalog and nonempty disjoint page ranges; physical parity is the implemented convention, not yet Word-reference verified. No actual pagination, shared continuous pages, body placement, inline field replacement or rendering. Next: validate, then begin C2.3 bounded table structure parser. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.

## C2.3a bounded table source structure

CI #37 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45530986547), revision 8709cafc9c8c7e6b2cf2fbeef3449e103b611f7b: 129 tests passed per platform, zero failed or ignored (387 executions); release builds passed. New standalone parse_table API retains source-order grid widths, rows, cells, direct cell paragraphs/runs, raw grid spans, vertical merge markers and omitted leading/trailing grid slots. Unsupported cell blocks are opaque, property diagnostics are explicit, and ragged grid rows are not silently repaired. Limits: 256 grid columns/row occupancy, 16384 rows, 65536 cells per table plus existing XML/part limits. Six new tests (135 source tests pending CI). Main document and story readers still preserve tables as unsupported blocks; no logical merge resolution, borders, style cascade, nested tables, geometry or rendering yet. Next: validate and resolve checked logical grid coordinates and merge topology, then integrate document loading. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.

## C2.3b logical table grid and merge topology

CI #38 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45531596804), revision 1ce7747c6cf216ea1bc3d927c77196a42dffd6c4: 135 tests passed per platform, zero failed or ignored (405 executions); release builds passed. New resolve_table_grid maps gridSpan and vMerge to sparse source-order logical cells, rectangular anchors, row/column spans and coordinate lookup. Continuations require an exact matching active span in the preceding row; restart replaces the active anchor. Source cells and continuation content are borrowed without modification. Omitted slots remain empty; missing grids infer maximum occupancy; explicit grid overflow is rejected in this subset (rather than Word-style grid expansion). Bounds rechecked for public model values. Six new tests (141 source tests pending CI). Legacy hMerge, nested tables, formatting, geometry and main-document integration remain pending. Next: validate then add bounded table width, border and padding properties. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.

## C2.3c-1 direct width, border and margin metadata

CI #39 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45531928538), revision 5c948f9411544ad01223cc755a7a181955006f4c: 141 tests passed per platform, zero failed or ignored (423 executions); release builds passed. Typed direct table/cell width (dxa, integer pct in 1/50 percent, auto, nil), border style/size/space/color and margin maps are retained independently of merge topology. Missing, empty, zero and none remain distinct; duplicate known properties and malformed values rejected; theme attributes diagnosed. Five new regressions (146 source tests pending CI). Prior unsupported-property regression now uses tblLook instead of supported tblBorders. No style cascade, conflict resolution, geometry, rendering or strict physical-unit string support. Next: validate then C2.3c-2 shading, row height, layout and vertical alignment. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.

## C2.3c-2 shading, height and layout metadata

CI #40 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45532371677), revision 2ce2165e0aa224d4e5a585e939cf705b6c5b7163: 146 tests passed per platform, zero failed or ignored (438 executions); release builds passed. Direct table/cell shading keeps RGB/auto foreground and fill, raw bounded pattern token and theme diagnostics. Table-only layout, cell-only vertical alignment and row height metadata are parsed separately from geometry and merges. Explicit and missing height values are retained; exact/atLeast require a value in this subset. Six new regressions (152 source tests pending CI). No shading painting, theme cascade, width allocation, row sizing or vertical placement yet. Next: validate then resolve effective direct cell margins and shading with explicit overrides. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.

## C2.3c-3 direct effective cell properties

CI #41 (https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45532908834), revision b82dcbfa5c728148b5aef1afd44bc6d7730286e7: 152 tests passed per platform, zero failed or ignored (456 executions); release builds passed. New resolve_effective_cell normalizes start/end to physical margins using caller-supplied direction, rejects conflicting same-layer aliases, overlays cell edges on table defaults, retains zero and missing values, and tracks table/cell provenance. Whole shading override is an initial direct subset, not a complete Word cascade; preferred cell width does not inherit table width. resolve_preferred_width converts integer pct using caller-supplied reference width, nearest-twip rounding and checked output, retaining Auto/Nil. Seven new regressions (159 source tests pending CI). No border conflict resolution, conditional styles, theme resolution, row layout or actual column allocation; source cells remain independent across merges. Next: validate then C2.3d direct three-line table border topology and selection. Real corpus, Word reference comparisons and dependency/security/license audits remain outstanding.
