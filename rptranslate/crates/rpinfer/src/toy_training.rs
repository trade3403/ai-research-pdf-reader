//! D1.12 bounded synthetic training fixture, not Marian training.
use rpmodel::sha256::digest_slice;

const FEATURES: usize = 3;
const CLASSES: usize = 3;
const WEIGHTS: usize = FEATURES * CLASSES;
const MAX_STEPS: u64 = 10_000;
const MAGIC: &[u8; 8] = b"RPTNY001";
const BODY: usize = 8 + 24 + WEIGHTS * 8;
const SIZE: usize = BODY + 64;
const DATA: [([f64; FEATURES], usize); 6] = [
    ([1.0, 0.0, 1.0], 0),
    ([2.0, 0.0, 1.0], 0),
    ([0.0, 1.0, 1.0], 1),
    ([0.0, 2.0, 1.0], 1),
    ([-1.0, -1.0, 1.0], 2),
    ([-2.0, -2.0, 1.0], 2),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Rate,
    Limit,
    NonFinite,
    Checkpoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trainer {
    seed: u64,
    steps: u64,
    rate: f64,
    weights: [f64; WEIGHTS],
}

fn valid_rate(rate: f64) -> bool {
    rate.is_finite() && rate > 0.0 && rate <= 1.0
}

fn evaluate(weights: &[f64; WEIGHTS]) -> Result<(f64, [f64; WEIGHTS]), Error> {
    let mut loss = 0.0;
    let mut gradient = [0.0; WEIGHTS];
    for (x, target) in DATA {
        let mut logits = [0.0; CLASSES];
        for (row, value) in weights.chunks_exact(FEATURES).zip(&mut logits) {
            *value = row.iter().zip(x).map(|(w, v)| w * v).sum();
        }
        let max = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let exp = logits.map(|v| (v - max).exp());
        let sum: f64 = exp.iter().sum();
        loss += max - logits[target] + sum.ln();
        for (class, row) in gradient.chunks_exact_mut(FEATURES).enumerate() {
            let error = exp[class] / sum - if class == target { 1.0 } else { 0.0 };
            for (g, v) in row.iter_mut().zip(x) {
                *g += error * v;
            }
        }
    }
    let count = DATA.len() as f64;
    loss /= count;
    for g in &mut gradient {
        *g /= count;
    }
    if !loss.is_finite() || gradient.iter().any(|g| !g.is_finite()) {
        return Err(Error::NonFinite);
    }
    Ok((loss, gradient))
}

impl Trainer {
    pub fn new(seed: u64, rate: f64) -> Result<Self, Error> {
        if !valid_rate(rate) {
            return Err(Error::Rate);
        }
        let mut state = seed;
        let weights = std::array::from_fn(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 32) as f64 / 4294967296.0 - 0.5) * 0.2
        });
        Ok(Self {
            seed,
            steps: 0,
            rate,
            weights,
        })
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    pub fn weights(&self) -> &[f64; WEIGHTS] {
        &self.weights
    }

    pub fn loss(&self) -> Result<f64, Error> {
        evaluate(&self.weights).map(|(loss, _)| loss)
    }

    /// Full-batch SGD; no state is changed on a rejected update.
    pub fn step(&mut self) -> Result<f64, Error> {
        if self.steps >= MAX_STEPS {
            return Err(Error::Limit);
        }
        let (_, gradient) = evaluate(&self.weights)?;
        let mut next = self.weights;
        for (w, g) in next.iter_mut().zip(gradient) {
            *w -= self.rate * g;
        }
        let (loss, _) = evaluate(&next)?;
        self.weights = next;
        self.steps += 1;
        Ok(loss)
    }

    /// Versioned little-endian fixture checkpoint with SHA256 corruption check.
    /// No external file is opened, and no unbounded allocation is performed.
    pub fn checkpoint(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::with_capacity(SIZE);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.steps.to_le_bytes());
        bytes.extend_from_slice(&self.rate.to_bits().to_le_bytes());
        for w in self.weights {
            bytes.extend_from_slice(&w.to_bits().to_le_bytes());
        }
        let (_, digest) = digest_slice(&bytes, BODY as u64).map_err(|_| Error::Checkpoint)?;
        bytes.extend_from_slice(digest.as_bytes());
        Ok(bytes)
    }

    pub fn restore(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SIZE || &bytes[..8] != MAGIC {
            return Err(Error::Checkpoint);
        }
        let (_, digest) =
            digest_slice(&bytes[..BODY], BODY as u64).map_err(|_| Error::Checkpoint)?;
        if digest.as_bytes() != &bytes[BODY..] {
            return Err(Error::Checkpoint);
        }
        let read = |at: usize| {
            let mut raw = [0; 8];
            raw.copy_from_slice(&bytes[at..at + 8]);
            u64::from_le_bytes(raw)
        };
        let seed = read(8);
        let steps = read(16);
        let rate = f64::from_bits(read(24));
        let weights = std::array::from_fn(|i| f64::from_bits(read(32 + i * 8)));
        if steps > MAX_STEPS || !valid_rate(rate) {
            return Err(Error::Checkpoint);
        }
        evaluate(&weights)?;
        Ok(Self {
            seed,
            steps,
            rate,
            weights,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_gradient_matches_finite_difference() {
        let trainer = Trainer::new(42, 0.2).unwrap();
        let (_, gradient) = evaluate(&trainer.weights).unwrap();
        for (i, expected) in gradient.iter().enumerate() {
            let mut plus = trainer.weights;
            let mut minus = trainer.weights;
            plus[i] += 1e-5;
            minus[i] -= 1e-5;
            let actual = (evaluate(&plus).unwrap().0 - evaluate(&minus).unwrap().0) / 2e-5;
            assert!((actual - expected).abs() < 1e-8);
        }
    }

    #[test]
    fn deterministic_training_and_resume() {
        let mut full = Trainer::new(42, 0.2).unwrap();
        let mut split = full.clone();
        let initial = full.loss().unwrap();
        for _ in 0..100 {
            full.step().unwrap();
        }
        for _ in 0..37 {
            split.step().unwrap();
        }
        let mut resumed = Trainer::restore(&split.checkpoint().unwrap()).unwrap();
        for _ in 37..100 {
            resumed.step().unwrap();
        }
        assert_eq!(resumed.checkpoint(), full.checkpoint());
        assert!(full.loss().unwrap() < initial * 0.1);
        assert_ne!(Trainer::new(43, 0.2).unwrap(), Trainer::new(42, 0.2).unwrap());
    }

    #[test]
    fn invalid_checkpoint_and_budget_fail_closed() {
        assert_eq!(Trainer::new(0, f64::NAN), Err(Error::Rate));
        assert_eq!(Trainer::new(0, 0.0), Err(Error::Rate));
        let mut trainer = Trainer::new(1, 0.2).unwrap();
        let mut bytes = trainer.checkpoint().unwrap();
        assert_eq!(Trainer::restore(&bytes[..SIZE - 1]), Err(Error::Checkpoint));
        bytes[40] ^= 1;
        assert_eq!(Trainer::restore(&bytes), Err(Error::Checkpoint));
        trainer.steps = MAX_STEPS;
        let old = trainer.clone();
        assert_eq!(trainer.step(), Err(Error::Limit));
        assert_eq!(trainer, old);
    }
}
