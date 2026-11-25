//! End-to-end fixed-profile English to Simplified Chinese translation.
use rpinfer::{
    marian_model::{GenerationOptions, MarianModel, ModelError, MODEL_PAYLOAD_BYTES},
    matmul::Kernel,
    tensor::Error as InferError,
};
use rpmodel::{
    marian::{Error as VocabularyError, Vocabulary},
    package,
    spm::{Error as SpmError, Model, NmtUnigram},
};
use std::path::Path;

pub const LANGUAGE: &str = ">>cmn_Hans<<";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Budget,
    Package(package::Error),
    Model(ModelError),
    Vocabulary(VocabularyError),
    SentencePiece(SpmError),
    Profile,
    Empty,
    LanguagePrefix,
    SourceTooLong,
    Inference(InferError),
    NoTranslation,
}

#[derive(Debug, PartialEq)]
pub struct Translation {
    pub text: String,
    pub source_tokens: Vec<u32>,
    /// Includes the decoder start token and any terminal EOS.
    pub target_tokens: Vec<u32>,
    pub score: f64,
    /// True if EOS was emitted at the configured cap (possibly forced).
    pub reached_limit: bool,
}

pub struct OfflineTranslator {
    model: MarianModel,
    vocabulary: Vocabulary,
    source_spm: Vec<u8>,
}

impl OfflineTranslator {
    pub fn load(root: &Path, max_weight_bytes: usize) -> Result<Self, Error> {
        if max_weight_bytes < MODEL_PAYLOAD_BYTES {
            return Err(Error::Budget);
        }
        let source_spm = package::tokenizer_asset(root, "source.spm").map_err(Error::Package)?;
        let vocab = package::tokenizer_asset(root, "vocab.json").map_err(Error::Package)?;
        let vocabulary = Vocabulary::parse(std::str::from_utf8(&vocab).map_err(|_| Error::Profile)?)
            .map_err(Error::Vocabulary)?;
        let model = MarianModel::load(root, max_weight_bytes).map_err(Error::Model)?;
        let config = model.config();
        if vocabulary.len() != config.vocab
            || vocabulary.unk != 1
            || vocabulary.eos != config.generation.eos
            || vocabulary.pad != config.generation.pad
            || !vocabulary.language_codes().any(|code| code == LANGUAGE)
            || (0..config.vocab).any(|id| vocabulary.piece(id as u32).is_err())
        {
            return Err(Error::Profile);
        }
        let translator = Self {
            model,
            vocabulary,
            source_spm,
        };
        // Fail during loading rather than on the first request.
        translator.session()?;
        Ok(translator)
    }

    pub fn default_options(&self) -> GenerationOptions {
        self.model.default_options()
    }

    /// Borrow verified source SPM bytes; build the tokenizer once per batch.
    pub fn session(&self) -> Result<TextSession<'_>, Error> {
        let tokenizer = Model::parse(&self.source_spm)
            .map_err(Error::SentencePiece)?
            .nmt_unigram()
            .map_err(Error::SentencePiece)?;
        Ok(TextSession {
            model: &self.model,
            vocabulary: &self.vocabulary,
            tokenizer,
        })
    }
}

pub struct TextSession<'a> {
    model: &'a MarianModel,
    vocabulary: &'a Vocabulary,
    tokenizer: NmtUnigram<'a>,
}

impl TextSession<'_> {
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, Error> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::Empty);
        }
        if text.starts_with(">>") {
            return Err(Error::LanguagePrefix);
        }
        if text.len() > 64 * 1024 - LANGUAGE.len() - 1 {
            return Err(Error::SourceTooLong);
        }
        let input = format!("{LANGUAGE} {text}");
        let tokens = self
            .vocabulary
            .encode(&self.tokenizer, &input)
            .map_err(Error::Vocabulary)?;
        if tokens.len() > self.model.config().max_positions {
            return Err(Error::SourceTooLong);
        }
        Ok(tokens)
    }

    pub fn translate(
        &self,
        text: &str,
        options: GenerationOptions,
        kernel: Kernel,
    ) -> Result<Translation, Error> {
        let source_tokens = self.encode(text)?;
        let mut candidates = self
            .model
            .generate_tokens(&source_tokens, None, options, kernel)
            .map_err(Error::Inference)?
            .into_iter();
        let best = candidates.next().ok_or(Error::NoTranslation)?;
        if !best.ended {
            return Err(Error::NoTranslation);
        }
        let text = self
            .vocabulary
            .decode_skipping_special(&best.tokens)
            .map_err(Error::Vocabulary)?;
        if text.is_empty() {
            return Err(Error::NoTranslation);
        }
        Ok(Translation {
            text,
            source_tokens,
            reached_limit: best.tokens.len() - 1 == options.max_new,
            target_tokens: best.tokens,
            score: best.score,
        })
    }
}
