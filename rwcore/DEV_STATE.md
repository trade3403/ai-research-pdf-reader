# C 线路当前研发检查点

- Last verified baseline: 5dbe493b11d78780cf09c05288ffe884dfe9a855
- CI #62: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45567776959
- Verified: Ubuntu 328/328, Windows 328/328, macOS 328/328; total 984/984; zero failed/ignored; locked release builds passed.
- Current stage: C3.3c unified text/structure history; 12 new synthetic regressions (340 per platform / 1020 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C4.1 native bounded font container and SFNT table directory parsing (first font-engine batch).
- Scope: existing body/nested body-cell paragraph range snapshots, shared LIFO text/structure undo/redo, section and picture metadata restore, revision-checked index path resolution, bounded retention. No automatic path remapping or stable node IDs; callers refresh paths after any revision. History counts logical text, offsets and run/paragraph/section snapshot slots, not heap bytes. No persistence, structural edits beyond C3.3b, or GUI/DOCX serialization.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
