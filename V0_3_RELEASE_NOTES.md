# AI Research PDF Reader V0.3 — Literature Library

## Added

- Three-column reference library, separate from the existing V0.2 PDF reader and annotation UI.
- Import one/many PDFs or recursively import a selected folder (explicit user selection).
- Display import progress, errors, cancel remaining work, detect duplicate files with streaming SHA-256 (also checks path).
- Store literature metadata in the **same** `research-reader-v02.db` SQLite database used by V0.2. Tables are additive; existing annotations and reading states remain intact.
- Collections and nested collections, tags, favorites, metadata editing, selection filters.
- Open an indexed PDF in the V0.2 reader; PDFs are *referenced in place*, never moved or modified.
- PDF embedded title/author recognition; DOI detection from first page; user-confirmed Crossref DOI lookup (network required).
- Opt-in/offline per-document full-text indexing. Page texts are saved locally in SQLite; searchable after indexing. Index cancellation and 8 MiB text safety cap result in **partial** status.
- RIS and BibTeX citation metadata import/export. Importing references does **not** automatically locate PDF attachments.
- JSON metadata-only export. **Not** a full backup of PDFs or annotations; do not use it as a disaster-recovery substitute.

## Limitations / upcoming work

- **No Zotero cloud sync**, browser connector, citation style processor, or attachment migration yet. Design is Zotero-inspired, not an affiliation with Zotero.
- Duplicate elimination is SHA-256 for identical PDF bytes plus file path. Different PDF files of the same paper are not automatically merged; DOI duplicates for imported citation entries are skipped.
- Metadata recognition is best-effort and should be reviewed by users.
- Full-text indexing is initiated per document, rather than indexing the entire library on first import. Scanned image-only PDFs require OCR, not provided in V0.3.
- Very large libraries (thousands of files) and interactive stress testing have **not** been benchmarked.
- Import/export uses native OS file selection. Test PDF annotations and existing reading history before relying on the upgraded app.

## Testing

Build automation runs `npm run test:v03` structural checks, TypeScript/Vite build, Rust/Tauri compilation, Windows NSIS build, EXE signature check and SHA-256 hash verification. This does *not* replace Windows desktop manual tests.
