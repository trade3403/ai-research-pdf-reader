# Word editor dependency licenses — developer audit notes

| Dependency | Expected license | Use |
|---|---|---|
| @tiptap/* open-source packages | MIT | Rich text editor and table/format extensions |
| ProseMirror (@tiptap/pm) | MIT | Editing engine |
| mammoth | BSD-2-Clause | DOCX semantic import |
| docx | MIT | Native DOCX and OMML export |
| DOMPurify | Apache-2.0 OR MPL-2.0 (choose Apache-2.0) | HTML sanitization |
| KaTeX | MIT | Local formula display |

Do not install `@tiptap-pro/*` or commercial pagination/conversion components.

Before shipping a commercial release, run dependency license scanning against the exact
resolved package lock, retain all third-party notices, and review all transitive packages.
This preliminary list is **not** a complete legal compliance certification.
