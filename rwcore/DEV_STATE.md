# C 线路当前研发检查点

- Verified baseline e1b6c38443d121628160dc046a018b2f34afb99a, CI #129 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45740236780 : 900/platform, 2700 total, zero failed/ignored; exact revision, locked tests and Release builds verified from three job logs.
- Current C4.4d4a: bounded autonomous Type 4 direct/wrapped ligature execution in Type 1/2/3/4 root passes, precomputed eligible logical positions, first font-ordered matching rule/subtable, preserve ignored gaps, consume trailing components once, no same-root rescan, count ligatures and removed glyphs, rule-attempt and component-comparison caps plus existing mixed budgets, strict legacy APIs. Ten new synthetic tests (910/platform, 2730 expected); CI pending.
- Next verify exact HEAD and logs, repair evidenced failures. Then C4.4d4b independent real-font Type 4 and mixed Type 1/2/3/4 execution reference.
- Native Type 8 reference and C4.3h-2e2b real point attachment outstanding. FeatureVariations, script stages, masks, runtime nested guards, clusters, GPOS, deferred GDEF children, DOCX corpus and security/license audit outstanding.
