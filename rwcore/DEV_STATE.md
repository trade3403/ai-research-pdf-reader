# C 线路当前研发检查点

- Verified baseline 0c634b3a9772ed98b0facfc13ddcb031d8410eb2, CI #107 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45682244347 : 706/platform, 2118 total, zero failed/ignored, locked tests and Release verified. All 1503 native Nastaliq Type 7 targets independently checked.
- Current C4.4b1: GDEF 1.0/1.2/1.3 directory, nullable 16-bit and 32-bit offsets, GlyphClassDef classes 0..4 and MarkAttachClassDef classes 0..255, cumulative stored-record/explicit-glyph budgets, independent query interfaces; deferred child offsets entry-checked only. Twelve synthetic tests (718/platform, 2154 expected); CI pending.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4b2 MarkGlyphSetsDef format 1 with 32-bit coverage offsets and cumulative limits; then real GDEF reference, lookup flag filtering and scheduling. C4.4a9b native Type 8 reference still fixture-blocked.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding. Nested execution/cycle guards, GDEF filtering, cluster merging, real DOCX corpus and security/license audit outstanding.
