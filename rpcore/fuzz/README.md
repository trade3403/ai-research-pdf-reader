# JBIG2 fuzzing (development only)
Isolated cargo-fuzz workspace; no new shipping decoder dependencies.
Build the locked cargo-fuzz CLI with Rust 1.85.1 stable; run the instrumented target with nightly. From rpcore/:
```sh
rustup toolchain install 1.85.1 --profile minimal
rustup toolchain install nightly --profile minimal
cargo +1.85.1 install cargo-fuzz --version 0.12.0 --locked
python3 fuzz/seed.py
cargo +nightly fuzz run jbig2_page -- -runs=5000 -max_total_time=90 -max_len=4096 -timeout=5 -rss_limit_mb=1024 -seed=12345
```
Input: two-byte little-endian globals length, then globals and page bytes;
overlong split clamps to available bytes. Three original Annex H seeds.
Tests actual PDF-image-compatible executor via a non-default feature, with
small resource caps, repeatability and bounds assertions. ASan and integer
overflow checks are development-only. Smoke run is not security certification.
CI uploads the log and any crash artifacts.

## Multi-seed CI campaign
CI now runs three independent corpora, seeded identically from Annex H,
with random seeds 12345, 67890, and 314159. Each must finish 50,000 runs
within its 120s limit, with ASan, overflow checks, 4096-byte inputs,
5s per-input timeout and 1024 MB RSS cap. The CI gate requires the actual
libFuzzer DONE and completed-run records, not merely a zero exit status.
Per-seed logs and crash artifacts are retained. Coverage counters overlap
across seeds and must not be summed or called percentage coverage.
