# C 线路当前研发检查点

- Last verified baseline: 0ee51291d9b4dc777bf346ea7d09c556cc287d2b
- CI #69: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45573801021
- Verified: Ubuntu 426/426, Windows 426/426, macOS 426/426; total 1278/1278; zero failed/ignored; locked release builds passed.
- Current stage: C4.3c unified simple/composite quadratic path and exact-rational outward bounds; 14 new synthetic regressions (440 per platform / 1320 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3d bounded adaptive quadratic flattening and contour geometry for raster preparation.
- Scope: validated assembled points and contour coverage, exact 17-fractional-bit implied midpoints, closed paths, independent axis extrema with integer rational floor/ceil, separate control hull bounds and metric-source metadata. Geometry is based on prior rounded 16.16 component assembly; not hinted ink bounds, fill, rasterization, CFF or real font corpus acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
