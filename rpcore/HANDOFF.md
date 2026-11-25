# RPCore B-line handoff

Branch: feature/rpcore-bootstrap. Do not advance until latest CI completes, all 10 jobs pass and actual logs confirm gates. Fix failures within current stage. No scheduled autonomous development or merge.

ICC scope closed CI #107; see ICC_SUPPORT.md. Last verified B11.1e e1a03cb435a2bf4b723efc2b15615734cb620ef4, CI #117 run 45571498356, 10/10, 235 Rust tests/platform (705 total); all seven AES-128 primitive tests passed.

Current B11.1e2: authenticated R4 AESV2 integration, R4 metadata flag in file-key calculation, salted AES object key, StrF/StmF/EFF RC4/AES/Identity routing, metadata exemption, decrypted AES stream Length normalization, no duplicate object-stream decryption. Existing open_with_legacy_password and authenticate_legacy_password now include R4; ordinary open still rejects encryption. Restricted user permissions, explicit Crypt, indirect stream Type in R4 and separate EFOpen authentication fail closed; signature/xref special object access rejected. Four added tests with pypdf AlgV4 and Python cryptography independent fixtures, target 239/platform (717 total). CI pending. No dependencies, no guaranteed side-channel/zeroization audit; no R5/R6 yet. Next after green: AES-256/SHA-2 primitives then R5/R6 authentication, independent real encrypted PDF end-to-end gate. Keep PDF.js until integration gates pass.
