# AI Research PDF Reader — V0.5.0 Word SCI Manuscript Editor

## New features
- Dedicated **Word 论文** workspace alongside the existing library, PDF reader, ChatGPT and research records.
- Create Word manuscripts with local draft autosave in an additive `word_manuscripts` SQLite table.
- Import ordinary `.docx` files (up to 16 MB), convert to editable paragraphs, headings, lists, simple tables and embedded images using Mammoth. Sanitizes output before inserting it into the editor. Existing source file is never changed during import.
- Edit paragraphs, headings, bold/italic/underline, fonts/sizes, alignment, bullet/number lists, tables and small locally embedded images.
- A4 / Letter page preview, 20/25/30 mm margins, SCI paper structure template and document outline.
- Insert LaTeX equations rendered locally with KaTeX. DOCX export uses `latexToMath` to generate editable native Word OMML when supported by docx.js; unsupported expressions fall back to visible LaTeX with a warning in the user guide.
- Export `.docx` on Windows with native save dialog, 32 MB limit, and a sibling backup on replacement; browser preview can download generated DOCX.
- One-click PDF selected-text and ChatGPT research-note insertion, preserving source labels.
- Insert editable author/year citation text and bibliography strings from existing literature library (not native Word citation fields).
- V0.4.3 SQLite backup includes Word drafts automatically, since they live in the existing database.

## Limitations and testing
- **No guarantee of pixel-perfect DOCX round-trip**. Mammoth imports semantic content rather than all Word styles; complex section breaks, floating shapes, tracked changes, comments, advanced tables, footnotes and some equations may change or be omitted. Keep the original DOCX.
- A4 page is a continuous **visual guide**, not a pagination engine. Final page breaks are determined by the DOCX renderer (e.g. Microsoft Word).
- Importing Word OMML equations into editable LaTeX is **not implemented**. Equations created within the app export as native OMML for supported LaTeX.
- **No automatic uploading** of DOCX content to ChatGPT or Azure. The imported DOCX is processed locally.
- Native DOCX output and equation fidelity must be manually opened in Word or compatible software to complete practical acceptance.
- The Windows EXE is an **unsigned** development build.

## License approach
- Tiptap open-source core MIT; Mammoth BSD-2-Clause; docx.js MIT; DOMPurify Apache-2.0 option. Verify exact installed transitive dependencies with an SBOM before commercial distribution. No paid Tiptap Pro components or Microsoft commercial SDK used.
