# C 线路当前研发检查点

- Verified baseline 3640bde5249068f6c521322e5e37088df4809c85, CI #86 https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45616346690 : 568/platform, 1704 total, zero failed/ignored, locked Release verified.
- Current C4.4a3b: independent real SFNT GSUB type 1 mappings for all direct type 1 subtables in three fonts; production layout-to-subtable test integration, full glyph domain match/miss validation and no-GSUB fixture. Four new tests (572/platform, 1716 expected), pending CI.
- Next verify exact HEAD and actual logs; repair evidenced failures. C4.4a4 standalone multiple substitution. No lookup scheduling, GDEF filtering, extension lookup execution or full shaping yet.
- C4.3h-2e2b real off-diagonal/point attachment remains outstanding.
