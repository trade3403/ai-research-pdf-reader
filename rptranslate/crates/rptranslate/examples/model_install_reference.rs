//! CI only: derive pins from already verified, private conversion fixtures.
//! Production imports instead require a separately trusted catalog.
use rpinfer::{marian_model::MODEL_PAYLOAD_BYTES, matmul::Kernel, mixed_model::PAYLOAD_BYTES};
use rpmodel::{
    catalog::{Asset, Catalog, Entry, Precision, Version},
    install::Store,
    lifecycle,
    sha256::digest_reader,
};
use rptranslate::{model_install, offline::OfflineTranslator};
use std::{collections::BTreeMap, fs, path::Path};

fn fixture_entry(source: &Path, precision: Precision) -> Result<Entry, String> {
    let mut files = BTreeMap::new();
    for item in fs::read_dir(source).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name().into_string().map_err(|_| "file name")?;
        let file = fs::File::open(item.path()).map_err(|e| e.to_string())?;
        let (bytes, sha256) = digest_reader(file, 512 * 1024 * 1024).map_err(|e| e.to_string())?;
        files.insert(name, Asset { bytes, sha256 });
    }
    Ok(Entry {
        id: "opus-en-zh".into(),
        version: Version(1, 0, 0),
        precision,
        min_engine: Version(1, 0, 0),
        files,
    })
}

fn probe(source: &Path, store: &Store, precision: Precision) -> Result<(), String> {
    let entry = fixture_entry(source, precision)?;
    let path = model_install::import(store, &entry, source).map_err(|e| format!("{e:?}"))?;
    store
        .verify_installed(&entry)
        .map_err(|e| format!("{e:?}"))?;
    let translator = match precision {
        Precision::Fp32 => OfflineTranslator::load(&path, MODEL_PAYLOAD_BYTES),
        Precision::Int8 => OfflineTranslator::load_mixed(&path, PAYLOAD_BYTES),
    }
    .map_err(|e| format!("{e:?}"))?;
    let mut options = translator.default_options();
    options.beam_width = 1;
    options.max_new = 64;
    let session = translator.session().map_err(|e| format!("{e:?}"))?;
    let result = session
        .translate("We measured maximum strain.", options, Kernel::Auto)
        .map_err(|e| format!("{e:?}"))?;
    if result.text.trim().is_empty() {
        return Err("empty installed translation".into());
    }
    println!(
        "MODEL_INSTALL PASS precision={precision:?} payload_bytes={} translation_nonempty=true",
        translator.payload_bytes()
    );
    lifecycle_probe(source, store, &entry)
}

fn lifecycle_probe(source: &Path, store: &Store, entry: &Entry) -> Result<(), String> {
    let mut next = entry.clone();
    // Same audited weights, different version identifiers: lifecycle test,
    // not a claim of a newly trained or independently released model.
    next.version = Version(2, 0, 0);
    let catalog =
        Catalog::new(vec![entry.clone(), next.clone()]).map_err(|e| format!("{e:?}"))?;
    model_install::activate(store, entry).map_err(|e| format!("{e:?}"))?;
    model_install::import(store, &next, source).map_err(|e| format!("{e:?}"))?;
    model_install::activate(store, &next).map_err(|e| format!("{e:?}"))?;
    if lifecycle::active(store, &catalog, entry).map_err(|e| format!("{e:?}"))? != Some(&next)
        || lifecycle::uninstall(store, entry) != Err(lifecycle::Error::ProtectedVersion)
    {
        return Err("upgrade or rollback protection".into());
    }
    model_install::rollback(store, &catalog, entry).map_err(|e| format!("{e:?}"))?;
    if lifecycle::active(store, &catalog, entry).map_err(|e| format!("{e:?}"))? != Some(entry) {
        return Err("rollback selection".into());
    }
    lifecycle::forget_previous(store, entry).map_err(|e| format!("{e:?}"))?;
    lifecycle::uninstall(store, &next).map_err(|e| format!("{e:?}"))?;
    let next_path = store.installed_path(&next).map_err(|e| format!("{e:?}"))?;
    if next_path.exists() {
        return Err("uninstalled version still present".into());
    }
    lifecycle::deactivate(store, entry).map_err(|e| format!("{e:?}"))?;
    lifecycle::uninstall(store, entry).map_err(|e| format!("{e:?}"))?;
    if lifecycle::selection(store, entry)
        .map_err(|e| format!("{e:?}"))?
        .is_some()
    {
        return Err("selection remains after deactivation".into());
    }
    println!(
        "MODEL_LIFECYCLE PASS precision={:?} upgrade=true rollback=true uninstall=true same_weights=true",
        entry.precision
    );
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [fp32, int8, root] = args.as_slice() else {
        return Err("usage: model_install_reference FP32 INT8 NEW_STORE".into());
    };
    fs::create_dir(root).map_err(|e| e.to_string())?;
    let store = Store::open(Path::new(root), Version(1, 0, 0)).map_err(|e| format!("{e:?}"))?;
    probe(Path::new(fp32), &store, Precision::Fp32)?;
    probe(Path::new(int8), &store, Precision::Int8)?;
    Ok(())
}
