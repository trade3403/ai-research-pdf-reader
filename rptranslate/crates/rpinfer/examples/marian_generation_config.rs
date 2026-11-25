//! Development-only adapter probe; accepts already parsed metadata.
use rpinfer::search::MarianGeneration;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 11);
    let suppress: Vec<u32> = args[10]
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.parse().unwrap())
        .collect();
    let profile = MarianGeneration {
        start: args[2].parse().unwrap(),
        eos: args[3].parse().unwrap(),
        pad: args[4].parse().unwrap(),
        num_beams: args[5].parse().unwrap(),
        min_length: args[6].parse().unwrap(),
        max_length: args[7].parse().unwrap(),
        length_penalty: args[8].parse().unwrap(),
        forced_eos: args[9].parse().ok(),
        suppress: &suppress,
    };
    let c = profile.search_config(args[1].parse().unwrap()).unwrap();
    println!(
        "MARIAN_CONFIG PASS start={} eos={} beams={} min_new={} max_new={} penalty={} forced_eos={} suppress={:?}",
        c.start, c.eos, c.beam_width, c.min_new, c.max_new,
        c.length_penalty, c.force_eos_at_limit, c.suppress
    );
}
