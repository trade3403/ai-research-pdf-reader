# C 线路当前研发检查点

- CI #196: 478271b3394c9c30ca44f90391db8c5a0208ffb7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45984213080 : 1286/platform, 3858 total, zero failed/ignored; exact SHA and Release verified in all three job logs.
- C4.5j3: GPOS Type 8 format 3 coverage chained context positioning, independent per-position Coverage in all three segments, nearest-first backtrack, forward input/lookahead, input-relative ordered action records, owning GPOS lookup domain, empty context/coverage semantics, cumulative record and expanded glyph budgets across segments charged per reference. Shared format 3 allocations now fallible. Twelve new regressions; 1298/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 8 reference. Next C4.5j4 independent real-font reference with per-format evidence.
- GPOS full execution, joining/syllable analysis, Unicode reordering, avar 2.0, gvar/HVAR/MVAR, real point attachment, DOCX corpus and security/performance remain outstanding.


CI #197: 86d37f0d408e3f37bb9cbe82b67f4dfb83cbe1f2, run 45988945486: 1298/platform, 3894 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5j4: independent raw-byte Type 8 reference test across nine retained fonts, Type 9 targets, rule/action equality, representative class/glyph matching with truncations and exhaustive one-position coverage domain checks. Positive native gate; per-format native evidence not yet known. Separate synthetic positive reference all three formats. Two tests, 1300/platform expected. Next C4.5k extension positioning after CI. No local compile.


CI #198: 40556894fffc8385931b8158f4c0468d92954163, run 45991303281: 1300/platform, 3900 total, zero failed/ignored; SHA and Release verified. Native chained context positive gate passed; per-format counts not exposed.

C4.5k: GPOS Type 9 extension checked 32-bit offsets, borrowed target suffix, homogeneous lookup resolution, all direct types 1..8 dispatch with per-child limits, no recursive wrappers. Six new tests; 1306/platform expected. No local compile; native extension differential and exhaustive dispatch fixtures still outstanding. Next unified GPOS lookup compilation/scheduling after CI.
