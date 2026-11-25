# C 线路当前研发检查点

- Last verified baseline: 487f5f3c400be62488a6b7cd0973c4436f153f64
- CI #66: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45571150099
- Verified: Ubuntu 384/384, Windows 384/384, macOS 384/384; total 1152/1152; zero failed/ignored; locked release builds passed.
- Current stage: C4.2c same-face nominal Unicode text measurement; 14 new synthetic regressions (398 per platform / 1194 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3a native TrueType loca/glyf simple outline parsing.
- Scope: single face, logical Unicode scalar order, UTF-8 source offsets, explicit missing glyph policy, bounded input and result count, cumulative design-unit scaling to twips or 26.6 device pixels, raw hhea line metrics. No actual shaping, font fallback, control layout, kerning, ink bounds, bidi, normalization, grapheme handling or rendering. Real font corpus pending.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
