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
