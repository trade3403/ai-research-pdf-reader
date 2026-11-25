# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not advance until latest CI completes, all 10 jobs pass and actual logs checked. Fix failures within current stage. No scheduled autonomous development or merge.

ICC closed CI #107. Encryption read subset closed CI #125: 756 cross-platform tests, 5 revisions and 80/80 comparisons. See ENCRYPTION_SUPPORT.md. Last verified B11.2a 7f2d3df04e96d61b803254a23af7edcd5991d27a, CI #127 run 45579805332, 10/10, 259 Rust tests/platform (777 total), 7 JP2 geometry tests/platform.

Current B11.2b: autonomous Part 1 COD/COC/QCD/QCC and SOT/SOD/EOC tile-part boundary parser, independent OpenJPEG 2.5.4 gray/RGB fixtures, 6 new tests target 265/platform (795 total). CI pending. Explicitly unsupported: tile-local overrides, packet header extensions, entropy/pixel decode. No runtime dependencies. See IMAGE_DECODER_SUPPORT.md. Next after green B11.2c tile-component resolution/subband/precinct/codeblock geometry and packet traversal plan. Keep PDF.js until overall integration gates pass.
