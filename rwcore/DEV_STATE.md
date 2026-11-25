# C 线路当前研发检查点

- Verified baseline 26799e5f9f2f2991bb0f113af268ebfd6c6d7502, CI #97 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45636896408 : 643/platform, 1929 total, zero failed/ignored, locked Release verified. macOS first attempt cancelled without logs; retry succeeded.
- Current C4.4a7e1: standalone GSUB Type 6 Format 1 glyph-based chaining. First covered input, nearest-first backtrack, forward lookahead, font-order first match including empty actions; input-relative checked action indices; cumulative rule/glyph/action limits charge shared offsets. Ten new synthetic tests (653/platform, 1959 expected); CI pending. No nested execution, GDEF filtering, class/coverage chaining, shaping or real Type 6 font acceptance.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a7e2 Type 6 chained context format 2 class parser and matching. Type 6 format 3, real-font references, nested execution/cycle guards, GDEF filtering, cluster merging and full shaping remain outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
