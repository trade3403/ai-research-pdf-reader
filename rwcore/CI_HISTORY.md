# Verified CI evidence

Earlier runs and their revision-specific acceptance records are retained in README.md. This index starts with RDHS 1.0; append verified evidence, not predictions.

| Run | Code revision | Ubuntu | Windows | macOS | Release |
|---|---|---:|---:|---:|---|
| [#53](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540812833) | 8c64381f980318e01a448d3f86fa1b982da73f4e | 238/238 | 238/238 | 238/238 | All passed |
| [#54](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541326941) | 87382c6711bfefd8af0819ff593a6c43bf40c9e8 | 238/238 | 238/238 | 238/238 | All passed |
| [#55](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541593350) | ba91c2ae8f6180494ed9f4490629738873638872 | 248/248 | 248/248 | 248/248 | All passed |
| [#56](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45542423200) | 5f1785db549ff47f0a966d549a45df1692eca63c | 260/260 | 260/260 | 260/260 | All passed |
| [#57](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543111414) | 7bfbb839ded46b4bdd1346f4d73e1707d9d37a7c | 272/272 | 272/272 | 272/272 | All passed |
| [#58](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45544241000) | c15c08f3ab9240738cd94938792b0d7793b31d69 | 280/280 | 280/280 | 280/280 | All passed |

#53 exact SHA present in all job logs; 0 failed, 0 ignored, 714 executions passed.
Artifacts metadata verified, unexpired at check:
- macOS lock: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540812833/artifacts/14989720296
- Windows lock: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540812833/artifacts/14989092124
- Ubuntu lock: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540812833/artifacts/14989087297

These are Cargo.lock artifacts, not installable software. Artifact bytes/hash comparison not established here.

#54 completed successfully: exact checkout revision verified in each job log, 238 passed per platform, zero failed/ignored, release profile finished. Documentation-only handoff baseline.
Lock artifacts (metadata, not installer): [macOS](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541326941/artifacts/14989810839), [Windows](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541326941/artifacts/14989142251), [Ubuntu](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541326941/artifacts/14988559215). Artifact ZIP bytes and extracted lockfile equality not verified.

#55 completed: exact revision verified in all three job logs; 248 passed per platform, zero failed/ignored, Release builds completed. Lock artifact metadata verified: [macOS](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541593350/artifacts/14989601580), [Ubuntu](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541593350/artifacts/14989083399), [Windows](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45541593350/artifacts/14988684345). Not installable binaries; extracted lock equality remains unverified.

#56 completed: exact revision present in three job logs; 260 passed per platform, zero failed/ignored, release profiles finished. Lock artifact metadata verified: [Windows](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45542423200/artifacts/14989927288), [Ubuntu](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45542423200/artifacts/14989817555), [macOS](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45542423200/artifacts/14989672760). Not installable binaries; extracted lock equality not verified.

#57 completed: exact revision verified in all three job logs; 272 passed per platform, zero failed/ignored, Release profiles finished. Lock artifact metadata verified: [macOS](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543111414/artifacts/14990048020), [Windows](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543111414/artifacts/14990033074), [Ubuntu](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543111414/artifacts/14989239543). Not installable binaries; extracted lock equality unverified.

#58 completed: exact revision verified in all three job logs; 280 passed per platform, zero failed/ignored, Release builds completed. Lock artifact metadata verified: [Windows](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45544241000/artifacts/14990916258), [macOS](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45544241000/artifacts/14990906297), [Ubuntu](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45544241000/artifacts/14990886242). Not installable binaries; extracted lock equality remains unverified.
