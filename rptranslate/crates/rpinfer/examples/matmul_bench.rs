//! Bounded release-mode benchmark; not a performance acceptance threshold.
use rpinfer::{
    matmul::{matmul, Kernel},
    tensor::Tensor,
};
use std::time::Instant;

fn median_ms(a: &Tensor, b: &Tensor, kernel: Kernel) -> f64 {
    let mut times = [0.0; 3];
    for time in &mut times {
        let start = Instant::now();
        let result = matmul(a, b, kernel).expect("valid benchmark dimensions");
        std::hint::black_box(&result);
        *time = start.elapsed().as_secs_f64() * 1000.0;
    }
    times.sort_by(f64::total_cmp);
    times[1]
}

fn main() {
    for n in [128, 256] {
        let a = Tensor::new(
            &[n, n],
            (0..n * n).map(|i| (i % 31) as f32 / 31.0).collect(),
        )
        .unwrap();
        let b = Tensor::new(
            &[n, n],
            (0..n * n).map(|i| (i % 17) as f32 / 17.0).collect(),
        )
        .unwrap();
        let reference = matmul(&a, &b, Kernel::Scalar).unwrap();
        let optimized = matmul(&a, &b, Kernel::Blocked).unwrap();
        assert_eq!(reference, optimized);
        let scalar = median_ms(&a, &b, Kernel::Scalar);
        let blocked = median_ms(&a, &b, Kernel::Blocked);
        println!(
            "matmul n={n} scalar_ms={scalar:.3} blocked_ms={blocked:.3} speedup={:.3}x exact=true",
            scalar / blocked
        );
    }
}
