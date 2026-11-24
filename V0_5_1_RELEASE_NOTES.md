# AI Research PDF Reader V0.5.1 — SCI academic layout + code version management

## Scope implemented

1. Stable ID for every new block equation and figure/table caption, including migration of old equations without IDs.
2. Equation, figure and table numbering derived from document order. Deleting, inserting or moving nodes recalculates numbering in the local editor preview and when exporting DOCX.
3. Cross-reference inline nodes keyed to stable target ID. Missing references are explicitly marked and never silently renumbered to a different target.
4. Literature-library citation nodes containing a bibliography snapshot (title, author, journal, year, DOI), with numeric `[1]` and author-year display modes.
5. Bibliography node auto-generates entries from first-citation order; duplicate citations use the same number. Referenced records remain printable even if removed from the local literature library after insertion.
6. DOCX export renders resolved formula labels, captions, cross-references and reference list, with native mathematical OMML content where supported by docx.js.
7. A4/Letter, margins, line spacing, configurable header/footer text and Word-native page-number fields.
8. General/IEEE-like/Elsevier-like/Springer-like **structural examples only**. They are not official publisher templates and cannot be treated as submission-ready without manual review.
9. Print / Save PDF via operating-system print dialog (not a standalone PDF converter).
10. GitHub Actions validates versions on `vX.Y.Z` tag builds and provides a manual **Publish Verified Windows Release** workflow that only tags/publishes a tested main-branch commit using previously verified NSIS EXE and SHA-256.
11. Manual code rollback workflow opens a Pull Request restoring source from a stable Git tag; never force-pushes `main` or changes the user's local research database.

## Data compatibility

- Keep the original Tauri application identifier `app.researchstudio.pdfreader` and SQLite connection `research-reader-v02.db`.
- Schema migration uses `PRAGMA table_info` and additive `ALTER TABLE` columns on `word_manuscripts` for `citation_style`, `line_spacing`, `header_text`, and `footer_text`.
- Existing drafts, PDF annotations, research records, translations, literature and API key storage remain untouched.
- Before upgrading, use the existing SQLite snapshot backup feature. SQLite backups do **not** contain original PDF/DOCX files or Windows credential-manager Azure keys.

## Explicit limitations and acceptance still needed

- Numbering/cross-references/bibliography refresh automatically in **this editor** and on **DOCX export**. They export as resolved text, **not live SEQ/REF bibliography fields** in Microsoft Word; Word-side edits will not automatically update them until the file is re-exported from this application.
- Figure and table captions are user-positioned blocks. Their association with nearby images and tables is not verified automatically.
- The preview is continuous; precise page breaks, multi-column article layouts, floating figures, revisions, and round-trip preservation of complex DOCX still require additional development.
- Generic structural presets do not reproduce official publisher formatting.
- PDF printing depends on system WebView2 print dialog and user selection of an installed PDF printer.
- No claim of successful human Windows interaction or real-world DOCX round-trip testing is made solely from build success.

## Windows functional checklist

- [ ] Open legacy V0.5.0 Word draft and check contents intact after schema migration.
- [ ] Insert 3 equations, delete middle equation, verify labels become (1),(2) without changing the target of cross-reference.
- [ ] Insert two figure captions and one table caption, reorder and verify independent numbering.
- [ ] Insert one citation twice and a second citation; verify first-citation numbering and bibliography unique entries.
- [ ] Switch to author-year style and verify display updates; confirm DOCX export preserves the active style.
- [ ] Export and inspect native math equations, header/footer page number and margins in Microsoft Word or compatible editor.
- [ ] Back up SQLite; restore in a test environment, confirm existing research records and Word drafts persist.
- [ ] Verify existing PDF reader, built-in ChatGPT, Azure translation, context menu and annotation functionality.
- [ ] Review GitHub tag release assets, sha256 and PR-only rollback workflow.
