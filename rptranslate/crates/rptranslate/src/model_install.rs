//! Product model installation with the existing real offline model loaders.
use crate::offline::OfflineTranslator;
use rpinfer::{marian_model::MODEL_PAYLOAD_BYTES, mixed_model::PAYLOAD_BYTES};
use rpmodel::{
    catalog::{Entry, Precision},
    install::{Error, Store},
};
use std::path::{Path, PathBuf};

fn validate(precision: Precision, root: &Path) -> Result<(), String> {
    let translator = match precision {
        Precision::Fp32 => OfflineTranslator::load(root, MODEL_PAYLOAD_BYTES),
        Precision::Int8 => OfflineTranslator::load_mixed(root, PAYLOAD_BYTES),
    }
    .map_err(|e| format!("{e:?}"))?;
    let session = translator.session().map_err(|e| format!("{e:?}"))?;
    let tokens = session
        .encode("We measured maximum strain.")
        .map_err(|e| format!("{e:?}"))?;
    if tokens.is_empty() {
        return Err("empty tokenizer probe".into());
    }
    Ok(())
}

/// The catalog must be independently trusted. Never generate production pins
/// from an untrusted import. The store and source must be exclusively owned.
pub fn import(store: &Store, entry: &Entry, source: &Path) -> Result<PathBuf, Error> {
    store.import_checked(entry, source, |stage| validate(entry.precision, stage))
}

/// Recheck a previously interrupted stage before allowing publication.
pub fn recover(store: &Store, entry: &Entry) -> Result<PathBuf, Error> {
    store.recover_checked(entry, |stage| validate(entry.precision, stage))
}
