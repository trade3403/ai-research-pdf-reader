//! Model metadata and bounded local weight-file reader.
pub mod config;
pub mod marian;
pub mod normalization;
pub mod package;
pub mod safetensors;
pub mod sha256;
pub mod spm;
pub mod tokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: String,
    pub parameters: u64,
    pub bits_per_weight: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModelError {
    InvalidSpec,
    Overflow,
}

impl ModelSpec {
    /// Ideal packed weight bytes, excluding scales, headers and runtime memory.
    pub fn estimated_weight_bytes(&self) -> Result<u64, ModelError> {
        if self.id.trim().is_empty()
            || self.parameters == 0
            || !matches!(self.bits_per_weight, 4 | 8 | 16 | 32)
        {
            return Err(ModelError::InvalidSpec);
        }
        let bits = self
            .parameters
            .checked_mul(u64::from(self.bits_per_weight))
            .ok_or(ModelError::Overflow)?;
        Ok(bits / 8 + u64::from(bits % 8 != 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_budget() {
        let mut spec = ModelSpec {
            id: "rpmodel-target".into(),
            parameters: 100_000_000,
            bits_per_weight: 8,
        };
        assert_eq!(spec.estimated_weight_bytes(), Ok(100_000_000));
        spec.parameters = 3;
        spec.bits_per_weight = 4;
        assert_eq!(spec.estimated_weight_bytes(), Ok(2));
        spec.bits_per_weight = 3;
        assert_eq!(spec.estimated_weight_bytes(), Err(ModelError::InvalidSpec));
        spec.bits_per_weight = 32;
        spec.parameters = u64::MAX;
        assert_eq!(spec.estimated_weight_bytes(), Err(ModelError::Overflow));
    }
}
