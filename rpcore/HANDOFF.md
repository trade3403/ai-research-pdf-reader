# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not start next stage until latest CI is complete, all 10 jobs pass and actual gate logs are checked. Fix failures within current stage. No autonomous scheduled development.

Last verified: B10.5c5c3d4b5d4k, commit 8839228b0bfda7c2109aed636e19afeb3fea8662, CI #103 run 45560029926 (10/10). Gray partial 160/160 and missing CHAD 160/160 max RGB8 error 0; non-Bradford 2/2. Prior 2576 render observations and 1536 real CMYK relative observations retained. Core independent Rust, no complete external PDF engine in runtime. LittleCMS/Poppler test-only.

Current stage: B10.5c5c3d4b5d4l. External installed RGB profile gate in icc_print_benchmark.py, 1024 target observations, numeric only, strict native accuracy and explicit alternate policy. New CI pending; inspect report and distinguish native passes from alternates. No measured-monitor coverage claim. Next stage after green CI: ICC support matrix and closure, then general PDF compatibility, text selection/search and performance per docs/RPCORE_ROADMAP.md. User wants practical SCI reader rather than indefinite ICC refinement. No merge without instruction.
