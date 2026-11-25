# C 线路当前研发检查点

- Last verified baseline: ebaddc59e4aabc93e41dd4646be6aba5957f06ae
- CI #63: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45568272949
- Verified: Ubuntu 340/340, Windows 340/340, macOS 340/340; total 1020/1020; zero failed/ignored; locked release builds passed.
- Current stage: C4.1 native SFNT/TTC font directory parsing; 14 new synthetic regressions (354 per platform / 1062 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.2a native head/maxp/hhea/hmtx metric parsing and unit conversions; cmap follows.
- Scope: raw TrueType and OpenType CFF-flavored SFNT directory, TTC/OTC versions 1/2, borrowed table lookup, bounded checked offsets/counts, shared table validation, optional table checksum and raw DSIG exposure. No required-table or whole-font validation, glyph outlines, cmap, shaping, WOFF, font discovery, rendering, or real font corpus acceptance. C3 foundational editing verified, not full Word fidelity.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
