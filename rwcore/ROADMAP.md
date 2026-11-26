# C 线路研发路线

Status below is baseline at CI #96, not full Word compatibility.

| Phase | Scope | Status |
|---|---|---|
| C1 | Rust/OPC/ZIP guard, package graph, locked CI | Basic batches verified |
| C2.0 | Paragraph/run attributes, style inheritance | Basic batches verified |
| C2.1 | Numbering definitions, counters, body traversal | Basic batches verified |
| C2.2 | Sections, page geometry, header/footer, page fields | Basic batches verified |
| C2.3 | Tables, merges, properties, borders, nested parse/layout | Basic batches verified |
| C2.4a | Offline media resource catalog | Verified |
| C2.4b | Six raster format header sizes | Verified |
| C2.4c-1 | Inline DrawingML metadata | Verified |
| C2.4c-2 | Ordered inline pictures in document runs | Verified |
| C2.4d-1 | Floating anchor metadata | Verified |
| C2.4d-2 | Position and text wrap algorithms | Verified |
| C2.4e | Picture crop, rotation, geometry | Verified |
| C2.4f | Media/document integration acceptance | Verified |
| C3.1 | Atomic run edits, positions and anchors | Verified |
| C3.2 | Selection ranges and cross-run edits | Verified |
| C3.3a | Bounded undo/redo transaction history | Verified |
| C3.3b | Structural transactions and validation | Verified |
| C3.3c | Structural history and path-safe editor integration | Verified |
| C4.1 | Font container and SFNT directory parsing | Verified |
| C4.2a | Native font metrics and unit conversions | Verified |
| C4.2b | Unicode cmap glyph mapping | Verified |
| C4.2c | Nominal glyph metrics and text measurement integration | Verified |
| C4.3a | Native TrueType loca/glyf simple outlines | Verified |
| C4.3b | Bounded composite glyph assembly | Verified |
| C4.3c | Unified outline paths and curve bounds | Verified |
| C4.3d | Adaptive quadratic flattening and contour geometry | Verified |
| C4.3e | Winding fill and bounded grayscale glyph rasterization | Verified |
| C4.3f | Integrated TrueType glyph bitmap and automatic device geometry | Verified |
| C4.3g | Nominal text-run bitmap placement and cumulative advances | Verified |
| C4.3h-1 | Bounded absolute-coordinate grayscale comparison | Verified |
| C4.3h-2a | Licensed real-font fixture and nominal metric reference checks | Verified |
| C4.3h-2b | Test-only independent grayscale sampling on shared paths | Verified |
| C4.3h-2c | Independent simple geometry and italic fixture | Verified |
| C4.3h-2d1 | Independent translated composite geometry | Verified |
| C4.3h-2d2 | Additional font family validation | Verified |
| C4.3h-2e1 | Real scaled/reflected composite geometry | Verified |
| C4.3h-2e2a | Exact synthetic general matrix and attachment boundary tests | Verified |
| C4.3h-2e2b | Real point attachment/general matrix reference (outstanding) | Planned |
| C4.4a1 | Shared OpenType Coverage format 1/2 decoder | Verified |
| C4.4a2a | GSUB header and top-level directory | Verified |
| C4.4a2b | GSUB child structures and cross-index validation | Verified |
| C4.4a3a | Standalone single substitution format 1/2 | Verified |
| C4.4a3b | Real-font direct type 1 reference and integration | Verified |
| C4.4a4a | Standalone multiple substitution and bounded expansion | Verified |
| C4.4a4b | Real-font direct type 2 reference | Verified |
| C4.4a5a | Standalone alternate substitution and explicit selection | Verified |
| C4.4a5b | Real-font direct type 3 reference | Verified |
| C4.4a6a | Standalone ligature substitution and contraction | Verified |
| C4.4a6b | Real-font direct type 4 reference | Verified |
| C4.4a7a | Contextual substitution format 1 parsing and matching | Verified |
| C4.4a7b | ClassDef and contextual format 2 | Verified |
| C4.4a7c | Contextual format 3 coverage matching | Verified |
| C4.4a7d | Real-font direct contextual references | Verified |
| C4.4a7e1 | Glyph-based chained contextual format 1 | Verified |
| C4.4a7e2 | Class-based chained contextual format 2 | Verified |
| C4.4a7e3 | Coverage-based chained contextual format 3 | Verified |
| C4.4a7e4 | Real-font direct Type 6 reference acceptance | Verified |
| C4.4a8a | GSUB extension wrapper and target dispatch | Verified |
| C4.4a8b1 | Real direct targets in synthetic extension wrappers | Verified |
| C4.4a8b2a | Native Type 7 wrapper inventory and representative dispatch | Verified |
| C4.4a8b2b | Full native Type 7 target decoding and reference checks | Verified |
| C4.4a9a | Reverse chained single substitution Type 8 | Verified |
| C4.4a9b | Real native Type 8 font reference | Outstanding: fixture needed |
| C4.4b1 | GDEF header/directory and glyph class tables | Verified |
| C4.4b2 | GDEF MarkGlyphSetsDef coverage sets | Verified (#110 after repair) |
| C4.4b3 | Real GDEF class and mark-set references | Verified |
| C4.4b4 | LookupFlag glyph eligibility and mark filtering | Verified |
| C4.4b5 | Real GSUB LookupFlag and GDEF references | Verified |
| C4.4c1 | GSUB script/language/feature selection and ordered lookup plan | Verified |
| C4.4c2 | Real Script/LangSys/feature base plan reference | Verified |
| C4.4c3a | Bounded GSUB lookup target compilation | Verified |
| C4.4c3b | Real-font lookup compilation reference | Verified |
| C4.4c4a | Bounded nested GSUB lookup dependency discovery | Verified |
| C4.4c4b | Real raw contextual dependency reference | Verified |
| C4.4c4c | Bounded static dependency closure and cycle diagnostics | Verified |
| C4.4c4d | Real-font dependency closure references | Verified |
| C4.4c5a | Bounded dependency compilation integration | Verified (#122) |
| C4.4c5b | Real-font dependency program reference | Verified (#123) |
| C4.4d1a | Bounded actual single substitution execution with eligibility and root scheduling | Verified (#124) |
| C4.4d1b | Independent real-font Type 1 execution reference | Source implemented; CI pending |
| C4.4d2a | Bounded Type 2 multiple substitution and mixed Type 1/2 execution | Planned |
| C4 | Fonts and glyph shaping/layout | Planned |
| C5 | Automatic pagination | Planned |
| C6 | Advanced graphics and table layout | Planned |
| C7 | Mathematical formula engine | Planned |
| C8 | Research references and cross references | Planned |
| C9 | DOCX writing and round-trip fidelity | Planned |
| C10 | Legal real-paper corpus, compatibility, security/performance | Planned |
| C11 | Electron integration and desktop acceptance | Planned |

Gate every code batch: actual GitHub-hosted Windows/Linux/macOS tests and locked Release builds at same revision, retained regression tests, documented limitations. Stage C10/C11 require actual corpus/reference/device evidence; unit tests do not replace those gates.
