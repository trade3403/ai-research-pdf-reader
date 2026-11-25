# C 线路当前研发检查点

- Last verified baseline: 5f1785db549ff47f0a966d549a45df1692eca63c
- CI #56: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45542423200
- Verified: Ubuntu 260/260, Windows 260/260, macOS 260/260; total 780/780; zero failed/ignored; all release builds passed.
- Current stage: C2.4e standalone picture crop and reversible geometry mapping; 12 new regressions (272 per platform / 816 executions expected, PENDING CI).
- Next: verify actual HEAD and three platform logs, repair evidenced failures. On pass C2.4f media/document integration acceptance (including wiring floating and geometry metadata to supported document nodes).
- Scope: standalone pic:pic metadata, nonnegative integer srcRect, explicit xfrm off/ext, flip before clockwise rotation about center, inverse mapping, outward pixel window and floating-point corner bounds. No pixel decode/render, group transforms, shape/anchor reconciliation or full document integration.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
