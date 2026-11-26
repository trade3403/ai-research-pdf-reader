# C 线路当前研发检查点

- Verified baseline 4f7af8979b7a78d11f0d856bb01937f0c5b193ff, CI #126 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45737802212 : 876/platform, 2628 total, zero failed/ignored; exact revision, locked tests and Release builds verified from three job logs.
- Current C4.4d2b: independent raw Type 2 sequence decoding, dense per-glyph reference decisions and independent output construction, seven real-font tests (four with Type 2, three Type 1-only controls); compare every Type 1/2 root over all glyph IDs, combined roots and Type 2-only passes over ordered/reversed/repeated input, exact counters and one-less budgets. Legacy Type 1 results cross-checked. 883/platform, 2649 expected; CI pending.
- Next verify exact HEAD and logs, repair evidenced failures. Then C4.4d3a bounded Type 3 alternate substitution execution and explicit selection policy in mixed roots.
- Native Type 8 reference and C4.3h-2e2b real point attachment outstanding. FeatureVariations, script stages, masks, runtime nested guards, clusters, GPOS, deferred GDEF children, DOCX corpus and security/license audit outstanding.
