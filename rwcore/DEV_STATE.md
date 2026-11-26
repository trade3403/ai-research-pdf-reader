# C 线路当前研发检查点

- CI #171, 93eb665e7e92059227ba28d0973a69067f6d3d15, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45870727460 : 1169/platform, 3507 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4h6 native two-positive-stage gate passed.
- C4.5a: shared bounded OpenType Layout directory and child decoder, separate GPOS entry points with kind domain 1..9, GSUB retains 1..8, script/language/feature references, lookup flags and subtable offsets, GPOS 1.1 variation offset. Five regressions including native raw lookup header comparison, 1174/platform expected, CI pending. Shared structural aliases and historical error prefix retained. No positioning subtable decoding or execution yet. Next C4.5b1 ValueRecord and Device/VariationIndex foundation.
- GPOS execution, joining/syllable analysis, Unicode reordering, avar 2.0, gvar/HVAR/MVAR, real point attachment, DOCX corpus and security/performance remain outstanding.
