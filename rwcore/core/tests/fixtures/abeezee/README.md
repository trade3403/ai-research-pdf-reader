# ABeeZee Regular real-font fixture

Unmodified upstream binary from https://github.com/google/fonts/tree/main/ofl/abeezee .
Git blob 2ecf01bdec7b6a8568825da840897f9b0aee8a86; 46016 bytes.
OFL.txt from the same directory, upstream blob fc163c7f211e6aa6ad5d66171e2e7913d66190f0.
SIL OFL 1.1, original copyright and license retained.
Expected glyph IDs and hmtx values independently extracted from the upstream binary using a separate JavaScript big-endian SFNT/cmap4/hmtx reader; not an independent rasterizer. Rendering tests are smoke/metamorphic checks, not reference grayscale acceptance.

Italic companion: original upstream blob da4bf4754d583f833e0dd276094697275c0a53d2, 47012 bytes, same OFL license. Independent JavaScript SFNT/loca/glyf flag and delta decoder produced exact simple-glyph point/endpoint fixtures for A,g,O; independent cmap4/hmtx values for five characters. No independent composite assembly reference yet.

Regular IDs 6 and 81 composite fixtures independently decoded in JS directly from original SFNT bytes, recursively expanding signed XY offsets and contour endpoint rebasing; no production parser used. Translation-only scope.
