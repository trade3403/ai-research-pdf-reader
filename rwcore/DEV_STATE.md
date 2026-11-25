# C 线路当前研发检查点

- Verified baseline a4410b5e8ceb6b43b6625a364441202d104173b6, CI #103 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45670719213 : 687/platform, 2061 total, zero failed/ignored, locked tests and Release verified.
- Current C4.4a8b1: original fixture census (7 files, 6 GSUB fonts, no native Type 7); real direct Type 1-6 targets wrapped at synthetic offsets 8 and 70000, comparison to direct parsing, all 321 Type 6 subtables and selected Type 1-5. Three new tests (690/platform, 2070 expected); CI pending. No original native Type 7 font acceptance: extra checked Google Fonts Noto Arabic/Bengali/Tamil/Myanmar/Khmer/Balinese also lacked native wrappers; larger Noto Sans/Serif retrieval incomplete. No new fonts committed.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4a9a Type 8 reverse chained single substitution, independent unblocked work. C4.4a8b2 original native Type 7 font reference remains outstanding. Nested execution/cycle guards, GDEF filtering, cluster merging and full shaping outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
