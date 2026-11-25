# C 线路当前研发检查点

- Last verified baseline: 957b441929da50dbc25d5fb37cdccc5f4cfa5827
- CI #70: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45577206257
- Verified: Ubuntu 440/440, Windows 440/440, macOS 440/440; total 1320/1320; zero failed/ignored; locked release builds passed.
- Current stage: C4.3d bounded quadratic flattening and contour geometry; 14 new synthetic regressions (454 per platform / 1362 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3e native nonzero/even-odd winding and bounded grayscale glyph rasterization.
- Scope: per-curve dyadic subdivision with proven componentwise parameter-matched chord deviation bound including vertex rounding, i128 polynomial evaluation, closed ordered contours, signed polygon area/orientation, polygon bounds and resource limits. No pixel mapping, fill, coverage, hinting, CFF or real font corpus acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
