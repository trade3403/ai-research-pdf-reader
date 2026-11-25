# C 线路当前研发检查点

- Last verified baseline: 7bfbb839ded46b4bdd1346f4d73e1707d9d37a7c
- CI #57: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543111414
- Verified: Ubuntu 272/272, Windows 272/272, macOS 272/272; total 816/816; zero failed/ignored; all release builds passed.
- Current stage: C2.4f document/media/drawing integration; 8 new synthetic regressions (280 per platform / 840 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs, repair evidenced failures only. On pass advance C3.1 native document tree editing transactions and anchor-safe operations.
- Scope: shared body/table/story drawing parser, source-order mixed picture index, scoped resource binding and header dimensions, explicit geometry state, restricted frame reconciliation and floating layout bridge. No pixel decode/paint, real DOCX corpus, complete line breaking, collision or pagination. Existing inline view retained; new public Run field requires downstream struct literal migration if not using defaults.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
