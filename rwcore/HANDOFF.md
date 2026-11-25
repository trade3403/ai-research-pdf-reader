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
C4.4a4b source pending CI; parent CI #88 verified 581 tests per platform. Check actual HEAD and CI; see DEV_STATE.md for next task. Preserve outstanding real point attachment/general matrix and corpus gates.
