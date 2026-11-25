//! Verify real package metadata and bind the autonomous search configuration.

use rpinfer::search::MarianGeneration;
use std::path::Path;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("missing package directory")?;
    let (c, reader) = package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    let search = MarianGeneration::from_profile(&c.generation)
        .search_config(c.vocab)
        .map_err(|e| format!("{e:?}"))?;
    println!(
        "MARIAN_PROFILE PASS vocab={} hidden={} enc_layers={} dec_layers={} enc_heads={} dec_heads={} enc_ffn={} dec_ffn={} positions={} activation={:?} scale={} tensors={}",
        c.vocab, c.hidden, c.encoder_layers, c.decoder_layers, c.encoder_heads, c.decoder_heads,
        c.encoder_ffn, c.decoder_ffn, c.max_positions, c.activation, c.embedding_scale, reader.tensors().len()
    );
    println!(
        "MARIAN_SEARCH_PROFILE PASS start={} pad={} eos={} beams={} min_new={} max_new={} penalty={} forced_eos={} suppress={:?}",
        search.start, c.generation.pad, search.eos, search.beam_width, search.min_new,
        search.max_new, search.length_penalty, search.force_eos_at_limit, search.suppress
    );
    Ok(())
}
