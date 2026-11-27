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
