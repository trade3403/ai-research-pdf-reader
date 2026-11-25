# C 线路当前研发检查点

- Last verified baseline: 79f86390e7233f79cfc5632fae344148f005f094
- CI #73: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45582289379
- Verified: Ubuntu 482/482, Windows 482/482, macOS 482/482; total 1446/1446; zero failed/ignored; locked release builds passed.
- Current stage: C4.3g nominal same-face text bitmap placement and alpha compositing; 14 new synthetic regressions (496 per platform / 1488 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3h legal real TrueType font corpus / independent reference geometry and grayscale comparison harness; do not claim real-font acceptance until actual evidence.
- Scope: logical scalar order, UTF-8 ranges, explicit missing policy, cumulative hmtx advance scaling, 26.6 baseline, union of nonempty bitmap viewports, source-over alpha layers, input/glyph-pixel/total sample/output bounds. Empty spaces retain advance. Per-glyph limits remain; no exact outline-union coverage, shaping, bidi, kerning, fallback, hinting, CFF, visual corpus or performance acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
