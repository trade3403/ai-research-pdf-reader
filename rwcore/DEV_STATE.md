# C 线路当前研发检查点

- Last verified baseline: f0485362c0a67de8c586a6b6f3ad42634e604020
- CI #64: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45570061454
- Verified: Ubuntu 354/354, Windows 354/354, macOS 354/354; total 1062/1062; zero failed/ignored; locked release builds passed.
- Current stage: C4.2a native head/maxp/hhea/hmtx metrics and integer font scaling; 14 new synthetic regressions (368 per platform / 1104 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.2b bounded Unicode cmap format 4/12 mapping and glyph-ID validation.
- Scope: strict metric-table lengths and versions, signed bounds/metrics, TrueType maxima, CFF maxp 0.5, borrowed hmtx with repeated final advance, signed rational twips and 26.6 pixel scaling with nearest ties away from zero. Default instance only; no cmap, outlines, vertical metrics, OS/2 metric selection, shaping, real font corpus or rendering.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
