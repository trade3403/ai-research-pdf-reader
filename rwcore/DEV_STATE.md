# C 线路当前研发检查点

- Last verified baseline: a887b4580dbd8abbe015851b43d9451a073151e5
- CI #67: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45571618497
- Verified: Ubuntu 398/398, Windows 398/398, macOS 398/398; total 1194/1194; zero failed/ignored; locked release builds passed.
- Current stage: C4.3a native TrueType loca/glyf simple outlines; 14 new synthetic regressions (412 per platform / 1236 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.3b bounded composite glyph decoding, component transforms and recursive outline assembly.
- Scope: short/long loca validation, lazy simple glyf decoding, bounded contour/point/flag/instruction reads, signed delta coordinates, exact half-unit implied midpoints and closed quadratic path. Empty glyphs supported, composite detected not decoded, hints inert. No rasterization, CFF outlines, variation or real font corpus acceptance.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
