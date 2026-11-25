# C 线路当前研发检查点

- Last verified baseline: 7085409da96d2d07a54ffdfc713b44c12602ef05
- CI #72: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45579802795
- Verified: Ubuntu 468/468, Windows 468/468, macOS 468/468; total 1404/1404; zero failed/ignored; locked release builds passed.
- Current stage: C4.3f integrated same-face Unicode-to-TrueType grayscale rendering; 14 new synthetic regressions (482 per platform / 1446 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3g bounded nominal text-run glyph bitmap placement and shared-face cumulative advances, followed by real-font/reference validation.
- Scope: same-face cmap/metrics/outlines, glyph-ID and scalar entrypoints, explicit missing policy, exact rational outward device viewport, baseline 26.6, design tolerance derived from 16.16 pixel tolerance including rounding, original hmtx advances and separate composite metric-source metadata. Unhinted grayscale supersampling only; no transformed phantom metrics, text-run composition, shaping, CFF, performance or real font corpus acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
