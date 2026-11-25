# C 线路研发路线

Status below is baseline at CI #66, not full Word compatibility.

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
| C4.2c | Nominal glyph metrics and text measurement integration | Source implemented; CI pending |
| C4.3a | Native TrueType loca/glyf simple outlines | Planned |
| C4 | Fonts and glyph shaping/layout | Planned |
| C5 | Automatic pagination | Planned |
| C6 | Advanced graphics and table layout | Planned |
| C7 | Mathematical formula engine | Planned |
| C8 | Research references and cross references | Planned |
| C9 | DOCX writing and round-trip fidelity | Planned |
| C10 | Legal real-paper corpus, compatibility, security/performance | Planned |
| C11 | Electron integration and desktop acceptance | Planned |

Gate every code batch: actual GitHub-hosted Windows/Linux/macOS tests and locked Release builds at same revision, retained regression tests, documented limitations. Stage C10/C11 require actual corpus/reference/device evidence; unit tests do not replace those gates.
