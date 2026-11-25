# C 线路当前研发检查点

- Last verified baseline: c15c08f3ab9240738cd94938792b0d7793b31d69
- CI #58: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45544241000
- Verified: Ubuntu 280/280, Windows 280/280, macOS 280/280; total 840/840; zero failed/ignored; all release builds passed.
- Current stage: C3.1 atomic text-only transactions, Unicode position conversion and safe picture anchor maintenance; 12 new regressions (292 per platform / 876 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On success C3.2 selection/range operations and cross-run editing; history/undo follows.
- Scope: existing body and nested body-table runs, staged touched-run copies, sequential exact expected-text checks, UTF-8/scalar/UTF-16 conversion and explicit anchor affinity/interior policy. No story edits, structural edits, grapheme segmentation, undo history, versioning or DOCX write. Text byte budgets exclude metadata heap size. Public direct model mutation still bypasses safety.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
