# RPCore 0.1 bootstrap
Independent, safe Rust prototype. No PDF engine dependencies. Not a production reader.

## Supported
- Basic PDF objects, classic and stream xref, compressed objects, hybrid and incremental index revisions, catalog/page tree, inherited MediaBox/CropBox/Rotate/Resources.
- Direct and indirect scalar stream lengths, bounded reference chains, raw/ASCIIHex/ASCII85/RunLength/Flate stream decoding, including zlib checksum verification.
- TIFF predictor 2 and PNG predictors 10–15 (1/2/4/8/16-bit samples), aligned direct DecodeParms dictionaries/arrays; indirect filter parameters remain unsupported.
- Page Contents stream or ordered stream array, bounded decoded output, lexical content operations (raw operands and operator bytes).
- Explicit errors on unsupported features; font-level Unicode mapping and strict horizontal page text extraction available; partial vector display-list interpretation and CPU rasterization available.

## Build
```
cargo test --manifest-path rpcore/Cargo.toml --locked
cargo run --manifest-path rpcore/Cargo.toml -- demo/Research_Reader_Test.pdf
```

The demo may use unsupported structures; failure is explicit.

## A-line integration contract (Rust API, additive)
`Document::page_content(&PageInfo, limit)` returns decoded bytes, at most min(limit, 16 MiB). Stream arrays are joined with LF at token boundaries. A missing or null Contents is empty.
`Document::page_operations(&PageInfo, limit)` returns ordered `Vec<content::Operation>`; each operation contains raw operator bytes and parsed Object operands. Unknown operators are preserved, not executed. Inline images and indirect operands are rejected. No CLI output changes and no Electron integration changes.

## B05 decoding contract
The existing `filters::decode(&Object, limit)` API is unchanged. Final output is capped by the requested limit and 100 MiB; each intermediate filter stage is capped at 100 MiB (including predictor row tags). The decoder rejects malformed Huffman trees, invalid distances, truncated streams, invalid zlib checksums and preset dictionaries. No new Cargo dependencies. These are per-stage bounds, not a global resource sandbox.

## B06 index contract
`Document::open` and `resolve(id, generation)` retain their signatures. Index revisions merge newest-first by object number, including free tombstones; hybrid streams override same-revision table entries. XRef streams support direct W/Index/Size/Length and existing filters. Object streams validate their header, ID/index and object bounds; a mutex-protected single-container cache retains at most 16 MiB of decoded bytes plus its bounded index. Limits include 64 revisions and 100,000 cumulative entries (hybrid entries counted conservatively). No encryption, recovery scanning, indirect XRef bootstrap fields, or unknown xref entry types. Synthetic regression fixtures exercise compressed pages, hybrid and sparse incremental revisions, deleted objects and malformed indices. Real-world compatibility still needs corpus validation.

## B07 font contract
`Document::page_font(&PageInfo, resource_name)` lazily resolves inherited font resources. `fonts::Font::decode(bytes)` yields source code bytes, optional CID, optional TrueType glyph ID, optional Unicode string, and optional horizontal width in 1/1000 text-space units. Supports Type1/MMType1 and TrueType simple font dictionaries, explicit WinAnsi and limited StandardEncoding/Differences, Type0 Identity-H/V and embedded explicit CID CMaps, CIDFontType0/2 W/DW, CIDToGIDMap, ToUnicode bfchar/bfrange (including UTF-16 surrogate pairs and multi-character mappings). Unknown mappings remain None; no guessed Unicode for arbitrary CIDs. CMap usecmap, predefined non-Identity CMaps, full Adobe Glyph List, MacRoman/Symbol encodings, Standard14 built-in metrics, vertical advances W2/DW2, Type3 and embedded TrueType/CFF outlines remain unsupported. CMap parsing does not execute PostScript; each map is capped at 4 MiB input and 65,536 mappings per kind. No existing API changes, no new dependencies.

## B08 text contract
`Document::page_text(&PageInfo, content_limit)` returns `text::PageText` with content-order glyphs, Unicode, source codes, optional CID/GID, user-space origin, advance endpoint, approximate em-height quads, and rendering mode. Interprets BT/ET, Tf, Tc/Tw/Tz/TL/Ts/Tr, Tm/Td/TD/T*, Tj/TJ/quote operators, q/Q/cm. Quads are NOT ink bounds; page CropBox/Rotate and screen coordinate conversion are caller responsibilities. `plain_text()` concatenates content order without inferred whitespace; `search()` provides exact case-sensitive non-overlapping substring matches as half-open glyph ranges, and `select()` intersects quad AABBs. No visual reading-order/column inference, Form XObject traversal, ActualText interpretation, clipping/visibility testing or vertical positioning yet. External objects and marked content produce explicit warnings; unsupported ExtGState/unknown operators and missing glyph widths fail rather than guess. 100,000 page glyphs and 16 MiB Unicode output limits; no new dependencies. Existing APIs unchanged.

## B09 vector graphics contract
`Document::page_display_list(&PageInfo, content_limit)` returns `graphics::DisplayList`: ordered Save/Restore, path Paint (nonzero/even-odd fill and stroke parameters), Clip, Image reference and deferred Text commands. Path segments are transformed at construction to unrotated PDF user space; strokes retain painting CTM, width, cap, join, miter and dash. Clips represent intersections in command order, not computed polygons; initial clip is CropBox. Supports q/Q/cm, m/l/c/v/y/h/re, paint and W/W*, device Gray/RGB/CMYK and explicit device color spaces. Form XObjects expand in place with local Matrix/BBox/Resources, isolated graphics state and cycle/depth detection; image samples and text outlines remain deferred with explicit warnings. Unsupported ExtGState, shading, patterns, transparency groups, text clipping and unknown operators return errors. Text operators retained for future rendering are not fully validated; use B08 for text extraction. No color-managed conversion, rasterization, or exact clip/stroke tessellation yet. Cumulative page+Form decoded bytes capped by content_limit (max 16 MiB), 100,000 operations/commands, 200,000 path segments, 64 local graphics saves, 32 Form depth. Existing API unchanged, no dependencies.

## B10.1 raster contract
`Document::render_page(page, raster::Options { scale, allow_missing_text }, content_limit)` returns opaque top-left row-major RGBA8 `Bitmap` with dimensions and missing_text flag; `Bitmap::png()` encodes PNG independently. CropBox and Rotate 0/90/180/270 applied; scale is pixels per PDF coordinate unit (include UserUnit externally). 2x2 sample antialiasing, adaptive cubic flattening, nonzero/even-odd scanline fill, clip-mask intersections, save/restore, solid uniform-transform strokes (caps, joins, miter), approximate device CMYK conversion, nearest-neighbor 8-bit unmasked DeviceGray/RGB/CMYK images using supported stream filters. Dash and nonuniform stroke transforms are rejected; unsupported image masks/decode/interpolation and text outlines fail explicitly. Only opt-in allow_missing_text permits incomplete preview with missing_text=true. No color management, transparency, glyph rendering, or full-page compatibility yet. Bounds: 4 million output/image pixels, 8192 side, 200k flattened edges, 200m work units, 64 MiB conservative clip stack budget. No dependencies.

Render a supported page: `cargo run --manifest-path rpcore/Cargo.toml --release -- input.pdf --render 1 output.png 1.5`. Generate a real synthetic PDF and its rendered PNG: `cargo run --manifest-path rpcore/Cargo.toml --release --example render_demo -- rpcore/target/render-demo`. CI uploads the pair per platform. Not a GUI or a real-paper rendering claim.

## B10.2 TrueType contract
`fonts::truetype::TrueType::parse(Vec<u8>)` parses bounded sfnt head/maxp/loca/glyf/hhea/hmtx; `outline(gid)` emits cubic paths from on/off-curve points and implied quadratic midpoints, with XY-offset compound glyph transforms and cycle/depth/point budgets. `horizontal_metrics(gid)` exposes font advances and bearings; PDF W/DW still governs text spacing. Strict `render_page` renders horizontal embedded Type0 CIDFontType2 page text via CIDToGIDMap and FontFile2, retaining text operators, matrices, spacing, rise, fill/stroke and invisible mode. No hinting, checksums, TrueType simple-font cmap/name mapping, point-attached compound glyphs, Form text, vertical text, CFF/Type1 or system-font substitution yet. Explicit `allow_missing_text=true` remains a graphics-only preview, skipping all text with the missing_text flag. Maximum 16 MiB per embedded font, 32 MiB total font bytes, 16 fonts, 100k points per glyph and 16 recursion depth. Glyphs are rasterized by the native B10.1 path engine; performance remains prototype-level.

## B10.3a CFF1 contract
`fonts::cff::Cff::parse` reads CFF1 INDEX, DICT, charset, CID FDArray/FDSelect 0/3, Private DICT and local/global Subrs. `outline(gid)` interprets Type2 relative moves, lines, cubic curves, flex and hint masks (hints ignored), subroutine bias and bounded operand/depth/work. Strict renderer supports embedded FontFile3 Type1C with explicit PDF Differences glyph names, and CIDFontType0C with charset CID lookup. Top FontMatrix applied, PDF W/DW/Widths control spacing. Unsupported predefined Expert charsets, nonidentity per-FD matrices, Type2 arithmetic/stack escaped operators, seac, CFF2/OpenType wrappers, traditional encrypted FontFile Type1 and implicit simple-font encodings fail explicitly. No dependencies; no full PostScript interpreter.

## B10.3b traditional Type1 contract
`fonts::type1::Type1` reads conventional Adobe-style PFA (ASCII hex), PFB segment containers and PDF FontFile Length1/2/3, decodes eexec (55665) and CharString (4330) encryption including lenIV=-1, recognizes FontMatrix, StandardEncoding or conventional explicit 256-entry encoding, Subrs and CharStrings RD/-| data declarations. No PostScript execution. Bounded Type1 CharString interpreter supports hsbw/sbw, relative moves/lines/cubics, closepath, callsubr/return, div, and ignored stem hints. Strict native rendering uses FontFile with explicit PDF Differences and/or embedded or StandardEncoding name mapping; PDF Widths govern advance. Unsupported OtherSubrs/flex, seac, arbitrary PostScript/encoding procedures, WinAnsi/MacRoman overrides, nonstandard binary declarations, and Type1 font substitution fail explicitly. No external dependencies. 16 MiB per font, 1m tokens, 65535 entries, 100k CharString steps, 16 subroutine depth, 24 operands. This is a restricted Type1 subset, not complete compatibility.

## Remaining
Encryption, complete font encoding/metrics and outlines, complete page text coverage and layout order, complete graphics/color interpretation, full rasterization and text rendering, resource graph expansion, global cumulative parsing/memory budgets and fuzzing. Existing parsing bounds are not a production sandbox.

## Deferred iOS validation (B-line core only)
Routine GitHub Actions currently runs Windows and Ubuntu only. iOS device cross-build and simulator jobs are deferred to keep iteration fast. The scripts below remain available for future manual validation; the previous workflow is preserved in commit `685ec546d6c82c1163e0cba6f92201357fd06b39`. Re-enable after core parsing and rendering stabilize. Planned iOS jobs:
- Device: cross-compile the test executables for `aarch64-apple-ios` (without running them), build and upload the release Rust library.
- Simulator: boot an installed iPhone simulator, run the Rust unit/integration test executables using `xcrun simctl spawn` through Cargo's target runner, then build and upload the `aarch64-apple-ios-sim` release library.
- Simulator startup or test failure fails its own job. Neither job implies a signed iOS app, a physical-device test, a Swift/C ABI, or App Store readiness.

To run locally on Apple Silicon with Xcode and an installed iOS runtime:
```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --manifest-path rpcore/Cargo.toml --locked --release --target aarch64-apple-ios --lib
xcrun simctl list devices available -j > /tmp/rpcore-simulators.json
export RPCORE_SIMULATOR_UDID="$(python3 rpcore/ci/select-ios-simulator.py /tmp/rpcore-simulators.json)"
xcrun simctl boot "$RPCORE_SIMULATOR_UDID" # skip if already booted
xcrun simctl bootstatus "$RPCORE_SIMULATOR_UDID" -b
chmod +x rpcore/ci/ios-simulator-runner.sh
export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER="$PWD/rpcore/ci/ios-simulator-runner.sh"
cargo test --manifest-path rpcore/Cargo.toml --locked --target aarch64-apple-ios-sim --lib --tests
```
