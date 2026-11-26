# C 线路当前研发检查点

- Verified baseline 617e6226ac45a02bca12860ac995d491a6f04447, CI #124 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45735200503 : 858/platform, 2574 total, zero failed/ignored; exact revision, locked tests and Release builds verified from three job logs.
- Current C4.4d1b: seven existing real-font Type 1 execution reference tests. Independent raw SFNT/GSUB/GDEF coverage, replacement maps, class and mark-set expansion, root selection and out-of-place execution; every single root over all glyph IDs, aggregate ordered roots over forward/reverse/repeated input, exact counters and budget failures. 865/platform, 2595 expected; CI pending.
- Next verify exact HEAD and logs, repair evidenced failures. Then C4.4d2a bounded Type 2 multiple substitution and mixed Type 1/2 root execution, with explicit expansion and cursor semantics. Nested actions, clusters and full shaping remain separate.
- Native Type 8 reference and C4.3h-2e2b real point attachment outstanding. FeatureVariations, script stages, masks, runtime nested guards, clusters, GPOS, deferred GDEF children, DOCX corpus and security/license audit outstanding.
