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


CI #156, 6edc88d741591fcca892316154419873096afbb6, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45818748339 : 1116/platform, 3348 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f4: execute_gsub_with_direct_provenance adds explicit Type 3 choices and Type 4 exact component-set union in the existing scheduler, including Type 7 wrappers and ignored gaps. Original Type 1/2 API retains rejection behavior. Five new synthetic lookup regressions with native cmap seeds; 1121/platform expected, CI pending. No contextual/reverse provenance or full cluster semantics. Next C4.4f5 integrate nested Type 5/6 and reverse Type 8 provenance.


CI #157, cd0a0a9b188cdf64c8563663e92ed4201e1864be, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45822148460 : all three jobs failed at direct_alternate_explicit_choice with Invalid("gsub provenance divergence"); each log records 652 passed, 1 failed, 0 ignored before abort. Exact SHA verified. No successful full Release gate.

C4.4f4 repair: Type 3 branch omitted provenance update. Centralize Type 1/3 replacement through State::replace_one, preserving counters and atomic output semantics. Retain failed regression unchanged; add Type 3 -> Type 4 source/counter parity and exact provenance work limit regressions. 1123/platform expected, CI pending. Do not advance C4.4f5 until full verification.


CI #158, 9547f8e2d047df9a7c115d6f6dd521d59ad497a1, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45825597415 : 1123/platform, 3369 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4f4 repaired and accepted.

C4.4f5: full-program tracked API, Type 8 uses shared replace_one, nested Type 5/6 reuse live edit anchors and exact source propagation. Six regressions for growth, consumed anchors, chain conditions, reverse cascade and nested reverse, budgets; 1129/platform expected, CI pending. Restricted APIs retain preflight. No full grapheme clusters, script staging or native all-kind provenance acceptance. Next C4.4f6 comprehensive native provenance acceptance and API contract.


CI #159, fcbf2bf908ac894069ff7fc17c94cdc1167f47a5, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45829163360 : 1129/platform, 3387 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4f6: VariableGsubFace::execute_with_provenance integrates existing dynamic scheduling with full tracked execution. Five native-font regressions for Archivo, Devanagari, Nastaliq default selection, Archivo rvrn scalar source oracle and source budget. Existing synthetic all-kind independent expected source sets retained. 1134/platform expected; CI pending. Native tests do not establish positive native coverage of every lookup kind or external shaper parity. Next C4.4g1 explicit per-glyph feature-mask foundation.


CI #160, de26d8cd614985ce3828f9a92521a047ee399d7b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45832722223 : 1134/platform, 3402 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4f6 accepted.

C4.4g1: bounded dense binary scalar feature masks (64 selected IDs), ordered range overrides, required-feature protection, lookup contribution bitsets, conservative intersection projection through exact provenance, resource caps. Six regressions including exhaustive small reference; 1140/platform expected, CI pending. No scheduler gating or per-feature alternate values yet. Next C4.4g2 integrate validated root lookup gating and live masks.


CI #161, fb271073de1f4b8c03a0c8405b8c91ef09877346, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45836441328 : 1140/platform, 3420 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4g2: retain validated static/dynamic plan in compiled program; masked root-anchor gating uses OR of contributing feature bits and live intersection of exact scalar provenance. Separate cumulative mask work including final projection, legacy execution unchanged, unvalidated explicit-root programs rejected. Three native regression tests; 1143/platform expected, CI pending. Root anchor only: contextual descendants and additional ligature components not independently gated; next C4.4g3 component and contextual mask policy, synthetic positive gating/edits. No claim of full shaping.


CI #162, c13b1f4c1bea9bfed4522d39792878472d1c8ea9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45840611187 : 1143/platform, 3429 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4g3: separate strict mask API, common feature intersection across matched input/ligature components, inherited feature bits and live action-anchor checks, mask-disabled components are barriers not ignored gaps; backtrack/lookahead remain conditions. Legacy root-only API preserved. Three synthetic positive/negative regressions with native cmap seeds, 1146/platform expected, CI pending. Explicit conservative policy, not external shaper parity. Next C4.4g4 strict native and independent reference acceptance.


CI #163, 7dc069e500b0c2294290db99c7c50db81ab928c0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45843654359 : all three jobs failed compilation E0061 at font_gsub_context_execute.rs:551, forward apply_at missing masking and bits; exact SHA verified, no tests executed.

C4.4g3 repair: pass mask state and root bits to forward scheduler call, audit all apply_at callers, ensure all six context candidate formats reject zero common bits before selecting a match (including coverage formats), preserve existing regressions unchanged. 1146/platform expected; CI pending. Do not advance C4.4g4 until full gate.


CI #164, d022359b89d5b05127f4814fe621438c988e29e3, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45847003383 : 1146/platform, 3438 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4g3 repaired and accepted.

C4.4g4: same-face variable strict mask wrapper, independent exhaustive 256 two-feature ligature cases and 16 reverse mask cases, native Archivo variable-coordinate parity and disabled-feature acceptance, exact runtime budget. Four regressions; 1150/platform expected, CI pending. Conservative policy only, not external shaper equivalence. Next C4.4h1 explicit bounded script-stage scheduling foundation.


CI #165, e13eff3fa51552abe7b865eacc2f187e8bbcd28d, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45850640589 : 1150/platform, 3450 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4g4 accepted.

C4.4h1: bounded explicit stage schedule from validated compiled plan, complete unique feature partition including required feature, caller-ordered stages, ascending roots per stage, shared roots deduplicated within stage and repeated across stages with exact contributor sets, separate cumulative caps. Five regressions with synthetic validated plan and independent reference; 1155/platform expected, CI pending. No execution, automatic script ordering or reordering yet. Next C4.4h2 staged execution on persistent provenance and masks.


CI #166, e0aa39366eedfea8884d5360efbdc645b39524dc, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45854217323 : 1155/platform, 3465 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h2: execute_staged_gsub builds validated partition and executes stage roots in order in one existing scheduler state, persistent glyph/source buffers, strict stage contributor masks, cumulative execution/source/mask budgets, one explicit alternate per reachable lookup. Three regressions for order, growth/contraction, shared masks, single-stage parity and cumulative limits; 1158/platform expected, CI pending. No automatic script ordering, reordering or per-stage callbacks. Next C4.4h3 native staged acceptance and explicit script feature ordering policy.


CI #167, c5912c746eb503d692b7354878322edbab70b875, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45857691944 : 1158/platform, 3474 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h3: explicit ordered feature-tag groups, duplicate rejection, absent tag omission, unlisted selected feature tail, same-face variable tag-stage wrapper. Four regressions for mapping/order/bounds and native Archivo parity/disabled multi-group execution; 1162/platform expected, CI pending. Tag mapping requires same FeatureList; variable wrapper enforces it. No automatic script-specific rules yet; native disabled tests do not prove positive multi-stage substitutions. Next C4.4h4 explicit script policy profiles and positive native stage acceptance.


CI #168, 7ef98a449d337c1a1819e6435e23f8896e496984, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45861096884 : 1162/platform, 3486 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h4: explicit experimental Latin, Arabic and IndicV2 feature ordering profiles, unsupported script rejection, no implicit enabling; tests for profile selection, unique tags, partition and positive native Archivo scalar substitution under multi-stage scheduling. Three regressions, 1165/platform expected, CI pending. Native test requires actual substitution and compares existing single-pass engine; does not prove two stages both mutate or independent external shaper parity. Arabic/Indic profiles require caller masks and preprocessing, not complete script shapers. Next C4.4h5 profile integration and positive multi-stage evidence before seal.


CI #169, 72bd70f901432c11f395e9355cd389abb17aa44a, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45864396745 : 1165/platform, 3495 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h5: traced staged execution with bounded per-root change counts aggregated by stage, same-face script profile wrapper rejecting DFLT fallback, synthetic positive two-stage growth/contraction evidence, no-op/disabled diagnostics and native profile parity. Three regressions, 1168/platform expected, CI pending. Native profile trace does not require two positive native stages; full external shaper parity, joining/syllable/reorder rules remain outstanding. Next C4.4h6 native two-positive-stage evidence and contract gate.


CI #170, 3ddf448753e932444669d301899515c95568a448, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45867643339 : 1168/platform, 3504 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.4h6: native Archivo maximum-weight BMP cmap acceptance, two nonzero stage change counters required (rvrn then case), exact scalar provenance, deterministic repeat. One regression; 1169/platform expected, CI pending. Native positive two-stage gate may expose unreachable cmap coverage; do not weaken assertion if it fails. Seal only after CI. Next C4.5a bounded GPOS header/directory foundation, complex script preprocessing remains outstanding.


CI #171, 93eb665e7e92059227ba28d0973a69067f6d3d15, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45870727460 : 1169/platform, 3507 total, zero failed/ignored; exact SHA and Release builds verified from three job logs. C4.4h6 native two-positive-stage gate passed.

C4.5a: shared bounded OpenType Layout directory and child decoder, separate GPOS entry points with kind domain 1..9, GSUB retains 1..8, script/language/feature references, lookup flags and subtable offsets, GPOS 1.1 variation offset. Five regressions including native raw lookup header comparison, 1174/platform expected, CI pending. Shared structural aliases and historical error prefix retained. No positioning subtable decoding or execution yet. Next C4.5b1 ValueRecord and Device/VariationIndex foundation.


CI #172, 98d74fce846a478c36ea00c315fcebe97d7852e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45873638947 : 1174/platform, 3522 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5b1: ValueRecord 8 defined format bits, signed placement/advance, parent-relative nullable Device offsets with caller-provided inline lower bound, packed 2/4/8-bit signed pixel deltas and opaque VariationIndex, cumulative per-record Device entry budget. Six regressions including all 256 formats, exhaustive signed values, truncation and budgets; 1180/platform expected, CI pending. No ItemVariationStore evaluation or positioning application. Next C4.5b2 Anchor and shared adjustment primitives.


CI #173, 2b58e61999d973b0b730e93a99fefca1ffefc3d6, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45876487806 : 1180/platform, 3540 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5b2: Anchor formats 1–3, signed nominal coordinates, contour-point index, relative nullable Device offsets with shared cumulative entries cap, checked four-field design adjustment addition and separate anisotropic pixel deltas. Five regressions; 1185/platform expected, CI pending. Unresolved contour and variation indices rejected in pixel resolution; no contour evaluation or ItemVariationStore yet. Next C4.5c1 SinglePos formats 1/2 parsing and application.


CI #174, b015a73591e4da545ef931fd0bc0d28896dadff0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45879217950 : 1185/platform, 3555 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5c1: standalone SinglePos formats 1/2, coverage index lookup, shared/individual value records, checked design/pixel adjustment, cumulative device entry cap across records. Five regressions, 1190/platform expected, CI pending. Nominal design-only path explicitly ignores devices; combined path rejects unresolved variation indices. No lookup flags or script scheduling yet. Next C4.5c2 independent native SinglePos reference acceptance.


CI #175, 1dbbedd4caa3a74043a1f66d1f4b16dc295ebd89, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45881899044 : 1190/platform, 3570 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5c2: independent native raw SinglePos oracle over eight retained font fixtures, direct and extension-contained targets, dense coverage reference, signed values, per-bit Device extraction, nominal and anisotropic pixel adjustment, unresolved variation rejection. One regression, 1191/platform expected, CI pending. Require positive native format 1 and 2 and nonzero nominal adjustment; fixture inventory not locally verified. No full lookup execution or external shaper parity. Next C4.5d1 PairPos format 1 glyph-pair decoding and adjustment.


CI #176, 8e3518cf12026185d0d032698da68c7c8ddaadfa, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45884649416 : 1191/platform, 3573 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d1: standalone PairPos format 1, coverage-indexed PairSets, ordered second-glyph binary search, two ValueRecords, cumulative pair/device caps, checked atomic design/pixel adjustments, zero-format matching. Five regressions, 1196/platform expected, CI pending. Device offsets are PairSet-relative; no lookup scheduling or class-based PairPos yet. Next C4.5d2 PairPos format 2 class matrix.


CI #177, 83d81985278920ed4669cdba557bef733868032f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45887358589 : 1196/platform, 3588 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d2: PairPos format 2 dense class matrix, both ClassDef formats, class zero fallback, first glyph coverage and second glyph domain, checked atomic dual adjustment, subtable-relative Device and cumulative cell/device limits including zero-stride matrices. Five regressions, 1201/platform expected, CI pending. Next C4.5d3 independent native PairPos format 1/2 acceptance. No lookup scheduling or variation resolution yet.


CI #178, eaa2a8d23a3789d00e408cef91614ffa4b29cde4, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45890110177 : 1201/platform, 3603 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5d3: independent native PairPos oracle over eight retained font fixtures, direct and extension targets, dense explicit-pair reference and class maps, all covered first glyphs and second glyph class mappings, reachable matrix cells, dual design/pixel adjustment, strict positive gates for both formats. One regression, 1202/platform expected, CI pending. No external shaper parity or full lookup execution. Next C4.5e1 CursivePos entry/exit anchor parsing and nominal connection math.


CI #179, 1a82af003aeb9e7c095d7122c8bfc9c0f708923d, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45892747565 : 1202/platform, 3606 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5e1: CursivePos format 1 nullable entry/exit anchors, coverage mapping, cumulative Device budget, checked two-origin connection geometry with explicitly fixed entry or exit, separate nominal design and pixel corrections. Five regressions, 1207/platform expected, CI pending. This is standalone geometry, not full OpenType cursive advance redistribution, RTL lookup behavior or attachment chain execution. Next C4.5e2 native independent cursive reference acceptance.


CI #180, d88e99e2bd5561a2f40a67cc54d915bbe9a79049, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45895332135 : 1207/platform, 3621 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5e2: independent native CursivePos oracle over five retained font fixtures, raw coverage and anchor format decoding, per-bit Device reference, covered-pair cross-product, fixed entry/exit geometry, pixel-space correction or explicit unresolved error. Strict positive native subtable, anchor and unequal-coordinate pair gates. One regression, 1208/platform expected, CI pending. No full cursive advance redistribution, RTL attachment chain or external shaper parity. Next C4.5f1 MarkToBase parsing and nominal attachment geometry.


CI #181, bf3dc8f0cd77ad4123fa9e6acf43f291d4803723, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45897928513 : 1208/platform, 3624 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5f1: MarkToBase format 1, MarkArray required anchors, BaseArray nullable class matrix, independent coverage, class and cell caps, cumulative device budget, checked nominal and separate pixel attachment geometry with base fixed. Five regressions, 1213/platform expected, CI pending. No automatic base selection, attachment chain or outline/variation resolution. Next C4.5f2 native independent MarkToBase reference acceptance.


CI #182, 883969021ba4fc485be6edb8014cecda6a61d8e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45900444622 : 1213/platform, 3639 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.

C4.5f2: independent raw-byte native MarkToBase oracle across nine retained font fixtures, including extension lookup traversal, coverage formats, mark class and anchor records, nullable base matrix cells, all covered pairs, independent world-coordinate equality and per-bit Device pixel checks or explicit unresolved errors. Strict positive gates for subtables, marks, base anchors, matched pairs, unequal coordinates and resolved pixel checks. One regression, 1214/platform expected, CI pending. No automatic base selection, attachment chain, external shaper parity or outline/variation resolution. Next C4.5g1 MarkToLigature format 1 component attachment foundation.


CI #183: a42cb2f64f1d097fa7e5562f8105582e6cdc108c, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45914632546 : 1214/platform, 3642 total, zero failed/ignored; exact SHA, native_mark_base_reference and Release verified in three job logs.

C4.5g1: standalone MarkToLigature format 1 with explicit zero-based component selection, cumulative component/cell/device budgets, nullable anchors, shared MarkArray and anchor/coverage decoding, checked design and separate pixel corrections. Five synthetic regressions; 1219/platform expected, CI pending. No automatic component provenance selection, full attachment chain, real-font reference, contour or variation resolution. Next C4.5g2 independent native reference.


CI #184: 56cb2c7aaa389ea6cf5d06b00f7d44b131d98330, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45956312488 : 1219/platform, 3657 total, zero failed/ignored; exact SHA and Release verified in three job logs.

C4.5g2: independent test-only raw-byte native MarkToLigature oracle across nine retained fonts, Type 5 and extension traversal, coverage, component/class matrix, all covered component/mark pairs, world-coordinate equality, separate per-bit Device corrections and explicit unresolved errors. Positive gates for native subtables, marks, multi-component ligatures, anchors, pairs, unequal coordinates and resolved pixel checks. One new test; 1220/platform expected, CI pending. No external shaper parity, automatic component selection, outline/variation resolution or attachment-chain integration. Next C4.5h1 MarkToMark foundation.


CI #185 failed: 00da1e0dc2e43b8426378cfcf3136b000b6af156, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45958213747 : each platform 919 passed, 1 failed, 0 ignored, 300 not run, Release skipped; exact SHA and native evidence [1,178,18,62,3436,3436,0] verified. Last green #184: 1219/platform.

C4.5g2 repair: retain nine full-font reference checks and original aggregate positive gates; add exact 370-byte DejaVu Sans 2.37 native Type 5 subtable excerpt (source and excerpt SHA-256, license retained), plus independent synthetic differential nonzero Device check. Native excerpt has coordinate anchors, not real nonzero Device deltas; native contour/variation resolution and real nonzero Device fixture remain outstanding. 1221/platform expected, CI pending. Next C4.5h1 only after green.


CI #186: 5bc81053a421b1142ee2643f309b6b7237fa7052, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45960201240 : 1221/platform, 3663 total, zero failed/ignored; exact SHA, native_mark_ligature_reference, independent_nonzero_device_attachment and Release verified in all three job logs. Real nonzero Device fixture and contour/variation resolution remain outstanding.

C4.5h1: GPOS Type 6 MarkToMark format 1 explicit fixed Mark2/moving Mark1 roles. Shared identical Type 4/6 binary matrix decoder, checked design/pixel translation, overlapping coverages permitted, cumulative Device/record/cell bounds and nullable anchors. Six regressions; 1227/platform expected, CI pending. Caller establishes lookup type and occurrence selection; no automatic attachment chain, real native Type 6 reference or contour/variation resolution. Next C4.5h2 native independent reference.


CI #187: b23be41a5a1dbfb009e91d6b76a63990c3bb98c9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45961764512 : 1227/platform, 3681 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5h2: independent raw-byte native Type 6 reference across nine retained fonts, including extension traversal, coverage, all Mark1/Mark2 class cells and covered pairs, independent world-coordinate equality and per-bit Device pixel checks or explicit unresolved errors. Native positive gates for subtables, marks, anchors, pairs and distinct coordinates; resolved native pixel count reported separately (may be zero due to fixture limitations). Separate synthetic nonzero Device differential requires exact positive/negative deltas. Two new regressions, 1229/platform expected, CI pending. Real nonzero Device and contour/variation resolution remain outstanding. Next C4.5i1 GPOS Contextual Positioning format 1, after acceptance.


CI #188: 5b61e89bf145a82eea3fd9eade739f4bc91707e7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45963737757 : 1229/platform, 3687 total, zero failed/ignored; exact SHA, native_mark_mark_reference, mark_mark_independent_nonzero_device and Release verified in all three logs. Native resolved pixel count not separately available in passing captured logs; real nonzero Device fixture remains outstanding.

C4.5i1: GPOS Type 7 format 1 glyph-based context positioning parser and immutable prefix/explicit-position matcher. Reuses identical GSUB sequence-rule grammar with owning GPOS lookup count, ordered action records, null sets, first-match priority, empty-action matches and cumulative budgets. Shared context format 1/2 allocations now fallible. Twelve new regressions; 1241/platform expected, CI pending. No positioning execution, GDEF filtering, recursion/cycle evaluation or native Type 7 reference. Next C4.5i2 class context positioning format 2.


CI #189: 24b3845d46032262c7a3f3f29ac56df4259f9a16, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45965933610 : 1241/platform, 3723 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i2: GPOS Type 7 format 2 class context positioning, shared ClassDef and bounded sequence-rule decoder, initial coverage gate, class zero, immutable prefix and explicit-position matching, ordered lookup references validated in owning GPOS domain. Seven regressions; 1248/platform expected, CI pending. No nested execution, filtering, cycle evaluation or native Type 7 reference. Next C4.5i3 coverage context positioning format 3.


CI #190: bc2aae75747e609f0a9193b2cf50b2a7e5700195, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45967906824 : 1248/platform, 3744 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i3: GPOS Type 7 format 3 coverage context positioning, independent coverage per input position, immutable prefix/explicit-position matching, ordered lookup actions, empty-action and empty-coverage semantics, cumulative coverage records/expanded glyph budgets charged per reference. Shared format 3 allocation handling now fallible. Eleven regressions; 1259/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 7 reference. Next C4.5i4 independent real-font reference, with explicit fixture coverage limitations.


CI #191: 8e44732c2198f3fcd832be398ea768b916996a69, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45970076717 : 1259/platform, 3777 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5i4: test-only independent raw-byte Type 7 context oracle for formats 1/2/3 across nine retained fonts, including Type 9 wrappers, full rule/action comparison, representative class/glyph matches and prefix checks, coverage domain checks. Native positive gate requires actual subtable, match and action evidence; per-format native coverage may be absent and is not claimed. Separate synthetic positive checks all three formats. Two tests; 1261/platform expected, CI pending. Missing native formats, nested execution, filtering and cycles remain outstanding. Next C4.5j1 chained context positioning format 1 after acceptance.


CI #192 failed: 8bf52a5260ac0bf79da18c74dcdb2cf644ea43c5, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45972476355 : 954 passed, 1 failed, 0 ignored per platform; 306 not run. All native Type 7 counters zero; independent synthetic oracle passed. Exact SHA verified; last green #191 1259/platform.

C4.5i4 repair: retain nine-font traversal and independent synthetic tests, add byte-exact native Gurmukhi Format 1 and Gujarati Format 2 excerpts from installed Debian Noto fonts, with provenance/hash/license and separate positive gates for both. No native Format 3 fixture yet; no nested execution or filtering claim. 1261/platform expected, CI pending. Next C4.5j1 after green.


CI #193 failure: c653a6268c3573168fa3c3121278ef828f3db21c, run 45974803781: 954 passed, 1 failed, 306 not run per platform; exact SHA verified. Native Gujarati Type 7 format 2 rejected with Invalid(class range or value).

C4.5i4 repair 2: verified local Noto Gujarati raw source, glyph count 798, class set count 2, trailing class 2 assigned in ClassDef. GPOS compatibility path permits trailing-only classes without initial rule sets, validates initial coverage indices and undefined trailing references, keeps GSUB strict behavior. Independent oracle representative domain now follows actual class IDs. Added one regression; 1262/platform expected, CI pending. Native Format 3 remains outstanding. Next C4.5j1 only after green.


CI #194: a9ba8c115d6e710eb8388fe6cce26926464c96e9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45977073609 : 1262/platform, 3786 total, zero failed/ignored; exact SHA and Release verified in all three job logs. Native Type 7 format 3 fixture remains outstanding.

C4.5j1: GPOS Type 8 format 1 glyph chained context positioning, shared bounded sequence grammar, reverse-stream backtrack, forward input/lookahead, first-match priority, empty-action semantics, input-only sequence indices and owning GPOS lookup bounds. Shared format 1/2 allocation handling now fallible. Twelve new regressions; 1274/platform expected, CI pending. No nested positioning execution, GDEF filtering, cycle evaluation or native Type 8 reference. Next C4.5j2 class chained context positioning format 2.


CI #195: cbab07ea5e34d562ddcf91eaf354b5e172636d3f, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45981814345 : 1274/platform, 3822 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5j2: GPOS Type 8 format 2 class chained context positioning with independent backtrack/input/lookahead ClassDefs, initial coverage gate, implicit class zero, ordered input-only lookup references and bounded matching. GPOS compatibility permits assigned trailing-only input classes without initial rule sets; rejects out-of-range covered initial classes and undefined trailing-only references. GSUB strict path unchanged. Twelve regressions; 1286/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 8 reference. Next C4.5j3 coverage chained context positioning format 3.


CI #196: 478271b3394c9c30ca44f90391db8c5a0208ffb7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45984213080 : 1286/platform, 3858 total, zero failed/ignored; exact SHA and Release verified in all three job logs.

C4.5j3: GPOS Type 8 format 3 coverage chained context positioning, independent per-position Coverage in all three segments, nearest-first backtrack, forward input/lookahead, input-relative ordered action records, owning GPOS lookup domain, empty context/coverage semantics, cumulative record and expanded glyph budgets across segments charged per reference. Shared format 3 allocations now fallible. Twelve new regressions; 1298/platform expected, CI pending. No nested execution, GDEF filtering, cycles or native Type 8 reference. Next C4.5j4 independent real-font reference with per-format evidence.


CI #197: 86d37f0d408e3f37bb9cbe82b67f4dfb83cbe1f2, run 45988945486: 1298/platform, 3894 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5j4: independent raw-byte Type 8 reference test across nine retained fonts, Type 9 targets, rule/action equality, representative class/glyph matching with truncations and exhaustive one-position coverage domain checks. Positive native gate; per-format native evidence not yet known. Separate synthetic positive reference all three formats. Two tests, 1300/platform expected. Next C4.5k extension positioning after CI. No local compile.


CI #198: 40556894fffc8385931b8158f4c0468d92954163, run 45991303281: 1300/platform, 3900 total, zero failed/ignored; SHA and Release verified. Native chained context positive gate passed; per-format counts not exposed.

C4.5k: GPOS Type 9 extension checked 32-bit offsets, borrowed target suffix, homogeneous lookup resolution, all direct types 1..8 dispatch with per-child limits, no recursive wrappers. Six new tests; 1306/platform expected. No local compile; native extension differential and exhaustive dispatch fixtures still outstanding. Next unified GPOS lookup compilation/scheduling after CI.


CI #199: f80c3a531e62f31ff68dc8cdffe6eafa22bc98c0, run 45993639612: 1306/platform, 3918 total, zero failed/ignored; exact SHA and Release verified.

C4.5l1: explicit selected GPOS LookupList compilation, original indices and source/target offsets, homogeneous Type 9 targets, shared GDEF filter construction, cumulative subtable and conservative suffix-byte budgets, fallible vector allocation. Eight new regressions; 1314/platform expected, CI pending. No execution, dependency closure or feature plan yet. Next C4.5l2 GPOS feature selection and plan provenance, then dependency closure and scheduling. Native extension differential remains outstanding.


CI #200 failed: b5126b6116edd58705d25d53f4cfc873c8e1f239, run 45995876561: 635 passed, 2 failed, 0 ignored, 677 not run per platform; exact SHA verified. direct_and_extension_order and cumulative_suffix_and_subtable_budgets both rejected GPOS type 9 with Invalid(lookup kind).

C4.5l1 repair: introduce LookupFilter::from_gpos_lookup with separate 1..=9 kind domain, sharing existing flag/GDEF validation; compile_gpos uses it. GSUB 1..=8 validation unchanged. Preserve all failing tests; add domain/reserved flag/missing mark set regression. Expected 1315/platform, 3945 total; CI pending, no local compile. Next C4.5l2 only after green.


CI #201: 6e17f0c0fde4713b1c85de451168ca5f5278f7d5, run 45998230655: 1315/platform, 3945 total, zero failed/ignored; exact SHA and Release verified.

C4.5l2: distinct GposPlan, shared base feature selection, required features, language/script fallback, ordered unique lookup provenance, compilation revalidation against original GPOS bytes with bounded references. Fourteen new tests (13 plan + 1 real compilation), 1329/platform expected. Base-only variation diagnostic; variable feature selection, dependency closure, execution and native extension differential remain outstanding. Next C4.5l3 dependency discovery/closure after green. No local compile.


CI #202: d346d71fbbdbee13c44a12dbfb85f87b22c3d899, run 46000598254: 1329/platform, 3987 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5l3: GPOS dependency enumeration for all six Type 7/8 contextual formats including Type 9 wrappers; ordered action provenance, duplicate and unreachable rule retention, bounded scans, iterative graph closure shared with GSUB, missing sources and deterministic cycle witnesses. 24 new regressions (11 discovery + 13 closure), 1353/platform expected. Static analysis only; no nested execution. Next C4.5l4 reachable program compilation and root scheduling after green. No local compile.


CI #203: cc7916c87a85ae767ab568dd1f7d0235776e90d6, run 46001862519: 1353/platform, 4059 total, zero failed/ignored; exact SHA and Release verified in all three logs.

C4.5l4: compile reachable GPOS programs from explicit roots or validated base plan, compile each dependency once, original lookup indices and ordered actions retained, root-only top-level schedule, cumulative lookup/subtable/byte/action/scan budgets, diagnostic cycle witnesses and complete closure verification. Eight new tests, 1361/platform expected. No runtime execution yet; next C4.5m1 bounded single/pair positioning execution and original-index filtering. Native extension differential and variable feature selection remain open. No local compile.


CI #204: d49e69210a051486894eea9ae678de6118c20acf, run 46002959737: 1361/platform, 4083 total, zero failed/ignored; exact SHA and Release verified.

C4.5m1: bounded GPOS Single/Pair/ClassPair root execution including extension targets, first matching subtable, pair ValueRecord2 consumption, GDEF filtered original indices, separate design/device pixel adjustments, atomic owned result, work budgets. Nine new regressions; 1370/platform expected. Unsupported contextual/attachment roots rejected even for empty runs. No local compile. Next C4.5m2 nested contextual execution, depth/work limits and cycle protection; attachment and RTL cursive execution later.


CI #205: 239261babef1776c8ac1f72f66de3d273ec56dcd, run 46005447958: 1370/platform, 4110 total, zero failed/ignored; exact SHA and Release verified.

C4.5m2: bounded GPOS context/chained context execution for all six formats, nested single/pair and extension dispatch, parent-filtered original-index mapping with child eligibility, ordered actions, first match and input consumption, active lookup/index cycle guard, hard depth cap 64 and cumulative call/action/context/filter/subtable budgets. Eight new regressions; 1378/platform expected. Existing m1 API retained. No local compile. Attachment kinds rejected, no runtime attachment graph or variation evaluation; next C4.5m3 cursive/mark attachment execution design after green.


CI #206: 330c79d40b6a7c1172bd256df4647345ea0f800c, run 46007916470: 1378/platform, 4134 total, zero failed/ignored; exact SHA and Release verified.

C4.5m3a: explicit attachment forest foundation, anchor delta extraction, iterative parent-chain propagation of final pen origins and local residual placement, independent design/pixel spaces, cycle/depth/edge limits, atomic overflow rejection. Nine new tests; 1387/platform expected. Caller supplies edges, final pen origins and residuals; automatic MarkToBase/MarkToMark/MarkToLigature parent/component selection and Cursive RTL advance redistribution not yet integrated. Next C4.5m3b attachment selection/execution integration, then cursive and ligature provenance. No local compile.


CI #207: 937553db9680af31158e74cb13f825bd5d16248c, run 46010328089: 1387/platform, 4161 total, zero failed/ignored; SHA and Release verified. C4.5m3b: independent mark execution result with residuals and attachment edges, Type 4/6 roots and nested dispatch, GDEF eligible parent search, mark barriers, replacement semantics, bounded parent visits/updates, final origin resolver. Five new tests; 1392/platform expected. No local compile. Mark ligature provenance, exact ligature-id compatibility and cursive/RTL advance redistribution remain open. Next C4.5m3c after green.


CI #208: c1416bee96bb6c2c4b63c961692b6004d6a72a2d run 46012819780, 1392/platform 4176 total zero failed/ignored, exact SHA and Release verified. C4.5m3c: Type5 execution using explicit occurrence-specific ligature ID/component source metadata; validates unique IDs, source references and component counts; last-component fallback for unmatched identities; null anchors not matched; conservative MarkToMark same-source compatibility in provenance API. Five new tests, 1397/platform expected. No local compile. GSUB automatic provenance generation and full MarkToMark compatibility (unmapped/ligature edge cases), RTL/cursive advance redistribution remain open. Next C4.5m3d cursive execution design after green.


CI #209: 87d0901cc8c865c9cb77c902ec724548951544b7 run 46015378965, 1397/platform 4191 total zero failed/ignored, exact SHA and Release verified. C4.5m3d1: standalone horizontal cursive root execution, adjacent eligible glyph pairs and extension wrappers, signed nominal advance interval solver, RTL flag fixed-end attachment, preserved skipped advances, final origin/attachment resolution, resource bounds. Seven new tests, 1404/platform expected. No local compile. This is a bounded horizontal constraint solver, not complete OpenType RTL advance redistribution or mixed nested mark/cursive execution. Next C4.5m3d2 mixed execution and direction-aware redistribution/differential validation.


CI #210: 43732dd7f69449390ab224c13cdd29346ea603b6, run https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46017959056, exact SHA and raw job logs verified: 1404 passed/platform, 4212 total, zero failed/ignored. Release Windows 26.65s, Ubuntu 12.33s, macOS 19.25s.

C4.5m3d2a: shared bounded horizontal mixed GPOS scheduler, effective types 1–8 including extensions, same root/nested index/filter/call/action budgets and residual/attachment state; reuses horizontal interval/origin helpers and explicit ligature source validation. Cursive charges shared attachment updates and separate connection/span budgets. Rejects repeated cursive child, replacement of its parent, and existing mark edge on cursive endpoint; final forest validation. DesignOnly preserves caller pixel values and returns zero pixel origins. Five new synthetic regressions; 1409/platform expected, CI pending. No local Rust toolchain or network checkout; GitHub CI is required. Still limited signed logical-order horizontal convention; no full direction-aware redistribution, chain reversal, automatic provenance or real-font differential evidence. Next C4.5m3d2b after green: explicit flow direction and advance redistribution; then mixed chain interactions and differential validation.


CI #211: 2d07759d696c16e0d27cf757a16ecff6e57d1908, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46020802845, verified all raw jobs and exact Checkout SHA: 1409/platform, 4227 total, zero failed/ignored. Release Windows 27.76s, Ubuntu 15.68s, macOS 18.04s.

C4.5m3d2b: explicit GposHorizontalFlow and execute_gpos_mixed_directional, independent from Lookup RightToLeft flag. Logical order retained; physical signed widths and LTR prefix / RTL right-edge post-width origins. Cursive interval [a,b) in LTR and (a,b] in RTL, solve the corresponding endpoint width after resetting moving-child placement, preserve intervening widths and separate device corrections. Checked terminal pen, interval and coordinate arithmetic. Legacy API retains old behavior. Seven new synthetic tests including independent width-search grid (324 cases), both lookup flags/directions, skipped mark, devices, nested extension, mark chain, terminal overflow; 1416/platform expected, CI pending. Not full OpenType direction-aware endpoint redistribution (two-end corrections and overlapping chain reversal), no bidi reordering, no external reference differential evidence; the explicit physical coordinate contract is bounded and caller supplied. No local compilation performed. Next C4.5m3d2c after green: mixed chain interaction, stale/replacement edge handling and overlapping constraints, then real-font differential.


CI #212: a6340184ab18429801fb4d590febce90c771690b, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46024889008, verified exact SHA and raw logs: 1416/platform, 4248 total, zero failed/ignored. Release Windows 24.83s, Ubuntu 9.43s, macOS 8.50s.

C4.5m3d2c-1: directional mixed attachment contribution ledger, per-child reverse index and ordered non-overlapping active intervals; checked removal of only old cursive design/pixel advance contributions, preserving intervening value adjustments. Mark and cursive can replace moving-child edges, exact-pair fixed-end reversal clears stale edge; final forest rejects cycles. Adjacent intervals accepted; other overlapping live intervals explicitly rejected in logarithmic lookup, not silently overwritten. Removal charges shared attachment-update budget; legacy signed API retains its old policy. Eight new synthetic regressions; 1424/platform expected, CI pending. No local compilation. Residual placement follows local contract after detach, not full world-position preservation. General chain reversal, simultaneous overlapping interval solver and full OpenType dual-end redistribution remain incomplete. Next C4.5m3d2c-2 after green: bounded overlapping interval reconciliation and chain semantics, then C4.5m3d2d real-font differential.


CI #213: 95126dcf2aca943277619ba21beee72b302395de, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46028084906, exact SHA and all raw job logs verified: 1424/platform, 4272 total, zero failed/ignored. Release Windows 26.56s, Ubuntu 15.02s, macOS 26.31s.

C4.5m3d2c-2a: standalone exact integer overlapping interval solver (font_gpos_intervals.rs), prefix potential equalities, iterative weighted union-find with union by size, contradiction detection, deterministic lexicographic soft nominal-width retention (LTR reverse, RTL forward), physical anchor-to-interval conversion, checked potential/width/index arithmetic and cumulative work/allocation/domain limits. Seven new tests including independent exhaustive small width reference (50 parameter cases), overlapping/nested/crossing spans, redundant and inconsistent cycles, order independence, overflow and 20,000-glyph chain. 1431/platform expected, CI pending. No local compilation. Signed widths permitted; no optimization of total absolute deviation or positivity guarantee. Solver is standalone; mixed runtime retains its existing overlap rejection and tests until integration. Next C4.5m3d2c-2b after green: integrate active constraints with mixed ledger, preserve lookup-order semantics and budget, then real-font differential. General OpenType chain reversal and two-end redistribution remain incomplete.


CI #214: 823c24a8432715ef129d506fe62e18f9af53d254, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46031232864, exact SHA and raw jobs verified: 1431/platform, 4293 total, zero failed/ignored. Release Windows 15.00s, Ubuntu 12.17s, macOS 9.32s.

C4.5m3d2c-2b: opt-in execute_gpos_mixed_joint integrates active pair constraints with existing root/nested scheduler, GDEF filters, mark/cursive moving-child replacement, exact-pair reversal and final forest. Captures hard interval equations at attachment time; defers solving until final pen calculation, noncursive advances remain soft preferences, local placement residuals remain ordered; shared design/pixel union-find work budget and active constraint/glyph bounds. Legacy immediate directional and signed APIs unchanged, including rejection regression. Six synthetic regressions: nested intervals both flows, crossing filters, replacement, nested extension/soft value order, reversal/cycle, device/budget. 1437/platform expected, CI pending. No local compilation. Explicit deferred hard-constraint policy, not full OpenType immediate advance semantics; later ValueRecords in constrained widths may be redistributed, captured equations are not recomputed after later placement changes. General chain reversal and actual external font differential remain incomplete. Next C4.5m3d2d after green: real-font differential and document coverage; refine semantics where evidence differs.


CI #215: 5026ede64f8b8d53608868870916d36fbfbf5722, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46034356840, exact SHA and all raw jobs verified: 1437/platform, 4311 total, zero failed/ignored. Release Windows 25.95s, Ubuntu 10.87s, macOS 22.99s.

C4.5m3d2d-1: real_gpos_mixed_run.rs adds independent test-only raw SFNT/GPOS/GDEF/hmtx oracle over five already-vendored OFL fonts (Aref Ruqaa, Noto Nastaliq Urdu, Noto Sans Mongolian, Padauk, ABeeZee). Selects actual native Cursive and MarkBase root lookups, up to 24 matched pairs per lookup, both physical flows, compares nominal widths, final design origins, all adjustments and attachment edges for joint and immediate directional engines. Requires positive nonzero evidence, excludes mark filtering sets/attachment-class flags, design only, explicit glyph IDs, not external HarfBuzz differential or full script shaping. Two independent test functions, 1439/platform expected, CI pending; no local compilation or real case counts confirmed yet. Next C4.5m3d2d-2 after green: native multi-lookup chains, device and external reference comparison with explicit unsupported scope, then combined GSUB/GPOS. No new font downloads or production dependencies.


CI #216: 99d983b058de27509a716dd6f1e8e775f8f47345, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46037296945, verified exact SHA and raw logs: 1439/platform, 4317 total, zero failed/ignored; both native coordinate differential tests passed. Release Windows 26.45s, Ubuntu 12.38s, macOS 12.78s. Captured successful test output suppresses per-font sample counts; no exact count claimed.

C4.5m3d2d-2a: extend existing real font oracle with independent bitwise Device decoding and anchor offset selection, ppem grid, supported and unresolved contour/VariationIndex behavior, separate design and pixel origins/adjustments, plus native two-root Cursive+MarkBase triple-glyph chain differential with raw filter and direct world-coordinate reference. Requires nonvacuous device-supported and native chain evidence; nonzero native Device deltas counted but not guaranteed. Two new tests, 1441/platform expected, CI pending. No local compiler or external shaping tool validation. Selected native lookup/glyph cases only, not full script shaping or arbitrary chains. Next C4.5m3d2d-2b after green: external reference baseline (HarfBuzz test-only, reproducible provenance), compare supported semantics and report gaps; no production dependency. No CI workflow changes or new font assets.


CI #217: cbf225f4e3dda3c8988299bc1b5c8b1d4d4a90c7, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46040306627, verified exact SHA and raw jobs: 1441/platform, 4323 total, zero failed/ignored; native device and two-lookup chain tests passed. Release Windows 16.64s, Ubuntu 15.86s, macOS 17.35s. Per-font counts not printed on successful default Rust test capture.

C4.5m3d2d-2b-1: external HarfBuzz OT Latin kern reference gate: test-only Rust example gpos_reference_probe, Python standard-library ctypes runner, actual ABeeZee regular/italic fixtures, 10 strings per font (20 cases), explicit kern on/off with stable HB glyph IDs, UPEM LTR design-unit deltas for four position components, exact comparison against native RWCore GPOS plan/joint runner. Requires nonzero evidence, records HB version, font SHA-256, glyph IDs, reference/actual and maximum error in JSON artifact; mismatch fails Linux CI. Ubuntu system libharfbuzz only (apt fallback if missing), no Cargo/production dependency; cross-platform normal CI remains. Expected 1441 Rust tests/platform unchanged plus 20 Linux external comparisons, pending actual CI. No local Rust compilation. Reference uses installed HB version rather than pinning binary; records version and font hash for reproduction. Not a Cursive/Mark/RTL external compatibility claim and not GSUB/cmap validation, because HB provides glyph IDs. Next C4.5m3d2d-2b-2 after green: broaden external differential (mark and complex-script supported scope, document divergences), then GSUB+GPOS integration.


CI #218: b648d1dbb8070a9a21ada76e42e0ed88ae7ba664, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46043289386, verified exact SHA and raw logs: 1441/platform, 4323 total, zero failed/ignored; Release Windows 27.32s, Ubuntu 16.35s, macOS 21.44s. Ubuntu HarfBuzz 12.3.2 actual external kern 20/20, 20 nonzero, 0 mismatches, max error 0 design units; artifact ID 15221177482.

C4.5m3d2d-2b-2a: external Latin mark toggle gate over existing Alegreya Sans Regular, 63 deterministic candidate strings; HB on/off identity/scale and zero baseline placements/vertical advances determine admissible two-glyph cases, all exclusions and actual comparisons reported. Rust probe accepts explicit HB mark-disabled x advances to isolate GPOS from external width normalization; selected mark plan, zero prior residual, four component comparison. All admitted rows must match exactly and at least 3 nonzero reference placements required; no selection by RWCore agreement. Separate Ubuntu CI gate and JSON artifact; old 20 kern comparisons preserved. 1441 Rust tests/platform unchanged, actual admitted mark count and error pending CI. No local compilation; direct binary fetch was unavailable, candidate coverage not confirmed before CI. Reference uses HB glyph IDs and widths, not independent cmap/GSUB/normalization; no RTL, mkmk, arbitrary ligature provenance or Cursive external equivalence. Next C4.5m3d2d-2b-2b after green: multi-mark and complex-script external evidence with explicit scope, then joint GSUB/GPOS. No production dependency.


CI #219: 17dbae73f351dc0fca4fe91861eaf1dbe5d31958, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46045978739, exact SHA and raw jobs verified: 1441/platform, 4323 total, zero failed/ignored. Release Windows 25.09s, Ubuntu 12.11s, macOS 19.90s. HarfBuzz 12.3.2 Kern 20/20 all nonzero, Mark 45/45 from 63 candidates (17 nonzero placement, 18 exclusions), both max error 0. Mark artifact 15222007853; Kern artifact 15222815992.

C4.5m3d2d-2b-2b-1: test-only native Latin stacked mark+mkmk external gate. Up to five vendored fonts, independent raw feature presence check, 441 candidate strings per eligible font, stable three-glyph identity and zero off baseline placement required; compare all admitted four-component deltas against native joint executor, plus independently count HB mkmk-on versus mark-only nonzero positioning. Requires at least one positive native mkmk case and zero mismatch. Retains every candidate exclusion and actual delta in separate JSON artifact. Existing Kern and Mark gates unchanged. 1441 Rust tests/platform expected, external admitted/nonzero count pending CI. No local compilation or binary-font access; exact candidate coverage unverified. Test-only HB IDs/widths and LTR default coordinate scope, no full shaping/RTL or arbitrary ligature claim. Next C4.5m3d2d-2b-2b-2 after green: complex-script/RTL external reference and explicit compatibility gaps, then combined GSUB+GPOS.


CI #220: d1972edafd836c8cf72df720970af53f8dd3f707, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/46048762243, exact SHA and raw logs verified: 1441/platform, 4323 total, zero failed/ignored. Release Windows 26.40s, Ubuntu 10.27s, macOS 22.67s. HarfBuzz 12.3.2 Kern 20/20, Mark 45/45, stack 1072/1072 from 1764 candidates (1024 nonzero, 852 mkmk effect), all max error 0. Stack artifact 15223770935. Total 1137 exact external rows. Abel lacks eligible feature pair; four fonts admitted.

C4.5m3d2d-2b-2b-2a: Arabic RTL external diagnostic baseline for Aref Ruqaa and Noto Nastaliq Urdu, separate Mark and Cursive toggles, native feature inventory, logical-order conversion of HB RTL output, baseline physical widths, per-glyph four-component and right-edge-relative world-coordinate errors; up to 114 candidates. Reports all exclusions, probe errors and mismatches, requires nonzero external evidence and actual comparisons for both features. Diagnostic gate enforces evidence/accounting, not zero mismatch; compatibility_pass is explicitly separate. Previous 1137 external exact gate unchanged. Expected 1441 Rust tests/platform, RTL counts and error pending CI. No local compilation or font binary inspection. HB supplies glyph IDs and baseline advances, no independent cmap/GSUB/Bidi/full-script claim. Next C4.5m3d2d-2b-2b-2b after green: use actual RTL discrepancy report to repair bounded algorithm/coordinate semantics and establish exact gate for supported cases, then joint GSUB+GPOS. No production dependencies.
