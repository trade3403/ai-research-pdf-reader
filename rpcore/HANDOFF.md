# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not advance until latest CI finishes, all 10 jobs pass and actual logs are inspected. Fix failures in current stage. No scheduled autonomous development or merge.

ICC scope closed at CI #107, see ICC_SUPPORT.md. Last verified B11.1b a0c6a798ffade2c1767094afdd5806a0510feec9, CI #111 run 45569798774 10/10, 217 Rust tests/platform (651 total); MD5/RC4 six tests and 41 known-answer vectors passed.

Current B11.1c: legacy R2/R3 authentication (user/owner roles, password padding/truncation, MD5 file key, RC4 U/O validation). Document::authenticate_legacy_password(bytes,password) returns PasswordStatus only; caller supplies legacy encoded bytes; no raw key exposed. Document::open still rejects encrypted documents. Four synthetic independent pypdf/cryptography fixtures for both roles, incorrect passwords, ID and permissions mutations, R3 U tail, truncation and public API. Seven tests target 224/platform (672 total). CI pending. No dependency change; not side-channel/zeroization audited. Next after green: B11.1d authenticated RC4 object/stream integration with independent encrypted PDF fixtures, correct per-object keys, exclusions and permissions policy. R4/AES/R5/R6 not authenticated yet. Keep PDF.js until overall integration gates pass.
