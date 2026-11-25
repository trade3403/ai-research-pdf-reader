# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not advance until latest CI completes, all 10 jobs pass and actual logs checked. Fix failures within current stage. No scheduled autonomous development or merge.

ICC closed CI #107; encryption read subset closed CI #125, 80/80 comparisons. Last verified B11.2c 27e47e68896a2634ae4c7551f1d96dc6a3834975, CI #131 run 45581965753, 10/10, 271 tests/platform (813 total).

Current B11.2c2: RPCL/PCRL/CPRL reference-grid packet ordering for dyadic spatial sampling, existing LRCP/RLCP unchanged, packet bit reader FF stuffing/alignment, persistent tag-tree threshold decoder (atomic rollback), seven added tests target 278/platform (834 total). CI pending. Synthetic spatial/bit/tag-tree checks, not independent external packet decoding parity. No full packet header parsing, entropy or pixel output. Non-dyadic spatial sampling explicitly rejected. Next after green B11.2c3 packet header fields, Lblock/segment lengths and independent full packet fixtures. Keep PDF.js until overall integration gates pass. See IMAGE_DECODER_SUPPORT.md.
