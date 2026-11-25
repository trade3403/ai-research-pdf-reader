# C 线路当前研发检查点

- Last verified baseline: 3c55953c02ffadb29632d0e7a9e2fec729723aa2
- CI #65: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45570569717
- Verified: Ubuntu 368/368, Windows 368/368, macOS 368/368; total 1104/1104; zero failed/ignored; locked release builds passed.
- Current stage: C4.2b native Unicode cmap 4/12 mapping; 16 new synthetic regressions (384 per platform / 1152 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.2c nominal Unicode glyph metrics and text measurement integration (not shaping).
- Scope: deterministic Unicode subtable choice, bounded selected format 4 segments and relative glyph array validation, format 12 group bounds and glyph count validation, binary search, missing glyph None and invalid scalar error. Selected subtable only; no per-character fallback, symbol mapping, variation selector sequences, shaping, rasterization or real font corpus.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
