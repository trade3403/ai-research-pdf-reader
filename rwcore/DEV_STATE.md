# C 线路当前研发检查点

- Last verified baseline: 78159a5c3f460bc8f648420ac5be05184b40afbd
- CI #68: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45572887999
- Verified: Ubuntu 412/412, Windows 412/412, macOS 412/412; total 1236/1236; zero failed/ignored; locked release builds passed.
- Current stage: C4.3b native TrueType composite decoding and recursive point assembly; 14 new synthetic regressions (426 per platform / 1278 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3c unified simple/composite quadratic paths and exact/conservative curve bounds.
- Scope: signed byte/word offsets, real point attachment, F2Dot14 transforms with 16.16 rounding, scaled/unscaled offsets, inert instructions, metric source metadata, recursive assembly and cycle/work limits. No phantom-point attachment, grid fitting, variation deltas, transformed advance metrics, composite path emission or real font corpus acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
