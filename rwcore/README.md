# RWCore 0.1.0 — C1.0 bootstrap

Independent Rust workspace for research manuscripts, on feature/rwcore.
The existing Tiptap Word editor remains unchanged. No Electron or RPCore integration in this milestone.

## Scope
- Read-only, bounded ZIP/OPC input using general-purpose zip and roxmltree crates.
- Validate conventional main-document content type and package relationship; read word/document.xml.
- Namespace-aware Word paragraphs/runs, Unicode text, tabs, line breaks, diagnostics for unsupported content.
- Renderer-neutral document tree and integer twip type for future layout.
- Synthetic DOCX tests and inspect CLI.

## Run
```sh
cargo test --manifest-path rwcore/Cargo.toml --workspace
cargo run --manifest-path rwcore/Cargo.toml -p rwcore --example inspect -- sample.docx
```

## Boundaries
Not a full OPC or DOCX implementation. Fixed conventional main part, UTF-8 only, basic paragraphs only.
No style resolution, tables, images, formulas, pagination, editing, export or layout fidelity yet.
No macros executed, no extraction, no network access. Archive/part/entry/node limits and DTD rejection are initial defenses, not a security audit. ZIP library may normalize or collapse duplicate names before the application sees them; strict central-directory validation is deferred.
Dependencies are provisional; generate and commit Cargo.lock after first successful CI, then review advisories and licenses. Never replace or overwrite a source DOCX.
No claim of Word compatibility or GUI testing.

## Roadmap
C1.0 foundations and bounded read-only probe.
C1.1 OPC relationship graph, safe part URI resolution, ZIP consistency and real corpus.
C2 styles, numbering, sections, tables, media and unsupported-node preservation.
C3 transactions, selection and undo; C4 typography; C5 pagination; C6 advanced layout;
C7 math; C8 citations/fields; C9 DOCX writer; C10 reference comparison and safety; C11 Electron integration.
Use ECMA-376 / ISO/IEC 29500 as reference; share renderer-neutral interfaces with RPCore after both APIs stabilize. Do not restructure A or B from this branch.
