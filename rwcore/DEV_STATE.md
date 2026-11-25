# C 线路当前研发检查点

- Verified baseline e4c1330824546387241b099b95bf3219119f7882, CI #108 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45685120002 : 718/platform, 2154 total, zero failed/ignored, locked tests and Release verified.
- Current C4.4b2: MarkGlyphSetsDef format 1, checked 32-bit relative Coverage offsets, coverage formats 1/2, cumulative set/record/expanded glyph budgets and alias charging, independent set membership API, GDEF 1.2/1.3 integration. Existing directory and class tests retained; directory-only invalid child fixture now explicitly distinguishes directory parsing from strict child decoding. Twelve new tests (730/platform, 2190 expected); CI pending.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4b3 real GDEF class and mark-set reference acceptance where available; then lookup flag filtering and scheduling. C4.4a9b native Type 8 reference still fixture-blocked.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding. AttachList, LigCaretList and ItemVariationStore child decoding, nested execution/cycle guards, GDEF filtering, cluster merging, real DOCX corpus and security/license audit outstanding.
