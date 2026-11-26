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
