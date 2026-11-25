# C 线路当前研发检查点

- Verified baseline 577697d5047dfd499b3b16321aac9004e7bec69f, CI #90 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45622245972 : 594/platform, 1782 total, zero failed/ignored, locked Release verified.
- Current C4.4a5b: original OFL Alegreya Sans fixture and independent real direct GSUB type 3 reference (165 mappings, 417 candidates), full domain and candidate selection tests, negative existing fixtures. Three new tests (597/platform, 1791 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a6a standalone ligature substitution. No scheduling, GDEF filtering, cluster propagation, extension lookup execution or full shaping yet.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
