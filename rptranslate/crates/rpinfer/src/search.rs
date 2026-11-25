//! Deterministic bounded full-prefix search; independent of model storage.
use crate::tensor::{Error, Tensor};

const MAX_BEAMS: usize = 16;
const MAX_NEW: usize = 512;
const MAX_VOCAB: usize = 200_000;

#[derive(Clone, Copy)]
pub struct SearchConfig<'a> {
    pub start: u32,
    pub eos: u32,
    pub beam_width: usize,
    pub min_new: usize,
    pub max_new: usize,
    pub length_penalty: f64,
    pub suppress: &'a [u32],
    pub force_eos_at_limit: bool,
}

#[derive(Debug, PartialEq)]
pub struct Hypothesis {
    /// Includes start token; generated length excludes it and includes EOS.
    pub tokens: Vec<u32>,
    pub log_probability: f64,
    pub score: f64,
    pub ended: bool,
}

pub(crate) fn validate(c: SearchConfig<'_>, vocab: usize) -> Result<(), Error> {
    if vocab == 0
        || vocab > MAX_VOCAB
        || c.beam_width == 0
        || c.beam_width > MAX_BEAMS
        || c.max_new == 0
        || c.max_new > MAX_NEW
        || c.suppress.len() > vocab
    {
        return Err(Error::Limit);
    }
    if c.start as usize >= vocab
        || c.eos as usize >= vocab
        || c.min_new > c.max_new
        || c.suppress.contains(&c.eos)
        || c.suppress.iter().any(|&id| id as usize >= vocab)
    {
        return Err(Error::Shape);
    }
    if !c.length_penalty.is_finite() {
        return Err(Error::NonFinite);
    }
    if !(0.0..=2.0).contains(&c.length_penalty) {
        return Err(Error::Shape);
    }
    Ok(())
}

fn rank(a: &Hypothesis, b: &Hypothesis) -> std::cmp::Ordering {
    b.score
        .total_cmp(&a.score)
        .then_with(|| a.tokens.cmp(&b.tokens))
}

/// Beam width 1 is greedy. At each depth retain the best B hypotheses,
/// including completed paths without expanding them. Ranking is normalized
/// cumulative log probability; ties choose lexicographically lower IDs.
/// This is an explicit bounded policy, not HF BeamSearchScorer parity.
pub fn search(
    vocab: usize,
    config: SearchConfig<'_>,
    mut logits: impl FnMut(&[u32]) -> Result<Tensor, Error>,
) -> Result<Vec<Hypothesis>, Error> {
    validate(config, vocab)?;
    let mut blocked = vec![false; vocab];
    for &id in config.suppress {
        blocked[id as usize] = true;
    }
    let mut beams = vec![Hypothesis {
        tokens: vec![config.start],
        log_probability: 0.0,
        score: 0.0,
        ended: false,
    }];
    for step in 1..=config.max_new {
        let mut next = Vec::new();
        for beam in beams {
            if beam.ended {
                next.push(beam);
                continue;
            }
            let out = logits(&beam.tokens)?;
            if out.shape() != [1, vocab] {
                return Err(Error::Shape);
            }
            let values = out.data();
            let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let log_sum = values
                .iter()
                .map(|&x| (f64::from(x) - f64::from(max)).exp())
                .sum::<f64>()
                .ln();
            let forced = config.force_eos_at_limit && step == config.max_new;
            let mut candidates: Vec<_> = values
                .iter()
                .enumerate()
                .filter_map(|(id, &x)| {
                    if blocked[id]
                        || (id == config.eos as usize && step < config.min_new)
                        || (forced && id != config.eos as usize)
                    {
                        None
                    } else {
                        // Preserve original model probability, even for forced EOS.
                        Some((id as u32, f64::from(x) - f64::from(max) - log_sum))
                    }
                })
                .collect();
            let order =
                |a: &(u32, f64), b: &(u32, f64)| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0));
            if candidates.len() > config.beam_width {
                candidates.select_nth_unstable_by(config.beam_width, order);
                candidates.truncate(config.beam_width);
            }
            for (id, logp) in candidates {
                let mut tokens = beam.tokens.clone();
                tokens.push(id);
                let log_probability = beam.log_probability + logp;
                let score = log_probability / (step as f64).powf(config.length_penalty);
                next.push(Hypothesis {
                    tokens,
                    log_probability,
                    score,
                    ended: id == config.eos,
                });
            }
        }
        if next.is_empty() {
            return Err(Error::Shape);
        }
        next.sort_by(rank);
        next.truncate(config.beam_width);
        beams = next;
        if beams.iter().all(|b| b.ended) {
            break;
        }
    }
    Ok(beams)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SearchConfig<'static> {
        SearchConfig {
            start: 3,
            eos: 0,
            beam_width: 1,
            min_new: 1,
            max_new: 4,
            length_penalty: 0.,
            suppress: &[3],
            force_eos_at_limit: false,
        }
    }

    fn row(p: &[f32]) -> Result<Tensor, Error> {
        Tensor::new(&[1, p.len()], p.iter().map(|x| x.ln()).collect())
    }

    fn fixture(prefix: &[u32]) -> Result<Tensor, Error> {
        match prefix {
            [3] => row(&[0.01, 0.55, 0.43, 0.01]),
            [3, 1] => row(&[0.4, 0.3, 0.29, 0.01]),
            [3, 2] => row(&[0.95, 0.02, 0.02, 0.01]),
            _ => row(&[0.9, 0.04, 0.05, 0.01]),
        }
    }

    #[test]
    fn greedy_vs_beam_known_probabilities() {
        let greedy = search(4, cfg(), fixture).unwrap();
        assert_eq!(greedy[0].tokens, [3, 1, 0]);
        assert!(greedy[0].ended);
        assert!((greedy[0].log_probability - 0.22_f64.ln()).abs() < 1e-6);
        let config = SearchConfig {
            beam_width: 2,
            ..cfg()
        };
        let beams = search(4, config, fixture).unwrap();
        assert_eq!(beams[0].tokens, [3, 2, 0]);
        assert!((beams[0].log_probability - 0.4085_f64.ln()).abs() < 1e-6);
        assert_eq!(beams, search(4, config, fixture).unwrap());
    }

    #[test]
    fn eos_minimum_forcing_and_length_score() {
        let c = SearchConfig {
            min_new: 2,
            max_new: 2,
            length_penalty: 1.,
            force_eos_at_limit: true,
            ..cfg()
        };
        let mut calls = 0;
        let best = search(4, c, |_| {
            calls += 1;
            row(&[0.6, 0.3, 0.09, 0.01])
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(best[0].tokens, [3, 1, 0]);
        assert!((best[0].score - 0.18_f64.ln() / 2.).abs() < 1e-6);
        let best = search(4, cfg(), |_| row(&[0.9, 0.04, 0.05, 0.01])).unwrap();
        assert_eq!(best[0].tokens, [3, 0]);
    }

    #[test]
    fn deterministic_ties_and_unfinished_limit() {
        let c = SearchConfig {
            max_new: 2,
            ..cfg()
        };
        let best = search(4, c, |_| row(&[0.1, 0.4, 0.4, 0.1])).unwrap();
        assert_eq!(best[0].tokens, [3, 1, 1]);
        assert!(!best[0].ended);
        let extreme = search(4, c, |_| {
            Tensor::new(&[1, 4], vec![-f32::MAX, f32::MAX, 0., 0.])
        })
        .unwrap();
        assert_eq!(extreme[0].tokens, [3, 1, 1]);
        assert!(extreme[0].score.is_finite());
    }

    #[test]
    fn validates_limits_shape_and_errors() {
        let invalid = SearchConfig {
            beam_width: 17,
            ..cfg()
        };
        assert_eq!(search(4, invalid, fixture), Err(Error::Limit));
        let invalid = SearchConfig {
            suppress: &[0],
            ..cfg()
        };
        assert_eq!(search(4, invalid, fixture), Err(Error::Shape));
        let invalid = SearchConfig {
            length_penalty: f64::NAN,
            ..cfg()
        };
        assert_eq!(search(4, invalid, fixture), Err(Error::NonFinite));
        assert_eq!(
            search(4, cfg(), |_| Tensor::zeros(&[1, 3])),
            Err(Error::Shape)
        );
        assert_eq!(search(4, cfg(), |_| Err(Error::Worker)), Err(Error::Worker));
        let invalid = SearchConfig {
            min_new: 2,
            suppress: &[1, 2, 3],
            ..cfg()
        };
        assert_eq!(search(4, invalid, fixture), Err(Error::Shape));
    }
}
