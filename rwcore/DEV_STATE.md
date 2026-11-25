# C 线路当前研发检查点

- Verified baseline b699b974c22bf1cfcdbfd11920867bf3f5bc0b9d, CI #93 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45628385509 : 611/platform, 1833 total, zero failed/ignored, locked Release verified.
- Current C4.4a7a: native standalone GSUB type 5 format 1 glyph-context parser, ordered prefix matching, nullable rule sets, empty-action matches, validated lookup/action indices and bounded budgets; ten new tests (621/platform, 1863 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a7b shared ClassDef parser and contextual format 2, then format 3 and chaining. Nested lookup execution/cycle guards, GDEF filtering, cluster merging and full shaping remain outstanding.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
