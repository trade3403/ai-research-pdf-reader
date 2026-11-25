# C 线路当前研发检查点

- Last verified baseline: 8ad0fdbfb0de7091a48c0b933fe4dab3fc3907aa
- CI #61: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45566499551
- Verified: Ubuntu 316/316, Windows 316/316, macOS 316/316; total 948/948; zero failed/ignored; locked release builds passed.
- Current stage: C3.3b atomic paragraph structure transactions; 12 new regressions (328 per platform / 984 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C3.3c integrate structural history and path/revision-safe undo/redo before C4.
- Scope: one contiguous existing paragraph-only sibling range in body or nested body cell, local staged copies, split at run boundary or Unicode point, explicit anchor side, join formatting policy, section partition validation and range shifts. No cross-section/cell joins, nonparagraph structural changes, story editing or structural history yet. Standalone API must not reuse stale text paths; DocumentEditor remains text-only until C3.3c.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
