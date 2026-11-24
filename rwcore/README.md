# RWCore — C1.1 OPC package graph

Independent Rust workspace for research manuscripts, on feature/rwcore.
The existing Tiptap Word editor remains unchanged. No Electron or RPCore integration in this milestone.

## Scope
- Read-only, bounded ZIP/OPC input using general-purpose zip and roxmltree crates.
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
Not a full OPC or DOCX implementation. Conservative ASCII URI subset (percent escapes, fragments, queries and case-mismatched lookups are rejected); UTF-8 XML only, basic paragraphs only. Resource detection is metadata only, not image decoding or style interpretation. Missing MIME declarations on non-main parts are exposed as None.
No style resolution, tables, images, formulas, pagination, editing, export or layout fidelity yet.
No macros executed, no extraction, no network access. Archive/part/entry/node limits and DTD rejection are initial defenses, not a security audit. ZIP library may normalize or collapse duplicate names before the application sees them; strict central-directory validation is deferred.
Dependencies are provisional; generate and commit Cargo.lock after first successful CI, then review advisories and licenses. Never replace or overwrite a source DOCX.
No claim of Word compatibility or GUI testing.

## Roadmap
C1.0 foundations and bounded read-only probe.
C1.1 OPC graph and bounded URI resolution implemented in source; build and real corpus not yet verified.
C1.2 strict ZIP central-directory/local-header validation, URI compatibility, lockfile and dependency review, real DOCX corpus regression.
C2 styles, numbering, sections, tables, media and unsupported-node preservation.
C3 transactions, selection and undo; C4 typography; C5 pagination; C6 advanced layout;
C7 math; C8 citations/fields; C9 DOCX writer; C10 reference comparison and safety; C11 Electron integration.
Use ECMA-376 / ISO/IEC 29500 as reference; share renderer-neutral interfaces with RPCore after both APIs stabilize. Do not restructure A or B from this branch.

## C1.1 validation

- 10 additional synthetic regression cases in core/tests/package.rs; 10 C1.0 cases retained.
- Run the existing Linux/Windows/macOS CI matrix. No successful execution is asserted here.
- Local corpus command (use lawful local test documents, no upload):

```sh
cargo run --manifest-path rwcore/Cargo.toml -p rwcore --example corpus_probe -- sample1.docx sample2.docx
```

The probe reports package/edge/block/warning counts and nonzero exit on parse errors. Success means the supported subset was read, not that layout or content fidelity was achieved. Full ZIP integrity validation, dependency lockfile and real-world corpus results remain pending.
