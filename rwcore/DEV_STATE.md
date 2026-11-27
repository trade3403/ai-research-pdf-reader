# C 线路当前研发检查点

- CI #191: 8e44732c2198f3fcd832be398ea768b916996a69, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45970076717 : 1259/platform, 3777 total, zero failed/ignored; exact SHA and Release verified in all three job logs.
- C4.5i4: test-only independent raw-byte Type 7 context oracle for formats 1/2/3 across nine retained fonts, including Type 9 wrappers, full rule/action comparison, representative class/glyph matches and prefix checks, coverage domain checks. Native positive gate requires actual subtable, match and action evidence; per-format native coverage may be absent and is not claimed. Separate synthetic positive checks all three formats. Two tests; 1261/platform expected, CI pending. Missing native formats, nested execution, filtering and cycles remain outstanding. Next C4.5j1 chained context positioning format 1 after acceptance.
- GPOS full execution, joining/syllable analysis, Unicode reordering, avar 2.0, gvar/HVAR/MVAR, real point attachment, DOCX corpus and security/performance remain outstanding.


CI #192 failed: 8bf52a5260ac0bf79da18c74dcdb2cf644ea43c5, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45972476355 : 954 passed, 1 failed, 0 ignored per platform; 306 not run. All native Type 7 counters zero; independent synthetic oracle passed. Exact SHA verified; last green #191 1259/platform.

C4.5i4 repair: retain nine-font traversal and independent synthetic tests, add byte-exact native Gurmukhi Format 1 and Gujarati Format 2 excerpts from installed Debian Noto fonts, with provenance/hash/license and separate positive gates for both. No native Format 3 fixture yet; no nested execution or filtering claim. 1261/platform expected, CI pending. Next C4.5j1 after green.
