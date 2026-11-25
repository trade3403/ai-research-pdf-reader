# C 线路当前研发检查点

- Verified baseline ce977a7d85c353f0f5fc314d3fb49ed2ff0c25cd, CI #89 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45620243980 : 584/platform, 1752 total, zero failed/ignored, locked Release verified.
- Current C4.4a5a: native standalone AlternateSubst format 1 parser, ordered candidate sets, explicit zero-based selection, atomic in-place replacement; ten new tests (594/platform, 1782 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a5b real-font direct type 3 reference and integration, then C4.4a6 ligature substitution. No scheduling, GDEF filtering, cluster propagation, extension lookup execution or full shaping yet.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
