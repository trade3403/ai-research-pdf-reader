# C 线路当前研发检查点

- Last verified baseline: b526964d3d4035e4174e2c68853b7af05816e21c
- CI #71: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45577647853
- Verified: Ubuntu 454/454, Windows 454/454, macOS 454/454; total 1362/1362; zero failed/ignored; locked release builds passed.
- Current stage: C4.3e native nonzero/even-odd grayscale glyph rasterization; 14 new synthetic regressions (468 per platform / 1404 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3f integrated Unicode-to-TrueType bitmap pipeline, automatic device bounds, pixel-space flattening tolerance and glyph placement.
- Scope: caller viewport, 26.6 ppem/baseline to bounded 16.16 device geometry, exact rational scanline intersections, half-open edge convention, winding fill and regular 1/2/4/8/16 sample grids, 8-bit alpha coverage, explicit pixel/edge/sample/scan budgets. No analytic coverage, optimized active-edge rasterization, hinting, LCD/gamma, CFF, real-font or visual acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
