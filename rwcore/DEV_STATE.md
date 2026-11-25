# C 线路当前研发检查点

- Verified baseline eb679ec59e05f76e3d2d4a19a44b92bcbdfed658, CI #92 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45626340014 : 607/platform, 1821 total, zero failed/ignored, locked Release verified.
- Current C4.4a6b: independently extracted real direct GSUB type 4 reference across four font files (18 subtables, 142 rules), full glyph-domain coverage, ordered prefix matching and atomic run contraction; four new tests (611/platform, 1833 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a7a standalone contextual substitution format 1 parser and matching. No GDEF filtering, cluster merging, lookup scheduling, extension lookup execution or full shaping yet.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
