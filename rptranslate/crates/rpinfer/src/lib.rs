//! Scalar numeric primitives and dense FP32 tensor core.
pub mod attention;
mod cache;
pub mod decoder;
pub mod encoder;
pub mod layers;
pub mod marian_decoder;
pub mod marian_encoder;
pub mod marian_generation;
pub mod marian_model;
pub mod marian_weights;
pub mod matmul;
pub mod mixed_inference;
pub mod mixed_model;
pub mod quantization;
pub mod quantized_linear;
pub mod quantized_table;
pub mod search;
pub mod seq2seq;
pub mod tensor;
pub mod toy_training;
pub mod transformer;
pub mod vocabulary;

#[derive(Debug, PartialEq, Eq)]
pub enum InferError {
    ShapeMismatch,
    NonFinite,
}

pub fn dot(a: &[f32], b: &[f32]) -> Result<f32, InferError> {
    if a.len() != b.len() || a.is_empty() {
        return Err(InferError::ShapeMismatch);
    }
    let mut sum = 0.0_f32;
    for (&x, &y) in a.iter().zip(b) {
        if !x.is_finite() || !y.is_finite() {
            return Err(InferError::NonFinite);
        }
        sum += x * y;
        if !sum.is_finite() {
            return Err(InferError::NonFinite);
        }
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_dot() {
        assert_eq!(dot(&[1.0, 2.0, -3.0], &[4.0, 5.0, 2.0]), Ok(8.0));
    }

    #[test]
    fn rejects_bad_inputs() {
        assert_eq!(dot(&[], &[]), Err(InferError::ShapeMismatch));
        assert_eq!(dot(&[1.0], &[1.0, 2.0]), Err(InferError::ShapeMismatch));
        assert_eq!(dot(&[f32::NAN], &[1.0]), Err(InferError::NonFinite));
        assert_eq!(dot(&[f32::MAX], &[2.0]), Err(InferError::NonFinite));
    }
}
