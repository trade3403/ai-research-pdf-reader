# C 线路当前研发检查点

- Verified baseline cb3c0bfda670036bb21ba7adcb57c1748b673184, CI #101 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45661629171 : 673/platform, 2019 total, zero failed/ignored, locked tests and Release verified.
- Current C4.4a7e4: test-only independent raw SFNT reference decoder for direct Type 6 formats 1/2/3; existing OFL Padauk (3/3/97 subtables) and Noto Sans Devanagari (5/1/212), 321 direct subtables total. Exact rule/action order, full first-glyph domain checks, representative reachable sequences and coverage membership mutations. Two new tests (675/platform, 2025 expected); CI pending.
- Next verify exact HEAD and logs; repair evidenced failures. C4.4a8a GSUB Type 7 extension wrapper parser and checked target dispatch. Nested execution/cycle guards, GDEF filtering, cluster merging and full shaping outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
