# C 线路当前研发检查点

- Last verified baseline: ddda4958732c114ddc9903221aec1c0fb1b71316
- CI #60: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45560027804
- Verified: Ubuntu 304/304, Windows 304/304, macOS 304/304; total 912/912; zero failed/ignored; all release builds passed.
- Current stage: C3.3a bounded in-memory undo/redo, incremental run text and anchor snapshots; 12 new regressions (316 per platform / 948 executions expected, PENDING CI).
- Next: verify actual HEAD and all three platform logs; repair evidenced failures only. On pass C3.3b structural transactions and validation (start with paragraph/run split and join constraints).
- Scope: private document ownership, session-local revision guards, C3.1 staged edits and C3.2 selection plan reuse, before/after text plus both anchor offset arrays, atomic rejection, oldest undo eviction, redo invalidation on real new edits. Limits count retained text bytes/offsets/run changes/entries, not allocator overhead. No persistent journal, crash recovery, structural history, external mutable access, or DOCX serialization.
- Draft PR #1 remains open; no merge. See HANDOFF.md and KNOWN_ISSUES.md.
