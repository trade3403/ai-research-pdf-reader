//! Test-only bridge for a tiny independently generated Marian checkpoint.
use rpinfer::{
    attention::softmax,
    decoder::{Decoder, DecoderLayer},
    encoder::{Encoder, EncoderLayer, Norm},
    layers::{Linear, MultiHead},
    matmul::Kernel,
    search::MarianGeneration,
    seq2seq::{OutputHead, Seq2Seq},
    tensor::Tensor,
    transformer::{input_embeddings, Activation, FeedForward},
    vocabulary::ChunkedTable,
};
use std::{collections::BTreeMap, io::Read};

type Weights = BTreeMap<String, Tensor>;

fn load(path: &str) -> Weights {
    let mut text = String::new();
    std::fs::File::open(path)
        .unwrap()
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.len() <= 1024 * 1024);
    let mut result = BTreeMap::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 3);
        let shape: Vec<usize> = fields[1].split(',').map(|s| s.parse().unwrap()).collect();
        let data = fields[2].split(',').map(|s| s.parse().unwrap()).collect();
        let tensor = Tensor::new(&shape, data).unwrap();
        assert!(result.insert(fields[0].to_owned(), tensor).is_none());
    }
    result
}

fn linear<'a>(w: &'a Weights, name: &str) -> Linear<'a> {
    Linear {
        weight: &w[&format!("{name}.weight")],
        bias: Some(&w[&format!("{name}.bias")]),
    }
}

fn norm<'a>(w: &'a Weights, name: &str) -> Norm<'a> {
    Norm {
        weight: &w[&format!("{name}.weight")],
        bias: &w[&format!("{name}.bias")],
        epsilon: 1e-5,
    }
}

fn attention<'a>(w: &'a Weights, name: &str) -> MultiHead<'a> {
    MultiHead {
        q: linear(w, &format!("{name}.q_proj")),
        k: linear(w, &format!("{name}.k_proj")),
        v: linear(w, &format!("{name}.v_proj")),
        output: linear(w, &format!("{name}.out_proj")),
        heads: 2,
    }
}

fn ffn<'a>(w: &'a Weights, name: &str) -> FeedForward<'a> {
    FeedForward {
        first: linear(w, &format!("{name}.fc1")),
        second: linear(w, &format!("{name}.fc2")),
        activation: Activation::Relu,
    }
}

fn emit(name: &str, value: &Tensor) {
    let shape = value
        .shape()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>();
    let data = value.data().iter().map(f32::to_string).collect::<Vec<_>>();
    println!("{name}|{}|{}", shape.join(","), data.join(","));
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!((4..=5).contains(&args.len()));
    let w = load(&args[1]);
    let case: usize = args[2].parse().unwrap();
    let kernel = match args[3].as_str() {
        "scalar" => Kernel::Scalar,
        "blocked" => Kernel::Blocked,
        "auto" => Kernel::Auto,
        "parallel" => Kernel::Parallel(4),
        _ => panic!("unknown kernel"),
    };
    let (source, sm, target, tm): (&[u32], &[bool], &[u32], &[bool]) = match case {
        0 => (
            &[2, 5, 16],
            &[true, true, false],
            &[16, 4, 7],
            &[true, false, true],
        ),
        1 => (&[3], &[true], &[16], &[true]),
        2 => (&[2, 8, 9, 0], &[true; 4], &[16, 6], &[true; 2]),
        _ => panic!("unknown fixture"),
    };
    let enc: Vec<_> = (0..2)
        .map(|i| {
            let n = format!("model.encoder.layers.{i}");
            EncoderLayer {
                attention: attention(&w, &format!("{n}.self_attn")),
                attention_norm: norm(&w, &format!("{n}.self_attn_layer_norm")),
                ffn: ffn(&w, &n),
                final_norm: norm(&w, &format!("{n}.final_layer_norm")),
            }
        })
        .collect();
    let dec: Vec<_> = (0..2)
        .map(|i| {
            let n = format!("model.decoder.layers.{i}");
            DecoderLayer {
                self_attention: attention(&w, &format!("{n}.self_attn")),
                self_norm: norm(&w, &format!("{n}.self_attn_layer_norm")),
                cross_attention: attention(&w, &format!("{n}.encoder_attn")),
                cross_norm: norm(&w, &format!("{n}.encoder_attn_layer_norm")),
                ffn: ffn(&w, &n),
                final_norm: norm(&w, &format!("{n}.final_layer_norm")),
            }
        })
        .collect();
    let dense = &w["model.shared.weight"];
    let table = ChunkedTable::from_rows(17, 8, 5, 17 * 8 * 4, |start, count| {
        Ok(dense.data()[start * 8..(start + count) * 8].to_vec())
    }).unwrap();
    let table = &table;
    let scale = 8_f32.sqrt();
    let model = Seq2Seq {
        encoder: Encoder {
            embeddings: table,
            embedding_scale: scale,
            max_positions: 16,
            layers: &enc,
        },
        decoder: Decoder {
            embeddings: table,
            embedding_scale: scale,
            max_positions: 16,
            layers: &dec,
        },
        head: OutputHead {
            weight: None,
            bias: Some(&w["final_logits_bias"]),
        },
    };
    let encoded = model.encode(source, Some(sm), kernel).unwrap();
    if args.get(4).is_some_and(|arg| arg == "generate") {
        for setting in 0..2 {
            for width in [1, 2, 4] {
                let profile = MarianGeneration {
                    start: 16,
                    eos: 0,
                    pad: 16,
                    num_beams: width,
                    min_length: 2,
                    max_length: 5 + setting,
                    length_penalty: setting as f64,
                    forced_eos: (setting == 0).then_some(0),
                    suppress: &[16],
                };
                let c = profile.search_config(17).unwrap();
                for (mode, results) in [
                    ("full", encoded.generate(c).unwrap()),
                    ("cached", encoded.generate_cached(c).unwrap()),
                ] {
                    for (rank, h) in results.iter().enumerate() {
                        let tokens = h.tokens.iter().map(u32::to_string).collect::<Vec<_>>();
                        println!(
                            "{setting}|{width}|{mode}|{rank}|{}|{:.17}|{:.17}|{}",
                            tokens.join(","),
                            h.log_probability,
                            h.score,
                            u8::from(h.ended)
                        );
                    }
                }
            }
        }
        return;
    }
    let mut state = input_embeddings(table, source, scale, 0, 16).unwrap();
    emit("enc.embedding", &state);
    for (i, layer) in enc.iter().enumerate() {
        let a = layer.attention_stage(&state, Some(sm), kernel).unwrap();
        emit(&format!("enc.{i}.self_norm"), &a);
        let ff = layer.ffn.forward(&a, kernel).unwrap();
        emit(&format!("enc.{i}.ffn"), &ff);
        state = layer.final_norm.residual(&a, &ff).unwrap();
        emit(&format!("enc.{i}.output"), &state);
    }
    assert_eq!(&state, encoded.memory());
    let mut state = input_embeddings(table, target, scale, 0, 16).unwrap();
    emit("dec.embedding", &state);
    for (i, layer) in dec.iter().enumerate() {
        let a = layer.self_stage(&state, Some(tm), kernel).unwrap();
        emit(&format!("dec.{i}.self_norm"), &a);
        let c = layer
            .cross_stage(&a, encoded.memory(), Some(sm), kernel)
            .unwrap();
        emit(&format!("dec.{i}.cross_norm"), &c);
        let ff = layer.ffn.forward(&c, kernel).unwrap();
        emit(&format!("dec.{i}.ffn"), &ff);
        state = layer.final_norm.residual(&c, &ff).unwrap();
        emit(&format!("dec.{i}.output"), &state);
    }
    assert_eq!(state, encoded.hidden(target, Some(tm)).unwrap());
    let logits = encoded.logits(target, Some(tm)).unwrap();
    let next = encoded.next_logits(target, Some(tm)).unwrap();
    assert_eq!(next.data(), &logits.data()[logits.data().len() - 17..]);
    emit(
        "cached_logits",
        &encoded.cached_logits(target, Some(tm)).unwrap(),
    );
    emit("logits", &logits);
    emit("next_logits", &next);
    emit("probabilities", &softmax(&next).unwrap());
}
