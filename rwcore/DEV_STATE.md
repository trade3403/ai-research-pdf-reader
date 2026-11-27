# C 线路当前研发检查点

- CI #184: 56cb2c7aaa389ea6cf5d06b00f7d44b131d98330, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45956312488 : 1219/platform, 3657 total, zero failed/ignored; exact SHA and Release verified in three job logs.
- C4.5g2: independent test-only raw-byte native MarkToLigature oracle across nine retained fonts, Type 5 and extension traversal, coverage, component/class matrix, all covered component/mark pairs, world-coordinate equality, separate per-bit Device corrections and explicit unresolved errors. Positive gates for native subtables, marks, multi-component ligatures, anchors, pairs, unequal coordinates and resolved pixel checks. One new test; 1220/platform expected, CI pending. No external shaper parity, automatic component selection, outline/variation resolution or attachment-chain integration. Next C4.5h1 MarkToMark foundation.
- GPOS full execution, joining/syllable analysis, Unicode reordering, avar 2.0, gvar/HVAR/MVAR, real point attachment, DOCX corpus and security/performance remain outstanding.


CI #185 failed: 00da1e0dc2e43b8426378cfcf3136b000b6af156, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45958213747 : each platform 919 passed, 1 failed, 0 ignored, 300 not run, Release skipped; exact SHA and native evidence [1,178,18,62,3436,3436,0] verified. Last green #184: 1219/platform.

C4.5g2 repair: retain nine full-font reference checks and original aggregate positive gates; add exact 370-byte DejaVu Sans 2.37 native Type 5 subtable excerpt (source and excerpt SHA-256, license retained), plus independent synthetic differential nonzero Device check. Native excerpt has coordinate anchors, not real nonzero Device deltas; native contour/variation resolution and real nonzero Device fixture remain outstanding. 1221/platform expected, CI pending. Next C4.5h1 only after green.
