# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not advance until latest CI finishes, all 10 jobs pass and actual logs are checked. Fix failures in current stage; no autonomous scheduled development or merge.

ICC closure verified: B10.5c5c3d4b5d4m, 7fe48de1000b21424b844784b285a411579ecb04, CI #107 run 45566224005, 10/10, 609 Rust tests, external RGB closure 1024/1024 native, 5458 render/diagnostic observations, 1875 raw MPE. Freeze validated source-color scope; see ICC_SUPPORT.md. Not full ICC conformance.

Current stage B11.1a: structural encryption inspection, separate Document::encryption_info API. Standard handler V/R and key sizes, O/U/OE/UE/Perms shape, crypt filter routing, document ID, direct encryption dictionary bootstrap, classic/stream/incremental xref fixtures. No password authentication or decryption. Document::open still rejects encrypted files; no bypass. Eight new Rust tests, target 211/platform, 633 total. CI pending. Next after green: autonomous MD5/RC4 primitives and independent known-answer tests, then authenticated legacy file-key and object decryption; AES/SHA later. Do not claim crypto production safety before security review and independent fixtures. Preserve old PDF.js until overall integration gates pass.
