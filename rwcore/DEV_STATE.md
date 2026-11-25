# C 线路当前研发检查点

- Verified baseline bd70a794012ac533d2e23718d34c4f12f5d34dfd, CI #102 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45667792319 : 675/platform, 2025 total, zero failed/ignored, locked tests and Release verified. All 321 selected direct Type 6 subtables passed.
- Current C4.4a8a: standalone GSUB Type 7 Format 1 wrapper, checked unsigned 32-bit relative target offset, borrowed target, same-type extension lookup resolution, bounded typed dispatch to existing Type 1-6 parsers (all context formats). Type 8 metadata accepted but parser explicitly unsupported. Twelve synthetic tests (687/platform, 2061 expected); CI pending.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4a8b real-font Type 7 wrapper reference acceptance where available, then Type 8 reverse chaining. Nested execution/cycle guards, GDEF filtering, cluster merging and full shaping outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
