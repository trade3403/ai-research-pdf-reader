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
