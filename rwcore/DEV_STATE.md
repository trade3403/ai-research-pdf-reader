# C 线路当前研发检查点

- Last verified baseline: 87382c6711bfefd8af0819ff593a6c43bf40c9e8
- CI #54: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541326941
- Verified baseline: Ubuntu 238/238, Windows 238/238, macOS 238/238; total 714/714; zero failed/ignored; all release builds passed.
- Current stage: C2.4d-1 bounded floating picture anchor metadata parser. Source and 10 new synthetic regressions added, 248 tests per platform / 744 total expected, PENDING actual CI. No local Rust toolchain available in this session.
- Next action: verify actual HEAD and full cross-platform CI before further development. On success C2.4d-2 position and text-wrap algorithms; on failure repair evidenced cause only.
- Scope: separate standalone anchor metadata API; shared picture payload/relationship binding; no document tree integration, positioning, wrap polygon interpretation or rendering. Existing inline API remains separate.
- Draft PR #1 remains open, no merge. See HANDOFF.md and KNOWN_ISSUES.md.
