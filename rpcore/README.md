# RPCore 0.1 bootstrap
Independent, safe Rust prototype. No PDF engine dependencies. Supports basic PDF objects, classic xref, catalog/page tree and bounded parsing. Does not yet support streams, compression, xref streams, incremental updates, encryption, rendering or text extraction. Rejects unsupported structures. Not a production reader.
```
cargo test --manifest-path rpcore/Cargo.toml --locked
cargo run --manifest-path rpcore/Cargo.toml -- demo/Research_Reader_Test.pdf
```
The demo may use unsupported structures; failure is explicit. Next: stream boundaries and decoding, xref streams, incremental revisions, resource inheritance, fuzzing. 
