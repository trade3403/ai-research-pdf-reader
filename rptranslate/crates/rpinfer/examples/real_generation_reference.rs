//! Test actual checkpoint cache and search; binary logits then ASCII hypotheses.
use rpinfer::{
    marian_decoder::{MarianDecoder, DECODER_PAYLOAD_BYTES},
    marian_encoder::{MarianEncoder, ENCODER_PAYLOAD_BYTES},
    marian_generation::MarianEncoded,
    marian_weights::MarianEmbeddings,
    matmul::Kernel,
    search::MarianGeneration,
    tensor::Error,
};
use rpmodel::package;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing model directory")?;
    let (config, mut reader) =
        package::open_profile(Path::new(&path)).map_err(|e| format!("{e:?}"))?;
    let embeddings =
        MarianEmbeddings::load(&mut reader, &config, 135_479_204).map_err(|e| format!("{e:?}"))?;
    let encoder = MarianEncoder::load(&mut reader, &config, ENCODER_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let decoder = MarianDecoder::load(&mut reader, &config, DECODER_PAYLOAD_BYTES)
        .map_err(|e| format!("{e:?}"))?;
    let cases = [
        (&[1, 2048, 0][..], None, &[65000, 4, 4096][..], None),
        (
            &[2047, 2048, 4095, 4096, 0, 65000][..],
            Some(&[true, true, true, true, true, false][..]),
            &[65000, 4, 65000, 4096][..],
            Some(&[true, true, false, true][..]),
        ),
    ];
    let mut out = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    for (source, source_keep, target, target_keep) in cases {
        let memory = encoder
            .forward(&embeddings, source, source_keep, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        let encoded = MarianEncoded::new(&decoder, &embeddings, &memory, source_keep, Kernel::Auto)
            .map_err(|e| format!("{e:?}"))?;
        if encoded.cached_logits(&[], None).err() != Some(Error::Shape)
            || encoded.cached_logits(&[65000], Some(&[false])).err() != Some(Error::Shape)
        {
            return Err("invalid cache prefix accepted".into());
        }
        let full = encoded
            .logits(target, target_keep, false)
            .map_err(|e| format!("{e:?}"))?;
        let cached = encoded
            .cached_logits(target, target_keep)
            .map_err(|e| format!("{e:?}"))?;
        for (&a, &b) in full.data().iter().zip(cached.data()) {
            if (a - b).abs() > 1e-4 + 1e-4 * a.abs() {
                return Err("cache and full logits differ".into());
            }
        }
        for tensor in [&full, &cached] {
            for &value in tensor.data() {
                out.write_all(&value.to_le_bytes())?;
            }
        }
        for width in [1, 4] {
            let mut generation = MarianGeneration::from_profile(&config.generation);
            generation.num_beams = width;
            generation.max_length = 5;
            let search = generation
                .search_config(config.vocab)
                .map_err(|e| format!("{e:?}"))?;
            for (mode, result) in [
                ("full", encoded.generate(search)),
                ("cached", encoded.generate_cached(search)),
            ] {
                for (rank, h) in result.map_err(|e| format!("{e:?}"))?.iter().enumerate() {
                    let tokens = h
                        .tokens
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",");
                    writeln!(
                        out,
                        "{width}|{mode}|{rank}|{tokens}|{:.17}|{:.17}|{}",
                        h.log_probability,
                        h.score,
                        u8::from(h.ended)
                    )?;
                }
            }
        }
        writeln!(out, "END")?;
    }
    out.flush()?;
    Ok(())
}
