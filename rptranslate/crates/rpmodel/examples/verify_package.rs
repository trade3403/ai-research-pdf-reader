//! Development probe for the autonomous fixed-profile package verifier.
use rpmodel::package;
use std::path::Path;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing package directory")?;
    let reader = package::open(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    println!(
        "RPMODEL_PACKAGE PASS tensors={} payload_bytes={} revision={}",
        reader.tensors().len(),
        package::PAYLOAD_BYTES,
        package::REVISION
    );
    Ok(())
}
