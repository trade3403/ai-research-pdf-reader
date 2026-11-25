//! Bounded typed config parser for the fixed OPUS-MT Marian v1 profile.
use crate::safetensors::{Error as JsonError, Parser};
use std::collections::BTreeMap;

const MAX_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 8;
const MAX_ITEMS: usize = 128;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Limit,
    Field(&'static str),
    Unsupported,
}

#[derive(Debug, PartialEq)]
enum Value {
    Text(String),
    Number(String),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

fn digits(p: &mut Parser<'_>) -> usize {
    let len = p.rest.bytes().take_while(u8::is_ascii_digit).count();
    p.rest = &p.rest[len..];
    len
}

fn number(p: &mut Parser<'_>) -> Result<Value, JsonError> {
    let start = p.rest;
    p.rest = p.rest.strip_prefix('-').unwrap_or(p.rest);
    if let Some(rest) = p.rest.strip_prefix('0') {
        p.rest = rest;
    } else if digits(p) == 0 {
        return Err(JsonError::Header);
    }
    if let Some(rest) = p.rest.strip_prefix('.') {
        p.rest = rest;
        if digits(p) == 0 {
            return Err(JsonError::Header);
        }
    }
    if let Some(rest) = p.rest.strip_prefix(['e', 'E']) {
        p.rest = rest.strip_prefix(['+', '-']).unwrap_or(rest);
        if digits(p) == 0 {
            return Err(JsonError::Header);
        }
    }
    let token = &start[..start.len() - p.rest.len()];
    if token.len() > 64 || !token.parse::<f64>().is_ok_and(f64::is_finite) {
        return Err(JsonError::Limit);
    }
    Ok(Value::Number(token.into()))
}

fn value(p: &mut Parser<'_>, depth: usize) -> Result<Value, JsonError> {
    if depth > MAX_DEPTH {
        return Err(JsonError::Limit);
    }
    p.ws();
    match p.rest.as_bytes().first() {
        Some(b'"') => Ok(Value::Text(p.string()?)),
        Some(b'{') => Ok(Value::Object(p.object(MAX_ITEMS, |p, _| value(p, depth + 1))?)),
        Some(b'[') => {
            p.expect('[')?;
            let mut out = Vec::new();
            if !p.take(']') {
                loop {
                    if out.len() == MAX_ITEMS {
                        return Err(JsonError::Limit);
                    }
                    out.push(value(p, depth + 1)?);
                    if p.take(']') {
                        break;
                    }
                    p.expect(',')?;
                }
            }
            Ok(Value::Array(out))
        }
        Some(b't') if p.rest.starts_with("true") => {
            p.rest = &p.rest[4..];
            Ok(Value::Bool(true))
        }
        Some(b'f') if p.rest.starts_with("false") => {
            p.rest = &p.rest[5..];
            Ok(Value::Bool(false))
        }
        Some(b'n') if p.rest.starts_with("null") => {
            p.rest = &p.rest[4..];
            Ok(Value::Null)
        }
        Some(b'-' | b'0'..=b'9') => number(p),
        _ => Err(JsonError::Header),
    }
}

type Object = BTreeMap<String, Value>;

fn object(text: &str) -> Result<Object, Error> {
    if text.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    let mut p = Parser { rest: text };
    let Value::Object(out) = value(&mut p, 0).map_err(|_| Error::Json)? else {
        return Err(Error::Json);
    };
    p.ws();
    if !p.rest.is_empty() {
        return Err(Error::Json);
    }
    Ok(out)
}

fn uint(o: &Object, key: &'static str) -> Result<usize, Error> {
    match o.get(key) {
        Some(Value::Number(n)) => n.parse().map_err(|_| Error::Field(key)),
        _ => Err(Error::Field(key)),
    }
}

fn flag(o: &Object, key: &'static str, expected: bool) -> Result<(), Error> {
    if o.get(key) != Some(&Value::Bool(expected)) {
        return Err(Error::Field(key));
    }
    Ok(())
}

fn text(o: &Object, key: &'static str, expected: &str) -> Result<(), Error> {
    if !matches!(o.get(key), Some(Value::Text(s)) if s == expected) {
        return Err(Error::Field(key));
    }
    Ok(())
}

fn bad_words(o: &Object, pad: usize) -> Result<(), Error> {
    let expected = Value::Array(vec![Value::Array(vec![Value::Number(pad.to_string())])]);
    if o.get("bad_words_ids") != Some(&expected) {
        return Err(Error::Field("bad_words_ids"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Swish,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Generation {
    pub start: u32,
    pub eos: u32,
    pub pad: u32,
    pub forced_eos: u32,
    pub num_beams: usize,
    pub min_length: usize,
    pub max_length: usize,
    pub length_penalty: f64,
    pub suppress: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarianConfig {
    pub vocab: usize,
    pub hidden: usize,
    pub encoder_layers: usize,
    pub decoder_layers: usize,
    pub encoder_heads: usize,
    pub decoder_heads: usize,
    pub encoder_ffn: usize,
    pub decoder_ffn: usize,
    pub max_positions: usize,
    pub activation: Activation,
    pub embedding_scale: f32,
    pub generation: Generation,
}

impl MarianConfig {
    /// Fixed architecture only. Package::open_profile additionally authenticates
    /// the exact pinned config bytes against the audited SHA256 constants.
    pub fn parse(model: &str, generation: &str) -> Result<Self, Error> {
        let m = object(model)?;
        let g = object(generation)?;
        text(&m, "model_type", "marian")?;
        text(&m, "activation_function", "swish")?;
        for (key, expected) in [
            ("is_encoder_decoder", true), ("scale_embedding", true),
            ("static_position_embeddings", true), ("normalize_before", false),
            ("normalize_embedding", false), ("add_final_layer_norm", false),
            ("add_bias_logits", false), ("force_bos_token_to_be_generated", false),
            ("do_blenderbot_90_layernorm", false),
        ] {
            flag(&m, key, expected)?;
        }
        let vocab = uint(&m, "vocab_size")?;
        let hidden = uint(&m, "d_model")?;
        let encoder_layers = uint(&m, "encoder_layers")?;
        let decoder_layers = uint(&m, "decoder_layers")?;
        let encoder_heads = uint(&m, "encoder_attention_heads")?;
        let decoder_heads = uint(&m, "decoder_attention_heads")?;
        let encoder_ffn = uint(&m, "encoder_ffn_dim")?;
        let decoder_ffn = uint(&m, "decoder_ffn_dim")?;
        let max_positions = uint(&m, "max_position_embeddings")?;
        if (vocab, hidden, encoder_layers, decoder_layers) != (65001, 512, 6, 6)
            || (encoder_heads, decoder_heads, encoder_ffn, decoder_ffn) != (8, 8, 2048, 2048)
            || max_positions != 512 || uint(&m, "extra_pos_embeddings")? != 0
            || uint(&m, "num_hidden_layers")? != encoder_layers
        {
            return Err(Error::Unsupported);
        }
        // The exact upstream generation file contains only this subset.
        // Unknown options are rejected, not silently interpreted as defaults.
        for key in g.keys() {
            if !matches!(key.as_str(), "_from_model_config" | "transformers_version"
                | "bad_words_ids" | "bos_token_id" | "decoder_start_token_id"
                | "eos_token_id" | "forced_eos_token_id" | "pad_token_id"
                | "num_beams" | "max_length" | "min_length" | "length_penalty"
                | "early_stopping" | "do_sample" | "renormalize_logits")
            {
                return Err(Error::Unsupported);
            }
        }
        for key in ["early_stopping", "do_sample", "renormalize_logits"] {
            if g.contains_key(key) {
                flag(&g, key, false)?;
            }
        }
        for (key, expected) in [
            ("decoder_start_token_id", 65000), ("pad_token_id", 65000),
            ("eos_token_id", 0), ("bos_token_id", 0), ("forced_eos_token_id", 0),
            ("num_beams", 4), ("max_length", 512),
        ] {
            if uint(&m, key)? != expected || uint(&g, key)? != expected {
                return Err(Error::Field(key));
            }
        }
        let min_length = if g.contains_key("min_length") { uint(&g, "min_length")? } else { 0 };
        let length_penalty = match g.get("length_penalty") {
            None => 1.,
            Some(Value::Number(s)) => s.parse::<f64>().map_err(|_| Error::Field("length_penalty"))?,
            _ => return Err(Error::Field("length_penalty")),
        };
        if min_length != 0 || length_penalty != 1. {
            return Err(Error::Unsupported);
        }
        let pad = uint(&g, "pad_token_id")?;
        bad_words(&m, pad)?;
        bad_words(&g, pad)?;
        Ok(Self {
            vocab, hidden, encoder_layers, decoder_layers, encoder_heads, decoder_heads,
            encoder_ffn, decoder_ffn, max_positions, activation: Activation::Swish,
            embedding_scale: (hidden as f32).sqrt(),
            generation: Generation {
                start: uint(&g, "decoder_start_token_id")? as u32,
                eos: uint(&g, "eos_token_id")? as u32,
                pad: pad as u32,
                forced_eos: uint(&g, "forced_eos_token_id")? as u32,
                num_beams: uint(&g, "num_beams")?,
                min_length, max_length: uint(&g, "max_length")?,
                length_penalty, suppress: vec![pad as u32],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = r#"{"model_type":"marian","activation_function":"swish",
        "is_encoder_decoder":true,"scale_embedding":true,"static_position_embeddings":true,
        "normalize_before":false,"normalize_embedding":false,"add_final_layer_norm":false,
        "add_bias_logits":false,"force_bos_token_to_be_generated":false,"do_blenderbot_90_layernorm":false,
        "vocab_size":65001,"d_model":512,"encoder_layers":6,"decoder_layers":6,
        "encoder_attention_heads":8,"decoder_attention_heads":8,"encoder_ffn_dim":2048,
        "decoder_ffn_dim":2048,"max_position_embeddings":512,"extra_pos_embeddings":0,
        "num_hidden_layers":6,"decoder_start_token_id":65000,"pad_token_id":65000,
        "eos_token_id":0,"bos_token_id":0,"forced_eos_token_id":0,"num_beams":4,
        "max_length":512,"bad_words_ids":[[65000]],"dropout":0.1,"id2label":{"0":"LABEL_0"}}"#;
    const GEN: &str = r#"{"bad_words_ids":[[65000]],"bos_token_id":0,"decoder_start_token_id":65000,
        "eos_token_id":0,"forced_eos_token_id":0,"max_length":512,"num_beams":4,"pad_token_id":65000}"#;

    #[test]
    fn fixed_profile_and_defaults() {
        let c = MarianConfig::parse(MODEL, GEN).unwrap();
        assert_eq!((c.vocab, c.hidden, c.encoder_layers, c.decoder_heads), (65001, 512, 6, 8));
        assert_eq!(c.generation.suppress, [65000]);
        assert_eq!(c.generation.max_length, 512);
        assert_eq!(c.generation.length_penalty, 1.);
        assert_eq!(c.embedding_scale, 512_f32.sqrt());
    }

    #[test]
    fn reject_wrong_architecture_and_generation() {
        for bad in [
            MODEL.replace(r#""d_model":512"#, r#""d_model":511"#),
            MODEL.replace(r#""normalize_before":false"#, r#""normalize_before":true"#),
            MODEL.replace(r#""d_model":512"#, r#""d_model":512,"d_model":512"#),
            MODEL.replace("0.1", "1e999"),
            MODEL.replace("0.1", "01"),
            MODEL.replace("0.1", "0."),
        ] {
            assert!(MarianConfig::parse(&bad, GEN).is_err(), "{bad}");
        }
        for bad in [
            GEN.replace(r#""num_beams":4"#, r#""num_beams":5"#),
            GEN.replace(r#""num_beams":4"#, r#""num_beams":4,"do_sample":true"#),
            GEN.replace(r#""num_beams":4"#, r#""num_beams":4,"renormalize_logits":true"#),
            GEN.replace(r#""num_beams":4"#, r#""num_beams":4,"unknown":false"#),
            GEN.replace("[[65000]]", "[[1,2]]"),
        ] {
            assert!(MarianConfig::parse(MODEL, &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn json_limits() {
        assert!(object(&" ".repeat(MAX_BYTES + 1)).is_err());
        assert!(object(r#"{"a":[[[[[[[[[[0]]]]]]]]]]}"#).is_err());
        assert!(object(r#"{"a":-1.2e+3,"b":[true,false,null]}"#).is_ok());
        assert!(object(r#"{"a":1e}"#).is_err());
        assert!(object(r#"{"a":1} false"#).is_err());
    }
}
