# C 线路当前研发检查点

- Verified baseline 4c41a27ccc65748b82e8344f48e9b7378a615ce3, CI #91 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45624378851 : 597/platform, 1791 total, zero failed/ignored, locked Release verified.
- Current C4.4a6a: native standalone LigatureSubst format 1 parser, font-order contiguous matching, bounded rule and component counts, atomic run contraction; ten new tests (607/platform, 1821 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a6b real-font direct type 4 reference and integration. No GDEF filtering, cluster merging, lookup scheduling, extension lookup execution or full shaping yet.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
