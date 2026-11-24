# RPCore 0.1 bootstrap
Independent, safe Rust prototype. No PDF engine dependencies. Not a production reader.

## Supported
- Basic PDF objects, classic xref, catalog/page tree, inherited MediaBox/CropBox/Rotate/Resources.
- Direct and indirect scalar stream lengths, bounded reference chains, raw/ASCIIHex/RunLength stream decoding.
- Page Contents stream or ordered stream array, bounded decoded output, lexical content operations (raw operands and operator bytes).
- Explicit errors on unsupported features; no graphics execution or Unicode text extraction yet.

## Build
```
cargo test --manifest-path rpcore/Cargo.toml --locked
cargo run --manifest-path rpcore/Cargo.toml -- demo/Research_Reader_Test.pdf
```

The demo may use unsupported structures; failure is explicit.

## A-line integration contract (Rust API, additive)
`Document::page_content(&PageInfo, limit)` returns decoded bytes, at most min(limit, 16 MiB). Stream arrays are joined with LF at token boundaries. A missing or null Contents is empty.
`Document::page_operations(&PageInfo, limit)` returns ordered `Vec<content::Operation>`; each operation contains raw operator bytes and parsed Object operands. Unknown operators are preserved, not executed. Inline images and indirect operands are rejected. No CLI output changes and no Electron integration changes.

## Remaining
Flate, predictors, xref/object streams, incremental updates, encryption, font decoding/ToUnicode, graphics interpretation, rendering, resource graph expansion, global cumulative parsing/memory budgets and fuzzing. Existing parsing bounds are not a production sandbox.

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
