# C 线路当前研发检查点

- Last verified code revision: 8c64381f980318e01a448d3f86fa1b982da73f4e
- Stage: C2.4c-2 ordered inline picture document integration
- CI #53: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540812833
- Verified: Ubuntu 238/238, Windows 238/238, macOS 238/238; total 714/714; zero failed/ignored; all release builds passed.
- Next: C2.4d-1 bounded floating picture anchor metadata parser (wp:anchor, positionH/V, relative references, wrap type, distances, overlap/layer flags). Keep separate from actual positioning/painting; preserve existing inline picture behavior.
- This checkpoint is being added in a documentation-only commit; its own HEAD/CI must be checked before the next code batch. No new algorithm or test is added by RDHS 1.0.
- Draft PR #1 remains open, no merge.
- Status updates and new test counts: update this file in each subsequent source batch, mark new tests pending until verified. Record verified baseline SHA rather than attempting to embed self-referential SHA.
- Open issues: KNOWN_ISSUES.md. Detailed historical algorithms: README.md.
