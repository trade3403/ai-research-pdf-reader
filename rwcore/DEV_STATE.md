# C 线路当前研发检查点

- Verified baseline 55fe5902e8e661ea1dd5cadca233ce180a338c9e, CI #128 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45739699180 : 893/platform, 2679 total, zero failed/ignored; exact revision, locked tests and Release builds verified from three job logs.
- Current C4.4d3b: independent raw Type 3 AlternateSet decoding and candidate reference; seven real-font tests (Alegreya and Padauk have Type 3, five controls), all eligible candidate indices, per-glyph one-past-end failures, misses with invalid index, combined Type 1/2/3 and alternate-only passes, first and highest universally valid choices, exact counters and one-less limits; legacy mixed compatibility. 900/platform, 2700 expected; CI pending.
- Next verify exact HEAD and logs, repair evidenced failures. Then C4.4d4a bounded Type 4 ligature execution with filtered logical positions, consumed-glyph accounting and mixed Type 1/2/3/4 root scheduling.
- Native Type 8 reference and C4.3h-2e2b real point attachment outstanding. FeatureVariations, script stages, masks, runtime nested guards, clusters, GPOS, deferred GDEF children, DOCX corpus and security/license audit outstanding.
