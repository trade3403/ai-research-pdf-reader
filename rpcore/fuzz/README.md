# JBIG2 fuzzing (development only)
Isolated cargo-fuzz workspace; no new shipping decoder dependencies.
Run from rpcore/ with nightly and cargo-fuzz 0.12.0:
```sh
python3 fuzz/seed.py
cargo +nightly fuzz run jbig2_page -- -runs=5000 -max_total_time=90 -max_len=4096 -timeout=5 -rss_limit_mb=1024 -seed=12345
```
Input: two-byte little-endian globals length, then globals and page bytes;
overlong split clamps to available bytes. Three original Annex H seeds.
Tests actual PDF-image-compatible executor via a non-default feature, with
small resource caps, repeatability and bounds assertions. ASan and integer
overflow checks are development-only. Smoke run is not security certification.
CI uploads the log and any crash artifacts.
