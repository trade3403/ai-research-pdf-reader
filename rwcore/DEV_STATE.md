# C 线路当前研发检查点

- Last verified baseline: ba91c2ae8f6180494ed9f4490629738873638872
- CI #55: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541593350
- Verified: Ubuntu 248/248, Windows 248/248, macOS 248/248; total 744/744; zero failed/ignored; all release builds passed.
- Current stage: C2.4d-2 signed EMU floating position and single-anchor rectangular text-wrap engine; 12 new regressions (260 per platform / 780 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair only evidenced failures. On pass advance C2.4e picture crop, rotation and geometry.
- Scope: measured frames supplied by caller, single-anchor line band intervals, explicit tight/through fallback. No automatic pagination, contour geometry, multi-anchor collision/flow, actual cell layout, drawing or document-tree integration.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
