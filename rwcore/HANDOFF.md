# RDHS 1.0 — C 线路研发接续入口

新窗口指令：继续 C 线路 RWCore 自主研发。读取本文件，核对实际 HEAD 和 CI，直接接续。

## 工程与权限
- Repository: trade3403/ai-research-pdf-reader
- Branch: feature/rwcore; worktree: rwcore/
- Existing draft PR: #1 (verify actual state; never merge main).
- Only write this branch and C-owned paths. No force push, merge, release, production/secret changes, paid APIs or paid/self-hosted runners.
- Rust autonomous algorithms; reviewed basic ZIP/XML dependencies permitted. No external Word engine as the replacement core.
- Use existing GitHub connection and authorized standard GitHub-hosted Windows/Linux/macOS runners.

## Startup and per-batch gate
1. Read DEV_STATE.md, ROADMAP.md, CI_HISTORY.md, ARCHITECTURE.md, KNOWN_ISSUES.md and README.md at actual branch HEAD. Actual source and Actions evidence override stale notes.
2. Check latest commits, PR head, workflow runs/jobs/logs/artifacts tied to exact SHA. Count actual test results and verify release builds on all three platforms.
3. If CI is queued/running, do not start the next batch or trigger duplicate runs. If failed, read logs and repair only the evidenced cause, preserving regressions; at most three evidence-based attempts for one cause. Record blockers without infinite retries.
4. After all gates pass, implement one bounded next batch (Rust source, regression tests, documentation), not merely a plan. User has authorized continued C development without repeated confirmations.
5. Before writing, recheck HEAD. Use atomic non-forced fast-forward with expected SHA; validate diff. If concurrent HEAD changed, stop or re-read, never overwrite. Keep A/B/D and main untouched.
6. In the same source commit update DEV_STATE.md, append newly verified evidence to CI_HISTORY.md (or README.md with a link in CI_HISTORY), update architecture/known issues/roadmap when changed. New tests are pending until actual CI verifies. Do not fabricate SHA of the commit containing the note: record parent baseline and use actual Git HEAD after commit.
7. Report in Chinese: stage, actual SHA, CI URL, test counts per platform, release status, real artifact links or lack thereof, limitations and next stage. Do not claim GUI, performance, compatibility or installable release from unit tests.
8. Keep real legal DOCX corpus verification, lockfile provenance/security/license review outstanding until supported by actual evidence. Continue independent unblocked work.

## Handoff / concurrency
- Git repository is durable engineering state; CI logs are verification evidence.
- Each batch persists a checkpoint. At least every 10 development batches (or sooner if context truncation/tool errors are noticed), give user the short startup prompt above. Exact remaining chat/account quota is not observable.
- Handoff does not itself transfer chat history, credentials or automation ownership. Before new window writes, stop manual development in old window; do not run two writers. Existing hourly C automation, if enabled, must be coordinated/paused or remain the sole writer. Do not create duplicate automation. Use expected HEAD guard even when coordinated.
- If tools/access are missing, explain blocker and provide this prompt; do not simulate commits or CI.
- Current checkpoint and immediate task are in DEV_STATE.md; never assume its historical SHA is current.

## Short startup prompt
继续 C 线路 RWCore 自主研发。读取 GitHub 仓库 trade3403/ai-research-pdf-reader 的 feature/rwcore 分支下 rwcore/HANDOFF.md，核对最新研发状态和 CI，接续上一窗口的任务，直接实际开发。

## Latest handoff checkpoint
C4.4a7d source pending CI; parent CI #96 verified 641 tests per platform. Check actual HEAD and CI; see DEV_STATE.md for next task. Preserve outstanding real point attachment/general matrix and corpus gates. Do not run concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a7e1 source pending CI; parent CI #97 verified 643 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a7e2 source pending CI; parent CI #98 verified 653 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a7e3 source pending CI; parent CI #99 verified 663 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a7e4 source pending CI; parent CI #101 verified 673 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a8a source pending CI; parent CI #102 verified 675 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a8b1 source pending CI; parent CI #103 verified 687 tests per platform. Native Type 7 reference deferred, not verified. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a9a source pending CI; parent CI #104 verified 690 tests per platform. Native Type 7 reference remains outstanding. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a8b2a source pending CI; parent CI #105 verified 702 tests per platform. Original Type 7 fixture found during Type 8 search; Type 8 reference remains outstanding. Read DEV_STATE.md and verify exact HEAD; no concurrent writers.

## Latest checkpoint superseding previous checkpoint
C4.4a8b2b source pending CI; parent CI #106 verified 704 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Native Type 8 reference still fixture-blocked.

## Latest checkpoint superseding previous checkpoint
C4.4b1 source pending CI; parent CI #107 verified 706 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Native Type 8 reference still fixture-blocked. GDEF deferred child bodies not decoded.

## Latest checkpoint superseding previous checkpoint
C4.4b2 source pending CI; parent CI #108 verified 718 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Native Type 8 reference still fixture-blocked. Recommended preventive new conversation handoff now.

Latest C4.4b2 repair 1 pending CI: #109 failed identically on three platforms due to a test fixture expectation for legal empty Coverage format 2. Read DEV_STATE.md and CI_HISTORY.md. Preserve the 730-test/platform target and verify exact revision before proceeding to C4.4b3.

## Latest checkpoint superseding previous checkpoint
C4.4b3 source pending CI; parent CI #110 verified 730 tests per platform. Repair 1 accepted. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Recommend preventive new conversation handoff now.

## Latest checkpoint superseding previous checkpoint
C4.4b4 source pending CI; parent CI #111 verified 738 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Recommend preventive new conversation handoff. LookupFlag policy is not yet actual filtered substitution execution.

## Latest checkpoint superseding previous checkpoint
C4.4b5 source pending CI; parent CI #112 verified 751 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Eligibility reference tests are not filtered substitution execution.

## Latest checkpoint superseding previous checkpoint
C4.4c1 source pending CI; parent CI #113 verified 757 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Base plan is not execution or a full script-specific shaping plan.

## Latest checkpoint superseding previous checkpoint
C4.4c2 source pending CI; parent CI #114 verified 770 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Real plan reference is not actual shaping execution.

## Latest checkpoint superseding previous checkpoint
C4.4c3a source pending CI; parent CI #115 verified 778 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Compiler does not execute or recursively compile nested lookups.

## Latest checkpoint superseding previous checkpoint
C4.4c3b source pending CI; parent CI #116 verified 790 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Real compilation reference does not execute lookup targets.

## Latest checkpoint superseding previous checkpoint
C4.4c4a source pending CI; parent CI #117 verified 797 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Direct dependency inventory retains cycles and does not execute or resolve closure.

## Latest checkpoint superseding previous checkpoint
C4.4c4b source pending CI; parent CI #118 verified 807 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Static reference inventory is not execution or cycle analysis.

## Latest checkpoint superseding previous checkpoint
C4.4c4c source pending CI; parent CI #119 verified 814 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Static closure diagnoses known cycles and missing source coverage, not runtime execution safety.

## Latest checkpoint superseding previous checkpoint
C4.4c4d source pending CI; parent CI #120 verified 827 tests per platform. Read DEV_STATE.md and verify exact HEAD; no concurrent writers. Preventive new conversation handoff recommended. Static closure reference does not certify runtime execution safety.

## Latest checkpoint
C4.4c5a pending CI; #121 verified 834/platform. Read DEV_STATE.md and exact HEAD. No concurrent writers. Program is static, not execution. New window recommended.

## Latest checkpoint
C4.4c5b pending CI; #122 verified 841/platform, 2523 total. Seven real-font program reference regressions, 848/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next actual single substitution execution only after CI passes.

## Latest checkpoint
C4.4d1a pending CI; #123 verified 848/platform, 2544 total. Ten new synthetic execution regressions, 858/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next real-font Type 1 execution reference only after CI passes.

## Latest checkpoint
C4.4d1b pending CI; #124 verified 858/platform, 2574 total. Seven real-font execution regressions, 865/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 2 multiple substitution after CI passes.

## Latest checkpoint
C4.4d2a pending CI; #125 verified 865/platform, 2595 total. Eleven mixed execution regressions, 876/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next real-font Type 2 reference only after CI passes.

## Latest checkpoint
C4.4d2b pending CI; #126 verified 876/platform, 2628 total. Seven real-font mixed execution regressions, 883/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 3 alternate selection only after CI passes.

## Latest checkpoint
C4.4d3a pending CI; #127 verified 883/platform, 2649 total. Ten alternate/mixed execution regressions, 893/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next real-font Type 3 reference only after CI passes.

## Latest checkpoint
C4.4d3b pending CI; #128 verified 893/platform, 2679 total. Seven real-font alternate execution regressions, 900/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 4 ligature execution only after CI passes.

## Latest checkpoint
C4.4d4a pending CI; #129 verified 900/platform, 2700 total. Ten synthetic ligature/mixed regressions, 910/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next real-font Type 4 reference only after CI passes.

## Latest checkpoint
C4.4d4b pending CI; #130 verified 910/platform, 2730 total. Seven real-font ligature reference regressions, 917/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next bounded Type 5 context foundation only after CI passes.

## Latest checkpoint
C4.4d5a pending CI; #131 verified 917/platform, 2751 total. Ten synthetic contextual regressions, 927/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next contextual Type 3 nested selection and reference checks only after CI passes.

## Latest checkpoint
C4.4d5b pending CI; #132 verified 927/platform, 2781 total. Ten context/alternate regressions, 937/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next independent native Type 5 length-preserving closure reference after CI passes.

## Latest checkpoint
C4.4d5c pending CI; #133 verified 937/platform, 2811 total. Seven native contextual reference/control regressions, 944/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next nested Type 2 expansion and position tracking after CI passes.

## Latest checkpoint
C4.4d5d pending CI; #134 verified 944/platform, 2832 total. Ten contextual expansion regressions, 954/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next independent native expansion reference only after CI passes.

## Latest checkpoint
C4.4d5e pending CI; #135 verified 954/platform, 2862 total. Seven native expansion reference/control regressions, 961/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next bounded nested Type 4 contraction after CI passes.

## Latest checkpoint
C4.4d5f pending CI; #136 verified 961/platform, 2883 total. Ten contextual contraction regressions, 971/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next native contraction reference only after CI passes.

## Latest checkpoint
C4.4d5g pending CI; #137 verified 971/platform, 2913 total. Seven native contraction reference regressions, 978/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 6 chained contextual execution foundation only after CI passes.

## Latest checkpoint
C4.4d6a pending CI; #138 verified 978/platform, 2934 total. Ten chained context regressions, 988/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 6 variable-length integration only after CI passes.

## Latest checkpoint
C4.4d6b pending CI; #139 verified 988/platform, 2964 total. Ten variable-length chained regressions, 998/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next native Type 6 execution reference only after CI passes.

## Latest checkpoint
C4.4d6c pending CI; #140 verified 998/platform, 2994 total. Seven native chained reference regressions, 1005/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next Type 8 reverse chained execution only after CI passes.

## Latest checkpoint
C4.4d8a pending CI; #141 verified 1005/platform, 3015 total. Ten reverse execution regressions, 1015/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next native Type 8 independent reference only after CI passes.

## Latest checkpoint
C4.4d8b pending CI; #142 verified 1015/platform, 3045 total. Ten full dispatch/reverse reference regressions, 1025/platform expected. Native positive Type 8 fixture absent. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next C4.4d8c native fixture/wrapper acceptance only after CI passes.

## Latest checkpoint
C4.4d8c pending CI; #143 verified 1025/platform, 3075 total. New OFL Noto Sans Mongolian native Type 8 fixture and all-wrapper regressions, 1028/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next bounded full GSUB execution acceptance/API consolidation only after CI passes.

## Latest checkpoint
C4.4d9 pending CI; #144 verified 1028/platform, 3084 total. Eight end-to-end native plan regressions; 1036/platform expected. Read DEV_STATE.md and GSUB_EXECUTION.md. Next C4.4e1 FeatureVariations foundation only after CI passes.

## C4.4d9 repair checkpoint
#145 failed identically on all platforms; complete native mong/rclt plan has 74 roots including lookup 93, not <=32. Pinned schedule repair committed without changing production limits or dropping tests; 1036/platform target unchanged. Next verify repair CI; no FeatureVariations development before pass.

## Latest checkpoint
C4.4d9 accepted at #146, exact baseline 5c06c9b0638380a8d093d1adc6f9c20155ce3951, 1036/platform, 3108 total. C4.4e1 independent FeatureVariations parser and selector pending CI, 12 regressions, 1048/platform expected. Next variation-aware plan/program compilation only after pass. Read DEV_STATE.md and GSUB_EXECUTION.md.

## Latest checkpoint
C4.4e2 pending CI; #147 verified 1048/platform, 3144 total. Ten synthetic dynamic plan/program tests, 1058/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next C4.4e3 native positive variation reference only after CI passes.

## Latest checkpoint
C4.4e3 pending CI; #148 verified 1058/platform, 3174 total. OFL Archivo positive native fixture and four regressions, 1062/platform expected. Read DEV_STATE.md and exact HEAD; no concurrent writers. Next C4.4e4 fvar normalization only after CI passes.

## Latest checkpoint
C4.4e4 pending CI; #149 verified 1062/platform, 3186 total. New fvar parser and pre-avar Fixed normalization, 12 tests, 1074/platform expected. Next C4.4e5 avar 1.0 only after exact CI pass. Read DEV_STATE.md; no concurrent writers.

## Latest checkpoint
C4.4e5 pending CI; #150 verified 1074/platform, 3222 total. New avar 1.0 mapping and ordered/tagged normalization, 12 tests, 1086/platform expected. Next C4.4e6 native normalization-to-GSUB integration only after exact CI pass. Read DEV_STATE.md; no concurrent writers.

## Latest checkpoint
C4.4e6 pending CI; #151 verified 1086/platform, 3258 total. Same-face variable GSUB user-coordinate integration and five native/negative regressions, 1091/platform expected. Next C4.4e7 acceptance and API contract only after exact CI pass. Read DEV_STATE.md; no concurrent writers.


CI #152, 3baf3be5ff3f891f5dab192d3a873dc9ba321d3b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45802058303 : 1091/platform, 3273 total, zero failed/ignored; exact HEAD and three Release builds verified from actual job logs.

C4.4e7: five additional integration regressions for raw named-instance coordinates across Archivo, Devanagari and Nastaliq, ordered/tagged/default coordinate equivalence, repeat execution and plan provenance. Existing independent raw design-space and glyph ledger reference tests retained unchanged. 1096/platform expected; CI pending. No production substitution changes or external shaper equivalence claim.


CI #153, 1cfd3f4d22737f26d82e04243739c1e9137b7f9e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45808724109 : 1096/platform, 3288 total, zero failed/ignored, exact SHA and Release builds verified from three job logs. C4.4e7 bounded variable GSUB scheduling accepted.

C4.4f1: font_source_run adds immutable logical scalar and nominal glyph provenance, UTF-8 byte lookup, checked scalar ranges, explicit missing glyph policy and bounded allocation. Six regressions; 1102/platform expected, CI pending. Scalar seeds are not extended grapheme clusters; no GSUB edit propagation, normalization or bidi yet. Next C4.4f2 bounded source-set union and expansion/contraction ledger.


CI #154, b17e2423e7679c8fe196c5ccace84b8655a5628f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45812104603 : 1102/platform, 3306 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f2: font_provenance adds a bounded explicit glyph edit ledger, sorted unique scalar source sets, expansion duplication, noncontiguous contraction preserving gaps, checked glyph domain, cumulative logical work and transactional limit failures. Nine regressions including independent BTreeSet subset reference; 1111/platform expected, CI pending. No deletion, grapheme segmentation or live GSUB integration. Next C4.4f3 integrate provenance with GSUB edit execution (begin with single/multiple substitution).


CI #155, 0cc58a89276964ca9e35056c6e51be825c66e92e, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45815344242 : 1111/platform, 3333 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f3: optional provenance ledger inside existing GSUB scheduler, public execute_gsub_with_provenance for effective Type 1/2 and Type 7 wrappers only, preflight rejection of other reachable kinds, atomic failure and glyph output consistency check. Five regressions with native cmap seeds and synthetic lookup tables; 1116/platform expected, CI pending. No external engine parity or full cluster shaping. Next C4.4f4 integrate Type 3/4 provenance including noncontiguous ligature components.
