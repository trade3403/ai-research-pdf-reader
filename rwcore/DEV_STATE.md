# C 线路当前研发检查点

- Last verified baseline: 1ad6cd1909a7e79349747991bcf04f7631818710
- CI #59: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45555498029
- Verified: Ubuntu 292/292, Windows 292/292, macOS 292/292; total 876/876; zero failed/ignored; all release builds passed.
- Current stage: C3.2 paragraph text index, directional selection and atomic cross-run replacement; 12 new synthetic regressions (304 per platform / 912 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On success C3.3a bounded undo/redo transaction history; structural operations follow separately.
- Scope: single existing body or nested-table paragraph, UTF-8/scalar/UTF-16 prefix counts, explicit shared-boundary run affinity, selected text, start-run replacement formatting, local anchor preservation or global ordered-anchor rejection. Existing run structure and images retained. No cross-paragraph/field editing, grapheme segmentation, history or serialization.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
