use rpinfer::{
    search::{search, search_constrained, SearchConfig},
    tensor::{Error, Tensor},
};

fn config(beam_width: usize, max_new: usize) -> SearchConfig<'static> {
    SearchConfig {
        start: 0,
        eos: 1,
        beam_width,
        min_new: 1,
        max_new,
        length_penalty: 1.0,
        suppress: &[0, 5],
        force_eos_at_limit: true,
    }
}

fn logits(_: &[u32]) -> Result<Tensor, Error> {
    Tensor::new(&[1, 6], vec![-3.0, 5.0, 1.0, 2.0, 3.0, 9.0])
}

#[test]
fn eos_is_delayed_and_repeated_phrases_are_distinct() {
    let phrases = vec![vec![2, 3], vec![2, 3]];
    let out = search_constrained(6, config(1, 12), &phrases, logits).unwrap();
    assert_eq!(out[0].tokens, [0, 2, 3, 2, 3, 1]);
    assert!(out[0].ended);
    let max = 9.0_f64;
    let norm = [-3.0_f64, 5.0, 1.0, 2.0, 3.0, 9.0]
        .iter()
        .map(|x| (x - max).exp())
        .sum::<f64>()
        .ln();
    let expected = 2.0 * (1.0 + 2.0 - 2.0 * max - 2.0 * norm) + 5.0 - max - norm;
    assert!((out[0].log_probability - expected).abs() < 1e-10);
}

#[test]
fn natural_phrase_then_eos_without_extra_tokens() {
    let out = search_constrained(6, config(1, 10), &[vec![2, 3]], |p| {
        let mut data = vec![-4.0; 6];
        data[match p.len() {
            1 => 4,
            2 => 2,
            3 => 3,
            _ => 1,
        }] = 5.0;
        Tensor::new(&[1, 6], data)
    })
    .unwrap();
    assert_eq!(out[0].tokens, [0, 4, 2, 3, 1]);
}

#[test]
fn budget_reserves_phrase_and_eos() {
    let out = search_constrained(6, config(1, 4), &[vec![2, 3]], |_| {
        Tensor::new(&[1, 6], vec![-3.0, -2.0, -1.0, 0.0, 5.0, -4.0])
    })
    .unwrap();
    assert_eq!(out[0].tokens, [0, 4, 2, 3, 1]);
}

#[test]
fn invalid_phrases_reject_before_logits_and_empty_is_identical() {
    for phrase in [vec![], vec![0], vec![1], vec![5], vec![6]] {
        assert!(search_constrained(6, config(1, 5), &[phrase], |_| {
            panic!("invalid phrase reached model")
        })
        .is_err());
    }
    assert_eq!(
        search_constrained(6, config(1, 2), &[vec![2, 3]], logits),
        Err(Error::Limit)
    );
    assert_eq!(
        search(6, config(2, 5), logits).unwrap(),
        search_constrained(6, config(2, 5), &[], logits).unwrap()
    );
}
