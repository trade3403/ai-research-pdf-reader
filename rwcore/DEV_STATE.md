# C 线路当前研发检查点

- Verified baseline 5b0dcdb830f4c68e957c8b971db4f3dcf98477c5, CI #104 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45673747940 : 690/platform, 2070 total, zero failed/ignored, locked tests and Release verified. Real direct targets in synthetic wrappers passed, not native Type 7 acceptance.
- Current C4.4a9a: standalone GSUB Type 8 Format 1 reverse chained single substitution, checked coverage-indexed mappings and cumulative budgets, nearest-first backtrack and forward lookahead, bounded right-to-left in-place pass with atomic prevalidation. Type 7 dispatch to Type 8 integrated. Twelve new synthetic tests (702/platform, 2106 expected); existing Type 8 truncated target test updated for supported decoding. CI pending.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4a9b real native Type 8 reference where available. C4.4a8b2 native Type 7 fixture outstanding. Nested execution/cycle guards, GDEF filtering, cluster merging and full shaping outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
