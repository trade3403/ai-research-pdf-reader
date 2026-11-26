# C 线路当前研发检查点

- CI #162, c13b1f4c1bea9bfed4522d39792878472d1c8ea9, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45840611187 : 1143/platform, 3429 total, zero failed/ignored; exact SHA and Release builds verified from three job logs.
- C4.4g3: separate strict mask API, common feature intersection across matched input/ligature components, inherited feature bits and live action-anchor checks, mask-disabled components are barriers not ignored gaps; backtrack/lookahead remain conditions. Legacy root-only API preserved. Three synthetic positive/negative regressions with native cmap seeds, 1146/platform expected, CI pending. Explicit conservative policy, not external shaper parity. Next C4.4g4 strict native and independent reference acceptance.
- GPOS, script stages, avar 2.0, gvar/HVAR/MVAR, real point attachment, DOCX corpus and security/performance remain outstanding.

- CI #163, 7dc069e500b0c2294290db99c7c50db81ab928c0, https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45843654359 : all three jobs failed compilation E0061 at font_gsub_context_execute.rs:551, forward apply_at missing masking and bits; exact SHA verified, no tests executed.
- C4.4g3 repair: pass mask state and root bits to forward scheduler call, audit all apply_at callers, ensure all six context candidate formats reject zero common bits before selecting a match (including coverage formats), preserve existing regressions unchanged. 1146/platform expected; CI pending. Do not advance C4.4g4 until full gate.
